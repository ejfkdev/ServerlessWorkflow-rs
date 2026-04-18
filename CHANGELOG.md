# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0-alpha7] - 2026-04-17

### Changed

- Improved public API encapsulation: internal modules (expression, json_pointer, json_schema, task_runner, tasks, utils) are now `pub(crate)`
- Added `EventSubscription` and `ErrorFields` re-exports to runtime crate
- `WorkflowError` now implements `Clone`
- Removed macOS-incompatible mold linker configuration
- Fixed builder API: `OIDC` variant renamed to `Oidc` per Rust naming conventions
- Replaced `ErrorType` enum with newtype struct for simpler serialization
- Added doc comments to all builder public API items
- Added code examples to `CallHandler`, `RunHandler`, `CustomTaskHandler` trait docs
- Fixed `README.md` custom handler registration example
- Various clippy warning fixes and code quality improvements

## [1.0.0-alpha6.3] - 2025-04-13

### Added

- Rust SDK for the Serverless Workflow DSL specification v1.0.0
- `serverless_workflow_core` — Data models, serialization (JSON/YAML), and validation
- `serverless_workflow_builders` — Fluent builder API for constructing workflow definitions
- `serverless_workflow_runtime` — Workflow execution engine
- All 12 DSL task types: set, do, for, fork, switch, try, raise, wait, call, run, emit, listen, custom
- JQ-based expression engine with built-in variables ($context, $secret, $workflow, $task, $runtime)
- HTTP call support (GET/POST/PUT/DELETE/HEAD/PATCH/OPTIONS) with headers, query, body
- Shell execution with arguments, environment, and return modes
- Authentication: Basic, Bearer, Digest, OAuth2, OIDC
- Secret manager trait with MapSecretManager and EnvSecretManager implementations
- Execution listener (WorkflowEvent) for workflow/task lifecycle monitoring
- JSON Schema validation for workflow input/output
- Sub-workflow execution (run: workflow)
- Custom task types via CustomTaskHandler trait
- Custom call/run handlers via CallHandler and RunHandler traits
- Try/catch with retry: constant, linear, exponential backoff + jitter
- Export.as context passing ($context variable)
- Input.from/output.as expression transformation
- Schedule support (after/cron/every)
- Workflow-level and task-level timeouts
- Fork compete/non-compete mode with concurrent execution
- Full validation matching Go SDK struct tag rules
- 1754 tests across 7 test suites
- 3 example programs (hello_workflow, custom_handler, secret_workflow)
