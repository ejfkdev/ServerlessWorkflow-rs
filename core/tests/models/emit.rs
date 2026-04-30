use super::*;

#[test]
fn test_emit_task_serialization() {
    // Test EmitTaskDefinition serialization (similar to Go SDK TestEmitTask_MarshalJSON)

    let emit_task_json = json!({
        "emit": {
            "event": {
                "with": {
                    "id": "event-id",
                    "source": "http://example.com/source",
                    "type": "example.event.type"
                }
            }
        }
    });
    let result: Result<EmitTaskDefinition, _> = serde_json::from_value(emit_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize emit task: {:?}",
        result.err()
    );
    let emit_task = result.unwrap();
    assert!(emit_task.emit.event.with.contains_key("id"));
    assert!(emit_task.emit.event.with.contains_key("type"));
}

#[test]
fn test_emit_task_roundtrip_serialization() {
    // Test EmitTaskDefinition roundtrip serialization
    use std::collections::HashMap;
    use swf_core::models::event::EventDefinition;

    let mut event_with = HashMap::new();
    event_with.insert("id".to_string(), serde_json::json!("my-event-id"));
    event_with.insert("type".to_string(), serde_json::json!("my.event.type"));

    let emit_task = EmitTaskDefinition::new(EventEmissionDefinition {
        event: EventDefinition {
            with: event_with,
            ..Default::default()
        },
    });

    let json_str = serde_json::to_string(&emit_task).expect("Failed to serialize emit task");
    let deserialized: EmitTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.emit.event.with.contains_key("id"));
}

#[test]
fn test_emit_task_with_all_fields() {
    // Test EmitTaskDefinition with all fields (similar to Go SDK TestEmitTask_MarshalJSON)
    use std::collections::HashMap;

    let mut event_with = HashMap::new();
    event_with.insert("id".to_string(), serde_json::json!("event-id"));
    event_with.insert(
        "source".to_string(),
        serde_json::json!("http://example.com/source"),
    );
    event_with.insert("type".to_string(), serde_json::json!("example.event.type"));
    event_with.insert(
        "time".to_string(),
        serde_json::json!("2023-01-01T00:00:00Z"),
    );
    event_with.insert("subject".to_string(), serde_json::json!("example.subject"));
    event_with.insert(
        "datacontenttype".to_string(),
        serde_json::json!("application/json"),
    );
    event_with.insert(
        "dataschema".to_string(),
        serde_json::json!("http://example.com/schema"),
    );
    event_with.insert("extra".to_string(), serde_json::json!("value"));

    let emit_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "emit": {
            "event": {
                "with": event_with
            }
        }
    });
    let result: Result<EmitTaskDefinition, _> = serde_json::from_value(emit_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize emit task with all fields: {:?}",
        result.err()
    );
    let emit_task = result.unwrap();
    assert_eq!(emit_task.common.if_, Some("${condition}".to_string()));
    assert!(emit_task.common.timeout.is_some());
    assert_eq!(emit_task.common.then, Some("continue".to_string()));
    assert!(emit_task.emit.event.with.contains_key("id"));
    assert_eq!(
        emit_task
            .emit
            .event
            .with
            .get("type")
            .and_then(|v| v.as_str()),
        Some("example.event.type")
    );
}

#[test]
fn test_complete_workflow_deserialization() {
    // Test complete workflow deserialization from JSON (similar to TypeScript SDK deserialization test)
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "name": "test",
            "version": "1.0.0",
            "namespace": "default"
        },
        "do": [
            {
                "step1": {
                    "set": {
                        "foo": "bar"
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow: {:?}",
        result.err()
    );
    let workflow = result.unwrap();
    assert_eq!(workflow.document.name, "test");
    assert_eq!(workflow.document.namespace, "default");
}

#[test]
fn test_emit_task_workflow() {
    // Test workflow with emit task for event emission
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "emit-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "sendNotification": {
                    "emit": {
                        "event": {
                            "with": {
                                "type": "notification.event",
                                "source": "http://example.com"
                            }
                        }
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with emit: {:?}",
        result.err()
    );
}

#[test]
fn test_emit_task() {
    // Test emit task from specification example
    use std::collections::HashMap;
    let mut with = HashMap::new();
    with.insert(
        "type".to_string(),
        serde_json::json!("com.fake.petclinic.pets.checkup.completed"),
    );
    with.insert("source".to_string(), serde_json::json!("test-source"));

    let emit_json = json!({
        "emit": {
            "event": {
                "with": with
            }
        }
    });

    let result: Result<swf_core::models::task::EmitTaskDefinition, _> =
        serde_json::from_value(emit_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize emit task: {:?}",
        result.err()
    );
}
