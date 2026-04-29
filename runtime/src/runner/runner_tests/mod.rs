use super::*;
use base64::Engine;

mod call_function;
mod call_http;
mod control_flow;
mod do_task;
mod e2e;
mod emit;
mod export;
mod expression;
mod for_loop;
mod fork;
mod listen;
mod raise;
mod run_shell;
mod schedule;
mod set;
mod switch;
mod task;
mod try_catch;
mod wait;
mod workflow;

use crate::error::WorkflowError;
use crate::events::EventBus;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use warp::Filter;
use warp::Reply;

/// Helper to load a workflow from a YAML test file and run it
async fn run_workflow_from_yaml(yaml_path: &str, input: Value) -> WorkflowResult<Value> {
    let yaml_str = std::fs::read_to_string(yaml_path).map_err(|e| {
        WorkflowError::runtime(format!("failed to read '{}': {}", yaml_path, e), "/", "/")
    })?;
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).map_err(|e| {
        WorkflowError::runtime(format!("failed to parse '{}': {}", yaml_path, e), "/", "/")
    })?;
    let runner = WorkflowRunner::new(workflow)?;
    runner.run(input).await
}

/// Helper to load a workflow from a YAML test file with a secret manager and run it
async fn run_workflow_from_yaml_with_secrets(
    yaml_path: &str,
    input: Value,
    secret_manager: Arc<dyn SecretManager>,
) -> WorkflowResult<Value> {
    let yaml_str = std::fs::read_to_string(yaml_path).map_err(|e| {
        WorkflowError::runtime(format!("failed to read '{}': {}", yaml_path, e), "/", "/")
    })?;
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).map_err(|e| {
        WorkflowError::runtime(format!("failed to parse '{}': {}", yaml_path, e), "/", "/")
    })?;
    let runner = WorkflowRunner::new(workflow)?.with_secret_manager(secret_manager);
    runner.run(input).await
}

/// Helper to run a workflow from an inline YAML string with input
async fn run_workflow_yaml(yaml_str: &str, input: Value) -> WorkflowResult<Value> {
    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml_str)
        .map_err(|e| WorkflowError::runtime(format!("failed to parse YAML: {}", e), "/", "/"))?;
    let runner = WorkflowRunner::new(workflow)?;
    runner.run(input).await
}

fn testdata(filename: &str) -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or(".".to_string());
    Path::new(&manifest_dir)
        .join("testdata")
        .join(filename)
        .to_str()
        .unwrap()
        .to_string()
}

/// Helper to start a mock HTTP server on an ephemeral port and run a workflow
/// whose YAML references port `9876` as a placeholder.
async fn run_workflow_with_mock_server(
    yaml_file: &str,
    filter: impl warp::Filter<Extract = impl warp::Reply> + Clone + Send + Sync + 'static,
    input: Value,
) -> Value {
    let (addr, server_fn) = warp::serve(filter).bind_ephemeral(([127, 0, 0, 1], 0));
    let port = addr.port();
    tokio::spawn(server_fn);
    let yaml_str = std::fs::read_to_string(testdata(yaml_file)).unwrap();
    let yaml_str = yaml_str.replace("9876", &port.to_string());
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
    let runner = WorkflowRunner::new(workflow).unwrap();
    runner.run(input).await.unwrap()
}

/// Starts a mock HTTP server on an ephemeral port, returns the port number.
fn start_mock_server(
    filter: impl warp::Filter<Extract = impl warp::Reply> + Clone + Send + Sync + 'static,
) -> u16 {
    let (addr, server_fn) = warp::serve(filter).bind_ephemeral(([127, 0, 0, 1], 0));
    let port = addr.port();
    tokio::spawn(server_fn);
    port
}

/// Creates a Bearer-token-protected endpoint at `/protected`.
#[allow(dead_code)]
/// Returns JSON `{"data": "secret", "authenticated": true}` on valid token,
/// or 401 unauthorized on invalid/missing token.
fn bearer_protected_endpoint(
    expected_token: &str,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone + Send + Sync + 'static
{
    let token = expected_token.to_string();
    warp::path("protected")
        .and(warp::header::optional("Authorization"))
        .map(move |auth: Option<String>| {
            let token = token.clone();
            match auth {
                Some(val) if val == format!("Bearer {}", token) => {
                    warp::reply::json(&serde_json::json!({"data": "secret", "authenticated": true}))
                        .into_response()
                }
                _ => warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({"error": "unauthorized"})),
                    warp::http::StatusCode::UNAUTHORIZED,
                )
                .into_response(),
            }
        })
}

/// Creates a Basic-auth-protected endpoint at `/protected`.
#[allow(dead_code)]
/// Returns JSON `{"access": "granted"}` on valid credentials,
/// or `{"access": "denied"}` otherwise.
fn basic_auth_protected_endpoint(
    expected_b64: &str,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone + Send + Sync + 'static
{
    let expected = format!("Basic {}", expected_b64);
    warp::path("protected")
        .and(warp::header::optional("Authorization"))
        .map(move |auth: Option<String>| match auth {
            Some(val) if val == expected => {
                warp::reply::json(&serde_json::json!({"access": "granted"}))
            }
            _ => warp::reply::json(&serde_json::json!({"access": "denied"})),
        })
}

/// Creates an OAuth2 token endpoint at `/oauth2/token` that validates grant_type
#[allow(dead_code)]
/// and returns a JWT-like access token.
fn oauth2_token_endpoint(
    expected_grant_type: &str,
    access_token: &str,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone + Send + Sync + 'static
{
    let grant = expected_grant_type.to_string();
    let token = access_token.to_string();
    warp::path("oauth2")
        .and(warp::path("token"))
        .and(warp::post())
        .and(warp::body::form::<HashMap<String, String>>())
        .map(move |params: HashMap<String, String>| {
            let gt = params.get("grant_type").map(|s| s.as_str()).unwrap_or("");
            if gt == grant {
                warp::reply::json(&serde_json::json!({
                    "access_token": token,
                    "token_type": "Bearer",
                    "expires_in": 3600
                }))
            } else {
                warp::reply::json(&serde_json::json!({"error": "invalid_client"}))
            }
        })
}

// === Chained Set Tasks ===
