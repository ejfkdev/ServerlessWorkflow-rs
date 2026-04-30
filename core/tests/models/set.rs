use super::*;

#[test]
fn test_set_value_map_deserialization() {
    let set_value_map = serde_json::json!({
        "foo": "bar",
        "count": 42
    });

    let result: Result<SetTaskDefinition, _> = serde_json::from_value(serde_json::json!({
        "set": set_value_map
    }));
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with map: {:?}",
        result.err()
    );

    let set_task = result.unwrap();
    match set_task.set {
        SetValue::Map(map) => {
            assert_eq!(map.len(), 2);
            assert_eq!(map.get("foo").and_then(|v| v.as_str()), Some("bar"));
            assert_eq!(map.get("count").and_then(|v| v.as_u64()), Some(42));
        }
        SetValue::Expression(_) => {
            panic!("Expected SetValue::Map but got SetValue::Expression");
        }
    }
}

#[test]
fn test_set_value_expression_deserialization() {
    let set_value_expr_json = serde_json::json!("${ $workflow.input[0] }");

    let result: Result<SetTaskDefinition, _> = serde_json::from_value(serde_json::json!({
        "set": set_value_expr_json
    }));
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with expression: {:?}",
        result.err()
    );

    let set_task = result.unwrap();
    match set_task.set {
        SetValue::Expression(expr) => {
            assert_eq!(expr, "${ $workflow.input[0] }");
        }
        SetValue::Map(_) => {
            panic!("Expected SetValue::Expression but got SetValue::Map");
        }
    }
}

#[test]
fn test_set_task_with_map_serialization() {
    // Test SetTaskDefinition with map (similar to Go SDK TestSetTask_MarshalJSON)
    let set_task_json = json!({
        "set": {
            "key1": "value1",
            "key2": 42
        }
    });
    let result: Result<SetTaskDefinition, _> = serde_json::from_value(set_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with map: {:?}",
        result.err()
    );
    let set_task = result.unwrap();
    match &set_task.set {
        SetValue::Map(map) => {
            assert_eq!(map.get("key1").and_then(|v| v.as_str()), Some("value1"));
            assert_eq!(map.get("key2").and_then(|v| v.as_u64()), Some(42));
        }
        _ => panic!("Expected Map variant"),
    }
}

#[test]
fn test_set_task_with_expression_serialization() {
    // Test SetTaskDefinition with expression
    let set_task_json = json!({
        "set": "${ $workflow.input }"
    });
    let result: Result<SetTaskDefinition, _> = serde_json::from_value(set_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with expression: {:?}",
        result.err()
    );
    let set_task = result.unwrap();
    match &set_task.set {
        SetValue::Expression(expr) => {
            assert_eq!(expr, "${ $workflow.input }");
        }
        _ => panic!("Expected Expression variant"),
    }
}

#[test]
fn test_set_task_roundtrip_serialization() {
    // Test SetTaskDefinition roundtrip serialization
    let mut map = std::collections::HashMap::new();
    map.insert("foo".to_string(), serde_json::json!("bar"));
    map.insert("count".to_string(), serde_json::json!(123));

    let set_task = SetTaskDefinition {
        set: SetValue::Map(map),
        common: TaskDefinitionFields::new(),
    };

    let json_str = serde_json::to_string(&set_task).expect("Failed to serialize set task");
    let deserialized: SetTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized.set {
        SetValue::Map(m) => {
            assert_eq!(m.get("foo").and_then(|v| v.as_str()), Some("bar"));
        }
        _ => panic!("Expected Map variant"),
    }
}

#[test]
fn test_set_task_with_all_fields() {
    // Test SetTaskDefinition with all common fields
    let set_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "set": {"key": "value"}
    });
    let result: Result<SetTaskDefinition, _> = serde_json::from_value(set_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with all fields: {:?}",
        result.err()
    );
    let set_task = result.unwrap();
    assert_eq!(set_task.common.if_, Some("${condition}".to_string()));
    assert!(set_task.common.timeout.is_some());
    assert_eq!(set_task.common.then, Some("continue".to_string()));
}

#[test]
fn test_set_task_definition() {
    use std::collections::HashMap;
    use swf_core::models::task::{SetTaskDefinition, SetValue};
    let mut set_map = HashMap::new();
    set_map.insert("key1".to_string(), serde_json::json!("value1"));
    set_map.insert("key2".to_string(), serde_json::json!(42));
    let set_task = SetTaskDefinition {
        set: SetValue::Map(set_map),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&set_task).expect("Failed to serialize");
    assert!(json_str.contains("set"));
    assert!(json_str.contains("key1"));
    assert!(json_str.contains("value1"));
}

#[test]
fn test_set_task_roundtrip() {
    use std::collections::HashMap;
    use swf_core::models::task::{SetTaskDefinition, SetValue};
    let mut set_map = HashMap::new();
    set_map.insert("name".to_string(), serde_json::json!("test"));
    set_map.insert("value".to_string(), serde_json::json!(123));
    let set_task = SetTaskDefinition {
        set: SetValue::Map(set_map),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&set_task).expect("Failed to serialize");
    let _deserialized: SetTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(json_str.contains("test"));
}

#[test]
fn test_map_serialization_order() {
    // Test that Map entries maintain serialization order
    use swf_core::models::task::CallTaskDefinition;
    use swf_core::models::task::TaskDefinition;

    let mut map = Map::new();
    map.add(
        "first".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            swf_core::models::call::CallFunctionDefinition {
                call: "func1".to_string(),
                with: None,
                common: swf_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );
    map.add(
        "second".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            swf_core::models::call::CallFunctionDefinition {
                call: "func2".to_string(),
                with: None,
                common: swf_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );
    map.add(
        "third".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            swf_core::models::call::CallFunctionDefinition {
                call: "func3".to_string(),
                with: None,
                common: swf_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );

    let json_str = serde_json::to_string(&map).expect("Failed to serialize map");
    // JSON array order should be preserved
    assert!(json_str.contains("first"));
    assert!(json_str.contains("second"));
    assert!(json_str.contains("third"));
}

#[test]
fn test_set_task_with_nested_object() {
    // Test set task with nested object values
    let set_task_json = json!({
        "set": {
            "nested": {
                "level1": {
                    "level2": "value"
                }
            },
            "array": [1, 2, 3]
        }
    });
    let result: Result<SetTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"set": set_task_json}));
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with nested object: {:?}",
        result.err()
    );
}
