use crate::tasks::task_name_impl;
mod auth;
mod digest;
#[cfg(test)]
mod tests;

use crate::error::{WorkflowError, WorkflowResult};
use crate::expression::{
    evaluate_expression_json, evaluate_expression_str, traverse_and_evaluate_obj,
};
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::tasks::define_simple_task_runner;
use serde_json::Value;
use serverless_workflow_core::models::call::CallTaskDefinition;
use serverless_workflow_core::models::resource::OneOfEndpointDefinitionOrUri;

use auth::apply_authentication;
use digest::{extract_digest_info, parse_digest_challenge};

define_simple_task_runner!(
    /// Runner for Call tasks - executes external service calls
    ///
    /// Supports HTTP calls natively. Other call types (gRPC, OpenAPI, AsyncAPI, A2A, Function)
    /// require custom handlers via the CallHandler trait.
    CallTaskRunner, CallTaskDefinition
);

#[async_trait::async_trait]
impl TaskRunner for CallTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        match &self.task {
            CallTaskDefinition::HTTP(http_task) => self.run_http(http_task, &input, support).await,
            CallTaskDefinition::Function(func_task) => {
                self.run_function(func_task, &input, support).await
            }
            // GRPC, OpenAPI, AsyncAPI, A2A all use handler-based dispatch
            _ => {
                self.run_with_handler(
                    self.task.call_type_name(),
                    &self.task.clone(),
                    &input,
                    support,
                )
                .await
            }
        }
    }

    task_name_impl!();
}

impl CallTaskRunner {
    /// Runs a call task using a registered CallHandler, or returns an error if no handler is found
    async fn run_with_handler(
        &self,
        call_type: &str,
        task: &CallTaskDefinition,
        input: &Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        let handler = support.get_handler_registry().get_call_handler(call_type);
        match handler {
            Some(handler) => {
                let config = crate::error::serialize_to_value(task, "call config", &self.name)?;
                handler.handle(&self.name, &config, input).await
            }
            None => Err(WorkflowError::runtime_simple(
                format!("{} calls require a custom CallHandler (register one via WorkflowRunner::with_call_handler())", call_type),
                &self.name,
            )),
        }
    }

    /// Runs a function call by looking up the function definition
    ///
    /// Resolution order:
    /// 1. Registered functions in WorkflowContext (catalog mechanism via with_function())
    /// 2. Workflow's use_.functions definitions
    ///
    /// Supports `functionName@catalogName` syntax where the catalog name is ignored
    /// (the function name before '@' is used for lookup).
    async fn run_function(
        &self,
        func_task: &serverless_workflow_core::models::call::CallFunctionDefinition,
        input: &Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        let raw_name = &func_task.call;
        // Strip catalog reference: "myFunc@catalog" -> "myFunc"
        let func_name = raw_name.split('@').next().unwrap_or(raw_name);

        // 1. Look up in registered functions (catalog mechanism)
        let func_def = if let Some(registered) = support.context.get_function(func_name) {
            registered.clone()
        } else {
            // 2. Look up in workflow.use_.functions
            support
                .workflow
                .use_
                .as_ref()
                .and_then(|u| u.functions.as_ref())
                .and_then(|fns| fns.get(func_name))
                .ok_or_else(|| {
                    WorkflowError::runtime_simple(
                        format!("function '{}' not found in workflow definitions or registered catalogs", func_name),
                        &self.name,
                    )
                })?
                .clone()
        };

        // Merge function's `with` parameters as input context
        let func_input = if let Some(ref with_params) = func_task.with {
            let mut base = match input.as_object() {
                Some(_) => input.clone(),
                None => serde_json::json!({}),
            };
            for (key, value) in with_params {
                let evaluated = support.eval_obj(Some(value), input, &self.name)?;
                if let Some(obj) = base.as_object_mut() {
                    obj.insert(key.clone(), evaluated);
                }
            }
            base
        } else {
            input.clone()
        };

        // Create a runner for the resolved task definition and execute it
        let runner =
            crate::task_runner::create_task_runner(func_name, &func_def, support.workflow)?;
        runner.run(func_input, support).await
    }

    async fn run_http(
        &self,
        http_task: &serverless_workflow_core::models::call::CallHTTPDefinition,
        input: &Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        // Extract and evaluate the endpoint URI
        let endpoint = match &http_task.with.endpoint {
            OneOfEndpointDefinitionOrUri::Uri(uri) => support.eval_str(uri, input, &self.name)?,
            OneOfEndpointDefinitionOrUri::Endpoint(def) => {
                support.eval_str(&def.uri, input, &self.name)?
            }
        };

        let method = http_task.with.method.to_uppercase();

        // Build the HTTP client with timeout and redirect policy
        let mut client_builder = reqwest::ClientBuilder::new();

        // Configure redirect handling
        // redirect: false (default) means DO NOT follow redirects
        // redirect: true means follow redirects
        let redirect_policy = if http_task.with.redirect.unwrap_or(false) {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        };
        client_builder = client_builder.redirect(redirect_policy);

        // Configure timeout
        if let Some(ref timeout) = http_task.common.timeout {
            let duration = crate::utils::parse_duration_with_context(
                timeout,
                input,
                &support.get_vars(),
                &self.name,
                Some(support.workflow),
            )?;
            client_builder = client_builder.timeout(duration);
        }

        let client = client_builder.build().map_err(|e| {
            WorkflowError::runtime_simple(format!("failed to build HTTP client: {}", e), &self.name)
        })?;

        let mut request_builder = build_request(&client, &method, &endpoint, &self.name)?;

        // Add authentication headers
        let auth_source = match &http_task.with.endpoint {
            OneOfEndpointDefinitionOrUri::Endpoint(def) => def.authentication.as_ref(),
            OneOfEndpointDefinitionOrUri::Uri(_) => None,
        };

        // Extract digest auth info (if any) for two-step digest flow
        let vars = support.get_vars();
        let digest_info = auth_source.as_ref().and_then(|auth_policy| {
            let auth_definitions = support
                .workflow
                .use_
                .as_ref()
                .and_then(|u| u.authentications.as_ref());
            extract_digest_info(auth_policy, auth_definitions, input, &vars, &self.name)
                .ok()
                .flatten()
        });

        if let Some(auth_policy) = auth_source {
            let auth_definitions = support
                .workflow
                .use_
                .as_ref()
                .and_then(|u| u.authentications.as_ref());
            let (rb, auth_info) = apply_authentication(
                request_builder,
                auth_policy,
                auth_definitions,
                input,
                &vars,
                &self.name,
            )
            .await?;
            request_builder = rb;
            // Set $authorization variable for use in export.as expressions
            if let Some((scheme, parameter)) = auth_info {
                support.context.set_authorization(&scheme, &parameter);
            }
        }

        // Add headers, body, and query parameters
        request_builder = apply_request_options(
            request_builder,
            http_task.with.headers.as_ref(),
            http_task.with.body.as_ref(),
            http_task.with.query.as_ref(),
            input,
            &vars,
            &self.name,
        )?;

        // Execute the request
        let response = request_builder.send().await.map_err(|e| {
            WorkflowError::communication(format!("HTTP request failed: {}", e), &self.name)
        })?;

        // Handle 401 with digest auth challenge
        let status = response.status();
        let response = if status.as_u16() == 401 && digest_info.is_some() {
            // Check for WWW-Authenticate: Digest header
            let www_authenticate = response
                .headers()
                .get("www-authenticate")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if www_authenticate.starts_with("Digest") {
                if let (Some(challenge), Some(info)) = (
                    parse_digest_challenge(www_authenticate),
                    digest_info.as_ref(),
                ) {
                    use digest::{
                        build_digest_auth_header, compute_digest_response, rand_nonce,
                        DigestAuthParams, DigestHeaderParams,
                    };

                    let cnonce = format!("{:08x}", rand_nonce());
                    let nc = "00000001";
                    let qop = challenge.qop.as_deref();

                    let digest_response = compute_digest_response(&DigestAuthParams {
                        username: &info.username,
                        password: &info.password,
                        realm: &challenge.realm,
                        method: &method,
                        uri: &endpoint,
                        nonce: &challenge.nonce,
                        qop,
                        nc,
                        cnonce: &cnonce,
                        algorithm: &challenge.algorithm,
                    });

                    let auth_header = build_digest_auth_header(&DigestHeaderParams {
                        username: &info.username,
                        realm: &challenge.realm,
                        nonce: &challenge.nonce,
                        uri: &endpoint,
                        response: &digest_response,
                        opaque: challenge.opaque.as_deref(),
                        qop,
                        nc,
                        cnonce: &cnonce,
                    });

                    // Rebuild and retry the request with the digest auth header
                    let retry_builder = build_request(&client, &method, &endpoint, &self.name)?;
                    let retry_builder = retry_builder.header("Authorization", &auth_header);
                    let retry_builder = apply_request_options(
                        retry_builder,
                        http_task.with.headers.as_ref(),
                        http_task.with.body.as_ref(),
                        http_task.with.query.as_ref(),
                        input,
                        &vars,
                        &self.name,
                    )?;

                    retry_builder.send().await.map_err(|e| {
                        WorkflowError::communication(
                            format!("HTTP digest retry request failed: {}", e),
                            &self.name,
                        )
                    })?
                } else {
                    response
                }
            } else {
                response
            }
        } else {
            response
        };

        // Check for error status codes
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let status_code = status.as_u16();
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response body: {}>", e));
            return Err(WorkflowError::communication_with_status(
                format!(
                    "HTTP request returned error status {}: {}",
                    status_code, body_text
                ),
                &self.name,
                status_code,
            ));
        }

        // Process response based on output format
        let output_format = http_task.with.output.as_deref().unwrap_or("content");

        match output_format {
            "response" => {
                let status_code = status.as_u16();
                let headers_obj = extract_response_headers(&response);
                let body: Value = response.json().await.unwrap_or(Value::Null);
                Ok(serde_json::json!({
                    "statusCode": status_code,
                    "headers": headers_obj,
                    "body": body,
                }))
            }
            _ => {
                // "content" or "raw" - return just the body
                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");

                if content_type.contains("application/json") {
                    response.json().await.map_err(|e| {
                        WorkflowError::communication(
                            format!("failed to parse JSON response: {}", e),
                            &self.name,
                        )
                    })
                } else {
                    // For non-JSON responses, return as string
                    let text = response.text().await.map_err(|e| {
                        WorkflowError::communication(
                            format!("failed to read response body: {}", e),
                            &self.name,
                        )
                    })?;
                    // Try to parse as JSON first, fall back to string
                    Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
                }
            }
        }
    }
}

/// Builds an HTTP request builder for the given method and endpoint.
fn build_request(
    client: &reqwest::Client,
    method: &str,
    endpoint: &str,
    task_name: &str,
) -> WorkflowResult<reqwest::RequestBuilder> {
    match method {
        "GET" => Ok(client.get(endpoint)),
        "POST" => Ok(client.post(endpoint)),
        "PUT" => Ok(client.put(endpoint)),
        "DELETE" => Ok(client.delete(endpoint)),
        "PATCH" => Ok(client.patch(endpoint)),
        "HEAD" => Ok(client.head(endpoint)),
        "OPTIONS" => Ok(client.request(reqwest::Method::OPTIONS, endpoint)),
        _ => Err(WorkflowError::communication(
            format!("unsupported HTTP method: {}", method),
            task_name,
        )),
    }
}

/// Applies headers, body, and query parameters to an HTTP request builder.
/// Shared between the initial request and digest-retry request paths.
fn apply_request_options(
    mut builder: reqwest::RequestBuilder,
    headers: Option<&serverless_workflow_core::models::call::OneOfHeadersOrExpression>,
    body: Option<&Value>,
    query: Option<&serverless_workflow_core::models::call::OneOfQueryOrExpression>,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<reqwest::RequestBuilder> {
    // Add headers
    if let Some(headers) = headers {
        match headers {
            serverless_workflow_core::models::call::OneOfHeadersOrExpression::Map(map) => {
                for (key, value) in map {
                    let evaluated_value = evaluate_expression_str(value, input, vars, task_name)?;
                    builder = builder.header(key.as_str(), evaluated_value.as_str());
                }
            }
            serverless_workflow_core::models::call::OneOfHeadersOrExpression::Expression(expr) => {
                let headers_val = evaluate_expression_json(expr, input, vars, task_name)?;
                if let Some(obj) = headers_val.as_object() {
                    for (key, val) in obj {
                        if let Some(str_val) = val.as_str() {
                            builder = builder.header(key.as_str(), str_val);
                        }
                    }
                }
            }
        }
    }

    // Add body
    if let Some(body) = body {
        let evaluated_body = traverse_and_evaluate_obj(Some(body), input, vars, task_name)?;
        builder = builder.json(&evaluated_body);
    }

    // Add query parameters
    if let Some(query) = query {
        match query {
            serverless_workflow_core::models::call::OneOfQueryOrExpression::Map(map) => {
                for (key, value) in map {
                    let evaluated_value = evaluate_expression_str(value, input, vars, task_name)?;
                    builder = builder.query(&[(key.as_str(), evaluated_value.as_str())]);
                }
            }
            serverless_workflow_core::models::call::OneOfQueryOrExpression::Expression(expr) => {
                let query_val = evaluate_expression_json(expr, input, vars, task_name)?;
                if let Some(obj) = query_val.as_object() {
                    for (key, val) in obj {
                        let str_val = match val {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        builder = builder.query(&[(key.as_str(), str_val.as_str())]);
                    }
                }
            }
        }
    }

    Ok(builder)
}

/// Extracts response headers into a JSON object
fn extract_response_headers(response: &reqwest::Response) -> Value {
    let mut headers = serde_json::Map::new();
    for (key, value) in response.headers() {
        if let Ok(str_val) = value.to_str() {
            headers.insert(key.to_string(), Value::String(str_val.to_string()));
        }
    }
    Value::Object(headers)
}
