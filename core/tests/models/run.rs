use super::*;

#[test]
fn test_script_process_arguments_array_deserialization() {
    use serverless_workflow_core::models::task::ScriptProcessDefinition;

    let script_process_json = serde_json::json!({
        "language": "javascript",
        "code": "console.log('test')",
        "arguments": ["hello", "world"]
    });
    let result: Result<ScriptProcessDefinition, _> = serde_json::from_value(script_process_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize script with array arguments: {:?}",
        result.err()
    );

    let script = result.unwrap();
    assert_eq!(script.language, "javascript");
    assert!(script.arguments.is_some());

    let args = script.arguments.unwrap();
    assert_eq!(args.as_array().unwrap().len(), 2);
    assert_eq!(args.as_array().unwrap()[0], "hello");
    assert_eq!(args.as_array().unwrap()[1], "world");
}

#[test]
fn test_script_process_with_stdin_deserialization() {
    use serverless_workflow_core::models::task::ScriptProcessDefinition;

    let script_process_json = serde_json::json!({
        "language": "python",
        "code": "print('test')",
        "stdin": "Hello Workflow",
        "arguments": ["arg1"],
        "environment": {"FOO": "bar"}
    });
    let result: Result<ScriptProcessDefinition, _> = serde_json::from_value(script_process_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize script with stdin: {:?}",
        result.err()
    );

    let script = result.unwrap();
    assert_eq!(script.language, "python");
    assert_eq!(script.stdin, Some("Hello Workflow".to_string()));
    assert!(script.arguments.is_some());
    assert_eq!(
        script.arguments.as_ref().unwrap().as_array().unwrap().len(),
        1
    );
    assert!(script.environment.is_some());
    assert_eq!(
        script.environment.as_ref().unwrap().get("FOO"),
        Some(&"bar".to_string())
    );
}

#[test]
fn test_run_task_with_container_serialization() {
    // Test RunTaskDefinition with container (similar to Go SDK TestRunTask_MarshalJSON)
    let run_task_json = json!({
        "run": {
            "await": true,
            "container": {
                "image": "example-image",
                "command": "example-command",
                "environment": {"ENV_VAR": "value"}
            }
        }
    });
    let result: Result<RunTaskDefinition, _> = serde_json::from_value(run_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize run task with container: {:?}",
        result.err()
    );
    let run_task = result.unwrap();
    assert!(run_task.run.container.is_some());
}

#[test]
fn test_run_task_roundtrip_serialization() {
    // Test RunTaskDefinition roundtrip serialization
    let container = ContainerProcessDefinition {
        image: "example-image".to_string(),
        command: Some("example-command".to_string()),
        ..Default::default()
    };
    let run_task = RunTaskDefinition::new(ProcessTypeDefinition::using_container(
        container,
        Some(true),
    ));

    let json_str = serde_json::to_string(&run_task).expect("Failed to serialize run task");
    let deserialized: RunTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.run.container.is_some());
}

#[test]
fn test_run_task_with_script() {
    // Test RunTaskDefinition with script
    let run_task_json = json!({
        "run": {
            "await": true,
            "script": {
                "language": "python",
                "code": "print('Hello')"
            }
        }
    });
    let result: Result<RunTaskDefinition, _> = serde_json::from_value(run_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize run task with script: {:?}",
        result.err()
    );
    let run_task = result.unwrap();
    assert!(run_task.run.script.is_some());
}

#[test]
fn test_run_task_with_workflow() {
    // Test RunTaskDefinition with workflow
    let run_task_json = json!({
        "run": {
            "workflow": {
                "namespace": "default",
                "name": "my-workflow",
                "version": "1.0.0"
            }
        }
    });
    let result: Result<RunTaskDefinition, _> = serde_json::from_value(run_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize run task with workflow: {:?}",
        result.err()
    );
    let run_task = result.unwrap();
    assert!(run_task.run.workflow.is_some());
}

#[test]
fn test_run_task_definition() {
    use serverless_workflow_core::models::task::{
        ProcessTypeDefinition, RunTaskDefinition, WorkflowProcessDefinition,
    };
    let run_task = RunTaskDefinition {
        run: ProcessTypeDefinition {
            workflow: Some(WorkflowProcessDefinition::new(
                "default",
                "my-workflow",
                "1.0.0",
                None,
            )),
            ..Default::default()
        },
        ..Default::default()
    };
    let json_str = serde_json::to_string(&run_task).expect("Failed to serialize");
    assert!(json_str.contains("run"));
    assert!(json_str.contains("my-workflow"));
}

#[test]
fn test_run_task_roundtrip() {
    use serverless_workflow_core::models::task::{
        ProcessTypeDefinition, RunTaskDefinition, WorkflowProcessDefinition,
    };
    let run_task = RunTaskDefinition {
        run: ProcessTypeDefinition {
            workflow: Some(WorkflowProcessDefinition::new(
                "ns",
                "example-workflow",
                "2.0.0",
                None,
            )),
            ..Default::default()
        },
        ..Default::default()
    };
    let json_str = serde_json::to_string(&run_task).expect("Failed to serialize");
    assert!(json_str.contains("example-workflow"));
}

#[test]
fn test_run_task_workflow() {
    // Test workflow with run task for process execution
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "run-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "executeProcess": {
                    "run": {
                        "workflow": {
                            "namespace": "default",
                            "name": "sub-workflow",
                            "version": "1.0.0"
                        }
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with run: {:?}",
        result.err()
    );
}

#[test]
fn test_run_task_container() {
    // Test run task with container
    let run_json = json!({
        "run": {
            "container": {
                "image": "nginx:latest"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::RunTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"run": run_json["run"]}));
    assert!(
        result.is_ok(),
        "Failed to deserialize run container task: {:?}",
        result.err()
    );
}

#[test]
fn test_run_task_script() {
    // Test run task with script
    let run_json = json!({
        "run": {
            "script": {
                "language": "sh",
                "source": {
                    "endpoint": "echo hello"
                }
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::RunTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"run": run_json["run"]}));
    assert!(
        result.is_ok(),
        "Failed to deserialize run script task: {:?}",
        result.err()
    );
}

#[test]
fn test_run_task_shell() {
    // Test run task with shell
    let run_json = json!({
        "run": {
            "shell": {
                "command": "echo hello"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::RunTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"run": run_json["run"]}));
    assert!(
        result.is_ok(),
        "Failed to deserialize run shell task: {:?}",
        result.err()
    );
}

#[test]
fn test_container_process_with_pull_policy() {
    // Test container process with pull policy
    let container_json = json!({
        "image": "nginx:latest",
        "pullPolicy": "Always"
    });

    let result: Result<serverless_workflow_core::models::task::ContainerProcessDefinition, _> =
        serde_json::from_value(container_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize container with pull policy: {:?}",
        result.err()
    );
    let container = result.unwrap();
    assert!(container.pull_policy.is_some());
    assert_eq!(container.pull_policy.unwrap(), "Always");
}

#[test]
fn test_container_process_with_lifetime_eventually() {
    // Test container process with lifetime cleanup=eventually
    let container_json = json!({
        "image": "hello-world",
        "lifetime": {
            "cleanup": "eventually",
            "after": {
                "seconds": 20
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ContainerProcessDefinition, _> =
        serde_json::from_value(container_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize container with lifetime: {:?}",
        result.err()
    );
    let container = result.unwrap();
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.unwrap();
    assert_eq!(lifetime.cleanup, "eventually");
    assert!(lifetime.after.is_some());
}

#[test]
fn test_container_process_with_lifetime_always() {
    // Test container process with lifetime cleanup=always
    let container_json = json!({
        "image": "hello-world",
        "lifetime": {
            "cleanup": "always"
        }
    });

    let result: Result<serverless_workflow_core::models::task::ContainerProcessDefinition, _> =
        serde_json::from_value(container_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize container with lifetime: {:?}",
        result.err()
    );
    let container = result.unwrap();
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.unwrap();
    assert_eq!(lifetime.cleanup, "always");
    assert!(lifetime.after.is_none());
}

#[test]
fn test_container_process_with_lifetime_never() {
    // Test container process with lifetime cleanup=never
    let container_json = json!({
        "image": "hello-world",
        "lifetime": {
            "cleanup": "never"
        }
    });

    let result: Result<serverless_workflow_core::models::task::ContainerProcessDefinition, _> =
        serde_json::from_value(container_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize container with lifetime: {:?}",
        result.err()
    );
    let container = result.unwrap();
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.unwrap();
    assert_eq!(lifetime.cleanup, "never");
}

#[test]
fn test_container_process_with_all_fields() {
    // Test container process with all fields including lifetime and pullPolicy
    let container_json = json!({
        "image": "nginx:latest",
        "name": "my-container",
        "command": "/bin/sh",
        "ports": {"8080": 80},
        "volumes": {"/data": "/storage"},
        "environment": {"NODE_ENV": "production"},
        "stdin": "input data",
        "arguments": ["arg1", "arg2"],
        "pullPolicy": "IfNotPresent",
        "lifetime": {
            "cleanup": "eventually",
            "after": {
                "minutes": 30
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ContainerProcessDefinition, _> =
        serde_json::from_value(container_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize container with all fields: {:?}",
        result.err()
    );
    let container = result.unwrap();
    assert_eq!(container.image, "nginx:latest");
    assert_eq!(container.name, Some("my-container".to_string()));
    assert_eq!(container.command, Some("/bin/sh".to_string()));
    assert!(container.ports.is_some());
    assert!(container.volumes.is_some());
    assert!(container.environment.is_some());
    assert_eq!(container.stdin, Some("input data".to_string()));
    assert_eq!(
        container.arguments,
        Some(vec!["arg1".to_string(), "arg2".to_string()])
    );
    assert!(container.pull_policy.is_some());
    assert_eq!(container.pull_policy.unwrap(), "IfNotPresent");
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.unwrap();
    assert_eq!(lifetime.cleanup, "eventually");
    assert!(lifetime.after.is_some());
}

#[test]
fn test_container_cleanup_policy_constants() {
    assert_eq!(ContainerCleanupPolicy::ALWAYS, "always");
    assert_eq!(ContainerCleanupPolicy::EVENTUALLY, "eventually");
    assert_eq!(ContainerCleanupPolicy::NEVER, "never");
}

#[test]
fn test_process_return_type_constants() {
    assert_eq!(ProcessReturnType::STDOUT, "stdout");
    assert_eq!(ProcessReturnType::STDERR, "stderr");
    assert_eq!(ProcessReturnType::CODE, "code");
    assert_eq!(ProcessReturnType::ALL, "all");
    assert_eq!(ProcessReturnType::NONE, "none");
}

#[test]
fn test_run_task_return_field() {
    // Test that ProcessTypeDefinition's return field deserializes correctly
    let json_str = r#"{"shell": {"command": "ls"}, "return": "all"}"#;
    let process: ProcessTypeDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize ProcessTypeDefinition");
    assert_eq!(process.return_, Some("all".to_string()));
    assert!(process.shell.is_some());
}

#[test]
fn test_process_return_type_constants_values() {
    assert_eq!(ProcessReturnType::STDOUT, "stdout");
    assert_eq!(ProcessReturnType::STDERR, "stderr");
    assert_eq!(ProcessReturnType::CODE, "code");
    assert_eq!(ProcessReturnType::ALL, "all");
    assert_eq!(ProcessReturnType::NONE, "none");
}
