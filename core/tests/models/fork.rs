use super::*;

#[test]
fn test_fork_task_serialization() {
    // Test ForkTaskDefinition serialization (similar to Go SDK TestForkTask_MarshalJSON)
    let fork_task_json = json!({
        "fork": {
            "branches": [
                {"task1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}},
                {"task2": {"call": "openapi", "with": {"document": {"name": "doc1", "endpoint": "http://example.com/openapi.json"}, "operationId": "op1"}}}
            ],
            "compete": true
        }
    });
    let result: Result<ForkTaskDefinition, _> = serde_json::from_value(fork_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize fork task: {:?}",
        result.err()
    );
    let fork_task = result.unwrap();
    assert!(fork_task.fork.compete);
    assert_eq!(fork_task.fork.branches.entries.len(), 2);
}

#[test]
fn test_fork_task_roundtrip_serialization() {
    // Test ForkTaskDefinition roundtrip serialization
    let mut branches = Map::new();
    branches.add(
        "task1".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            swf_core::models::call::CallFunctionDefinition {
                call: "func1".to_string(),
                with: None,
                common: swf_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );
    branches.add(
        "task2".to_string(),
        TaskDefinition::Wait(WaitTaskDefinition::new(
            OneOfDurationOrIso8601Expression::Iso8601Expression("PT5S".to_string()),
        )),
    );

    let fork_task = ForkTaskDefinition::new(BranchingDefinition::new(branches, true));

    let json_str = serde_json::to_string(&fork_task).expect("Failed to serialize fork task");
    let deserialized: ForkTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.fork.compete);
    assert_eq!(deserialized.fork.branches.entries.len(), 2);
}

#[test]
fn test_fork_task_with_multiple_branches() {
    // Test ForkTaskDefinition with multiple branches
    let fork_task_json = json!({
        "fork": {
            "branches": [
                {"branch1": {"call": "function1"}},
                {"branch2": {"call": "function2"}},
                {"branch3": {"call": "function3"}}
            ],
            "compete": false
        }
    });
    let result: Result<ForkTaskDefinition, _> = serde_json::from_value(fork_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize fork task: {:?}",
        result.err()
    );
    let fork_task = result.unwrap();
    assert!(!fork_task.fork.compete);
    assert_eq!(fork_task.fork.branches.entries.len(), 3);
}

#[test]
fn test_branching_definition_serialization() {
    // Test BranchingDefinition serialization
    let mut branches = Map::new();
    branches.add(
        "task1".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            swf_core::models::call::CallFunctionDefinition {
                call: "http".to_string(),
                with: None,
                common: swf_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );

    let branching = BranchingDefinition::new(branches, true);
    let json_str = serde_json::to_string(&branching).expect("Failed to serialize branching");
    assert!(json_str.contains("compete"));
    assert!(json_str.contains("branches"));
}

#[test]
fn test_fork_task_definition() {
    use swf_core::models::task::BranchingDefinition;
    use swf_core::models::task::ForkTaskDefinition;
    let fork_task = ForkTaskDefinition {
        fork: BranchingDefinition {
            branches: Map::new(),
            compete: true,
        },
        ..Default::default()
    };
    let json_str = serde_json::to_string(&fork_task).expect("Failed to serialize");
    assert!(json_str.contains("fork"));
}

#[test]
fn test_fork_task_roundtrip() {
    use swf_core::models::task::BranchingDefinition;
    use swf_core::models::task::ForkTaskDefinition;
    let fork_task = ForkTaskDefinition {
        fork: BranchingDefinition {
            branches: Map::new(),
            compete: true,
        },
        ..Default::default()
    };
    let json_str = serde_json::to_string(&fork_task).expect("Failed to serialize");
    let deserialized: ForkTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(fork_task.fork.compete, deserialized.fork.compete);
}

#[test]
fn test_fork_task_with_branches() {
    // Test fork task with multiple branches
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "fork-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "parallelTasks": {
                    "fork": {
                        "branches": [
                            {"task1": {"call": "task1"}},
                            {"task2": {"call": "task2"}},
                            {"task3": {"call": "task3"}}
                        ],
                        "compete": true
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with fork: {:?}",
        result.err()
    );
}

#[test]
fn test_fork_task_branches_construction() {
    // Test ForkTaskDefinition with multiple branches and compete flag
    use swf_core::models::map::Map;
    use swf_core::models::task::{
        BranchingDefinition, CallTaskDefinition, ForkTaskDefinition, TaskDefinition,
    };

    let mut branch1 = Map::new();
    branch1.add(
        "callNurse".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            swf_core::models::call::CallFunctionDefinition {
                call: "http".to_string(),
                with: None,
                common: swf_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );

    let mut branch2 = Map::new();
    branch2.add(
        "callDoctor".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            swf_core::models::call::CallFunctionDefinition {
                call: "http".to_string(),
                with: None,
                common: swf_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );

    let mut branches = Map::new();
    branches.add(
        "branch1".to_string(),
        TaskDefinition::Do(swf_core::models::task::DoTaskDefinition::new(branch1)),
    );
    branches.add(
        "branch2".to_string(),
        TaskDefinition::Do(swf_core::models::task::DoTaskDefinition::new(branch2)),
    );

    let fork = BranchingDefinition::new(branches, true);
    let fork_task = ForkTaskDefinition::new(fork);

    let json_str = serde_json::to_string(&fork_task).expect("Failed to serialize fork task");
    assert!(json_str.contains("fork"));
    assert!(json_str.contains("compete"));
}

#[test]
fn test_fork_task_deserialization() {
    // Test deserialization of fork task from specification example
    let fork_json = json!({
        "fork": {
            "branches": [
                {
                    "callNurse": {
                        "call": "http",
                        "with": {
                            "method": "put",
                            "endpoint": "https://fake-hospital.com/api/v3/alert/nurses"
                        }
                    }
                },
                {
                    "callDoctor": {
                        "call": "http",
                        "with": {
                            "method": "put",
                            "endpoint": "https://fake-hospital.com/api/v3/alert/doctor"
                        }
                    }
                }
            ],
            "compete": true
        }
    });

    let result: Result<ForkTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"fork": fork_json["fork"]}));
    assert!(
        result.is_ok(),
        "Failed to deserialize fork task: {:?}",
        result.err()
    );
}
