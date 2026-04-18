use super::*;

#[test]
fn test_wait_task_iso8601_deserialization() {
    let wait_task_json = serde_json::json!({
        "wait": "PT30S"
    });
    let result: Result<WaitTaskDefinition, _> = serde_json::from_value(wait_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait task with ISO 8601: {:?}",
        result.err()
    );

    let wait_task = result.unwrap();
    match wait_task.wait {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT30S");
        }
        OneOfDurationOrIso8601Expression::Duration(_) => {
            panic!("Expected Iso8601Expression but got Duration");
        }
    }
}

#[test]
fn test_wait_task_inline_duration_deserialization() {
    let wait_task_json = serde_json::json!({
        "wait": {
            "seconds": 30
        }
    });
    let result: Result<WaitTaskDefinition, _> = serde_json::from_value(wait_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait task with inline duration: {:?}",
        result.err()
    );

    let wait_task = result.unwrap();
    match wait_task.wait {
        OneOfDurationOrIso8601Expression::Duration(duration) => {
            assert_eq!(duration.seconds, Some(30));
        }
        OneOfDurationOrIso8601Expression::Iso8601Expression(_) => {
            panic!("Expected Duration but got Iso8601Expression");
        }
    }
}

#[test]
fn test_wait_task_with_iso8601() {
    // Test WaitTaskDefinition with ISO 8601 expression
    let wait_task_json = json!({
        "wait": "P1DT1H"
    });
    let result: Result<WaitTaskDefinition, _> = serde_json::from_value(wait_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait task with ISO 8601: {:?}",
        result.err()
    );
    let wait_task = result.unwrap();
    match wait_task.wait {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "P1DT1H");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_wait_task_with_duration() {
    // Test WaitTaskDefinition with inline duration
    let wait_task_json = json!({
        "wait": {"hours": 1}
    });
    let result: Result<WaitTaskDefinition, _> = serde_json::from_value(wait_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait task with duration: {:?}",
        result.err()
    );
    let wait_task = result.unwrap();
    match wait_task.wait {
        OneOfDurationOrIso8601Expression::Duration(d) => {
            assert_eq!(d.hours, Some(1));
        }
        _ => panic!("Expected Duration variant"),
    }
}

#[test]
fn test_wait_task_roundtrip_serialization() {
    // Test WaitTaskDefinition roundtrip serialization
    let wait_task = WaitTaskDefinition::new(OneOfDurationOrIso8601Expression::Iso8601Expression(
        "PT30S".to_string(),
    ));
    let json_str = serde_json::to_string(&wait_task).expect("Failed to serialize wait task");
    let deserialized: WaitTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized.wait {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT30S");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_wait_task_with_all_fields() {
    // Test WaitTaskDefinition with all common fields
    let wait_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "wait": "PT1H"
    });
    let result: Result<WaitTaskDefinition, _> = serde_json::from_value(wait_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait task with all fields: {:?}",
        result.err()
    );
    let wait_task = result.unwrap();
    assert_eq!(wait_task.common.if_, Some("${condition}".to_string()));
    assert!(wait_task.common.timeout.is_some());
    assert_eq!(wait_task.common.then, Some("continue".to_string()));
}

#[test]
fn test_wait_task_with_duration_new() {
    use serverless_workflow_core::models::task::WaitTaskDefinition;
    let wait_task = WaitTaskDefinition::new(
        serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Duration(
            serverless_workflow_core::models::duration::Duration::from_seconds(10),
        ),
    );
    let json_str = serde_json::to_string(&wait_task).expect("Failed to serialize");
    assert!(json_str.contains("wait"));
    let _deserialized: WaitTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(json_str.contains("10"));
}

#[test]
fn test_wait_duration_iso8601() {
    // Test wait task with ISO8601 duration (matches Go SDK: WaitTask.Wait is *Duration)
    let wait_json = json!({
        "wait": "PT1S"
    });

    let result: Result<serverless_workflow_core::models::task::WaitTaskDefinition, _> =
        serde_json::from_value(wait_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait with ISO8601: {:?}",
        result.err()
    );
}

#[test]
fn test_wait_duration_inline() {
    // Test wait task with inline duration (matches Go SDK: DurationInline with valid keys)
    let wait_json = json!({
        "wait": {
            "seconds": 5
        }
    });

    let result: Result<serverless_workflow_core::models::task::WaitTaskDefinition, _> =
        serde_json::from_value(wait_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait with inline duration: {:?}",
        result.err()
    );
}
