use super::*;

#[test]
fn test_for_loop_definition_each_field_deserialization() {
    let for_loop_json = serde_json::json!({
        "each": "item",
        "in": ".items"
    });

    let result: Result<ForLoopDefinition, _> = serde_json::from_value(for_loop_json);

    match result {
        Ok(for_loop) => {
            assert_eq!(for_loop.each, "item", "The 'each' field should be 'item'");
            assert_eq!(for_loop.in_, ".items", "The 'in' field should be '.items'");
        }
        Err(e) => {
            panic!(
                "Failed to deserialize ForLoopDefinition with 'each' field: {}",
                e
            );
        }
    }
}

#[test]
fn test_for_task_deserialization() {
    let for_task_json = json!({
        "for": {
            "each": "item",
            "in": ".items"
        },
        "do": [
            {
                "processItem": {
                    "call": "processFunction",
                    "with": {
                        "item": "${ .item }"
                    }
                }
            }
        ]
    });

    let result: Result<TaskDefinition, _> = serde_json::from_value(for_task_json.clone());

    match result {
        Ok(TaskDefinition::For(for_def)) => {
            assert_eq!(for_def.for_.each, "item");
            assert_eq!(for_def.for_.in_, ".items");
            assert_eq!(for_def.do_.entries.len(), 1);
            let has_process_item = for_def
                .do_
                .entries
                .iter()
                .any(|(name, _)| name == "processItem");
            assert!(
                has_process_item,
                "For task should contain processItem subtask"
            );
        }
        Ok(TaskDefinition::Do(_)) => {
            panic!("For task incorrectly deserialized as DoTaskDefinition");
        }
        Ok(other) => {
            panic!("For task deserialized as unexpected variant: {:?}", other);
        }
        Err(e) => {
            panic!("Failed to deserialize For task: {}", e);
        }
    }
}

#[test]
fn test_for_task_with_while_condition() {
    let for_task_json = json!({
        "for": {
            "each": "user",
            "in": ".users",
            "at": "index"
        },
        "while": "${ .index < 10 }",
        "do": [
            {
                "notifyUser": {
                    "call": "notifyUser",
                    "with": {
                        "user": "${ .user }",
                        "index": "${ .index }"
                    }
                }
            }
        ]
    });

    let result: Result<TaskDefinition, _> = serde_json::from_value(for_task_json.clone());

    match result {
        Ok(TaskDefinition::For(for_def)) => {
            assert_eq!(for_def.for_.each, "user");
            assert_eq!(for_def.for_.in_, ".users");
            assert_eq!(for_def.for_.at, Some("index".to_string()));
            assert_eq!(for_def.while_, Some("${ .index < 10 }".to_string()));
            assert_eq!(for_def.do_.entries.len(), 1);
        }
        Ok(TaskDefinition::Do(_)) => {
            panic!("For task incorrectly deserialized as DoTaskDefinition");
        }
        Ok(other) => {
            panic!("For task deserialized as unexpected variant: {:?}", other);
        }
        Err(e) => {
            panic!("Failed to deserialize For task with while: {}", e);
        }
    }
}

#[test]
fn test_for_task_definition() {
    use swf_core::models::task::{ForLoopDefinition, ForTaskDefinition};
    let for_task = ForTaskDefinition::new(
        ForLoopDefinition::new("item", "${items}", Some("index".to_string()), None),
        Map::new(),
        Some("${ .condition }".to_string()),
    );
    let json_str = serde_json::to_string(&for_task).expect("Failed to serialize");
    assert!(json_str.contains("for"));
    assert!(json_str.contains("item"));
    assert!(json_str.contains("items"));
}

#[test]
fn test_for_task_roundtrip() {
    use swf_core::models::task::{ForLoopDefinition, ForTaskDefinition};
    let for_task = ForTaskDefinition::new(
        ForLoopDefinition::new("pet", ".pets", Some("index".to_string()), None),
        Map::new(),
        Some("${ .vet != null }".to_string()),
    );
    let json_str = serde_json::to_string(&for_task).expect("Failed to serialize");
    let deserialized: ForTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(for_task.for_.each, deserialized.for_.each);
}

#[test]
fn test_for_task_with_while_condition_spec() {
    // Test for task with while condition from specification example
    let for_json = json!({
        "for": {
            "each": "pet",
            "in": ".pets",
            "at": "index"
        },
        "while": ".vet != null",
        "do": [
            {
                "waitForCheckup": {
                    "listen": {
                        "to": {
                            "one": {
                                "with": {
                                    "type": "com.fake.petclinic.pets.checkup.completed.v2"
                                }
                            }
                        }
                    },
                    "output": {
                        "as": ".pets + [{ \"id\": $pet.id }]"
                    }
                }
            }
        ]
    });

    let result: Result<swf_core::models::task::ForTaskDefinition, _> =
        serde_json::from_value(for_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize for task with while: {:?}",
        result.err()
    );
}

#[test]
fn test_for_task_without_while() {
    // Test for task without while condition (iterate all items)
    let for_json = json!({
        "for": {
            "each": "item",
            "in": ".items"
        },
        "do": [
            {
                "processItem": {
                    "set": {
                        "processed": true
                    }
                }
            }
        ]
    });

    let result: Result<swf_core::models::task::ForTaskDefinition, _> =
        serde_json::from_value(for_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize for without while: {:?}",
        result.err()
    );
}
