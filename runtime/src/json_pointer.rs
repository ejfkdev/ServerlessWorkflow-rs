use crate::error::{WorkflowError, WorkflowResult};
use serde_json::Value;
#[cfg(test)]
use swf_core::models::workflow::WorkflowDefinition;

/// Generates a JSON Pointer (RFC 6901) to a target node within a workflow definition
#[cfg(test)]
pub fn generate_json_pointer(
    workflow: &WorkflowDefinition,
    target: &str,
) -> WorkflowResult<String> {
    let json_value = serde_json::to_value(workflow).map_err(|e| {
        WorkflowError::runtime(format!("failed to serialize workflow: {}", e), target, "/")
    })?;

    find_json_pointer(&json_value, target, "").ok_or_else(|| {
        WorkflowError::runtime(
            format!("node '{}' not found in workflow", target),
            target,
            "/",
        )
    })
}

/// Generates a JSON Pointer (RFC 6901) to a target node within a pre-serialized JSON value.
/// Use this when the workflow has already been serialized to avoid redundant serialization.
pub fn generate_json_pointer_from_value(
    json_value: &Value,
    target: &str,
) -> WorkflowResult<String> {
    find_json_pointer(json_value, target, "").ok_or_else(|| {
        WorkflowError::runtime(
            format!("node '{}' not found in workflow", target),
            target,
            "/",
        )
    })
}

/// Recursively searches for a target key in a JSON structure and returns the JSON Pointer path
fn find_json_pointer(data: &Value, target: &str, path: &str) -> Option<String> {
    match data {
        Value::Object(map) => {
            for (key, value) in map {
                let new_path = if path.is_empty() {
                    format!("/{}", key)
                } else {
                    format!("{}/{}", path, key)
                };
                if key == target {
                    return Some(new_path);
                }
                if let Some(result) = find_json_pointer(value, target, &new_path) {
                    return Some(result);
                }
            }
        }
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                let new_path = format!("{}/{}", path, i);
                if let Some(result) = find_json_pointer(item, target, &new_path) {
                    return Some(result);
                }
            }
        }
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::make_set_task;
    use serde_json::json;
    use swf_core::models::map::Map;
    use swf_core::models::task::{
        BranchingDefinition, ForkTaskDefinition, TaskDefinition, TaskDefinitionFields,
    };
    use swf_core::models::workflow::WorkflowDefinition;

    fn make_workflow(tasks: Vec<(&str, TaskDefinition)>) -> WorkflowDefinition {
        let entries: Vec<(String, TaskDefinition)> = tasks
            .into_iter()
            .map(|(name, task)| (name.to_string(), task))
            .collect();
        WorkflowDefinition {
            do_: Map { entries },
            ..WorkflowDefinition::default()
        }
    }

    #[test]
    fn test_find_json_pointer_simple() {
        let data = json!({"do": {"task1": {"call": "func"}}});
        let result = find_json_pointer(&data, "task1", "").unwrap();
        assert_eq!(result, "/do/task1");
    }

    #[test]
    fn test_find_json_pointer_nested() {
        let data = json!({"document": {"name": "test"}, "do": {"task1": {}}});
        let result = find_json_pointer(&data, "name", "").unwrap();
        assert_eq!(result, "/document/name");
    }

    #[test]
    fn test_find_json_pointer_not_found() {
        let data = json!({"foo": "bar"});
        let result = find_json_pointer(&data, "missing", "");
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_json_pointer_simple_task() {
        let workflow = make_workflow(vec![
            ("task1", make_set_task("value", 10)),
            ("task2", make_set_task("double", "${ .value * 2 }")),
        ]);
        let result = generate_json_pointer(&workflow, "task2").unwrap();
        assert!(result.contains("task2"));
    }

    #[test]
    fn test_generate_json_pointer_document() {
        let workflow = make_workflow(vec![("task1", make_set_task("value", 10))]);
        let result = generate_json_pointer(&workflow, "name").unwrap();
        assert_eq!(result, "/document/name");
    }

    #[test]
    fn test_generate_json_pointer_fork_task() {
        let mut branches = Map::default();
        branches
            .entries
            .push(("callNurse".to_string(), make_set_task("result", "nurse")));
        branches
            .entries
            .push(("callDoctor".to_string(), make_set_task("result", "doctor")));

        let fork_task = TaskDefinition::Fork(ForkTaskDefinition {
            fork: BranchingDefinition {
                branches,
                compete: true,
            },
            common: TaskDefinitionFields::new(),
        });

        let workflow = make_workflow(vec![("raiseAlarm", fork_task)]);
        let result = generate_json_pointer(&workflow, "callDoctor").unwrap();
        assert!(result.contains("callDoctor"));
    }

    #[test]
    fn test_generate_json_pointer_non_existent() {
        let workflow = make_workflow(vec![("task1", make_set_task("value", 10))]);
        let result = generate_json_pointer(&workflow, "nonExistentTask");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_json_pointer_deep_nested_fork() {
        // Deep nested fork: step1 > branchA > deepTask
        let mut inner_branches = Map::default();
        inner_branches
            .entries
            .push(("deepTask".to_string(), make_set_task("result", "done")));

        let inner_fork = TaskDefinition::Fork(ForkTaskDefinition {
            fork: BranchingDefinition {
                branches: inner_branches,
                compete: false,
            },
            common: TaskDefinitionFields::new(),
        });

        let mut outer_branches = Map::default();
        outer_branches
            .entries
            .push(("branchA".to_string(), inner_fork));

        let outer_fork = TaskDefinition::Fork(ForkTaskDefinition {
            fork: BranchingDefinition {
                branches: outer_branches,
                compete: false,
            },
            common: TaskDefinitionFields::new(),
        });

        let workflow = make_workflow(vec![("step1", outer_fork)]);
        let result = generate_json_pointer(&workflow, "deepTask").unwrap();
        assert!(
            result.contains("deepTask"),
            "deep nested task pointer should contain 'deepTask', got: {}",
            result
        );
    }
}
