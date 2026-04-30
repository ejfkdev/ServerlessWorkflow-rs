//! Custom task handler example: extend the runtime with a custom task type.
//!
//! This example demonstrates how to register a custom task handler
//! that uppercases the "text" field from the workflow input.
//!
//! Run with: cargo run -p swf_runtime --example custom_handler

use async_trait::async_trait;
use serde_json::{json, Value};
use swf_core::models::task::{
    CustomTaskDefinition, TaskDefinition, TaskDefinitionFields,
};
use swf_core::models::workflow::{WorkflowDefinition, WorkflowDefinitionMetadata};
use swf_runtime::{CustomTaskHandler, WorkflowResult, WorkflowRunner};

/// A custom task handler that uppercases the "text" field from input
struct UppercaseHandler;

#[async_trait]
impl CustomTaskHandler for UppercaseHandler {
    fn task_type(&self) -> &str {
        "uppercase"
    }

    async fn handle(
        &self,
        _task_name: &str,
        _task_type: &str,
        _config: &Value,
        input: &Value,
    ) -> WorkflowResult<Value> {
        let text = input.get("text").and_then(|v| v.as_str()).unwrap_or("");
        Ok(json!({ "text": text.to_uppercase() }))
    }
}

#[tokio::main]
async fn main() {
    // Build a workflow with a custom task programmatically
    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default",
        "custom-handler-example",
        "1.0.0",
        None,
        None,
        None,
    ));

    let custom_task = CustomTaskDefinition {
        type_: Some("uppercase".to_string()),
        config: json!(null),
        common: TaskDefinitionFields::default(),
    };
    workflow
        .do_
        .add("uppercase".to_string(), TaskDefinition::Custom(custom_task));

    let runner = WorkflowRunner::new(workflow)
        .expect("failed to create runner")
        .with_custom_task_handler(Box::new(UppercaseHandler));

    let result = runner
        .run(json!({ "text": "hello world" }))
        .await
        .expect("workflow execution failed");

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
