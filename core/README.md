# serverless_workflow_core

Core data models, serialization, and validation for the [Serverless Workflow](https://serverlessworkflow.io/) DSL specification v1.0.0.

This crate provides:

- **Models** — strongly-typed Rust structs for all workflow definition elements (tasks, calls, authentication, durations, errors, etc.)
- **Serialization** — full `serde` support for JSON and YAML, with custom deserializers for oneOf polymorphism and runtime expressions
- **Validation** — comprehensive validation rules matching the Go SDK's `validate:` struct tags (semver, hostname, URI, mutual exclusivity, etc.)
