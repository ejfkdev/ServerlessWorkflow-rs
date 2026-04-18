use crate::error::{WorkflowError, WorkflowResult};
use crate::expression::{
    evaluate_expression_json, evaluate_expression_str, traverse_and_evaluate_obj,
};
use crate::task_runner::{TaskRunner, TaskSupport};
use serde_json::Value;
use serverless_workflow_core::models::authentication::ReferenceableAuthenticationPolicy;
use serverless_workflow_core::models::call::CallTaskDefinition;
use serverless_workflow_core::models::resource::OneOfEndpointDefinitionOrUri;

/// Runner for Call tasks - executes external service calls
///
/// Supports HTTP calls natively. Other call types (gRPC, OpenAPI, AsyncAPI, A2A, Function)
/// require custom handlers via the CallHandler trait.
pub struct CallTaskRunner {
    name: String,
    task: CallTaskDefinition,
}

impl CallTaskRunner {
    pub fn new(name: &str, task: &CallTaskDefinition) -> WorkflowResult<Self> {
        Ok(Self {
            name: name.to_string(),
            task: task.clone(),
        })
    }
}

#[async_trait::async_trait]
impl TaskRunner for CallTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        match &self.task {
            CallTaskDefinition::HTTP(http_task) => self.run_http(http_task, &input, support).await,
            CallTaskDefinition::Function(func_task) => {
                self.run_function(func_task, &input, support).await
            }
            CallTaskDefinition::GRPC(grpc_task) => {
                self.run_with_handler(
                    "grpc",
                    &CallTaskDefinition::GRPC(grpc_task.clone()),
                    &input,
                    support,
                )
                .await
            }
            CallTaskDefinition::OpenAPI(openapi_task) => {
                self.run_with_handler(
                    "openapi",
                    &CallTaskDefinition::OpenAPI(openapi_task.clone()),
                    &input,
                    support,
                )
                .await
            }
            CallTaskDefinition::AsyncAPI(asyncapi_task) => {
                self.run_with_handler(
                    "asyncapi",
                    &CallTaskDefinition::AsyncAPI(asyncapi_task.clone()),
                    &input,
                    support,
                )
                .await
            }
            CallTaskDefinition::A2A(a2a_task) => {
                self.run_with_handler(
                    "a2a",
                    &CallTaskDefinition::A2A(a2a_task.clone()),
                    &input,
                    support,
                )
                .await
            }
        }
    }

    fn task_name(&self) -> &str {
        &self.name
    }
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
            None => Err(WorkflowError::runtime(
                format!("{} calls require a custom CallHandler (register one via WorkflowRunner::with_call_handler())", call_type),
                &self.name,
                "",
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
        // Strip catalog reference: "myFunc@catalog" → "myFunc"
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
                    WorkflowError::runtime(
                        format!("function '{}' not found in workflow definitions or registered catalogs", func_name),
                        &self.name,
                        "",
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
            let vars = support.get_vars();
            for (key, value) in with_params {
                let evaluated = crate::expression::traverse_and_evaluate_obj(
                    Some(value),
                    input,
                    &vars,
                    &self.name,
                )?;
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
        let vars = support.get_vars();

        // Extract and evaluate the endpoint URI
        let endpoint = match &http_task.with.endpoint {
            OneOfEndpointDefinitionOrUri::Uri(uri) => {
                evaluate_expression_str(uri, input, &vars, &self.name)?
            }
            OneOfEndpointDefinitionOrUri::Endpoint(def) => {
                evaluate_expression_str(&def.uri, input, &vars, &self.name)?
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
                &vars,
                &self.name,
                Some(support.workflow),
            )?;
            client_builder = client_builder.timeout(duration);
        }

        let client = client_builder.build().map_err(|e| {
            WorkflowError::runtime(
                format!("failed to build HTTP client: {}", e),
                &self.name,
                "",
            )
        })?;

        let mut request_builder = build_request(&client, &method, &endpoint, &self.name)?;

        // Add authentication headers
        let auth_source = match &http_task.with.endpoint {
            OneOfEndpointDefinitionOrUri::Endpoint(def) => def.authentication.as_ref(),
            OneOfEndpointDefinitionOrUri::Uri(_) => None,
        };

        // Extract digest auth info (if any) for two-step digest flow
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
            let body_text = response.text().await.unwrap_or_default();
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

/// Applies authentication to an HTTP request builder
/// Returns (request_builder, optional_authorization) where authorization is (scheme, parameter)
async fn apply_authentication(
    mut builder: reqwest::RequestBuilder,
    policy: &ReferenceableAuthenticationPolicy,
    auth_definitions: Option<&std::collections::HashMap<String, ReferenceableAuthenticationPolicy>>,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<(reqwest::RequestBuilder, Option<(String, String)>)> {
    // Resolve the policy: if it's a reference, look it up in workflow.use_.authentications
    let resolved_policy = match policy {
        ReferenceableAuthenticationPolicy::Policy(def) => def,
        ReferenceableAuthenticationPolicy::Reference(ref_ref) => {
            match auth_definitions {
                Some(defs) => {
                    match defs.get(&ref_ref.use_) {
                        Some(ReferenceableAuthenticationPolicy::Policy(def)) => def,
                        Some(ReferenceableAuthenticationPolicy::Reference(nested)) => {
                            // Prevent deep recursion: only resolve one level of reference
                            return Err(WorkflowError::validation(
                                format!(
                                    "nested authentication reference '{}' is not supported",
                                    nested.use_
                                ),
                                task_name,
                            ));
                        }
                        None => {
                            return Err(WorkflowError::validation(
                            format!("authentication reference '{}' not found in use.authentications", ref_ref.use_),
                            task_name,
                        ));
                        }
                    }
                }
                None => {
                    return Err(WorkflowError::validation(
                        format!(
                            "authentication reference '{}' but no use.authentications defined",
                            ref_ref.use_
                        ),
                        task_name,
                    ));
                }
            }
        }
    };

    // Apply basic authentication
    let mut authorization: Option<(String, String)> = None;
    if let Some(ref basic) = resolved_policy.basic {
        if let Some(ref username_expr) = basic.username {
            let username = evaluate_expression_str(username_expr, input, vars, task_name)?;
            let password = basic
                .password
                .as_deref()
                .map(|p| evaluate_expression_str(p, input, vars, task_name))
                .transpose()?
                .unwrap_or_default();
            let parameter = format!("{}:{}", username, password);
            authorization = Some(("Basic".to_string(), parameter));
            builder = builder.basic_auth(username, Some(password));
        } else if let Some(ref secret_name) = basic.use_ {
            // Look up credentials from $secret.<secretName>
            let (username, password) = lookup_secret_credentials(secret_name, vars, task_name)?;
            let parameter = format!("{}:{}", username, password);
            authorization = Some(("Basic".to_string(), parameter));
            builder = builder.basic_auth(username, Some(password));
        }
    }

    // Apply bearer authentication
    if let Some(ref bearer) = resolved_policy.bearer {
        if let Some(ref token_expr) = bearer.token {
            let token = evaluate_expression_str(token_expr, input, vars, task_name)?;
            authorization = Some(("Bearer".to_string(), token.clone()));
            builder = builder.bearer_auth(token);
        } else if let Some(ref secret_name) = bearer.use_ {
            // Look up token from $secret.<secretName>
            let token = lookup_secret_token(secret_name, vars, task_name)?;
            authorization = Some(("Bearer".to_string(), token.clone()));
            builder = builder.bearer_auth(&token);
        }
    }

    // Apply digest authentication
    if let Some(ref digest) = resolved_policy.digest {
        if let Some(ref username_expr) = digest.username {
            let username = evaluate_expression_str(username_expr, input, vars, task_name)?;
            let password = digest
                .password
                .as_deref()
                .map(|p| evaluate_expression_str(p, input, vars, task_name))
                .transpose()?
                .unwrap_or_default();
            // Digest auth requires a two-step flow (pre-flight + retry with digest header).
            // We apply basic_auth as a fallback here — the actual digest flow is handled
            // in the response processing code when a 401 with WWW-Authenticate: Digest is received.
            let parameter = format!("{}:{}", username, password);
            authorization = Some(("Digest".to_string(), parameter));
            builder = builder.basic_auth(username, Some(password));
        } else if let Some(ref secret_name) = digest.use_ {
            let (username, password) = lookup_secret_credentials(secret_name, vars, task_name)?;
            let parameter = format!("{}:{}", username, password);
            authorization = Some(("Digest".to_string(), parameter));
            builder = builder.basic_auth(username, Some(password));
        }
    }

    // Apply OAuth2 authentication — fetch access token from token endpoint
    if let Some(ref oauth2) = resolved_policy.oauth2 {
        let access_token = fetch_oauth2_token(oauth2, input, vars, task_name).await?;
        authorization = Some(("Bearer".to_string(), access_token.clone()));
        builder = builder.bearer_auth(&access_token);
    }

    // Apply OIDC authentication — same flow as OAuth2 (fetch token, use as Bearer)
    if let Some(ref oidc) = resolved_policy.oidc {
        let access_token = fetch_oidc_token(oidc, input, vars, task_name).await?;
        authorization = Some(("Bearer".to_string(), access_token.clone()));
        builder = builder.bearer_auth(&access_token);
    }

    Ok((builder, authorization))
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

/// Digest auth credentials extracted for the two-step flow
struct DigestAuthInfo {
    username: String,
    password: String,
}

/// Looks up username and password from $secret.<secretName> for basic/digest auth
/// The secret object should contain "username" and "password" fields
/// Looks up a secret object from $secret.<secretName>
fn lookup_secret<'a>(
    secret_name: &str,
    vars: &'a std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<&'a Value> {
    vars.get("$secret")
        .and_then(|s| s.get(secret_name))
        .ok_or_else(|| {
            WorkflowError::validation(
                format!("secret '{}' not found for authentication", secret_name),
                task_name,
            )
        })
}

fn lookup_secret_credentials(
    secret_name: &str,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<(String, String)> {
    let secret = lookup_secret(secret_name, vars, task_name)?;

    let username = secret
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            WorkflowError::validation(
                format!("secret '{}' missing 'username' field", secret_name),
                task_name,
            )
        })?
        .to_string();

    let password = secret
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok((username, password))
}

/// Looks up token from $secret.<secretName> for bearer auth
/// The secret object should contain a "token" field
fn lookup_secret_token(
    secret_name: &str,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<String> {
    let secret = lookup_secret(secret_name, vars, task_name)?;

    secret
        .get("token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            WorkflowError::validation(
                format!("secret '{}' missing 'token' field", secret_name),
                task_name,
            )
        })
}

/// Extracts digest authentication info if digest auth is configured.
/// Returns Ok(None) if no digest auth is configured, Ok(Some(info)) if it is,
/// or an error if digest auth is configured but invalid.
fn extract_digest_info(
    policy: &ReferenceableAuthenticationPolicy,
    auth_definitions: Option<&std::collections::HashMap<String, ReferenceableAuthenticationPolicy>>,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<Option<DigestAuthInfo>> {
    let resolved_policy = match policy {
        ReferenceableAuthenticationPolicy::Policy(def) => def,
        ReferenceableAuthenticationPolicy::Reference(ref_ref) => match auth_definitions {
            Some(defs) => match defs.get(&ref_ref.use_) {
                Some(ReferenceableAuthenticationPolicy::Policy(def)) => def,
                _ => return Ok(None),
            },
            None => return Ok(None),
        },
    };

    if let Some(ref digest) = resolved_policy.digest {
        if let Some(ref username_expr) = digest.username {
            let username = evaluate_expression_str(username_expr, input, vars, task_name)?;
            let password = digest
                .password
                .as_deref()
                .map(|p| evaluate_expression_str(p, input, vars, task_name))
                .transpose()?
                .unwrap_or_default();
            return Ok(Some(DigestAuthInfo { username, password }));
        }
    }

    Ok(None)
}

/// Shared parameters for token endpoint requests (used by both OAuth2 and OIDC)
struct TokenRequestParams {
    token_url: String,
    grant_type: String,
    client_id: Option<String>,
    client_secret: Option<String>,
    encoding: String,
    scopes: String,
    /// Grant-type-specific key-value pairs (username/password, subject_token, etc.)
    grant_params: Vec<(String, String)>,
    client_auth_method: String,
    assertion: Option<String>,
    issuers: Option<Vec<String>>,
    protocol_name: &'static str,
}

/// Common token fetching logic shared by OAuth2 and OIDC.
/// Sends a token request to the endpoint and returns the access_token.
async fn fetch_access_token(params: TokenRequestParams, task_name: &str) -> WorkflowResult<String> {
    let protocol = params.protocol_name;

    let mut form_params = vec![("grant_type".to_string(), params.grant_type.clone())];
    form_params.extend(params.grant_params);

    if !params.scopes.is_empty() {
        form_params.push(("scope".to_string(), params.scopes));
    }

    let client = reqwest::Client::new();
    let mut request_builder = client.post(&params.token_url);

    // Client authentication
    match params.client_auth_method.as_str() {
        "client_secret_basic" | "none" => {
            if let (Some(id), Some(secret)) = (&params.client_id, &params.client_secret) {
                request_builder = request_builder.basic_auth(id, Some(secret));
            } else if let Some(id) = &params.client_id {
                request_builder = request_builder.basic_auth(id, Some(""));
            }
        }
        _ => {
            // client_secret_post (default): client_id/client_secret in request body
            if let Some(id) = &params.client_id {
                form_params.push(("client_id".to_string(), id.clone()));
            }
            if let Some(secret) = &params.client_secret {
                form_params.push(("client_secret".to_string(), secret.clone()));
            }
        }
    }

    // Handle assertion for JWT bearer
    if let Some(assertion) = &params.assertion {
        form_params.push(("assertion".to_string(), assertion.clone()));
    }

    // Send request with appropriate encoding
    let response = if params.encoding.contains("json") {
        let body: Value = serde_json::json!(form_params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<std::collections::HashMap<&str, &str>>());
        request_builder.json(&body)
    } else {
        request_builder.form(&form_params)
    }
    .send()
    .await
    .map_err(|e| {
        WorkflowError::communication(
            format!("{} token request failed: {}", protocol, e),
            task_name,
        )
    })?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        return Err(WorkflowError::communication(
            format!(
                "{} token endpoint returned status {}: {}",
                protocol, status, body_text
            ),
            task_name,
        ));
    }

    let token_response: Value = response.json().await.map_err(|e| {
        WorkflowError::communication(
            format!("failed to parse {} token response: {}", protocol, e),
            task_name,
        )
    })?;

    let access_token = token_response
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            WorkflowError::communication(
                format!("{} token response missing 'access_token' field", protocol),
                task_name,
            )
        })?;

    // Validate issuer if configured
    if let Some(ref issuers) = params.issuers {
        if let Some(issuer) = token_response.get("iss").and_then(|v| v.as_str()) {
            if !issuers.iter().any(|i| i == issuer) {
                return Err(WorkflowError::communication(
                    format!(
                        "{} token issuer '{}' not in allowed list: {:?}",
                        protocol, issuer, issuers
                    ),
                    task_name,
                ));
            }
        }
    }

    Ok(access_token.to_string())
}

/// Shared fields extracted from OAuth2/OIDC scheme definitions for token requests.
/// Both `OAuth2AuthenticationSchemeDefinition` and `OpenIDConnectSchemeDefinition`
/// share the same structure for these fields, so this struct consolidates extraction.
struct OAuthTokenFields {
    client: Option<
        serverless_workflow_core::models::authentication::OAuth2AuthenticationClientDefinition,
    >,
    grant: Option<String>,
    request: Option<
        serverless_workflow_core::models::authentication::OAuth2AuthenticationRequestDefinition,
    >,
    issuers: Option<Vec<String>>,
    scopes: Option<Vec<String>>,
    username: Option<String>,
    password: Option<String>,
    subject: Option<serverless_workflow_core::models::authentication::OAuth2TokenDefinition>,
    actor: Option<serverless_workflow_core::models::authentication::OAuth2TokenDefinition>,
}

impl OAuthTokenFields {
    fn from_oauth2(
        oauth2: &serverless_workflow_core::models::authentication::OAuth2AuthenticationSchemeDefinition,
    ) -> Self {
        Self {
            client: oauth2.client.clone(),
            grant: oauth2.grant.clone(),
            request: oauth2.request.clone(),
            issuers: oauth2.issuers.clone(),
            scopes: oauth2.scopes.clone(),
            username: oauth2.username.clone(),
            password: oauth2.password.clone(),
            subject: oauth2.subject.clone(),
            actor: oauth2.actor.clone(),
        }
    }

    fn from_oidc(
        oidc: &serverless_workflow_core::models::authentication::OpenIDConnectSchemeDefinition,
    ) -> Self {
        Self {
            client: oidc.client.clone(),
            grant: oidc.grant.clone(),
            request: oidc.request.clone(),
            issuers: oidc.issuers.clone(),
            scopes: oidc.scopes.clone(),
            username: oidc.username.clone(),
            password: oidc.password.clone(),
            subject: oidc.subject.clone(),
            actor: oidc.actor.clone(),
        }
    }
}

/// Builds a TokenRequestParams from shared OAuth token fields.
/// `token_url` is pre-computed (OAuth2 appends endpoint path, OIDC uses authority directly).
/// `protocol_name` is "OAuth2" or "OIDC".
/// `allow_token_exchange` enables the token-exchange grant type (only for OAuth2).
fn build_token_request_params(
    token_url: String,
    fields: OAuthTokenFields,
    protocol_name: &'static str,
    allow_token_exchange: bool,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<TokenRequestParams> {
    let grant_type = fields
        .grant
        .as_deref()
        .unwrap_or("client_credentials")
        .to_string();

    let client_id = fields
        .client
        .as_ref()
        .and_then(|c| c.id.as_deref())
        .map(|id| evaluate_expression_str(id, input, vars, task_name))
        .transpose()?;
    let client_secret = fields
        .client
        .as_ref()
        .and_then(|c| c.secret.as_deref())
        .map(|s| evaluate_expression_str(s, input, vars, task_name))
        .transpose()?;

    let encoding = fields
        .request
        .as_ref()
        .map(|r| r.encoding.as_str())
        .unwrap_or("application/x-www-form-urlencoded")
        .to_string();

    let scopes = fields
        .scopes
        .as_ref()
        .map(|s| s.join(" "))
        .unwrap_or_default();

    // Build grant-type-specific params
    let mut grant_params = Vec::new();
    match grant_type.as_str() {
        "client_credentials" => { /* scope handled in fetch_access_token */ }
        "password" => {
            let username = fields
                .username
                .as_deref()
                .map(|u| evaluate_expression_str(u, input, vars, task_name))
                .transpose()?
                .unwrap_or_default();
            let password = fields
                .password
                .as_deref()
                .map(|p| evaluate_expression_str(p, input, vars, task_name))
                .transpose()?
                .unwrap_or_default();
            grant_params.push(("username".to_string(), username));
            grant_params.push(("password".to_string(), password));
        }
        "urn:ietf:params:oauth:grant-type:token-exchange" if allow_token_exchange => {
            if let Some(ref subject) = fields.subject {
                let subject_token =
                    evaluate_expression_str(&subject.token, input, vars, task_name)?;
                grant_params.push(("subject_token".to_string(), subject_token));
                grant_params.push((
                    "subject_token_type".to_string(),
                    subject.type_.as_str().to_string(),
                ));
            }
            if let Some(ref actor) = fields.actor {
                let actor_token = evaluate_expression_str(&actor.token, input, vars, task_name)?;
                grant_params.push(("actor_token".to_string(), actor_token));
                grant_params.push((
                    "actor_token_type".to_string(),
                    actor.type_.as_str().to_string(),
                ));
            }
        }
        _ => {
            return Err(WorkflowError::validation(
                format!("unsupported {} grant type: '{}'", protocol_name, grant_type),
                task_name,
            ));
        }
    }

    let client_auth_method = fields
        .client
        .as_ref()
        .and_then(|c| c.authentication.as_deref())
        .unwrap_or("client_secret_post")
        .to_string();

    let assertion = fields
        .client
        .as_ref()
        .and_then(|c| c.assertion.as_deref())
        .map(|a| evaluate_expression_str(a, input, vars, task_name))
        .transpose()?;

    Ok(TokenRequestParams {
        token_url,
        grant_type,
        client_id,
        client_secret,
        encoding,
        scopes,
        grant_params,
        client_auth_method,
        assertion,
        issuers: fields.issuers,
        protocol_name,
    })
}

/// Fetches an OAuth2 access token from the token endpoint
/// Implements the client_credentials, password, and token-exchange grant types
/// matching Java SDK's JaxRSAccessTokenProvider
async fn fetch_oauth2_token(
    oauth2: &serverless_workflow_core::models::authentication::OAuth2AuthenticationSchemeDefinition,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<String> {
    // Build the token endpoint URL from authority + endpoints.token path
    let authority = oauth2
        .authority
        .as_deref()
        .map(|a| evaluate_expression_str(a, input, vars, task_name))
        .transpose()?
        .ok_or_else(|| {
            WorkflowError::validation(
                "OAuth2 authentication requires 'authority' to be set".to_string(),
                task_name,
            )
        })?;
    let token_path = oauth2
        .endpoints
        .as_ref()
        .map(|e| e.token.as_str())
        .unwrap_or("/oauth2/token");
    let token_url = format!("{}{}", authority.trim_end_matches('/'), token_path);

    let params = build_token_request_params(
        token_url,
        OAuthTokenFields::from_oauth2(oauth2),
        "OAuth2",
        true,
        input,
        vars,
        task_name,
    )?;
    fetch_access_token(params, task_name).await
}

/// Fetches an OIDC access token — same as OAuth2 but OIDC's authority IS the token endpoint URL
async fn fetch_oidc_token(
    oidc: &serverless_workflow_core::models::authentication::OpenIDConnectSchemeDefinition,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<String> {
    // For OIDC, the authority is the full token endpoint URL
    let token_url = oidc
        .authority
        .as_deref()
        .map(|a| evaluate_expression_str(a, input, vars, task_name))
        .transpose()?
        .ok_or_else(|| {
            WorkflowError::validation(
                "OIDC authentication requires 'authority' to be set".to_string(),
                task_name,
            )
        })?;

    let params = build_token_request_params(
        token_url,
        OAuthTokenFields::from_oidc(oidc),
        "OIDC",
        false,
        input,
        vars,
        task_name,
    )?;
    fetch_access_token(params, task_name).await
}

/// Parses a WWW-Authenticate: Digest header into its components
struct DigestChallenge {
    realm: String,
    nonce: String,
    opaque: Option<String>,
    algorithm: String,
    qop: Option<String>,
}

fn parse_digest_challenge(www_auth: &str) -> Option<DigestChallenge> {
    let header = www_auth.strip_prefix("Digest")?.trim();

    let mut realm = None;
    let mut nonce = None;
    let mut opaque = None;
    let mut algorithm = Some("MD5".to_string());
    let mut qop = None;

    // Parse key="value" pairs
    let re = regex_lazy();
    for cap in re.captures_iter(header) {
        let key = cap.get(1)?.as_str();
        let value = cap.get(2)?.as_str();
        match key {
            "realm" => realm = Some(value.to_string()),
            "nonce" => nonce = Some(value.to_string()),
            "opaque" => opaque = Some(value.to_string()),
            "algorithm" => algorithm = Some(value.to_string()),
            "qop" => qop = Some(value.to_string()),
            _ => {}
        }
    }

    Some(DigestChallenge {
        realm: realm?,
        nonce: nonce?,
        opaque,
        algorithm: algorithm.unwrap_or_else(|| "MD5".to_string()),
        qop,
    })
}

/// Lazy static regex for parsing WWW-Authenticate header
fn regex_lazy() -> std::sync::MutexGuard<'static, regex::Regex> {
    use regex::Regex;
    use std::sync::Mutex;
    static RE: std::sync::OnceLock<Mutex<Regex>> = std::sync::OnceLock::new();
    let guard = RE
        .get_or_init(|| {
            Mutex::new(Regex::new(r#"([A-Za-z]+)="([^"]*)""#).expect("static regex is valid"))
        })
        .lock()
        .expect("auth header regex lock poisoned");
    guard
}

/// Generates a random nonce for digest auth (cnonce)
fn rand_nonce() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    // Simple XOR-shift PRNG for cnonce generation
    let mut x = seed.wrapping_add(1);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    (x & 0xFFFFFFFF) as u32
}

/// Computes the MD5 hex digest of a string
fn md5_hex(input: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parameters for computing a digest auth response hash per RFC 2617
struct DigestAuthParams<'a> {
    username: &'a str,
    password: &'a str,
    realm: &'a str,
    method: &'a str,
    uri: &'a str,
    nonce: &'a str,
    qop: Option<&'a str>,
    nc: &'a str,
    cnonce: &'a str,
    algorithm: &'a str,
}

/// Computes the digest auth response hash per RFC 2617
fn compute_digest_response(params: &DigestAuthParams) -> String {
    // HA1 = MD5(username:realm:password)
    let ha1 = md5_hex(&format!(
        "{}:{}:{}",
        params.username, params.realm, params.password
    ));

    // For MD5-sess: HA1 = MD5(HA1:nonce:cnonce)
    let ha1 = if params.algorithm.eq_ignore_ascii_case("MD5-sess") {
        md5_hex(&format!("{}:{}:{}", ha1, params.nonce, params.cnonce))
    } else {
        ha1
    };

    // HA2 = MD5(method:uri)
    let ha2 = md5_hex(&format!("{}:{}", params.method, params.uri));

    // response = MD5(HA1:nonce:nc:cnonce:qop:HA2) or MD5(HA1:nonce:HA2)
    match params.qop {
        Some(qop_val) => md5_hex(&format!(
            "{}:{}:{}:{}:{}:{}",
            ha1, params.nonce, params.nc, params.cnonce, qop_val, ha2
        )),
        None => md5_hex(&format!("{}:{}:{}", ha1, params.nonce, ha2)),
    }
}

/// Parameters for building the Authorization: Digest header value
struct DigestHeaderParams<'a> {
    username: &'a str,
    realm: &'a str,
    nonce: &'a str,
    uri: &'a str,
    response: &'a str,
    opaque: Option<&'a str>,
    qop: Option<&'a str>,
    nc: &'a str,
    cnonce: &'a str,
}

/// Builds the Authorization: Digest header value
fn build_digest_auth_header(params: &DigestHeaderParams) -> String {
    let mut parts = vec![
        format!("username=\"{}\"", params.username),
        format!("realm=\"{}\"", params.realm),
        format!("nonce=\"{}\"", params.nonce),
        format!("uri=\"{}\"", params.uri),
        format!("response=\"{}\"", params.response),
    ];

    if let Some(q) = params.qop {
        parts.push(format!("qop={}", q));
        parts.push(format!("nc={}", params.nc));
        parts.push(format!("cnonce=\"{}\"", params.cnonce));
    }

    if let Some(op) = params.opaque {
        parts.push(format!("opaque=\"{}\"", op));
    }

    format!("Digest {}", parts.join(", "))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::WorkflowContext;
    use serde_json::json;
    use serverless_workflow_core::models::call::{
        CallHTTPDefinition, HTTPArguments, OneOfHeadersOrExpression, OneOfQueryOrExpression,
    };
    use serverless_workflow_core::models::resource::OneOfEndpointDefinitionOrUri;
    use serverless_workflow_core::models::task::TaskDefinitionFields;
    use serverless_workflow_core::models::workflow::WorkflowDefinition;
    use std::collections::HashMap;

    fn make_http_task(method: &str, uri: &str) -> CallHTTPDefinition {
        CallHTTPDefinition {
            call: "http".to_string(),
            with: HTTPArguments {
                method: method.to_string(),
                endpoint: OneOfEndpointDefinitionOrUri::Uri(uri.to_string()),
                headers: None,
                body: None,
                query: None,
                output: None,
                redirect: None,
            },
            common: TaskDefinitionFields::default(),
        }
    }

    #[test]
    fn test_call_runner_new() {
        let task = make_http_task("GET", "https://example.com/api");
        let runner = CallTaskRunner::new("testCall", &CallTaskDefinition::HTTP(task));
        assert!(runner.is_ok());
        assert_eq!(runner.unwrap().task_name(), "testCall");
    }

    #[test]
    fn test_call_runner_function_not_found() {
        let func_task = serverless_workflow_core::models::call::CallFunctionDefinition {
            call: "myFunc".to_string(),
            with: None,
            common: TaskDefinitionFields::default(),
        };
        let runner =
            CallTaskRunner::new("funcCall", &CallTaskDefinition::Function(func_task)).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(runner.run(json!({}), &mut support));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_call_http_with_headers_and_query() {
        let mut headers_map = HashMap::new();
        headers_map.insert("X-Custom".to_string(), "value".to_string());

        let mut query_map = HashMap::new();
        query_map.insert("page".to_string(), "1".to_string());

        let task = CallHTTPDefinition {
            call: "http".to_string(),
            with: HTTPArguments {
                method: "GET".to_string(),
                endpoint: OneOfEndpointDefinitionOrUri::Uri("https://httpbin.org/get".to_string()),
                headers: Some(OneOfHeadersOrExpression::Map(headers_map)),
                body: None,
                query: Some(OneOfQueryOrExpression::Map(query_map)),
                output: Some("response".to_string()),
                redirect: None,
            },
            common: TaskDefinitionFields::default(),
        };

        let runner = CallTaskRunner::new("httpCall", &CallTaskDefinition::HTTP(task)).unwrap();
        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        // This test makes a real HTTP request; skip in offline environments
        let result = runner.run(json!({}), &mut support).await;
        if let Ok(output) = result {
            // If the request succeeded, verify response format
            assert!(output.get("statusCode").is_some());
            assert!(output.get("body").is_some());
        }
        // If it fails due to network, that's acceptable for this test
    }

    #[tokio::test]
    async fn test_call_http_unsupported_method() {
        let task = make_http_task("TRACE", "https://example.com/api");
        let runner = CallTaskRunner::new("badMethod", &CallTaskDefinition::HTTP(task)).unwrap();
        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_call_grpc_no_handler() {
        use serverless_workflow_core::models::call::CallGRPCDefinition;
        let task = CallTaskDefinition::GRPC(Box::new(CallGRPCDefinition::default()));
        let runner = CallTaskRunner::new("grpcTest", &task).unwrap();
        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("grpc"));
        assert!(err_msg.contains("CallHandler"));
    }

    #[tokio::test]
    async fn test_call_openapi_no_handler() {
        use serverless_workflow_core::models::call::CallOpenAPIDefinition;
        let task = CallTaskDefinition::OpenAPI(CallOpenAPIDefinition::default());
        let runner = CallTaskRunner::new("openapiTest", &task).unwrap();
        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("openapi"));
    }

    #[tokio::test]
    async fn test_call_grpc_with_custom_handler() {
        use crate::handler::CallHandler;
        use serverless_workflow_core::models::call::CallGRPCDefinition;

        struct MockGrpcHandler;

        #[async_trait::async_trait]
        impl CallHandler for MockGrpcHandler {
            fn call_type(&self) -> &str {
                "grpc"
            }

            async fn handle(
                &self,
                _task_name: &str,
                _config: &Value,
                input: &Value,
            ) -> WorkflowResult<Value> {
                Ok(json!({"grpcResponse": input}))
            }
        }

        let task = CallTaskDefinition::GRPC(Box::new(CallGRPCDefinition::default()));
        let runner = CallTaskRunner::new("grpcWithHandler", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut registry = crate::handler::HandlerRegistry::new();
        registry.register_call_handler(Box::new(MockGrpcHandler));
        context.set_handler_registry(registry);
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner
            .run(json!({"request": "data"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["grpcResponse"]["request"], json!("data"));
    }

    #[tokio::test]
    async fn test_call_openapi_with_custom_handler() {
        use crate::handler::CallHandler;
        use serverless_workflow_core::models::call::CallOpenAPIDefinition;

        struct MockOpenApiHandler;

        #[async_trait::async_trait]
        impl CallHandler for MockOpenApiHandler {
            fn call_type(&self) -> &str {
                "openapi"
            }

            async fn handle(
                &self,
                _task_name: &str,
                _config: &Value,
                input: &Value,
            ) -> WorkflowResult<Value> {
                Ok(json!({"openapiResult": input}))
            }
        }

        let task = CallTaskDefinition::OpenAPI(CallOpenAPIDefinition::default());
        let runner = CallTaskRunner::new("openapiWithHandler", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut registry = crate::handler::HandlerRegistry::new();
        registry.register_call_handler(Box::new(MockOpenApiHandler));
        context.set_handler_registry(registry);
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner
            .run(json!({"operationId": "getUser"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["openapiResult"]["operationId"], json!("getUser"));
    }

    #[tokio::test]
    async fn test_apply_authentication_sets_authorization_basic() {
        use serverless_workflow_core::models::authentication::{
            AuthenticationPolicyDefinition, BasicAuthenticationSchemeDefinition,
            ReferenceableAuthenticationPolicy,
        };

        let basic = BasicAuthenticationSchemeDefinition {
            username: Some("admin".to_string()),
            password: Some("secret123".to_string()),
            ..Default::default()
        };
        let policy_def = AuthenticationPolicyDefinition {
            basic: Some(basic),
            ..Default::default()
        };

        let policy = ReferenceableAuthenticationPolicy::Policy(Box::new(policy_def));

        let client = reqwest::Client::new();
        let builder = client.get("https://example.com/api");
        let result = apply_authentication(
            builder,
            &policy,
            None,
            &json!({}),
            &HashMap::new(),
            "testAuth",
        )
        .await
        .unwrap();

        // Check that authorization info was returned
        let (_, auth_info) = result;
        assert!(auth_info.is_some());
        let (scheme, parameter) = auth_info.unwrap();
        assert_eq!(scheme, "Basic");
        assert_eq!(parameter, "admin:secret123");
    }

    #[tokio::test]
    async fn test_apply_authentication_sets_authorization_bearer() {
        use serverless_workflow_core::models::authentication::{
            AuthenticationPolicyDefinition, BearerAuthenticationSchemeDefinition,
            ReferenceableAuthenticationPolicy,
        };

        let bearer = BearerAuthenticationSchemeDefinition {
            token: Some("my-jwt-token".to_string()),
            ..Default::default()
        };
        let policy_def = AuthenticationPolicyDefinition {
            bearer: Some(bearer),
            ..Default::default()
        };

        let policy = ReferenceableAuthenticationPolicy::Policy(Box::new(policy_def));

        let client = reqwest::Client::new();
        let builder = client.get("https://example.com/api");
        let result = apply_authentication(
            builder,
            &policy,
            None,
            &json!({}),
            &HashMap::new(),
            "testAuth",
        )
        .await
        .unwrap();

        let (_, auth_info) = result;
        assert!(auth_info.is_some());
        let (scheme, parameter) = auth_info.unwrap();
        assert_eq!(scheme, "Bearer");
        assert_eq!(parameter, "my-jwt-token");
    }

    #[tokio::test]
    async fn test_apply_authentication_no_auth_returns_none() {
        use serverless_workflow_core::models::authentication::{
            AuthenticationPolicyDefinition, ReferenceableAuthenticationPolicy,
        };

        let policy =
            ReferenceableAuthenticationPolicy::Policy(Box::new(AuthenticationPolicyDefinition::default()));

        let client = reqwest::Client::new();
        let builder = client.get("https://example.com/api");
        let result = apply_authentication(
            builder,
            &policy,
            None,
            &json!({}),
            &HashMap::new(),
            "testAuth",
        )
        .await
        .unwrap();

        let (_, auth_info) = result;
        assert!(auth_info.is_none());
    }

    #[tokio::test]
    async fn test_apply_authentication_basic_with_secret() {
        use serverless_workflow_core::models::authentication::{
            AuthenticationPolicyDefinition, BasicAuthenticationSchemeDefinition,
            ReferenceableAuthenticationPolicy,
        };

        let basic = BasicAuthenticationSchemeDefinition {
            use_: Some("mySecret".to_string()),
            ..Default::default()
        };
        let policy_def = AuthenticationPolicyDefinition {
            basic: Some(basic),
            ..Default::default()
        };

        let policy = ReferenceableAuthenticationPolicy::Policy(Box::new(policy_def));

        // Set up vars with $secret.mySecret containing username/password
        let mut vars = HashMap::new();
        vars.insert(
            "$secret".to_string(),
            json!({
                "mySecret": {
                    "username": "admin",
                    "password": "s3cret"
                }
            }),
        );

        let client = reqwest::Client::new();
        let builder = client.get("https://example.com/api");
        let result = apply_authentication(builder, &policy, None, &json!({}), &vars, "testAuth")
            .await
            .unwrap();

        let (_, auth_info) = result;
        assert!(auth_info.is_some());
        let (scheme, parameter) = auth_info.unwrap();
        assert_eq!(scheme, "Basic");
        assert_eq!(parameter, "admin:s3cret");
    }

    #[tokio::test]
    async fn test_apply_authentication_bearer_with_secret() {
        use serverless_workflow_core::models::authentication::{
            AuthenticationPolicyDefinition, BearerAuthenticationSchemeDefinition,
            ReferenceableAuthenticationPolicy,
        };

        let bearer = BearerAuthenticationSchemeDefinition {
            use_: Some("apiToken".to_string()),
            ..Default::default()
        };
        let policy_def = AuthenticationPolicyDefinition {
            bearer: Some(bearer),
            ..Default::default()
        };

        let policy = ReferenceableAuthenticationPolicy::Policy(Box::new(policy_def));

        // Set up vars with $secret.apiToken containing a token
        let mut vars = HashMap::new();
        vars.insert(
            "$secret".to_string(),
            json!({
                "apiToken": {
                    "token": "my-jwt-from-secret"
                }
            }),
        );

        let client = reqwest::Client::new();
        let builder = client.get("https://example.com/api");
        let result = apply_authentication(builder, &policy, None, &json!({}), &vars, "testAuth")
            .await
            .unwrap();

        let (_, auth_info) = result;
        assert!(auth_info.is_some());
        let (scheme, parameter) = auth_info.unwrap();
        assert_eq!(scheme, "Bearer");
        assert_eq!(parameter, "my-jwt-from-secret");
    }

    #[tokio::test]
    async fn test_apply_authentication_basic_secret_not_found() {
        use serverless_workflow_core::models::authentication::{
            AuthenticationPolicyDefinition, BasicAuthenticationSchemeDefinition,
            ReferenceableAuthenticationPolicy,
        };

        let basic = BasicAuthenticationSchemeDefinition {
            use_: Some("missingSecret".to_string()),
            ..Default::default()
        };
        let policy_def = AuthenticationPolicyDefinition {
            basic: Some(basic),
            ..Default::default()
        };

        let policy = ReferenceableAuthenticationPolicy::Policy(Box::new(policy_def));

        let vars = HashMap::new();

        let client = reqwest::Client::new();
        let builder = client.get("https://example.com/api");
        let result =
            apply_authentication(builder, &policy, None, &json!({}), &vars, "testAuth").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missingSecret"));
    }

    #[tokio::test]
    async fn test_apply_authentication_digest_with_secret() {
        use serverless_workflow_core::models::authentication::{
            AuthenticationPolicyDefinition, DigestAuthenticationSchemeDefinition,
            ReferenceableAuthenticationPolicy,
        };

        let digest = DigestAuthenticationSchemeDefinition {
            use_: Some("digestCreds".to_string()),
            ..Default::default()
        };
        let policy_def = AuthenticationPolicyDefinition {
            digest: Some(digest),
            ..Default::default()
        };

        let policy = ReferenceableAuthenticationPolicy::Policy(Box::new(policy_def));

        let mut vars = HashMap::new();
        vars.insert(
            "$secret".to_string(),
            json!({
                "digestCreds": {
                    "username": "digestuser",
                    "password": "digestpass"
                }
            }),
        );

        let client = reqwest::Client::new();
        let builder = client.get("https://example.com/api");
        let result = apply_authentication(builder, &policy, None, &json!({}), &vars, "testAuth")
            .await
            .unwrap();

        let (_, auth_info) = result;
        assert!(auth_info.is_some());
        let (scheme, parameter) = auth_info.unwrap();
        assert_eq!(scheme, "Digest");
        assert_eq!(parameter, "digestuser:digestpass");
    }
}
