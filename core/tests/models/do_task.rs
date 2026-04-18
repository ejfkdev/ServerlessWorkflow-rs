use super::*;

#[test]
fn test_do_task_deserialization() {
    let do_task_json = json!({
        "do": [
            {
                "step1": {
                    "call": "function1"
                }
            },
            {
                "step2": {
                    "call": "function2"
                }
            }
        ]
    });

    let result: Result<TaskDefinition, _> = serde_json::from_value(do_task_json);

    match result {
        Ok(TaskDefinition::Do(do_def)) => {
            assert_eq!(do_def.do_.entries.len(), 2);
            let has_step1 = do_def.do_.entries.iter().any(|(name, _)| name == "step1");
            let has_step2 = do_def.do_.entries.iter().any(|(name, _)| name == "step2");
            assert!(has_step1, "Do task should contain step1");
            assert!(has_step2, "Do task should contain step2");
        }
        Ok(other) => {
            panic!("Do task deserialized as unexpected variant: {:?}", other);
        }
        Err(e) => {
            panic!("Failed to deserialize Do task: {}", e);
        }
    }
}

#[test]
fn test_do_task_serialization() {
    // Test DoTaskDefinition serialization (similar to Go SDK TestDoTask_MarshalJSON)
    let do_task_json = json!({
        "do": [
            {"task1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}},
            {"task2": {"call": "function2"}}
        ]
    });
    let result: Result<DoTaskDefinition, _> = serde_json::from_value(do_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize do task: {:?}",
        result.err()
    );
    let do_task = result.unwrap();
    assert_eq!(do_task.do_.entries.len(), 2);
}

#[test]
fn test_do_task_roundtrip_serialization() {
    // Test DoTaskDefinition roundtrip serialization
    let mut do_tasks = Map::new();
    do_tasks.add(
        "task1".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            serverless_workflow_core::models::call::CallFunctionDefinition {
                call: "func1".to_string(),
                with: None,
                common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );
    do_tasks.add(
        "task2".to_string(),
        TaskDefinition::Wait(WaitTaskDefinition::new(
            OneOfDurationOrIso8601Expression::Iso8601Expression("PT5S".to_string()),
        )),
    );

    let do_task = DoTaskDefinition::new(do_tasks);
    let json_str = serde_json::to_string(&do_task).expect("Failed to serialize do task");
    let deserialized: DoTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(deserialized.do_.entries.len(), 2);
}

#[test]
fn test_do_task_with_multiple_subtasks() {
    // Test DoTaskDefinition with multiple subtasks
    let do_task_json = json!({
        "do": [
            {"step1": {"call": "function1"}},
            {"step2": {"call": "function2"}},
            {"step3": {"call": "function3"}}
        ]
    });
    let result: Result<DoTaskDefinition, _> = serde_json::from_value(do_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize do task: {:?}",
        result.err()
    );
    let do_task = result.unwrap();
    assert_eq!(do_task.do_.entries.len(), 3);
}

#[test]
fn test_nested_do_task() {
    // Test nested do tasks (do within do)
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "nested-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "outerTask": {
                    "do": [
                        {"innerTask1": {"call": "func1"}},
                        {"innerTask2": {"call": "func2"}}
                    ]
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with nested do: {:?}",
        result.err()
    );
}
