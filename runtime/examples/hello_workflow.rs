//! Minimal workflow example: build, run, and print the result.
//!
//! Run with: cargo run -p serverless_workflow_runtime --example hello_workflow

use serde_json::json;
use serverless_workflow_builders::WorkflowBuilder;
use serverless_workflow_runtime::WorkflowRunner;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let workflow = WorkflowBuilder::new()
        .use_dsl("1.0.0")
        .with_namespace("default")
        .with_name("hello-workflow")
        .with_version("1.0.0")
        .do_(
            "greet",
            |task: &mut serverless_workflow_builders::services::task::TaskDefinitionBuilder| {
                task.set().variables(HashMap::from([(
                    "message".to_string(),
                    json!("Hello, Serverless Workflow!"),
                )]));
            },
        )
        .build();

    let runner = WorkflowRunner::new(workflow).expect("failed to create runner");
    let result = runner
        .run(json!({}))
        .await
        .expect("workflow execution failed");

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
