//! Secret manager example: use MapSecretManager to provide secrets to expressions.
//!
//! Run with: cargo run -p swf_runtime --example secret_workflow

use serde_json::json;
use swf_core::models::workflow::WorkflowDefinition;
use swf_runtime::{MapSecretManager, WorkflowRunner};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: default
  name: secret-example
  version: '1.0.0'
do:
  - readSecret:
      set:
        user: ${ $secret.db.username }
"#;

    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).expect("failed to parse YAML");

    let secret_mgr = MapSecretManager::new()
        .with_secret("db", json!({ "username": "admin", "password": "s3cret" }));

    let runner = WorkflowRunner::new(workflow)
        .expect("failed to create runner")
        .with_secret_manager(Arc::new(secret_mgr));

    let result = runner
        .run(json!({}))
        .await
        .expect("workflow execution failed");

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
