use super::*;

#[test]
fn test_switch_task_serialization() {
    // Test SwitchTaskDefinition serialization (similar to Go SDK TestSwitchTask_MarshalJSON)
    let switch_task_json = json!({
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}},
            {"case2": {"when": "${condition2}", "then": "end"}}
        ]
    });
    let result: Result<SwitchTaskDefinition, _> = serde_json::from_value(switch_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch task: {:?}",
        result.err()
    );
    let switch_task = result.unwrap();
    assert_eq!(switch_task.switch.entries.len(), 2);
}

#[test]
fn test_switch_task_roundtrip_serialization() {
    // Test SwitchTaskDefinition roundtrip serialization
    let mut switch_cases = Map::new();
    switch_cases.add(
        "case1".to_string(),
        SwitchCaseDefinition {
            when: Some("${condition1}".to_string()),
            then: Some("next".to_string()),
        },
    );
    switch_cases.add(
        "case2".to_string(),
        SwitchCaseDefinition {
            when: Some("${condition2}".to_string()),
            then: Some("end".to_string()),
        },
    );

    let switch_task = SwitchTaskDefinition {
        switch: switch_cases,
        common: TaskDefinitionFields::new(),
    };

    let json_str = serde_json::to_string(&switch_task).expect("Failed to serialize switch task");
    let deserialized: SwitchTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(deserialized.switch.entries.len(), 2);
}

#[test]
fn test_switch_task_with_all_fields() {
    // Test SwitchTaskDefinition with all common fields
    let switch_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}}
        ]
    });
    let result: Result<SwitchTaskDefinition, _> = serde_json::from_value(switch_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch task with all fields: {:?}",
        result.err()
    );
    let switch_task = result.unwrap();
    assert_eq!(switch_task.common.if_, Some("${condition}".to_string()));
    assert!(switch_task.common.timeout.is_some());
    assert_eq!(switch_task.common.then, Some("continue".to_string()));
}

#[test]
fn test_switch_case_definition_serialization() {
    // Test SwitchCaseDefinition serialization
    let case_json = json!({
        "when": "${myCondition}",
        "then": "continue"
    });
    let result: Result<SwitchCaseDefinition, _> = serde_json::from_value(case_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch case: {:?}",
        result.err()
    );
    let case_def = result.unwrap();
    assert_eq!(case_def.when, Some("${myCondition}".to_string()));
    assert_eq!(case_def.then, Some("continue".to_string()));
}

#[test]
fn test_switch_case_definition_without_when() {
    // Test SwitchCaseDefinition without when (default case)
    let case_json = json!({
        "then": "end"
    });
    let result: Result<SwitchCaseDefinition, _> = serde_json::from_value(case_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch case without when: {:?}",
        result.err()
    );
    let case_def = result.unwrap();
    assert_eq!(case_def.when, None);
    assert_eq!(case_def.then, Some("end".to_string()));
}

#[test]
fn test_switch_task_with_all_common_fields() {
    // Test SwitchTaskDefinition with all common fields (if, input, output, timeout, then, metadata)
    let switch_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}},
            {"case2": {"when": "${condition2}", "then": "end"}}
        ]
    });
    let result: Result<SwitchTaskDefinition, _> = serde_json::from_value(switch_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch task with all fields: {:?}",
        result.err()
    );
    let switch_task = result.unwrap();
    assert_eq!(switch_task.common.if_, Some("${condition}".to_string()));
    assert!(switch_task.common.timeout.is_some());
    assert_eq!(switch_task.common.then, Some("continue".to_string()));
    assert_eq!(switch_task.switch.entries.len(), 2);
}

#[test]
fn test_switch_task_unmarshaling() {
    // Test SwitchTaskDefinition unmarshaling from JSON
    let json_data = r#"{
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}},
            {"case2": {"when": "${condition2}", "then": "end"}}
        ]
    }"#;
    let result: Result<SwitchTaskDefinition, _> = serde_json::from_str(json_data);
    assert!(
        result.is_ok(),
        "Failed to unmarshal switch task: {:?}",
        result.err()
    );
    let switch_task = result.unwrap();
    assert_eq!(switch_task.common.if_, Some("${condition}".to_string()));
    assert_eq!(switch_task.switch.entries.len(), 2);
}

#[test]
fn test_switch_task_with_multiple_cases() {
    // Test SwitchTaskDefinition with multiple cases
    use swf_core::models::map::Map;
    use swf_core::models::task::{SwitchCaseDefinition, SwitchTaskDefinition};

    let mut switch_cases = Map::new();
    switch_cases.add(
        "case1".to_string(),
        SwitchCaseDefinition {
            when: Some("${ .orderType == \"electronic\" }".to_string()),
            then: Some("processElectronicOrder".to_string()),
        },
    );
    switch_cases.add(
        "case2".to_string(),
        SwitchCaseDefinition {
            when: Some("${ .orderType == \"physical\" }".to_string()),
            then: Some("processPhysicalOrder".to_string()),
        },
    );
    switch_cases.add(
        "default".to_string(),
        SwitchCaseDefinition {
            when: None,
            then: Some("handleUnknownOrderType".to_string()),
        },
    );

    let switch_task = SwitchTaskDefinition {
        switch: switch_cases,
        ..Default::default()
    };

    let json_str = serde_json::to_string(&switch_task).expect("Failed to serialize switch task");
    assert!(json_str.contains("switch"));
    assert!(json_str.contains("case1"));
    assert!(json_str.contains("case2"));
}

#[test]
fn test_switch_task_deserialization() {
    // Test deserialization of switch task from specification example
    let switch_json = json!({
        "switch": [
            {
                "case1": {
                    "when": ".orderType == \"electronic\"",
                    "then": "processElectronicOrder"
                }
            },
            {
                "case2": {
                    "when": ".orderType == \"physical\"",
                    "then": "processPhysicalOrder"
                }
            },
            {
                "default": {
                    "then": "handleUnknownOrderType"
                }
            }
        ]
    });

    let result: Result<SwitchTaskDefinition, _> = serde_json::from_value(switch_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch task: {:?}",
        result.err()
    );
    if let Ok(switch_task) = result {
        assert_eq!(switch_task.switch.entries.len(), 3);
    }
}

#[test]
fn test_switch_task_with_string_then() {
    // Test switch task where 'then' is a string (transition)
    let switch_json = json!({
        "switch": [
            {
                "case1": {
                    "when": ".status == 'active'",
                    "then": "processActive"
                }
            },
            {
                "default": {
                    "then": "handleDefault"
                }
            }
        ]
    });

    let result: Result<swf_core::models::task::SwitchTaskDefinition, _> =
        serde_json::from_value(switch_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch with string then: {:?}",
        result.err()
    );
}
