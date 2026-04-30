use crate::error::{WorkflowError, WorkflowResult};
use crate::task_runner::{create_task_runner, OwnedTaskSupport, TaskRunner, TaskSupport};
use crate::tasks::task_name_impl;

use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use swf_core::models::task::{ForkTaskDefinition, TaskDefinition};
use swf_core::models::workflow::WorkflowDefinition;

/// Runner for Fork tasks - executes branches concurrently
pub struct ForkTaskRunner {
    name: String,
    compete: bool,
    branch_tasks: Vec<(String, TaskDefinition)>,
    workflow: WorkflowDefinition,
}

impl ForkTaskRunner {
    pub fn new(
        name: &str,
        task: &ForkTaskDefinition,
        workflow: &WorkflowDefinition,
    ) -> WorkflowResult<Self> {
        let compete = task.fork.compete;

        let mut branch_tasks = Vec::new();
        for (branch_name, branch_task) in &task.fork.branches.entries {
            branch_tasks.push((branch_name.to_string(), branch_task.clone()));
        }

        Ok(Self {
            name: name.to_string(),
            compete,
            branch_tasks,
            workflow: workflow.clone(),
        })
    }
}

#[async_trait::async_trait]
impl TaskRunner for ForkTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        if self.branch_tasks.is_empty() {
            return Ok(input);
        }

        if self.compete {
            self.run_compete(input, support).await
        } else {
            self.run_concurrent(input, support).await
        }
    }

    task_name_impl!();
}

impl ForkTaskRunner {
    /// Creates a future that runs a single branch task in an owned context
    fn spawn_branch_future(
        branch_name: String,
        branch_task: TaskDefinition,
        workflow: WorkflowDefinition,
        input: Value,
        support: &mut TaskSupport<'_>,
    ) -> Pin<Box<dyn Future<Output = WorkflowResult<Value>> + Send + 'static>> {
        let owned_support = OwnedTaskSupport::from_support(support);
        Box::pin(async move {
            let runner = create_task_runner(&branch_name, &branch_task, &workflow)?;
            let mut owned = owned_support;
            let mut task_support = owned.as_task_support();
            runner.run(input, &mut task_support).await
        })
    }

    /// Non-compete mode: run all branches concurrently, collect all results
    async fn run_concurrent(
        &self,
        input: Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        let mut handles = Vec::new();

        for (branch_name, branch_task) in &self.branch_tasks {
            let future = Self::spawn_branch_future(
                branch_name.clone(),
                branch_task.clone(),
                self.workflow.clone(),
                input.clone(),
                support,
            );
            handles.push((branch_name.clone(), tokio::spawn(future)));
        }

        let mut results = serde_json::Map::new();
        for (branch_name, handle) in handles {
            let result = handle.await.map_err(|e| {
                WorkflowError::runtime_simple(format!("fork branch panicked: {}", e), &self.name)
            })??;
            results.insert(branch_name, result);
        }

        Ok(Value::Object(results))
    }

    /// Compete mode: first branch to complete successfully wins, others are cancelled
    async fn run_compete(
        &self,
        input: Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        let mut set = tokio::task::JoinSet::new();

        for (branch_name, branch_task) in &self.branch_tasks {
            let future = Self::spawn_branch_future(
                branch_name.clone(),
                branch_task.clone(),
                self.workflow.clone(),
                input.clone(),
                support,
            );
            set.spawn(future);
        }

        while let Some(result) = set.join_next().await {
            match result {
                Ok(Ok(value)) => {
                    // Winner found - abort all remaining branches
                    set.abort_all();
                    return Ok(value);
                }
                Ok(Err(_)) | Err(_) => {
                    // This branch failed, continue waiting for others
                    continue;
                }
            }
        }

        // All branches failed
        Err(WorkflowError::runtime_simple(
            "all fork branches failed",
            &self.name,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_support;
    use crate::test_utils::test_helpers::make_set_task;
    use serde_json::json;
    use std::collections::HashMap;
    use swf_core::models::map::Map;
    use swf_core::models::task::{
        BranchingDefinition, ForkTaskDefinition, SetTaskDefinition, SetValue, TaskDefinitionFields,
    };

    fn make_workflow_with_fork(
        compete: bool,
        branches: Map<String, TaskDefinition>,
    ) -> WorkflowDefinition {
        let fork_task = ForkTaskDefinition {
            fork: BranchingDefinition { branches, compete },
            common: TaskDefinitionFields::new(),
        };
        let do_entries = vec![("forkTask".to_string(), TaskDefinition::Fork(fork_task))];

        WorkflowDefinition {
            do_: Map {
                entries: do_entries,
            },
            ..WorkflowDefinition::default()
        }
    }

    #[tokio::test]
    async fn test_fork_non_compete() {
        let mut branches = Map::default();
        branches
            .entries
            .push(("branch1".to_string(), make_set_task("result", "r1")));
        branches
            .entries
            .push(("branch2".to_string(), make_set_task("result", "r2")));

        let workflow = make_workflow_with_fork(false, branches);
        default_support!(workflow, context, support);

        for (name, task_def) in &workflow.do_.entries {
            if let TaskDefinition::Fork(ref fork_task) = task_def {
                let runner = ForkTaskRunner::new(name, fork_task, &workflow).unwrap();
                let output = runner.run(json!({}), &mut support).await.unwrap();
                // Non-compete with multiple branches returns object with branch names as keys
                assert!(output.is_object());
                assert_eq!(output.as_object().unwrap().len(), 2);
            }
        }
    }

    #[tokio::test]
    async fn test_fork_compete_single_branch() {
        let mut branches = Map::default();
        branches
            .entries
            .push(("fast".to_string(), make_set_task("winner", "fast")));

        let workflow = make_workflow_with_fork(true, branches);
        default_support!(workflow, context, support);

        for (name, task_def) in &workflow.do_.entries {
            if let TaskDefinition::Fork(ref fork_task) = task_def {
                let runner = ForkTaskRunner::new(name, fork_task, &workflow).unwrap();
                let output = runner.run(json!({}), &mut support).await.unwrap();
                assert_eq!(output["winner"], json!("fast"));
            }
        }
    }

    #[tokio::test]
    async fn test_fork_empty_branches() {
        let branches = Map::default();
        let workflow = make_workflow_with_fork(false, branches);
        default_support!(workflow, context, support);

        for (name, task_def) in &workflow.do_.entries {
            if let TaskDefinition::Fork(ref fork_task) = task_def {
                let runner = ForkTaskRunner::new(name, fork_task, &workflow).unwrap();
                let output = runner
                    .run(json!({"input": "data"}), &mut support)
                    .await
                    .unwrap();
                assert_eq!(output["input"], json!("data"));
            }
        }
    }

    #[tokio::test]
    async fn test_fork_compete_multiple_branches() {
        let mut branches = Map::default();
        branches
            .entries
            .push(("branch1".to_string(), make_set_task("winner", "b1")));
        branches
            .entries
            .push(("branch2".to_string(), make_set_task("winner", "b2")));

        let workflow = make_workflow_with_fork(true, branches);
        default_support!(workflow, context, support);

        for (name, task_def) in &workflow.do_.entries {
            if let TaskDefinition::Fork(ref fork_task) = task_def {
                let runner = ForkTaskRunner::new(name, fork_task, &workflow).unwrap();
                let output = runner.run(json!({}), &mut support).await.unwrap();
                // With compete, one branch wins
                assert_eq!(output["winner"], json!("b1"));
            }
        }
    }

    #[tokio::test]
    async fn test_fork_non_compete_single_branch() {
        let mut branches = Map::default();
        branches
            .entries
            .push(("only".to_string(), make_set_task("value", "42")));

        let workflow = make_workflow_with_fork(false, branches);
        default_support!(workflow, context, support);

        for (name, task_def) in &workflow.do_.entries {
            if let TaskDefinition::Fork(ref fork_task) = task_def {
                let runner = ForkTaskRunner::new(name, fork_task, &workflow).unwrap();
                let output = runner.run(json!({}), &mut support).await.unwrap();
                // Single branch returns its output with branch name as key
                assert_eq!(output["only"]["value"], json!("42"));
            }
        }
    }

    #[tokio::test]
    async fn test_fork_non_compete_with_wait() {
        // Matches Java SDK's fork-wait.yaml - non-compete with wait tasks in branches
        use swf_core::models::duration::{Duration, OneOfDurationOrIso8601Expression};
        use swf_core::models::map::Map as CoreMap;
        use swf_core::models::task::{DoTaskDefinition, WaitTaskDefinition};

        // Branch 1: wait + set value=1
        let wait1 = TaskDefinition::Wait(WaitTaskDefinition {
            wait: OneOfDurationOrIso8601Expression::Duration(Duration::from_milliseconds(50)),
            common: TaskDefinitionFields::new(),
        });
        let set1 = make_set_task("value", "1");
        let do_entries1 = vec![
            ("waitABit".to_string(), wait1),
            ("setVal".to_string(), set1),
        ];
        let do_task1 = TaskDefinition::Do(DoTaskDefinition {
            do_: CoreMap {
                entries: do_entries1,
            },
            common: TaskDefinitionFields::new(),
        });

        // Branch 2: wait + set value=2
        let wait2 = TaskDefinition::Wait(WaitTaskDefinition {
            wait: OneOfDurationOrIso8601Expression::Duration(Duration::from_milliseconds(50)),
            common: TaskDefinitionFields::new(),
        });
        let set2 = make_set_task("value", "2");
        let do_entries2 = vec![
            ("waitABit".to_string(), wait2),
            ("setVal".to_string(), set2),
        ];
        let do_task2 = TaskDefinition::Do(DoTaskDefinition {
            do_: CoreMap {
                entries: do_entries2,
            },
            common: TaskDefinitionFields::new(),
        });

        let mut branches = Map::default();
        branches.entries.push(("helloBranch".to_string(), do_task1));
        branches.entries.push(("byeBranch".to_string(), do_task2));

        let workflow = make_workflow_with_fork(false, branches);
        default_support!(workflow, context, support);

        for (name, task_def) in &workflow.do_.entries {
            if let TaskDefinition::Fork(ref fork_task) = task_def {
                let runner = ForkTaskRunner::new(name, fork_task, &workflow).unwrap();
                let output = runner.run(json!({}), &mut support).await.unwrap();
                // Non-compete with multiple branches returns object with branch names as keys
                assert!(output.is_object());
                assert_eq!(output.as_object().unwrap().len(), 2);
            }
        }
    }

    #[tokio::test]
    async fn test_fork_no_compete_multiple_branches_with_do() {
        // Matches Java SDK's fork-no-compete.yaml - non-compete with multiple do branches
        use swf_core::models::duration::{Duration, OneOfDurationOrIso8601Expression};
        use swf_core::models::map::Map as CoreMap;
        use swf_core::models::task::{DoTaskDefinition, WaitTaskDefinition};

        // Branch 1: wait + set
        let wait1 = TaskDefinition::Wait(WaitTaskDefinition {
            wait: OneOfDurationOrIso8601Expression::Duration(Duration::from_milliseconds(50)),
            common: TaskDefinitionFields::new(),
        });
        let mut set_map1 = HashMap::new();
        set_map1.insert("patientId".to_string(), json!("John"));
        set_map1.insert("room".to_string(), json!(1));
        let set1 = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map1),
            common: TaskDefinitionFields::new(),
        });
        let do_entries1 = vec![
            ("waitForNurse".to_string(), wait1),
            ("nurseArrived".to_string(), set1),
        ];
        let do_task1 = TaskDefinition::Do(DoTaskDefinition {
            do_: CoreMap {
                entries: do_entries1,
            },
            common: TaskDefinitionFields::new(),
        });

        // Branch 2: wait + set
        let wait2 = TaskDefinition::Wait(WaitTaskDefinition {
            wait: OneOfDurationOrIso8601Expression::Duration(Duration::from_milliseconds(50)),
            common: TaskDefinitionFields::new(),
        });
        let mut set_map2 = HashMap::new();
        set_map2.insert("patientId".to_string(), json!("Smith"));
        set_map2.insert("room".to_string(), json!(2));
        let set2 = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map2),
            common: TaskDefinitionFields::new(),
        });
        let do_entries2 = vec![
            ("waitForDoctor".to_string(), wait2),
            ("doctorArrived".to_string(), set2),
        ];
        let do_task2 = TaskDefinition::Do(DoTaskDefinition {
            do_: CoreMap {
                entries: do_entries2,
            },
            common: TaskDefinitionFields::new(),
        });

        let mut branches = Map::default();
        branches.entries.push(("callNurse".to_string(), do_task1));
        branches.entries.push(("callDoctor".to_string(), do_task2));

        let workflow = make_workflow_with_fork(false, branches);
        default_support!(workflow, context, support);

        for (name, task_def) in &workflow.do_.entries {
            if let TaskDefinition::Fork(ref fork_task) = task_def {
                let runner = ForkTaskRunner::new(name, fork_task, &workflow).unwrap();
                let output = runner.run(json!({}), &mut support).await.unwrap();
                assert!(output.is_object());
                assert_eq!(output.as_object().unwrap().len(), 2);
            }
        }
    }
}
