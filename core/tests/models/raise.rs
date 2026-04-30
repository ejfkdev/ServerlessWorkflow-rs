use super::*;

#[test]
fn test_raise_task_serialization() {
    // Test RaiseTaskDefinition serialization (similar to Go SDK TestRaiseTask_MarshalJSON)
    use swf_core::models::error::OneOfErrorDefinitionOrReference;

    let raise_task_json = json!({
        "raise": {
            "error": {
                "type": "http://example.com/error",
                "status": 500,
                "title": "Internal Server Error",
                "detail": "An unexpected error occurred."
            }
        }
    });
    let result: Result<RaiseTaskDefinition, _> = serde_json::from_value(raise_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize raise task: {:?}",
        result.err()
    );
    let raise_task = result.unwrap();
    match raise_task.raise.error {
        OneOfErrorDefinitionOrReference::Error(err) => {
            assert_eq!(err.type_.as_str(), "http://example.com/error");
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
fn test_raise_task_roundtrip_serialization() {
    // Test RaiseTaskDefinition roundtrip serialization
    use swf_core::models::error::ErrorDefinition;
    use swf_core::models::error::OneOfErrorDefinitionOrReference;

    let raise_task = RaiseTaskDefinition::new(RaiseErrorDefinition::new(
        OneOfErrorDefinitionOrReference::Error(ErrorDefinition::new(
            "http://example.com/error",
            "Internal Server Error",
            serde_json::json!(500),
            Some("An unexpected error occurred.".to_string()),
            None,
        )),
    ));

    let json_str = serde_json::to_string(&raise_task).expect("Failed to serialize raise task");
    let deserialized: RaiseTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized.raise.error {
        OneOfErrorDefinitionOrReference::Error(err) => {
            assert_eq!(err.type_.as_str(), "http://example.com/error");
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_raise_task_definition() {
    use swf_core::models::error::OneOfErrorDefinitionOrReference;
    use swf_core::models::task::{RaiseErrorDefinition, RaiseTaskDefinition};
    let raise_error = RaiseErrorDefinition {
        error: OneOfErrorDefinitionOrReference::Error(
            swf_core::models::error::ErrorDefinition::new(
                "https://example.com/errors",
                "Bad Request",
                serde_json::json!(400),
                Some("Error occurred".to_string()),
                None,
            ),
        ),
    };
    let raise_task = RaiseTaskDefinition::new(raise_error);
    let json_str = serde_json::to_string(&raise_task).expect("Failed to serialize");
    assert!(json_str.contains("raise"));
    assert!(json_str.contains("Bad Request"));
}

#[test]
fn test_raise_task_roundtrip() {
    use swf_core::models::error::{ErrorDefinition, OneOfErrorDefinitionOrReference};
    use swf_core::models::task::{RaiseErrorDefinition, RaiseTaskDefinition};
    let raise_error = RaiseErrorDefinition {
        error: OneOfErrorDefinitionOrReference::Error(ErrorDefinition::new(
            "https://example.com/errors",
            "Not Found",
            serde_json::json!(404),
            Some("Resource not found".to_string()),
            None,
        )),
    };
    let raise_task = RaiseTaskDefinition::new(raise_error);
    let json_str = serde_json::to_string(&raise_task).expect("Failed to serialize");
    let deserialized: RaiseTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    if let OneOfErrorDefinitionOrReference::Error(err) = &deserialized.raise.error {
        assert_eq!(err.title, Some("Not Found".to_string()));
    }
}

#[test]
fn test_raise_task_with_all_fields() {
    // Test RaiseTaskDefinition with all common fields
    use swf_core::models::error::OneOfErrorDefinitionOrReference;

    let raise_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
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
    });
    let result: Result<RaiseTaskDefinition, _> = serde_json::from_value(raise_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize raise task with all fields: {:?}",
        result.err()
    );
    let raise_task = result.unwrap();
    assert_eq!(raise_task.common.if_, Some("${condition}".to_string()));
    assert!(raise_task.common.timeout.is_some());
    assert_eq!(raise_task.common.then, Some("continue".to_string()));
    match raise_task.raise.error {
        OneOfErrorDefinitionOrReference::Error(err) => {
            assert_eq!(err.type_.as_str(), "http://example.com/error");
            assert_eq!(err.title, Some("Internal Server Error".to_string()));
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_raise_task_workflow() {
    // Test workflow with raise task for error raising
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "raise-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "failOperation": {
                    "raise": {
                        "error": {
                            "type": "https://example.com/errors/failure",
                            "status": 500,
                            "title": "Operation Failed"
                        }
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with raise: {:?}",
        result.err()
    );
}

#[test]
fn test_raise_task_with_error_reference() {
    // Test raise task with error reference (string reference directly)
    let raise_json = json!({
        "raise": {
            "error": "someError"
        }
    });

    let result: Result<swf_core::models::task::RaiseTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"raise": raise_json["raise"]}));
    assert!(
        result.is_ok(),
        "Failed to deserialize raise task with error reference: {:?}",
        result.err()
    );
}
