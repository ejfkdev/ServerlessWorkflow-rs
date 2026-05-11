//! Typed custom task handler example: use TypedCustomTaskHandler for type-safe config.
//!
//! This example demonstrates how to implement TypedCustomTaskHandler to receive
//! a strongly-typed config struct instead of raw JSON Value, eliminating manual parsing.
//!
//! Run with: cargo run -p swf-runtime --example typed_custom_handler

use serde::Deserialize;
use serde_json::{json, Value};
use swf_core::models::task::{CustomTaskDefinition, TaskDefinition, TaskDefinitionFields};
use swf_core::models::workflow::{WorkflowDefinition, WorkflowDefinitionMetadata};
use swf_runtime::{HandlerContext, TypedCustomTaskHandler, WorkflowResult, WorkflowRunner};

/// Strongly-typed config for the provider task — no more manual JSON parsing
#[derive(Deserialize)]
struct ProviderConfig {
    name: String,
    operation: String,
    #[serde(default)]
    timeout: Option<String>,
}

struct ProviderHandler;

#[async_trait::async_trait]
impl TypedCustomTaskHandler for ProviderHandler {
    type Config = ProviderConfig;

    fn task_type(&self) -> &str {
        "provider"
    }

    async fn handle(
        &self,
        _task_name: &str,
        config: &ProviderConfig,
        input: &Value,
        _context: &HandlerContext,
    ) -> WorkflowResult<Value> {
        // config.name, config.operation, config.timeout — all directly accessible
        let timeout = config.timeout.as_deref().unwrap_or("30s");
        Ok(json!({
            "provider": config.name,
            "operation": config.operation,
            "timeout": timeout,
            "input": input,
        }))
    }
}

#[tokio::main]
async fn main() {
    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default",
        "typed-handler-example",
        "1.0.0",
        None,
        None,
        None,
    ));

    let custom_task = CustomTaskDefinition {
        type_: Some("provider".to_string()),
        config: json!({
            "name": "my-service",
            "operation": "translate",
            "timeout": "60s"
        }),
        common: TaskDefinitionFields::default(),
    };
    workflow
        .do_
        .add("call_svc".to_string(), TaskDefinition::Custom(custom_task));

    // TypedCustomTaskHandler::into_boxed() converts to Box<dyn CustomTaskHandler>
    let runner = WorkflowRunner::new(workflow)
        .expect("failed to create runner")
        .with_custom_task_handler(ProviderHandler.into_boxed());

    let result = runner
        .run(json!({ "text": "hello" }))
        .await
        .expect("workflow execution failed");

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
