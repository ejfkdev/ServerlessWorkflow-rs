# ServerlessWorkflow-rs

[![CI](https://github.com/ejfdkev/ServerlessWorkflow-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ejfdkev/ServerlessWorkflow-rs/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Rust SDK for the [Serverless Workflow](https://serverlessworkflow.io/) DSL specification v1.0.0.

## Crates

| Crate | Description |
|-------|-------------|
| [`serverless_workflow_core`](./core) | Data models, serialization, and validation |
| [`serverless_workflow_builders`](./builders) | Fluent builder API for constructing workflow definitions |
| [`serverless_workflow_runtime`](./runtime) | Workflow execution engine |

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
serverless_workflow_core = "1.0.0-alpha7"
serverless_workflow_runtime = "1.0.0-alpha7"
```

### Parse and run a workflow from YAML

```rust,no_run
use serverless_workflow_core::models::workflow::WorkflowDefinition;
use serverless_workflow_runtime::WorkflowRunner;

let yaml = std::fs::read_to_string("workflow.yaml")?;
let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml)?;

let runner = WorkflowRunner::new(workflow)?;
let result = runner.run(serde_json::json!({})).await?;
println!("{}", serde_json::to_string_pretty(&result)?);
```

### Build a workflow programmatically

```rust,no_run
use serverless_workflow_builders::WorkflowBuilder;
use serde_json::json;
use std::collections::HashMap;

let workflow = WorkflowBuilder::new()
    .use_dsl("1.0.0")
    .with_namespace("default")
    .with_name("hello-workflow")
    .with_version("1.0.0")
    .do_("greet", |task| {
        task.set().variables(
            HashMap::from([("message".to_string(), json!("Hello, Serverless Workflow!"))])
        );
    })
    .build();
```

### Extend with custom task handlers

```rust,no_run
use async_trait::async_trait;
use serde_json::Value;
use serverless_workflow_runtime::{CustomTaskHandler, WorkflowError, WorkflowResult};

struct MyHandler;

#[async_trait]
impl CustomTaskHandler for MyHandler {
    fn task_type(&self) -> &str { "my-task" }

    async fn handle(
        &self,
        task_name: &str,
        task_type: &str,
        config: &Value,
        input: &Value,
    ) -> WorkflowResult<Value> {
        Ok(serde_json::json!({ "result": "custom output" }))
    }
}

// Register with: WorkflowRunner::new(workflow)?.with_custom_task_handler(Box::new(MyHandler))
```

## Features

### Task Types

All 12 DSL task types are supported:

| Task | Description |
|------|-------------|
| `set` | Set workflow data |
| `do` | Sequential task execution |
| `for` | Loop over collections |
| `fork` | Concurrent execution (compete/non-compete) |
| `switch` | Conditional branching |
| `try` | Error catching with retry/backoff |
| `raise` | Raise errors |
| `wait` | Delay execution |
| `call` | HTTP/gRPC/OpenAPI/AsyncAPI/Function calls |
| `run` | Shell/container/script/sub-workflow |
| `emit` | Emit CloudEvents |
| `listen` | Listen for events |
| `custom` | User-defined task types |

### Expression Engine

JQ-based expression evaluation with built-in variables:

- `$context` — workflow context (set via `export.as`)
- `$secret` — secret manager values
- `$workflow` — workflow metadata (`$workflow.definition.document.name`)
- `$task` — current task name
- `$runtime` — runtime info (`$runtime.version`)

### Authentication

- Basic, Bearer, Digest, OAuth2, OIDC
- Secret-based (`use:`) and inline credential support

### Validation

Full validation matching the official Go SDK's struct tag rules:
- Document name (hostname), version (semver), namespace (hostname)
- Mutual exclusivity checks (auth schemes, schedule modes, process types, backoff strategies)
- Format validation (URI, JSON Pointer, ISO 8601 duration, HTTP methods, pull policies)

## Running Tests

```bash
# All tests
cargo test --workspace

# Release mode
cargo test --workspace --release

# With clippy + format check
cargo clippy --workspace --all-targets && cargo fmt --check
```

## Project Structure

```
ServerlessWorkflow-rs/
├── core/           # Models, serialization, validation
├── builders/       # Fluent builder API
├── runtime/        # Execution engine
│   └── testdata/   # 131 YAML test fixtures
└── Cargo.toml      # Workspace configuration
```

## Comparison with Official SDKs

| Feature | Go SDK | Java SDK | Rust SDK |
|---------|--------|----------|----------|
| Data models | ✅ | ✅ | ✅ |
| Serialization (JSON/YAML) | ✅ | ✅ | ✅ |
| Validation | ✅ | ✅ | ✅ |
| Builder API | ✅ | ✅ | ✅ |
| Runtime execution | Partial | ✅ | ✅ |
| HTTP call | Stub | ✅ | ✅ |
| Shell execution | — | ✅ | ✅ |
| OAuth2/OIDC auth | — | ✅ | ✅ |
| Digest auth | — | ✅ | ✅ |
| Custom task types | ✅ | — | ✅ |
| Sub-workflow | — | ✅ | ✅ |
| Secret manager | — | ✅ | ✅ |
| JSON Schema validation | — | ✅ | ✅ |
| Execution listeners | — | ✅ | ✅ |

## License

Licensed under the [Apache License 2.0](./LICENSE).
