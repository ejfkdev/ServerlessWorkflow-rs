# serverless_workflow_builders

Fluent builder API for constructing [Serverless Workflow](https://serverlessworkflow.io/) definitions programmatically in Rust.

## Example

```rust,no_run
use serverless_workflow_builders::WorkflowBuilder;
use serde_json::json;
use std::collections::HashMap;

let workflow = WorkflowBuilder::new()
    .use_dsl("1.0.0")
    .with_namespace("default")
    .with_name("my-workflow")
    .with_version("1.0.0")
    .do_("greet", |task| {
        task.set().variables(
            HashMap::from([("message".to_string(), json!("Hello, Serverless Workflow!"))])
        );
    })
    .build();
```
