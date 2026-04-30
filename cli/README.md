# swf_cli

CLI tool for executing [Serverless Workflow](https://serverlessworkflow.io/) definitions.

## Installation

```bash
cargo install swf_cli
```

Or download a pre-built binary from [GitHub Releases](https://github.com/ejfkdev/ServerlessWorkflow-rs/releases).

## Usage

```bash
# Run a workflow
swf workflow.yaml

# With input data
swf workflow.yaml --input '{"key": "value"}'

# With input from file
swf workflow.yaml --input @input.json

# Verbose output (shows workflow events)
swf workflow.yaml --verbose

# Disable secret manager
swf workflow.yaml --no-secret

# Custom secret env prefix
swf workflow.yaml --secret-prefix MY_SECRET_

# Show version
swf --version
```

## Features

- Execute Serverless Workflow YAML/JSON definitions
- Expression evaluation (JQ-based runtime expressions)
- Secret management via environment variables
- Verbose event logging for debugging
- Sub-workflow auto-discovery from `./workflows/` directory

## License

Apache-2.0
