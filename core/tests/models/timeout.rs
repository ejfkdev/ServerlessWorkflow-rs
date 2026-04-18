use super::*;

#[test]
fn test_timeout_definition_with_inline_duration() {
    // Test TimeoutDefinition with inline duration
    let timeout_json = json!({
        "after": {"days": 1, "hours": 2}
    });
    let result: Result<TimeoutDefinition, _> = serde_json::from_value(timeout_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout with inline duration: {:?}",
        result.err()
    );
    let timeout = result.unwrap();
    match timeout.after {
        OneOfDurationOrIso8601Expression::Duration(d) => {
            assert_eq!(d.days, Some(1));
            assert_eq!(d.hours, Some(2));
        }
        _ => panic!("Expected Duration variant"),
    }
}

#[test]
fn test_timeout_definition_with_iso8601_expression() {
    // Test TimeoutDefinition with ISO 8601 expression
    let timeout_json = json!({
        "after": "PT1H"
    });
    let result: Result<TimeoutDefinition, _> = serde_json::from_value(timeout_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout with ISO 8601: {:?}",
        result.err()
    );
    let timeout = result.unwrap();
    match timeout.after {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT1H");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_timeout_or_reference_as_timeout() {
    // Test OneOfTimeoutDefinitionOrReference as Timeout
    let tor_json = json!({
        "after": {"days": 1, "hours": 2}
    });
    let result: Result<OneOfTimeoutDefinitionOrReference, _> = serde_json::from_value(tor_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout or reference: {:?}",
        result.err()
    );
    match result.unwrap() {
        OneOfTimeoutDefinitionOrReference::Timeout(t) => match t.after {
            OneOfDurationOrIso8601Expression::Duration(d) => {
                assert_eq!(d.days, Some(1));
            }
            _ => panic!("Expected Duration"),
        },
        _ => panic!("Expected Timeout variant"),
    }
}

#[test]
fn test_timeout_or_reference_as_reference() {
    // Test OneOfTimeoutDefinitionOrReference as Reference
    let tor_json = json!("my-timeout-ref");
    let result: Result<OneOfTimeoutDefinitionOrReference, _> = serde_json::from_value(tor_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout reference: {:?}",
        result.err()
    );
    match result.unwrap() {
        OneOfTimeoutDefinitionOrReference::Reference(s) => {
            assert_eq!(s, "my-timeout-ref");
        }
        _ => panic!("Expected Reference variant"),
    }
}

#[test]
fn test_timeout_definition_serialization() {
    // Test TimeoutDefinition serialization
    let timeout = TimeoutDefinition {
        after: OneOfDurationOrIso8601Expression::Duration(Duration::from_hours(2)),
    };
    let json_str = serde_json::to_string(&timeout).expect("Failed to serialize timeout");
    assert!(json_str.contains("\"hours\":2") || json_str.contains("hours"));
}

#[test]
fn test_timeout_reference_serialization() {
    // Test OneOfTimeoutDefinitionOrReference as Reference serialization
    let tor = OneOfTimeoutDefinitionOrReference::Reference("my-timeout-ref".to_string());
    let json_str = serde_json::to_string(&tor).expect("Failed to serialize timeout reference");
    assert!(json_str.contains("my-timeout-ref"));
}

#[test]
fn test_timeout_with_iso8601_expression() {
    // Test TimeoutDefinition with ISO 8601 expression
    let timeout_json = json!({
        "after": "PT1H"
    });
    let result: Result<TimeoutDefinition, _> = serde_json::from_value(timeout_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout with ISO 8601: {:?}",
        result.err()
    );
    let timeout = result.unwrap();
    match timeout.after {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT1H");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_timeout_roundtrip_serialization() {
    // Test TimeoutDefinition roundtrip serialization
    let timeout = TimeoutDefinition {
        after: OneOfDurationOrIso8601Expression::Iso8601Expression("PT2H".to_string()),
    };
    let json_str = serde_json::to_string(&timeout).expect("Failed to serialize timeout");
    let deserialized: TimeoutDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized.after {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT2H");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_timeout_reference_serialization_roundtrip() {
    // Test OneOfTimeoutDefinitionOrReference as Reference roundtrip
    let tor = OneOfTimeoutDefinitionOrReference::Reference("my-timeout-ref".to_string());
    let json_str = serde_json::to_string(&tor).expect("Failed to serialize timeout reference");
    let deserialized: OneOfTimeoutDefinitionOrReference =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized {
        OneOfTimeoutDefinitionOrReference::Reference(s) => {
            assert_eq!(s, "my-timeout-ref");
        }
        _ => panic!("Expected Reference variant"),
    }
}

#[test]
fn test_timeout_with_inline_duration() {
    // Test TimeoutDefinition with inline duration
    let timeout_json = json!({
        "after": {
            "days": 1,
            "hours": 2,
            "minutes": 30
        }
    });

    let result: Result<serverless_workflow_core::models::timeout::TimeoutDefinition, _> =
        serde_json::from_value(timeout_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout with inline duration: {:?}",
        result.err()
    );
}

#[test]
fn test_timeout_with_iso8601_duration() {
    // Test TimeoutDefinition with ISO 8601 duration
    let timeout_json = json!({
        "after": "PT1H30M"
    });

    let result: Result<serverless_workflow_core::models::timeout::TimeoutDefinition, _> =
        serde_json::from_value(timeout_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout with ISO8601: {:?}",
        result.err()
    );
}
