use super::*;
use crate::models::duration::OneOfDurationOrIso8601Expression;
use crate::models::error::OneOfErrorDefinitionOrReference;
use crate::models::retry::OneOfRetryPolicyDefinitionOrReference;

// === Set Task Tests ===

#[test]
fn test_set_task_deserialize() {
    let json = r#"{
        "set": {"key1": "value1", "key2": 42}
    }"#;
    let task: SetTaskDefinition = serde_json::from_str(json).unwrap();
    match &task.set {
        SetValue::Map(map) => {
            assert_eq!(map.get("key1").unwrap(), "value1");
            assert_eq!(map.get("key2").unwrap(), 42);
        }
        _ => panic!("Expected Map variant"),
    }
}

#[test]
fn test_set_task_expression() {
    let json = r#"{"set": "${ .items }"}"#;
    let task: SetTaskDefinition = serde_json::from_str(json).unwrap();
    match &task.set {
        SetValue::Expression(expr) => assert_eq!(expr, "${ .items }"),
        _ => panic!("Expected Expression variant"),
    }
}

#[test]
fn test_set_task_with_common_fields() {
    let json = r#"{
        "set": {"result": "done"},
        "if": "${ .enabled }",
        "then": "continue",
        "output": {"as": {"transformed": true}}
    }"#;
    let task: SetTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_.as_deref(), Some("${ .enabled }"));
    assert_eq!(task.common.then.as_deref(), Some("continue"));
    assert!(task.common.output.is_some());
}

#[test]
fn test_set_task_roundtrip() {
    let json = r#"{"set": {"key1": "value1", "key2": 42}}"#;
    let task: SetTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: SetTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Do Task Tests ===
// Note: Map<String, T> serializes as array of single-key objects: [{"k": v}, ...]

#[test]
fn test_do_task_deserialize() {
    let json = r#"{
        "do": [
            {"step1": {"set": {"a": 1}}},
            {"step2": {"set": {"b": 2}}}
        ]
    }"#;
    let task: DoTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.do_.entries.len(), 2);
    assert!(task.do_.contains_key(&"step1".to_string()));
    assert!(task.do_.contains_key(&"step2".to_string()));
}

#[test]
fn test_do_task_roundtrip() {
    let json = r#"{"do": [{"step1": {"set": {"a": 1}}}]}"#;
    let task: DoTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: DoTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === For Task Tests ===

#[test]
fn test_for_task_deserialize() {
    let json = r#"{
        "for": {"each": "item", "in": "${ .items }", "at": "index"},
        "while": "${ .hasMore }",
        "do": [{"processItem": {"set": {"result": "done"}}}]
    }"#;
    let task: ForTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.for_.each, "item");
    assert_eq!(task.for_.in_, "${ .items }");
    assert_eq!(task.for_.at.as_deref(), Some("index"));
    assert_eq!(task.while_.as_deref(), Some("${ .hasMore }"));
    assert!(task.do_.contains_key(&"processItem".to_string()));
}

#[test]
fn test_for_task_minimal() {
    let json = r#"{
        "for": {"each": "item", "in": "${ .items }"},
        "do": [{"step": {"set": {"x": 1}}}]
    }"#;
    let task: ForTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.for_.each, "item");
    assert!(task.for_.at.is_none());
    assert!(task.while_.is_none());
}

#[test]
fn test_for_task_roundtrip() {
    let json = r#"{
        "for": {"each": "item", "in": "${ .items }", "at": "idx"},
        "do": [{"step": {"set": {"x": 1}}}]
    }"#;
    let task: ForTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: ForTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Fork Task Tests ===

#[test]
fn test_fork_task_deserialize() {
    let json = r#"{
        "fork": {
            "branches": [
                {"branch1": {"set": {"a": 1}}},
                {"branch2": {"set": {"b": 2}}}
            ],
            "compete": true
        }
    }"#;
    let task: ForkTaskDefinition = serde_json::from_str(json).unwrap();
    assert!(task.fork.compete);
    assert_eq!(task.fork.branches.entries.len(), 2);
}

#[test]
fn test_fork_task_no_compete() {
    let json = r#"{
        "fork": {
            "branches": [
                {"branch1": {"set": {"a": 1}}},
                {"branch2": {"set": {"b": 2}}}
            ],
            "compete": false
        }
    }"#;
    let task: ForkTaskDefinition = serde_json::from_str(json).unwrap();
    assert!(!task.fork.compete);
}

#[test]
fn test_fork_task_roundtrip() {
    let json = r#"{
        "fork": {
            "branches": [{"b1": {"set": {"x": 1}}}],
            "compete": true
        }
    }"#;
    let task: ForkTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: ForkTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Switch Task Tests ===

#[test]
fn test_switch_task_deserialize() {
    let json = r#"{
        "switch": [
            {"case1": {"when": ".color == \"red\"", "then": "setRed"}},
            {"case2": {"when": ".color == \"green\"", "then": "setGreen"}}
        ]
    }"#;
    let task: SwitchTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.switch.entries.len(), 2);
    let (name, case1) = &task.switch.entries[0];
    assert_eq!(name, "case1");
    assert_eq!(case1.when.as_deref(), Some(".color == \"red\""));
    assert_eq!(case1.then.as_deref(), Some("setRed"));
}

#[test]
fn test_switch_task_default_case() {
    let json = r#"{
        "switch": [
            {"red": {"when": ".color == \"red\"", "then": "setRed"}},
            {"fallback": {"then": "setDefault"}}
        ]
    }"#;
    let task: SwitchTaskDefinition = serde_json::from_str(json).unwrap();
    let (name, fallback) = &task.switch.entries[1];
    assert_eq!(name, "fallback");
    assert!(fallback.when.is_none());
    assert_eq!(fallback.then.as_deref(), Some("setDefault"));
}

#[test]
fn test_switch_task_roundtrip() {
    let json = r#"{
        "switch": [{"case1": {"when": ".x > 0", "then": "end"}}]
    }"#;
    let task: SwitchTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: SwitchTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Raise Task Tests ===

#[test]
fn test_raise_task_deserialize() {
    let json = r#"{
        "raise": {
            "error": {
                "type": "http://example.com/error",
                "status": 500,
                "title": "Internal Server Error",
                "detail": "An unexpected error occurred."
            }
        }
    }"#;
    let task: RaiseTaskDefinition = serde_json::from_str(json).unwrap();
    match &task.raise.error {
        crate::models::error::OneOfErrorDefinitionOrReference::Error(err) => {
            assert_eq!(err.type_.as_str(), "http://example.com/error");
            assert_eq!(err.status, serde_json::json!(500));
            assert_eq!(err.title, Some("Internal Server Error".to_string()));
            assert_eq!(
                err.detail,
                Some("An unexpected error occurred.".to_string())
            );
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_raise_task_with_reference() {
    let json = r#"{
        "raise": {
            "error": "authError"
        }
    }"#;
    let task: RaiseTaskDefinition = serde_json::from_str(json).unwrap();
    match &task.raise.error {
        crate::models::error::OneOfErrorDefinitionOrReference::Reference(name) => {
            assert_eq!(name, "authError");
        }
        _ => panic!("Expected Reference variant"),
    }
}

#[test]
fn test_raise_task_roundtrip() {
    let json = r#"{
        "raise": {
            "error": {
                "type": "http://example.com/error",
                "status": 500,
                "title": "Error"
            }
        }
    }"#;
    let task: RaiseTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: RaiseTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Run Task Tests ===

#[test]
fn test_run_task_shell_deserialize() {
    let json = r#"{
        "run": {
            "shell": {
                "command": "echo",
                "arguments": {"msg": "hello"},
                "environment": {"HOME": "/tmp"}
            },
            "await": false
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    assert!(task.run.shell.is_some());
    assert_eq!(task.run.await_, Some(false));
    let shell = task.run.shell.as_ref().unwrap();
    assert_eq!(shell.command, "echo");
    assert!(shell.arguments.is_some());
    assert!(shell.environment.is_some());
}

#[test]
fn test_run_task_container_deserialize() {
    let json = r#"{
        "run": {
            "container": {
                "image": "example-image",
                "name": "example-name",
                "command": "example-command",
                "environment": {"ENV_VAR": "value"},
                "arguments": ["arg1", "arg2"],
                "pullPolicy": "always",
                "lifetime": {
                    "cleanup": "eventually",
                    "after": {"seconds": 20}
                }
            },
            "await": true
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let container = task.run.container.as_ref().unwrap();
    assert_eq!(container.image, "example-image");
    assert_eq!(container.name.as_deref(), Some("example-name"));
    assert_eq!(container.command.as_deref(), Some("example-command"));
    assert_eq!(container.pull_policy.as_deref(), Some("always"));
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.as_ref().unwrap();
    assert_eq!(lifetime.cleanup, "eventually");
    assert!(lifetime.after.is_some());
}

#[test]
fn test_run_task_script_deserialize() {
    let json = r#"{
        "run": {
            "script": {
                "language": "python",
                "code": "print('Hello')",
                "arguments": ["arg1=value1"],
                "environment": {"ENV_VAR": "value"},
                "stdin": "example-input"
            },
            "await": true
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let script = task.run.script.as_ref().unwrap();
    assert_eq!(script.language, "python");
    assert_eq!(script.code.as_deref(), Some("print('Hello')"));
    assert_eq!(script.stdin.as_deref(), Some("example-input"));
}

#[test]
fn test_run_task_roundtrip() {
    let json = r#"{
        "run": {
            "shell": {"command": "ls"},
            "await": true
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: RunTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Wait Task Tests ===

#[test]
fn test_wait_task_iso8601_deserialize() {
    let json = r#"{"wait": "PT1S"}"#;
    let task: WaitTaskDefinition = serde_json::from_str(json).unwrap();
    match &task.wait {
        crate::models::duration::OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT1S");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_wait_task_inline_duration_deserialize() {
    let json = r#"{"wait": {"milliseconds": 25}}"#;
    let task: WaitTaskDefinition = serde_json::from_str(json).unwrap();
    match &task.wait {
        crate::models::duration::OneOfDurationOrIso8601Expression::Duration(d) => {
            assert_eq!(d.milliseconds, Some(25));
        }
        _ => panic!("Expected Duration variant"),
    }
}

#[test]
fn test_wait_task_with_common_fields() {
    let json = r#"{
        "wait": "PT1S",
        "if": "${ .shouldWait }",
        "then": "continue",
        "output": {"as": {"waited": true}}
    }"#;
    let task: WaitTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_.as_deref(), Some("${ .shouldWait }"));
    assert_eq!(task.common.then.as_deref(), Some("continue"));
    assert!(task.common.output.is_some());
}

#[test]
fn test_wait_task_roundtrip() {
    let json = r#"{"wait": "PT1S"}"#;
    let task: WaitTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: WaitTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Try Task Tests ===

#[test]
fn test_try_task_deserialize() {
    let json = r#"{
        "try": [{"doWork": {"set": {"x": 1}}}],
        "catch": {
            "errors": {"with": {"type": "https://example.com/errors/work"}},
            "as": "error",
            "do": [{"handleError": {"set": {"handled": true}}}]
        }
    }"#;
    let task: TryTaskDefinition = serde_json::from_str(json).unwrap();
    assert!(task.try_.contains_key(&"doWork".to_string()));
    assert!(task.catch.errors.is_some());
    assert_eq!(task.catch.as_.as_deref(), Some("error"));
    assert!(task.catch.do_.is_some());
}

#[test]
fn test_try_task_with_retry() {
    let json = r#"{
        "try": [{"doWork": {"set": {"x": 1}}}],
        "catch": {
            "retry": {
                "delay": {"seconds": 3},
                "limit": {"attempt": {"count": 5}}
            }
        }
    }"#;
    let task: TryTaskDefinition = serde_json::from_str(json).unwrap();
    assert!(task.catch.retry.is_some());
}

#[test]
fn test_try_task_with_when() {
    let json = r#"{
        "try": [{"doWork": {"set": {"x": 1}}}],
        "catch": {
            "when": "${ .error.status >= 500 }",
            "do": [{"handleError": {"set": {"handled": true}}}]
        }
    }"#;
    let task: TryTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(
        task.catch.when.as_deref(),
        Some("${ .error.status >= 500 }")
    );
}

#[test]
fn test_try_task_roundtrip() {
    let json = r#"{
        "try": [{"step": {"set": {"x": 1}}}],
        "catch": {
            "as": "err",
            "do": [{"handle": {"set": {"ok": true}}}]
        }
    }"#;
    let task: TryTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: TryTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === FlowDirective Tests ===

#[test]
fn test_flow_directive_enumerated() {
    let json = r#""continue""#;
    let fd: FlowDirectiveValue = serde_json::from_str(json).unwrap();
    assert!(fd.is_enumerated());
    assert!(!fd.is_termination());
}

#[test]
fn test_flow_directive_end() {
    let json = r#""end""#;
    let fd: FlowDirectiveValue = serde_json::from_str(json).unwrap();
    assert!(fd.is_enumerated());
    assert!(fd.is_termination());
}

#[test]
fn test_flow_directive_custom() {
    let json = r#""someTaskName""#;
    let fd: FlowDirectiveValue = serde_json::from_str(json).unwrap();
    assert!(!fd.is_enumerated());
    assert!(!fd.is_termination());
    match fd {
        FlowDirectiveValue::Custom(s) => assert_eq!(s, "someTaskName"),
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_flow_directive_roundtrip() {
    let fd = FlowDirectiveValue::Enumerated(FlowDirectiveType::Exit);
    let serialized = serde_json::to_string(&fd).unwrap();
    let deserialized: FlowDirectiveValue = serde_json::from_str(&serialized).unwrap();
    assert_eq!(fd, deserialized);
}

// === TaskDefinition Dispatch Tests ===

#[test]
fn test_task_definition_set_dispatch() {
    let json = r#"{"set": {"key": "value"}}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Set(_)));
}

#[test]
fn test_task_definition_do_dispatch() {
    let json = r#"{"do": [{"step": {"set": {"x": 1}}}]}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Do(_)));
}

#[test]
fn test_task_definition_for_dispatch() {
    let json = r#"{"for": {"each": "i", "in": ".items"}, "do": [{"step": {"set": {"x": 1}}}]}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::For(_)));
}

#[test]
fn test_task_definition_fork_dispatch() {
    let json = r#"{"fork": {"branches": [{"b": {"set": {"x": 1}}}], "compete": false}}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Fork(_)));
}

#[test]
fn test_task_definition_switch_dispatch() {
    let json = r#"{"switch": [{"c1": {"when": ".x > 0", "then": "end"}}]}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Switch(_)));
}

#[test]
fn test_task_definition_raise_dispatch() {
    let json = r#"{"raise": {"error": "someError"}}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Raise(_)));
}

#[test]
fn test_task_definition_wait_dispatch() {
    let json = r#"{"wait": "PT1S"}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Wait(_)));
}

#[test]
fn test_task_definition_try_dispatch() {
    let json = r#"{"try": [{"step": {"set": {"x": 1}}}], "catch": {}}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Try(_)));
}

#[test]
fn test_task_definition_run_dispatch() {
    let json = r#"{"run": {"shell": {"command": "ls"}}}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Run(_)));
}

#[test]
fn test_task_definition_call_dispatch() {
    let json =
        r#"{"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Call(_)));
}

#[test]
fn test_task_definition_emit_dispatch() {
    let json = r#"{"emit": {"event": {"type": "test.event"}}}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Emit(_)));
}

#[test]
fn test_task_definition_listen_dispatch() {
    let json = r#"{"listen": {"to": {"one": {"with": {"type": "test.event"}}}}}"#;
    let task: TaskDefinition = serde_json::from_str(json).unwrap();
    assert!(matches!(task, TaskDefinition::Listen(_)));
}

// === Emit Task Tests ===

#[test]
fn test_emit_task_deserialize() {
    let json = r#"{"emit": {"event": {"type": "test.event", "source": "test-source"}}}"#;
    let task: EmitTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.emit.event.type_, Some("test.event".to_string()));
}

#[test]
fn test_emit_task_roundtrip() {
    let json = r#"{"emit": {"event": {"type": "test.event"}}}"#;
    let task: EmitTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: EmitTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Listen Task Tests ===

#[test]
fn test_listen_task_deserialize() {
    let json = r#"{"listen": {"to": {"one": {"with": {"type": "test.event"}}}}}"#;
    let task: ListenTaskDefinition = serde_json::from_str(json).unwrap();
    assert!(task.listen.to.one.is_some());
    let one = task.listen.to.one.as_ref().unwrap();
    assert!(one.with.is_some());
}

#[test]
fn test_listen_task_roundtrip() {
    let json = r#"{"listen": {"to": {"one": {"with": {"type": "test.event"}}}}}"#;
    let task: ListenTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: ListenTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === ContainerProcess Tests ===

#[test]
fn test_container_process_deserialize() {
    let json = r#"{
        "image": "nginx:latest",
        "name": "web-server",
        "command": "nginx",
        "ports": {"80": 8080},
        "environment": {"HOST": "0.0.0.0"},
        "arguments": ["-g", "daemon off"],
        "pullPolicy": "always",
        "lifetime": {"cleanup": "eventually", "after": {"minutes": 30}}
    }"#;
    let container: ContainerProcessDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(container.image, "nginx:latest");
    assert_eq!(container.name.as_deref(), Some("web-server"));
    assert_eq!(container.pull_policy.as_deref(), Some("always"));
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.as_ref().unwrap();
    assert_eq!(lifetime.cleanup, "eventually");
}

#[test]
fn test_container_process_roundtrip() {
    let json = r#"{"image": "nginx", "name": "web"}"#;
    let container: ContainerProcessDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&container).unwrap();
    let deserialized: ContainerProcessDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(container, deserialized);
}

// === ShellProcess Tests ===

#[test]
fn test_shell_process_deserialize() {
    let json = r#"{
        "command": "echo",
        "arguments": {"msg": "hello"},
        "environment": {"HOME": "/tmp"}
    }"#;
    let shell: ShellProcessDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(shell.command, "echo");
    assert!(shell.arguments.is_some());
    // Arguments should be Map variant
    assert!(shell.arguments.as_ref().unwrap().is_map());
    let map = shell.arguments.as_ref().unwrap().as_map().unwrap();
    assert_eq!(map.get("msg").unwrap(), &Value::String("hello".to_string()));
    assert!(shell.environment.is_some());
}

#[test]
fn test_shell_process_array_args_deserialize() {
    // Matches Go SDK's Shell.Arguments supporting []string form
    let json = r#"{
        "command": "echo",
        "arguments": ["arg1=value1", "arg2=value2"]
    }"#;
    let shell: ShellProcessDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(shell.command, "echo");
    assert!(shell.arguments.is_some());
    // Arguments should be Array variant
    assert!(shell.arguments.as_ref().unwrap().is_array());
    let arr = shell.arguments.as_ref().unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0], "arg1=value1");
    assert_eq!(arr[1], "arg2=value2");
}

#[test]
fn test_shell_process_roundtrip() {
    let json = r#"{
        "command": "echo",
        "arguments": {"msg": "hello"},
        "environment": {"HOME": "/tmp"}
    }"#;
    let shell: ShellProcessDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&shell).unwrap();
    let deserialized: ShellProcessDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(shell, deserialized);
}

#[test]
fn test_shell_process_array_args_roundtrip() {
    let json = r#"{
        "command": "echo",
        "arguments": ["arg1=value1", "arg2"]
    }"#;
    let shell: ShellProcessDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&shell).unwrap();
    let deserialized: ShellProcessDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(shell, deserialized);
}

// === ErrorCatcher Tests ===

#[test]
fn test_error_catcher_deserialize() {
    let json = r#"{
        "errors": {"with": {"type": "https://example.com/errors/work", "status": 500}},
        "as": "err",
        "when": "${ .err.status >= 500 }",
        "exceptWhen": "${ .err.type == \"https://example.com/ignore\" }",
        "do": [{"handleError": {"set": {"handled": true}}}]
    }"#;
    let catcher: ErrorCatcherDefinition = serde_json::from_str(json).unwrap();
    assert!(catcher.errors.is_some());
    assert_eq!(catcher.as_.as_deref(), Some("err"));
    assert!(catcher.when.is_some());
    assert!(catcher.except_when.is_some());
    assert!(catcher.do_.is_some());
}

// === ErrorFilter Tests ===

#[test]
fn test_error_filter_deserialize() {
    let json = r#"{
        "with": {
            "type": "https://example.com/errors/auth",
            "status": 401,
            "title": "Unauthorized",
            "detail": "Token expired",
            "instance": "/auth/login"
        }
    }"#;
    let filter: ErrorFilterDefinition = serde_json::from_str(json).unwrap();
    let props = filter.with.as_ref().unwrap();
    assert_eq!(
        props.type_.as_deref(),
        Some("https://example.com/errors/auth")
    );
    assert_eq!(props.status, Some(serde_json::json!(401)));
    assert_eq!(props.title.as_deref(), Some("Unauthorized"));
}

// === SubscriptionIterator Tests ===

#[test]
fn test_subscription_iterator_deserialize() {
    let json = r#"{
        "item": "event",
        "at": "index",
        "do": [{"processEvent": {"set": {"processed": true}}}],
        "output": {"as": {"result": "ok"}}
    }"#;
    let iter: SubscriptionIteratorDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(iter.item.as_deref(), Some("event"));
    assert_eq!(iter.at.as_deref(), Some("index"));
    assert!(iter.do_.is_some());
    assert!(iter.output.is_some());
}

// === SetValue Tests ===

#[test]
fn test_set_value_map() {
    let json = r#"{"key1": "value1", "key2": 42}"#;
    let sv: SetValue = serde_json::from_str(json).unwrap();
    match sv {
        SetValue::Map(map) => {
            assert_eq!(map.len(), 2);
            assert_eq!(map.get("key1").unwrap(), "value1");
        }
        _ => panic!("Expected Map variant"),
    }
}

#[test]
fn test_set_value_expression() {
    let json = r#""${ .items }""#;
    let sv: SetValue = serde_json::from_str(json).unwrap();
    match sv {
        SetValue::Expression(expr) => assert_eq!(expr, "${ .items }"),
        _ => panic!("Expected Expression variant"),
    }
}

// === OneOfRunArguments Tests (matching Go SDK's RunArguments) ===

#[test]
fn test_run_arguments_array_deserialize() {
    // Matches Go SDK's TestRunTaskScriptArgArray_UnmarshalJSON
    let json = r#"{
        "run": {
            "await": true,
            "script": {
                "language": "python",
                "arguments": ["arg1=value1", "arg2"],
                "environment": {"ENV_VAR": "value"},
                "code": "print('Hello, World!')",
                "stdin": "example-input"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let script = task.run.script.as_ref().unwrap();
    assert_eq!(script.language, "python");
    assert!(script.arguments.is_some());
    let args = script.arguments.as_ref().unwrap();
    assert!(args.is_array());
    assert_eq!(args.as_array().unwrap(), &["arg1=value1", "arg2"]);
    assert_eq!(script.code.as_deref(), Some("print('Hello, World!')"));
}

#[test]
fn test_run_arguments_map_deserialize() {
    // Matches Go SDK's TestRunTaskScriptArgsMap_UnmarshalJSON
    let json = r#"{
        "run": {
            "await": true,
            "script": {
                "language": "python",
                "arguments": {"arg1": "value1"},
                "environment": {"ENV_VAR": "value"},
                "code": "print('Hello, World!')",
                "stdin": "example-input"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let script = task.run.script.as_ref().unwrap();
    assert_eq!(script.language, "python");
    assert!(script.arguments.is_some());
    let args = script.arguments.as_ref().unwrap();
    assert!(args.is_map());
    assert_eq!(
        args.as_map().unwrap().get("arg1").unwrap(),
        &serde_json::json!("value1")
    );
}

#[test]
fn test_run_arguments_array_roundtrip() {
    let json = r#"{
        "run": {
            "script": {
                "language": "python",
                "arguments": ["arg1=value1"],
                "code": "print('test')"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: RunTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

#[test]
fn test_run_arguments_map_roundtrip() {
    let json = r#"{
        "run": {
            "script": {
                "language": "python",
                "arguments": {"key1": "val1"},
                "code": "print('test')"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: RunTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

#[test]
fn test_run_arguments_no_arguments() {
    let json = r#"{
        "run": {
            "script": {
                "language": "bash",
                "code": "echo hello"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let script = task.run.script.as_ref().unwrap();
    assert!(script.arguments.is_none());
}

#[test]
fn test_run_arguments_default() {
    let args = OneOfRunArguments::default();
    assert!(args.is_array());
    assert_eq!(args.as_array().unwrap().len(), 0);
}

// === Tests matching Go SDK patterns ===

// Matches Go SDK's TestRetryPolicy_MarshalJSON (task_try_test.go)
#[test]
fn test_try_task_retry_with_iso8601_delay() {
    let json = r#"{
        "try": [{"doWork": {"set": {"x": 1}}}],
        "catch": {
            "retry": {
                "delay": "PT5S",
                "backoff": {"exponential": {"factor": 2}},
                "limit": {
                    "attempt": {"count": 3, "duration": "PT1M"},
                    "duration": "PT10M"
                },
                "jitter": {"from": {"seconds": 1}, "to": {"seconds": 3}}
            }
        }
    }"#;
    let task: TryTaskDefinition = serde_json::from_str(json).unwrap();
    let retry = task.catch.retry.as_ref().unwrap();
    match retry {
        OneOfRetryPolicyDefinitionOrReference::Retry(policy) => {
            assert!(policy.delay.is_some());
            match policy.delay.as_ref().unwrap() {
                OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
                    assert_eq!(expr, "PT5S");
                }
                _ => panic!("Expected Iso8601Expression for delay"),
            }
            assert!(policy.backoff.is_some());
            assert!(policy.limit.is_some());
            let limit = policy.limit.as_ref().unwrap();
            match limit.duration.as_ref().unwrap() {
                OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
                    assert_eq!(expr, "PT10M");
                }
                _ => panic!("Expected Iso8601Expression for limit duration"),
            }
        }
        _ => panic!("Expected Retry variant"),
    }
}

// Matches Go SDK's TestRetryPolicy_UnmarshalJSON_WithReference (task_try_test.go)
#[test]
fn test_try_task_retry_with_reference() {
    let json = r#"{
        "try": [{"doWork": {"set": {"x": 1}}}],
        "catch": {
            "retry": "defaultRetry"
        }
    }"#;
    let task: TryTaskDefinition = serde_json::from_str(json).unwrap();
    let retry = task.catch.retry.as_ref().unwrap();
    match retry {
        OneOfRetryPolicyDefinitionOrReference::Reference(name) => {
            assert_eq!(name, "defaultRetry");
        }
        _ => panic!("Expected Reference variant"),
    }
}

// Matches Go SDK's TestRetryPolicy_UnmarshalJSON_Inline (task_try_test.go)
#[test]
fn test_try_task_retry_inline() {
    let json = r#"{
        "try": [{"doWork": {"set": {"x": 1}}}],
        "catch": {
            "retry": {
                "delay": {"seconds": 3},
                "backoff": {"exponential": {}},
                "limit": {"attempt": {"count": 5}}
            }
        }
    }"#;
    let task: TryTaskDefinition = serde_json::from_str(json).unwrap();
    let retry = task.catch.retry.as_ref().unwrap();
    match retry {
        OneOfRetryPolicyDefinitionOrReference::Retry(policy) => {
            assert!(policy.delay.is_some());
            assert!(policy.backoff.is_some());
            assert!(policy.backoff.as_ref().unwrap().exponential.is_some());
            assert_eq!(
                policy
                    .limit
                    .as_ref()
                    .unwrap()
                    .attempt
                    .as_ref()
                    .unwrap()
                    .count,
                Some(5)
            );
        }
        _ => panic!("Expected Retry variant"),
    }
}

// TryTask with constant backoff retry
#[test]
fn test_try_task_retry_constant_backoff() {
    let json = r#"{
        "try": [{"doWork": {"set": {"x": 1}}}],
        "catch": {
            "retry": {
                "delay": {"seconds": 3},
                "backoff": {"constant": {"delay": "PT5S"}},
                "limit": {"attempt": {"count": 5}}
            }
        }
    }"#;
    let task: TryTaskDefinition = serde_json::from_str(json).unwrap();
    let retry = task.catch.retry.as_ref().unwrap();
    match retry {
        OneOfRetryPolicyDefinitionOrReference::Retry(policy) => {
            assert!(policy.backoff.as_ref().unwrap().constant.is_some());
            let constant = policy.backoff.as_ref().unwrap().constant.as_ref().unwrap();
            assert_eq!(constant.delay(), Some("PT5S"));
        }
        _ => panic!("Expected Retry variant"),
    }
}

// Matches Go SDK's TestSwitchTask_UnmarshalJSON - with common fields
#[test]
fn test_switch_task_with_common_fields() {
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}},
            {"case2": {"when": "${condition2}", "then": "end"}}
        ]
    }"#;
    let task: SwitchTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_.as_deref(), Some("${condition}"));
    assert!(task.common.input.is_some());
    assert!(task.common.output.is_some());
    assert!(task.common.timeout.is_some());
    assert_eq!(task.common.then.as_deref(), Some("continue"));
    assert_eq!(task.switch.entries.len(), 2);
    let (_, case1) = &task.switch.entries[0];
    assert_eq!(case1.when.as_deref(), Some("${condition1}"));
    assert_eq!(case1.then.as_deref(), Some("next"));
}

// Matches Go SDK's TestRaiseTask_UnmarshalJSON - with common fields
#[test]
fn test_raise_task_with_common_fields() {
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "raise": {
            "error": {
                "type": "http://example.com/error",
                "status": 500,
                "title": "Internal Server Error",
                "detail": "An unexpected error occurred."
            }
        }
    }"#;
    let task: RaiseTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_.as_deref(), Some("${condition}"));
    assert!(task.common.input.is_some());
    assert!(task.common.output.is_some());
    assert!(task.common.timeout.is_some());
    assert_eq!(task.common.then.as_deref(), Some("continue"));
    match &task.raise.error {
        crate::models::error::OneOfErrorDefinitionOrReference::Error(err) => {
            assert_eq!(err.type_.as_str(), "http://example.com/error");
            assert_eq!(err.status, serde_json::json!(500));
            assert_eq!(err.title, Some("Internal Server Error".to_string()));
            assert_eq!(
                err.detail,
                Some("An unexpected error occurred.".to_string())
            );
        }
        _ => panic!("Expected Error variant"),
    }
}

// Matches Go SDK's TestEmitTask_UnmarshalJSON - with full event properties
#[test]
fn test_emit_task_full_event_properties() {
    let json = r#"{
        "emit": {
            "event": {
                "id": "event-id",
                "source": "http://example.com/source",
                "type": "example.event.type",
                "time": "2023-01-01T00:00:00Z",
                "subject": "example.subject",
                "datacontenttype": "application/json",
                "dataschema": "http://example.com/schema",
                "with": {"extra": "value"}
            }
        }
    }"#;
    let task: EmitTaskDefinition = serde_json::from_str(json).unwrap();
    let event = &task.emit.event;
    assert_eq!(event.id.as_deref(), Some("event-id"));
    assert_eq!(event.source.as_deref(), Some("http://example.com/source"));
    assert_eq!(event.type_.as_deref(), Some("example.event.type"));
    assert_eq!(event.time.as_deref(), Some("2023-01-01T00:00:00Z"));
    assert_eq!(event.subject.as_deref(), Some("example.subject"));
    assert_eq!(event.data_content_type.as_deref(), Some("application/json"));
    assert_eq!(
        event.data_schema.as_deref(),
        Some("http://example.com/schema")
    );
    assert!(!event.with.is_empty());
    assert_eq!(event.with.get("extra").unwrap(), "value");
}

// Matches Go SDK's TestEmitTask_MarshalJSON - with common fields and full event
#[test]
fn test_emit_task_with_common_fields() {
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "emit": {
            "event": {
                "type": "example.event.type",
                "source": "http://example.com/source",
                "with": {"extra": "value"}
            }
        }
    }"#;
    let task: EmitTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_.as_deref(), Some("${condition}"));
    assert!(task.common.input.is_some());
    assert!(task.common.output.is_some());
    assert!(task.common.timeout.is_some());
    assert_eq!(task.common.then.as_deref(), Some("continue"));
    assert_eq!(task.emit.event.type_.as_deref(), Some("example.event.type"));
}

// Matches Go SDK's TestListenTask_MarshalJSON_WithUntilCondition
#[test]
fn test_listen_task_with_until_condition() {
    let json = r#"{
        "listen": {
            "to": {
                "any": [
                    {"with": {"type": "example.event.type", "source": "http://example.com/source"}}
                ],
                "until": "workflow.data.condition == true"
            }
        }
    }"#;
    let task: ListenTaskDefinition = serde_json::from_str(json).unwrap();
    assert!(task.listen.to.any.is_some());
    assert!(task.listen.to.until.is_some());
    let until = task.listen.to.until.as_ref().unwrap();
    match **until {
        crate::models::event::OneOfEventConsumptionStrategyDefinitionOrExpression::Expression(ref expr) => {
            assert_eq!(expr, "workflow.data.condition == true");
        }
        _ => panic!("Expected Expression variant for until"),
    }
}

// Listen task with until as boolean (disabled)
#[test]
fn test_listen_task_with_until_disabled() {
    let json = r#"{
        "listen": {
            "to": {
                "one": {"with": {"type": "example.event.type"}},
                "until": false
            }
        }
    }"#;
    let task: ListenTaskDefinition = serde_json::from_str(json).unwrap();
    let until = task.listen.to.until.as_ref().unwrap();
    match **until {
        crate::models::event::OneOfEventConsumptionStrategyDefinitionOrExpression::Bool(
            val,
        ) => {
            assert!(!val);
        }
        _ => panic!("Expected Bool variant for until"),
    }
}

// Listen task with common fields
#[test]
fn test_listen_task_with_common_fields() {
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "listen": {
            "to": {
                "one": {"with": {"type": "example.event.type", "source": "http://example.com/source"}}
            }
        }
    }"#;
    let task: ListenTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_.as_deref(), Some("${condition}"));
    assert!(task.common.input.is_some());
    assert!(task.common.output.is_some());
    assert!(task.common.timeout.is_some());
    assert_eq!(task.common.then.as_deref(), Some("continue"));
}

// ForTask with call HTTP sub-task (matches Go SDK's TestForTask_UnmarshalJSON)
#[test]
fn test_for_task_with_call_subtask() {
    let json = r#"{
        "for": {"each": "item", "in": "${items}", "at": "index"},
        "while": "${condition}",
        "do": [
            {"task1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}},
            {"task2": {"call": "openapi", "with": {"document": {"name": "doc1", "endpoint": "http://example.com/openapi.json"}, "operationId": "op1"}}}
        ]
    }"#;
    let task: ForTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.for_.each, "item");
    assert_eq!(task.for_.in_, "${items}");
    assert_eq!(task.for_.at.as_deref(), Some("index"));
    assert_eq!(task.while_.as_deref(), Some("${condition}"));
    assert_eq!(task.do_.entries.len(), 2);
    assert!(task.do_.contains_key(&"task1".to_string()));
    assert!(task.do_.contains_key(&"task2".to_string()));
}

// ForTask roundtrip with while (matches Go SDK's TestForTask_MarshalJSON)
#[test]
fn test_for_task_with_while_roundtrip() {
    let json = r#"{
        "for": {"each": "item", "in": "${items}", "at": "index"},
        "while": "${condition}",
        "do": [{"step": {"set": {"x": 1}}}]
    }"#;
    let task: ForTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: ForTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// DoTask with call HTTP sub-task (matches Go SDK's TestDoTask_UnmarshalJSON)
#[test]
fn test_do_task_with_call_subtask() {
    let json = r#"{
        "do": [
            {"task1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}},
            {"task2": {"call": "openapi", "with": {"document": {"name": "doc1", "endpoint": "http://example.com/openapi.json"}, "operationId": "op1"}}}
        ]
    }"#;
    let task: DoTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.do_.entries.len(), 2);
    assert!(task.do_.contains_key(&"task1".to_string()));
    assert!(task.do_.contains_key(&"task2".to_string()));
}

// ForkTask with call HTTP branches (matches Go SDK's TestForkTask_UnmarshalJSON)
#[test]
fn test_fork_task_with_call_branches() {
    let json = r#"{
        "fork": {
            "branches": [
                {"task1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}},
                {"task2": {"call": "openapi", "with": {"document": {"name": "doc1", "endpoint": "http://example.com/openapi.json"}, "operationId": "op1"}}}
            ],
            "compete": true
        }
    }"#;
    let task: ForkTaskDefinition = serde_json::from_str(json).unwrap();
    assert!(task.fork.compete);
    assert_eq!(task.fork.branches.entries.len(), 2);
    assert!(task.fork.branches.contains_key(&"task1".to_string()));
    assert!(task.fork.branches.contains_key(&"task2".to_string()));
}

// WaitTask with ISO8601 and common fields (matches Go SDK's TestWaitTask_MarshalJSON)
#[test]
fn test_wait_task_with_iso8601_and_common_fields() {
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "wait": "P1DT1H"
    }"#;
    let task: WaitTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_.as_deref(), Some("${condition}"));
    assert!(task.common.input.is_some());
    assert!(task.common.output.is_some());
    assert!(task.common.timeout.is_some());
    assert_eq!(task.common.then.as_deref(), Some("continue"));
    match &task.wait {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "P1DT1H");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

// SwitchTask roundtrip with common fields (matches Go SDK's TestSwitchTask_MarshalJSON)
#[test]
fn test_switch_task_with_common_fields_roundtrip() {
    let json = r#"{
        "if": "${condition}",
        "output": {"as": {"result": "output"}},
        "then": "continue",
        "switch": [{"case1": {"when": "${c1}", "then": "next"}}]
    }"#;
    let task: SwitchTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: SwitchTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// SetTask with common fields roundtrip (matches Go SDK's TestSetTask_MarshalJSON)
#[test]
fn test_set_task_with_common_fields_roundtrip() {
    let json = r#"{
        "if": "${condition}",
        "output": {"as": {"result": "output"}},
        "then": "continue",
        "set": {"key1": "value1", "key2": 42}
    }"#;
    let task: SetTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: SetTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// RaiseTask with common fields roundtrip
#[test]
fn test_raise_task_with_common_fields_roundtrip() {
    let json = r#"{
        "if": "${condition}",
        "output": {"as": {"result": "output"}},
        "then": "continue",
        "raise": {
            "error": {
                "type": "http://example.com/error",
                "status": 500,
                "title": "Error"
            }
        }
    }"#;
    let task: RaiseTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: RaiseTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// EmitTask with full event properties roundtrip
#[test]
fn test_emit_task_full_event_roundtrip() {
    let json = r#"{
        "emit": {
            "event": {
                "id": "event-id",
                "source": "http://example.com/source",
                "type": "example.event.type",
                "time": "2023-01-01T00:00:00Z",
                "subject": "example.subject",
                "datacontenttype": "application/json",
                "dataschema": "http://example.com/schema",
                "with": {"extra": "value"}
            }
        }
    }"#;
    let task: EmitTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: EmitTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// ListenTask with until condition roundtrip
#[test]
fn test_listen_task_with_until_roundtrip() {
    let json = r#"{
        "listen": {
            "to": {
                "any": [{"with": {"type": "example.event.type"}}],
                "until": "workflow.data.condition == true"
            }
        }
    }"#;
    let task: ListenTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: ListenTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Tests matching Go SDK's task_run_test.go ===

#[test]
fn test_run_task_container_with_common_fields() {
    // Matches Go SDK's TestRunTask_MarshalJSON/UnmarshalJSON
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "metadata": {"meta": "data"},
        "run": {
            "await": true,
            "container": {
                "image": "example-image",
                "name": "example-name",
                "command": "example-command",
                "ports": {"8080": "80"},
                "environment": {"ENV_VAR": "value"},
                "stdin": "example-input",
                "arguments": ["arg1", "arg2"],
                "pullPolicy": "always",
                "lifetime": {
                    "cleanup": "eventually",
                    "after": {"seconds": 20}
                }
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_, Some("${condition}".to_string()));
    assert!(task.common.input.is_some());
    assert!(task.common.output.is_some());
    assert!(task.common.timeout.is_some());
    assert_eq!(task.common.then, Some("continue".to_string()));
    assert!(task.common.metadata.is_some());
    assert_eq!(task.run.await_, Some(true));
    assert!(task.run.container.is_some());
    let container = task.run.container.as_ref().unwrap();
    assert_eq!(container.image, "example-image");
    assert_eq!(container.command, Some("example-command".to_string()));
    assert!(container.ports.is_some());
    assert!(container.environment.is_some());
    assert_eq!(container.stdin, Some("example-input".to_string()));
    assert!(container.arguments.is_some());
    let args = container.arguments.as_ref().unwrap();
    assert_eq!(args, &vec!["arg1".to_string(), "arg2".to_string()]);
    assert_eq!(container.pull_policy, Some("always".to_string()));
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.as_ref().unwrap();
    assert_eq!(lifetime.cleanup, "eventually");
}

#[test]
fn test_run_task_script_with_map_args() {
    // Matches Go SDK's TestRunTaskScriptArgsMap_MarshalJSON/UnmarshalJSON
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "metadata": {"meta": "data"},
        "run": {
            "await": true,
            "script": {
                "language": "python",
                "arguments": {"arg1": "value1"},
                "environment": {"ENV_VAR": "value"},
                "code": "print('Hello, World!')",
                "stdin": "example-input"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.run.await_, Some(true));
    assert!(task.run.script.is_some());
    let script = task.run.script.as_ref().unwrap();
    assert_eq!(script.language, "python");
    assert!(script.arguments.is_some());
    let args = script.arguments.as_ref().unwrap();
    assert!(args.is_map());
    assert_eq!(script.code, Some("print('Hello, World!')".to_string()));
    assert_eq!(script.stdin, Some("example-input".to_string()));
    assert!(script.environment.is_some());
}

#[test]
fn test_run_task_script_with_array_args() {
    // Matches Go SDK's TestRunTaskScriptArgsArray_MarshalJSON/UnmarshalJSON
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "metadata": {"meta": "data"},
        "run": {
            "await": true,
            "script": {
                "language": "python",
                "arguments": ["arg1=value1"],
                "environment": {"ENV_VAR": "value"},
                "code": "print('Hello, World!')",
                "stdin": "example-input"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    assert!(task.run.script.is_some());
    let script = task.run.script.as_ref().unwrap();
    assert!(script.arguments.is_some());
    let args = script.arguments.as_ref().unwrap();
    assert!(args.is_array());
    assert_eq!(args.as_array().unwrap(), &vec!["arg1=value1".to_string()]);
}

#[test]
fn test_run_task_container_roundtrip() {
    // Roundtrip test for container with full fields
    let json = r#"{
        "if": "${condition}",
        "then": "continue",
        "run": {
            "await": true,
            "container": {
                "image": "example-image",
                "command": "example-command",
                "environment": {"ENV_VAR": "value"}
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: RunTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

#[test]
fn test_run_task_script_map_args_roundtrip() {
    let json = r#"{
        "run": {
            "script": {
                "language": "python",
                "arguments": {"arg1": "value1"},
                "code": "print('hello')"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: RunTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

#[test]
fn test_run_task_script_array_args_roundtrip() {
    let json = r#"{
        "run": {
            "script": {
                "language": "python",
                "arguments": ["arg1=value1"],
                "code": "print('hello')"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: RunTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Tests matching Go SDK's task_set_test.go ===

#[test]
fn test_set_task_full_common_fields() {
    // Matches Go SDK's TestSetTask_MarshalJSON/UnmarshalJSON
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "metadata": {"meta": "data"},
        "set": {
            "key1": "value1",
            "key2": 42
        }
    }"#;
    let task: SetTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_, Some("${condition}".to_string()));
    assert!(task.common.input.is_some());
    assert!(task.common.output.is_some());
    assert!(task.common.timeout.is_some());
    assert_eq!(task.common.then, Some("continue".to_string()));
    assert!(task.common.metadata.is_some());
    match &task.set {
        SetValue::Map(map) => {
            assert_eq!(map.len(), 2);
            assert_eq!(map.get("key1").unwrap(), &serde_json::json!("value1"));
            assert_eq!(map.get("key2").unwrap(), &serde_json::json!(42));
        }
        _ => panic!("Expected Map variant"),
    }
}

#[test]
fn test_set_task_full_common_fields_roundtrip() {
    let json = r#"{
        "if": "${condition}",
        "then": "continue",
        "set": {"key1": "value1"}
    }"#;
    let task: SetTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: SetTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Tests matching Go SDK's task_do_test.go ===

#[test]
fn test_do_task_with_multiple_subtasks() {
    // Matches Go SDK's TestDoTask_UnmarshalJSON
    let json = r#"{
        "do": [
            {"task1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}},
            {"task2": {"set": {"result": "ok"}}}
        ]
    }"#;
    let task: DoTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.do_.len(), 2);
}

#[test]
fn test_do_task_with_subtasks_roundtrip() {
    let json = r#"{
        "do": [
            {"task1": {"set": {"x": 1}}},
            {"task2": {"set": {"y": 2}}}
        ]
    }"#;
    let task: DoTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: DoTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Tests matching Go SDK's task_fork_test.go ===

#[test]
fn test_fork_task_with_compete_and_branches() {
    // Matches Go SDK's TestForkTask_UnmarshalJSON
    let json = r#"{
        "fork": {
            "branches": [
                {"task1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}},
                {"task2": {"set": {"result": "ok"}}}
            ],
            "compete": true
        }
    }"#;
    let task: ForkTaskDefinition = serde_json::from_str(json).unwrap();
    assert!(task.fork.compete);
    assert_eq!(task.fork.branches.len(), 2);
}

#[test]
fn test_fork_task_with_compete_roundtrip() {
    let json = r#"{
        "fork": {
            "branches": [
                {"task1": {"set": {"x": 1}}},
                {"task2": {"set": {"y": 2}}}
            ],
            "compete": true
        }
    }"#;
    let task: ForkTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: ForkTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Tests matching Go SDK's task_switch_test.go ===

#[test]
fn test_switch_task_with_common_fields_and_cases() {
    // Matches Go SDK's TestSwitchTask_MarshalJSON/UnmarshalJSON
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "metadata": {"meta": "data"},
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}},
            {"case2": {"when": "${condition2}", "then": "end"}}
        ]
    }"#;
    let task: SwitchTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_, Some("${condition}".to_string()));
    assert!(task.common.input.is_some());
    assert!(task.common.output.is_some());
    assert!(task.common.timeout.is_some());
    assert_eq!(task.common.then, Some("continue".to_string()));
    assert!(task.common.metadata.is_some());
    assert_eq!(task.switch.len(), 2);
}

#[test]
fn test_switch_task_with_cases_roundtrip() {
    let json = r#"{
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}},
            {"case2": {"when": "${condition2}", "then": "end"}}
        ]
    }"#;
    let task: SwitchTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: SwitchTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Tests matching Go SDK's task_raise_test.go ===

#[test]
fn test_raise_task_with_common_fields_and_error() {
    // Matches Go SDK's TestRaiseTask_MarshalJSON/UnmarshalJSON
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "metadata": {"meta": "data"},
        "raise": {
            "error": {
                "type": "http://example.com/error",
                "status": 500,
                "title": "Internal Server Error",
                "detail": "An unexpected error occurred."
            }
        }
    }"#;
    let task: RaiseTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_, Some("${condition}".to_string()));
    assert!(task.common.input.is_some());
    assert!(task.common.output.is_some());
    assert!(task.common.timeout.is_some());
    assert_eq!(task.common.then, Some("continue".to_string()));
    assert!(task.common.metadata.is_some());
    match &task.raise.error {
        OneOfErrorDefinitionOrReference::Error(error) => {
            assert_eq!(error.type_.as_str(), "http://example.com/error");
            assert_eq!(error.status, serde_json::json!(500));
            assert_eq!(error.title, Some("Internal Server Error".to_string()));
            assert_eq!(
                error.detail,
                Some("An unexpected error occurred.".to_string())
            );
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_raise_task_with_error_roundtrip() {
    let json = r#"{
        "raise": {
            "error": {
                "type": "http://example.com/error",
                "status": 500,
                "title": "Internal Server Error",
                "detail": "An unexpected error occurred."
            }
        }
    }"#;
    let task: RaiseTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: RaiseTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Tests matching Go SDK's task_wait_test.go ===

#[test]
fn test_wait_task_full_common_fields() {
    // Matches Go SDK's TestWaitTask_MarshalJSON/UnmarshalJSON
    let json = r#"{
        "if": "${condition}",
        "input": {"from": {"key": "value"}},
        "output": {"as": {"result": "output"}},
        "timeout": {"after": "PT10S"},
        "then": "continue",
        "metadata": {"meta": "data"},
        "wait": "P1DT1H"
    }"#;
    let task: WaitTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.common.if_, Some("${condition}".to_string()));
    assert!(task.common.input.is_some());
    assert!(task.common.output.is_some());
    assert!(task.common.timeout.is_some());
    assert_eq!(task.common.then, Some("continue".to_string()));
    assert!(task.common.metadata.is_some());
    assert!(task.wait.is_iso8601());
}

#[test]
fn test_wait_task_iso8601_roundtrip() {
    let json = r#"{
        "wait": "P1DT1H"
    }"#;
    let task: WaitTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: WaitTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

// === Tests matching Go SDK's task_test.go (TaskList) ===

#[test]
fn test_task_definition_dispatch_with_nested_do() {
    // Matches Go SDK's TestTaskList_UnmarshalJSON - nested tasks
    let json = r#"{
        "do": [
            {"task1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}},
            {"task2": {"do": [{"task3": {"set": {"result": "ok"}}}]}}
        ]
    }"#;
    let task: DoTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.do_.len(), 2);
    // Verify task2 is a Do task with nested task3
    let (name, task2_value) = &task.do_.entries[1];
    assert_eq!(name, "task2");
    match task2_value {
        TaskDefinition::Do(nested_do) => {
            assert_eq!(nested_do.do_.len(), 1);
        }
        _ => panic!("Expected Do variant for task2"),
    }
}

// === Container ports compatibility test ===

#[test]
fn test_container_ports_string_values() {
    // Go SDK uses map[string]interface{} which serializes ports with string values
    let json = r#"{
        "image": "nginx",
        "ports": {"8080": "80", "443": "443"}
    }"#;
    let container: ContainerProcessDefinition = serde_json::from_str(json).unwrap();
    assert!(container.ports.is_some());
    let ports = container.ports.as_ref().unwrap();
    assert_eq!(ports.len(), 2);
    assert_eq!(ports.get("8080").unwrap(), &serde_json::json!("80"));
}

#[test]
fn test_container_ports_numeric_values() {
    // Go SDK also supports numeric port values
    let json = r#"{
        "image": "nginx",
        "ports": {"8080": 80, "443": 443}
    }"#;
    let container: ContainerProcessDefinition = serde_json::from_str(json).unwrap();
    assert!(container.ports.is_some());
    let ports = container.ports.as_ref().unwrap();
    assert_eq!(ports.get("8080").unwrap(), &serde_json::json!(80));
}

// === Container lifetime and run task tests matching Go SDK task_run_test.go ===

#[test]
fn test_container_with_lifetime_deserialize() {
    // Matches Go SDK's TestRunTask_UnmarshalJSON - container with lifetime cleanup/after
    let json = r#"{
        "run": {
            "await": true,
            "container": {
                "image": "example-image",
                "name": "example-name",
                "command": "example-command",
                "ports": {"8080": "80"},
                "environment": {"ENV_VAR": "value"},
                "stdin": "example-input",
                "arguments": ["arg1", "arg2"],
                "lifetime": {
                    "cleanup": "eventually",
                    "after": {
                        "seconds": 20
                    }
                }
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let container = task.run.container.as_ref().unwrap();
    assert_eq!(container.image, "example-image");
    assert_eq!(container.name, Some("example-name".to_string()));
    assert_eq!(container.command, Some("example-command".to_string()));
    assert_eq!(container.stdin, Some("example-input".to_string()));
    assert_eq!(
        container.arguments,
        Some(vec!["arg1".to_string(), "arg2".to_string()])
    );
    let lifetime = container.lifetime.as_ref().unwrap();
    assert_eq!(lifetime.cleanup, "eventually");
    assert!(lifetime.after.is_some());
}

#[test]
fn test_container_with_lifetime_roundtrip() {
    // Matches Go SDK's TestRunTask_MarshalJSON - container with lifetime serialized as ISO8601
    let json = r#"{
        "run": {
            "await": true,
            "container": {
                "image": "example-image",
                "command": "example-command",
                "stdin": "example-input",
                "arguments": ["arg1","arg2"],
                "pullPolicy": "always",
                "lifetime": {
                    "cleanup": "eventually",
                    "after": "PT20S"
                }
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let serialized = serde_json::to_string(&task).unwrap();
    let deserialized: RunTaskDefinition = serde_json::from_str(&serialized).unwrap();
    assert_eq!(task, deserialized);
}

#[test]
fn test_run_task_with_await() {
    // Matches Go SDK's TestRunTask_MarshalJSON with await field
    let json = r#"{
        "run": {
            "await": true,
            "shell": {
                "command": "echo hello"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.run.await_, Some(true));
}

#[test]
fn test_run_task_without_await() {
    // await defaults to None when not specified
    let json = r#"{
        "run": {
            "shell": {
                "command": "echo hello"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(task.run.await_, None);
}

#[test]
fn test_script_with_stdin_and_arguments() {
    // Matches Go SDK's TestRunTaskScriptArgsMap_UnmarshalJSON
    let json = r#"{
        "run": {
            "await": true,
            "script": {
                "language": "python",
                "arguments": {"arg1": "value1"},
                "environment": {"ENV_VAR": "value"},
                "code": "print('Hello, World!')",
                "stdin": "example-input"
            }
        }
    }"#;
    let task: RunTaskDefinition = serde_json::from_str(json).unwrap();
    let script = task.run.script.as_ref().unwrap();
    assert_eq!(script.language, "python");
    assert_eq!(script.stdin, Some("example-input".to_string()));
    assert_eq!(script.code, Some("print('Hello, World!')".to_string()));
    assert!(script.arguments.is_some());
    match script.arguments.as_ref().unwrap() {
        OneOfRunArguments::Map(map) => {
            assert_eq!(map.get("arg1").unwrap(), &serde_json::json!("value1"));
        }
        _ => panic!("Expected Map variant"),
    }
}

#[test]
fn test_container_pull_policy_deserialize() {
    let json = r#"{
        "image": "nginx",
        "pullPolicy": "always"
    }"#;
    let container: ContainerProcessDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(container.pull_policy, Some("always".to_string()));
}
