# swf_runtime

Execution engine for [Serverless Workflow](https://serverlessworkflow.io/) DSL v1.0.0 — parse, validate, and run workflows.

## Features

- **Task runners** — set, wait, raise, emit, switch, for, fork, do, try, listen, call, run, custom
- **Expression engine** — JQ-based evaluation (jaq) with runtime variable support (`$context`, `$secret`, `$workflow`, `$task`, `$runtime`)
- **HTTP client** — GET/POST/PUT/DELETE/PATCH with headers, query, auth (Basic, Bearer, Digest, OAuth2, OIDC)
- **Shell execution** — command, args, environment, return formats
- **Sub-workflow** — run nested workflow definitions with context propagation
- **JSON Schema** — input/output validation
- **Secret manager** — pluggable secret resolution (`$secret` variable)
- **Execution listener** — event-driven workflow monitoring
- **Custom tasks** — extensible via `CustomTaskHandler` trait
