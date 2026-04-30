#![deny(unsafe_code)]

use serde_json::Value;
use std::env;
use std::path::Path;
use std::process;
use std::sync::Arc;
use swf_core::models::workflow::WorkflowDefinition;
use swf_runtime::{
    EnvSecretManager, InMemoryEventBus, WorkflowEvent, WorkflowExecutionListener, WorkflowRunner,
};

/// Build-time version: prefers SWF_VERSION env var (set by CI from git tag), falls back to Cargo.toml version
const BUILD_VERSION: &str = match option_env!("SWF_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

/// Returns the version string, stripping leading 'v' if present (e.g., "v1.0.0" → "1.0.0")
fn version() -> &'static str {
    match BUILD_VERSION.strip_prefix('v') {
        Some(stripped) if !stripped.is_empty() => stripped,
        _ => BUILD_VERSION,
    }
}

fn print_usage() {
    eprintln!(
        "\
swf — Serverless Workflow CLI v{}

USAGE:
    swf <workflow.yaml> [OPTIONS]

ARGS:
    <workflow.yaml>       Path to workflow YAML definition file

OPTIONS:
    --input <json|@file>  Workflow input as JSON string or @path to read from file
    --secret-prefix <str> Environment variable prefix for secrets (default: WORKFLOW_SECRET_)
    --no-secret           Disable secret manager
    --verbose             Print workflow execution events to stderr
    --help                Print this help message
    --version             Print version

AUTO-DISCOVERY:
    Sub-workflows are auto-discovered from YAML files in the same directory
    as the main workflow, matched by namespace/name/version.
    An in-memory EventBus is always enabled for emit/listen task support.",
        version()
    );
}

fn parse_args() -> Result<CliArgs, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut cli = CliArgs::default();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            "--version" | "-V" => {
                println!("swf {}", version());
                process::exit(0);
            }
            "--input" => {
                i += 1;
                if i >= args.len() {
                    return Err("--input requires a value".to_string());
                }
                cli.input_str = Some(args[i].clone());
            }
            "--secret-prefix" => {
                i += 1;
                if i >= args.len() {
                    return Err("--secret-prefix requires a value".to_string());
                }
                cli.secret_prefix = Some(args[i].clone());
            }
            "--no-secret" => {
                cli.no_secret = true;
            }
            "--verbose" | "-v" => {
                cli.verbose = true;
            }
            s if s.starts_with('-') => {
                return Err(format!("unknown option: {s}"));
            }
            _ => {
                if cli.workflow_path.is_some() {
                    return Err(format!("unexpected argument: {}", args[i]));
                }
                cli.workflow_path = Some(args[i].clone());
            }
        }
        i += 1;
    }

    if cli.workflow_path.is_none() {
        return Err("missing workflow YAML file path".to_string());
    }

    Ok(cli)
}

#[derive(Default)]
struct CliArgs {
    workflow_path: Option<String>,
    input_str: Option<String>,
    secret_prefix: Option<String>,
    no_secret: bool,
    verbose: bool,
}

fn parse_input(input_str: &Option<String>) -> Result<Value, String> {
    match input_str {
        None => Ok(Value::Object(serde_json::Map::new())),
        Some(s) if s.starts_with('@') => {
            let path = &s[1..];
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("failed to read '{path}': {e}"))?;
            serde_json::from_str(&content).map_err(|e| format!("invalid JSON in '{path}': {e}"))
        }
        Some(s) => serde_json::from_str(s).map_err(|e| format!("invalid JSON input: {e}")),
    }
}

/// Discovers sub-workflow YAML files in the same directory as the main workflow.
/// Returns a list of parsed WorkflowDefinitions (excluding the main workflow itself).
fn discover_sub_workflows(
    main_path: &str,
    main_workflow: &WorkflowDefinition,
) -> Vec<WorkflowDefinition> {
    let main_file = Path::new(main_path);
    let dir = match main_file.parent() {
        Some(d) => d,
        None => return Vec::new(),
    };

    let main_key = format!(
        "{}/{}/{}",
        main_workflow.document.namespace,
        main_workflow.document.name,
        main_workflow.document.version
    );

    let mut sub_workflows = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Only process .yaml and .yml files
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "yaml" && ext != "yml" {
                continue;
            }
            // Skip the main workflow file itself
            if path == main_file {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(wf) = serde_yaml::from_str::<WorkflowDefinition>(&content) {
                    let key = format!(
                        "{}/{}/{}",
                        wf.document.namespace, wf.document.name, wf.document.version
                    );
                    // Don't register the same workflow as the main one
                    if key != main_key {
                        sub_workflows.push(wf);
                    }
                }
            }
        }
    }

    sub_workflows
}

/// Listener that prints events to stderr in verbose mode
struct VerboseListener;

impl WorkflowExecutionListener for VerboseListener {
    fn on_event(&self, event: &WorkflowEvent) {
        match event {
            WorkflowEvent::WorkflowStarted { instance_id, .. } => {
                eprintln!("[START] workflow instance: {instance_id}");
            }
            WorkflowEvent::WorkflowCompleted {
                instance_id,
                output,
            } => {
                eprintln!("[DONE]  workflow instance: {instance_id}");
                eprintln!(
                    "  output: {}",
                    serde_json::to_string(output).unwrap_or_default()
                );
            }
            WorkflowEvent::WorkflowFailed { instance_id, error } => {
                eprintln!("[FAIL]  workflow instance: {instance_id}");
                eprintln!("  error: {error}");
            }
            WorkflowEvent::TaskStarted { task_name, .. } => {
                eprintln!("  [>] {task_name}");
            }
            WorkflowEvent::TaskCompleted { task_name, .. } => {
                eprintln!("  [<] {task_name}");
            }
            WorkflowEvent::TaskFailed {
                task_name, error, ..
            } => {
                eprintln!("  [!] {task_name}: {error}");
            }
            WorkflowEvent::TaskRetried {
                task_name, attempt, ..
            } => {
                eprintln!("  [↻] {task_name} (attempt {attempt})");
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("run 'swf --help' for usage");
            process::exit(1);
        }
    };

    let workflow_path = cli.workflow_path.as_ref().unwrap();

    // Read and parse workflow YAML
    let yaml_str = match std::fs::read_to_string(workflow_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read '{}': {e}", workflow_path);
            process::exit(1);
        }
    };

    let workflow: WorkflowDefinition = match serde_yaml::from_str(&yaml_str) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: failed to parse '{}': {e}", workflow_path);
            process::exit(1);
        }
    };

    // Parse input
    let input = match parse_input(&cli.input_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    // Discover sub-workflows from same directory
    let sub_workflows = discover_sub_workflows(workflow_path, &workflow);

    // Build runner
    let mut runner = match WorkflowRunner::new(workflow) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: failed to create runner: {e}");
            process::exit(1);
        }
    };

    // Register sub-workflows
    for wf in sub_workflows {
        runner = runner.with_sub_workflow(wf);
    }

    // Always enable EventBus for emit/listen task support
    runner = runner.with_event_bus(Arc::new(InMemoryEventBus::new()));

    // Configure secret manager
    if !cli.no_secret {
        let prefix = cli.secret_prefix.as_deref().unwrap_or("WORKFLOW_SECRET_");
        runner = runner.with_secret_manager(Arc::new(EnvSecretManager::with_prefix(prefix)));
    }

    // Configure listener
    if cli.verbose {
        runner = runner.with_listener(Arc::new(VerboseListener));
    }

    // Run workflow
    match runner.run(input).await {
        Ok(output) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&output).unwrap_or_default()
            );
        }
        Err(e) => {
            eprintln!("error: workflow execution failed: {e}");
            process::exit(1);
        }
    }
}
