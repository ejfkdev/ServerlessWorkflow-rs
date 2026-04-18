use super::*;

mod call_http;
mod call_function;
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

    use super::*;
    use crate::error::WorkflowError;
    use crate::events::EventBus;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::Path;

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
        let yaml_str = std::fs::read_to_string(&testdata(yaml_file)).unwrap();
        let yaml_str = yaml_str.replace("9876", &port.to_string());
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        runner.run(input).await.unwrap()
    }

    /// Helper to start a mock HTTP server and run a workflow, returning the Result
    /// (for tests that expect errors).
    async fn run_workflow_with_mock_server_result(
        yaml_file: &str,
        filter: impl warp::Filter<Extract = impl warp::Reply> + Clone + Send + Sync + 'static,
        input: Value,
    ) -> WorkflowResult<Value> {
        let (addr, server_fn) = warp::serve(filter).bind_ephemeral(([127, 0, 0, 1], 0));
        let port = addr.port();
        tokio::spawn(server_fn);
        let yaml_str = std::fs::read_to_string(&testdata(yaml_file)).unwrap();
        let yaml_str = yaml_str.replace("9876", &port.to_string());
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        runner.run(input).await
    }

    // === Chained Set Tasks ===
