# Serverless Workflow Rust SDK

[![CI](https://github.com/ejfkdev/ServerlessWorkflow-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ejfkdev/ServerlessWorkflow-rs/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Rust SDK for the [Serverless Workflow](https://serverlessworkflow.io/) DSL specification v1.0.0.

## Overview

This project provides a complete Rust implementation of the Serverless Workflow DSL, including data models, validation, a fluent builder API, and a fully functional workflow execution engine. It is designed for safety, performance, and ease of integration into Rust applications.

## Crates

| Crate | Version | Description |
|-------|---------|-------------|
| [`swf_core`](./core) | `1.0.0-alpha7` | Data models, serialization (JSON/YAML), and validation |
| [`swf_builders`](./builders) | `1.0.0-alpha7` | Fluent builder API for constructing workflow definitions |
| [`swf_runtime`](./runtime) | `1.0.0-alpha7` | Workflow execution engine with HTTP/shell/auth support |
| [`swf_cli`](./cli) | `1.0.0-alpha7` | CLI tool (`swf`) for executing workflow YAML files |

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
swf_core = "1.0.0-alpha7"
swf_runtime = "1.0.0-alpha7"
```

### Parse and run a workflow from YAML

```rust,no_run
use swf_core::models::workflow::WorkflowDefinition;
use swf_runtime::WorkflowRunner;

let yaml = std::fs::read_to_string("workflow.yaml")?;
let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml)?;

let runner = WorkflowRunner::new(workflow)?;
let result = runner.run(serde_json::json!({})).await?;
println!("{}", serde_json::to_string_pretty(&result)?);
```

### Build a workflow programmatically

```rust,no_run
use swf_builders::WorkflowBuilder;
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

### Use secrets in workflows

```rust,no_run
use swf_runtime::{WorkflowRunner, secret::MapSecretManager};
use serde_json::json;
use std::sync::Arc;

let secret_mgr = Arc::new(
    MapSecretManager::new()
        .with_secret("apiCreds", json!({ "username": "admin", "password": "s3cret" }))
);

let runner = WorkflowRunner::new(workflow)?
    .with_secret_manager(secret_mgr);

let result = runner.run(json!({})).await?;
```

### Extend with custom task handlers

```rust,no_run
use async_trait::async_trait;
use serde_json::Value;
use swf_runtime::{CustomTaskHandler, WorkflowError, WorkflowResult};

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

## CLI — `swf`

The `swf` command-line tool executes Serverless Workflow YAML definitions directly from the terminal.

### Install

Download a pre-built binary from [GitHub Releases](https://github.com/ejfkdev/ServerlessWorkflow-rs/releases) (8 platforms), or build from source:

```bash
cargo install swf_cli
```

### Usage

```
swf <workflow.yaml> [OPTIONS]

ARGS:
  <workflow.yaml>         Path to workflow YAML definition file

OPTIONS:
  --input <json|@file>    Workflow input as JSON string or @path to read from file
  --secret-prefix <str>   Environment variable prefix for secrets (default: WORKFLOW_SECRET_)
  --no-secret             Disable secret manager
  --verbose               Print workflow execution events to stderr
  --help                  Print help message
  --version               Print version
```

### Examples

Run a workflow with default empty input:

```bash
swf workflow.yaml
```

Pass JSON input directly:

```bash
swf workflow.yaml --input '{"name": "Alice"}'
```

Read input from a file:

```bash
swf workflow.yaml --input @input.json
```

Verbose mode with custom secret prefix:

```bash
swf workflow.yaml --verbose --secret-prefix MY_SECRET_
```

### Features

- **Sub-workflow auto-discovery** — YAML files in the same directory are automatically registered as sub-workflows, matched by `namespace/name/version`
- **EventBus** — always enabled for `emit`/`listen` task coordination
- **Secret manager** — reads secrets from environment variables with configurable prefix
- **Version from git tag** — binary version is set from the git tag during CI release builds

### Supported Platforms

| Platform | Target | Compressed |
|----------|--------|------------|
| macOS ARM64 | `aarch64-apple-darwin` | — |
| macOS x86_64 | `x86_64-apple-darwin` | — |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | UPX |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | UPX |
| Linux ARM64 (static) | `aarch64-unknown-linux-musl` | UPX |
| Linux x86_64 (static) | `x86_64-unknown-linux-musl` | UPX |
| Windows x86_64 | `x86_64-pc-windows-msvc` | UPX |
| Windows ARM64 | `aarch64-pc-windows-msvc` | UPX |

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
- `$authorization` — current task authentication info (scheme + parameter)
- `$secret` — secret manager values
- `$workflow` — workflow metadata (`$workflow.definition.document.name`)
- `$task` — current task name
- `$runtime` — runtime info (`$runtime.version`)

### Authentication

| Scheme | Description |
|--------|-------------|
| Basic | Username/password with secret reference support |
| Bearer | Token-based with expression or secret reference |
| Digest | Digest authentication with challenge-response |
| OAuth2 | Client credentials, password, token-exchange grant types |
| OIDC | OpenID Connect with same grant type support |

### Validation

Full validation matching the official Go SDK's struct tag rules:

- Document name (hostname), version (semver), namespace (hostname)
- Mutual exclusivity checks (auth schemes, schedule modes, process types, backoff strategies)
- Format validation (URI, JSON Pointer, ISO 8601 duration, HTTP methods, pull policies)

## Examples

| Example | Description |
|---------|-------------|
| [`hello_workflow`](./runtime/examples/hello_workflow.rs) | Parse and run a simple workflow from YAML |
| [`custom_handler`](./runtime/examples/custom_handler.rs) | Register a custom task type handler |
| [`secret_workflow`](./runtime/examples/secret_workflow.rs) | Use `MapSecretManager` for secrets in expressions |

Run an example:

```bash
cargo run -p swf_runtime --example hello_workflow
```

## Running Tests

```bash
# All tests
cargo test --workspace

# Release mode
cargo test --workspace --release

# Lint + format check
cargo clippy --workspace --all-targets && cargo fmt --check
```

## Project Structure

```
ServerlessWorkflow-rs/
├── core/               # Models, serialization, validation
│   └── src/
│       ├── models/     # All DSL data models (task, call, auth, etc.)
│       └── validation/ # Full DSL validation
├── builders/           # Fluent builder API
│   └── src/
│       └── services/   # Workflow, task, authentication builders
├── runtime/            # Execution engine
│   ├── src/
│   │   ├── tasks/      # Task runners (12 types + custom)
│   │   └── runner/     # WorkflowRunner, test suites
│   ├── examples/       # 3 example programs
│   └── testdata/       # 220 YAML test fixtures
├── cli/                # swf CLI binary
│   └── src/
│       └── main.rs     # CLI entry point
└── Cargo.toml          # Workspace configuration
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
| CLI tool | — | ✅ | ✅ |
| Secret manager | — | ✅ | ✅ |
| JSON Schema validation | — | ✅ | ✅ |
| Execution listeners | — | ✅ | ✅ |

## Minimum Supported Rust Version (MSRV)

Rust **1.80** or later.

## License

Licensed under the [Apache License 2.0](./LICENSE).

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development setup and guidelines.

## Security

To report a security vulnerability, please email [cncf.serverless.workflow@gmail.com](mailto:cncf.serverless.workflow@gmail.com) instead of filing a public issue.
