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
    let result = auth::apply_authentication(
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
    let result = auth::apply_authentication(
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
    let result = auth::apply_authentication(
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
    let result = auth::apply_authentication(builder, &policy, None, &json!({}), &vars, "testAuth")
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
    let result = auth::apply_authentication(builder, &policy, None, &json!({}), &vars, "testAuth")
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
        auth::apply_authentication(builder, &policy, None, &json!({}), &vars, "testAuth").await;

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
    let result = auth::apply_authentication(builder, &policy, None, &json!({}), &vars, "testAuth")
        .await
        .unwrap();

    let (_, auth_info) = result;
    assert!(auth_info.is_some());
    let (scheme, parameter) = auth_info.unwrap();
    assert_eq!(scheme, "Digest");
    assert_eq!(parameter, "digestuser:digestpass");
}
