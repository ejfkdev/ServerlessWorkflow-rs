use crate::error::{WorkflowError, WorkflowResult};
use crate::listener::WorkflowEvent;
use crate::status::StatusPhase;
use crate::task_runner::{create_task_runner, TaskRunner, TaskSupport};
use crate::tasks::task_name_impl;

use serde_json::Value;
use std::collections::HashMap;
use swf_core::models::map::Map;
use swf_core::models::task::{
    DoTaskDefinition, SwitchTaskDefinition, TaskDefinition, TaskDefinitionFields,
};
use swf_core::models::workflow::WorkflowDefinition;

/// Flow directive returned after running a task
enum FlowDirective {
    /// Continue to the next task in sequence
    Continue,
    /// Jump to a specific task by name
    Goto(String),
    /// End the workflow (stop executing tasks)
    End,
    /// Exit the current composite task
    Exit,
}

impl FlowDirective {
    fn from_then(then: &str) -> Self {
        match then {
            "end" => FlowDirective::End,
            "exit" => FlowDirective::Exit,
            "continue" => FlowDirective::Continue,
            task_name => FlowDirective::Goto(task_name.to_string()),
        }
    }
}

/// Runner for Do tasks - executes subtasks sequentially
pub struct DoTaskRunner {
    name: String,
    tasks: Map<String, TaskDefinition>,
}

impl DoTaskRunner {
    /// Creates a new DoTaskRunner from a DoTaskDefinition
    pub fn new(name: &str, task: &DoTaskDefinition) -> WorkflowResult<Self> {
        Ok(Self {
            name: name.to_string(),
            tasks: task.do_.clone(),
        })
    }

    /// Creates a DoTaskRunner from the workflow's top-level do tasks
    pub fn new_from_workflow(workflow: &WorkflowDefinition) -> WorkflowResult<Self> {
        let name = workflow.document.name.clone();
        Ok(Self {
            name,
            tasks: workflow.do_.clone(),
        })
    }

    /// Runs all tasks with flow directive support (then/switch jump)
    pub async fn run_tasks(
        &self,
        input: Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        let mut output = input;
        let mut index: usize = 0;

        while index < self.tasks.entries.len() {
            let (name, task) = &self.tasks.entries[index];

            // Set task context
            let task_value = crate::error::serialize_to_value(task, "task", name)?;
            support.set_task_def(&task_value);
            support.set_task_reference_from_name(name)?;

            // Check if condition
            let if_condition = get_if_condition(task);
            if !support.should_run_task(if_condition, &output)? {
                index += 1;
                continue;
            }

            // Suspend check: wait if workflow is suspended
            if support.context.is_suspended() {
                support.set_task_status(name, StatusPhase::Suspended);
                support.emit_event(WorkflowEvent::TaskSuspended {
                    instance_id: support.context.instance_id().to_string(),
                    task_name: name.to_string(),
                });
                support.context.wait_for_resume().await;
                if support.context.is_cancelled() {
                    return Err(WorkflowError::runtime_simple(
                        "workflow cancelled while suspended",
                        name,
                    ));
                }
            }

            support.set_task_status(name, StatusPhase::Pending);

            let task_type = task_type_str(task);
            let task_span = tracing::info_span!(
                "task",
                name = %name,
                type = %task_type,
            );
            let _task_enter = task_span.enter();

            // Run extension before tasks
            run_extension_before_tasks(task, &output, support).await?;

            // Restore main task context after before extensions
            let task_value =
                crate::error::serialize_to_value(task, "task", name)?;
            support.set_task_def(&task_value);
            support.set_task_reference_from_name(name)?;

            // Determine flow directive from task execution
            let directive = if let TaskDefinition::Switch(switch_task) = task {
                // Switch tasks: evaluate conditions to get then directive
                let common = &switch_task.common;
                support.set_task_status(name, StatusPhase::Running);

                // Process input for switch
                let task_input =
                    support.process_task_input(common.input.as_ref(), &output, name)?;

                // Evaluate switch conditions
                let then_str = self
                    .evaluate_switch(&task_input, support, name, switch_task)
                    .await?;

                // Switch output is the input (switch doesn't transform data)
                output = support
                    .execute_task_lifecycle(name, common, &output, task_input)
                    .await?;

                FlowDirective::from_then(&then_str)
            } else {
                // Regular tasks: run the task, then check its `then` field
                let runner = create_task_runner(name, task, support.workflow)?;
                support.set_task_status(name, StatusPhase::Running);

                let common = task.common_fields();
                output = self
                    .run_single_task(&output, support, &*runner, common)
                    .await?;

                support.set_task_status(name, StatusPhase::Completed);
                tracing::debug!(task = %name, type = %task_type, "task completed");

                // Check the task's `then` directive
                match common.then.as_deref() {
                    Some(then) => FlowDirective::from_then(then),
                    None => FlowDirective::Continue,
                }
            };

            // Run extension after tasks
            run_extension_after_tasks(task, &output, support).await?;

            // Apply flow directive
            match directive {
                FlowDirective::Continue => {
                    index += 1;
                }
                FlowDirective::End | FlowDirective::Exit => {
                    break;
                }
                FlowDirective::Goto(target) => match self.find_task_index(&target) {
                    Some(target_index) => {
                        index = target_index;
                    }
                    None => {
                        return Err(WorkflowError::runtime_simple(
                            format!("switch/goto target '{}' not found in task list", target),
                            &self.name,
                        ));
                    }
                },
            }
        }

        Ok(output)
    }

    /// Finds the index of a task by name in the task list
    fn find_task_index(&self, target: &str) -> Option<usize> {
        self.tasks
            .entries
            .iter()
            .position(|(name, _)| name == target)
    }

    /// Runs a single task with input/output/export/timeout processing
    async fn run_single_task(
        &self,
        input: &Value,
        support: &mut TaskSupport<'_>,
        runner: &dyn TaskRunner,
        common: &TaskDefinitionFields,
    ) -> WorkflowResult<Value> {
        let raw_output = support
            .run_task_with_input_and_timeout(runner.task_name(), common, input, runner)
            .await?;
        support
            .execute_task_lifecycle(runner.task_name(), common, input, raw_output)
            .await
    }

    /// Evaluates a switch task and returns the matched then directive
    async fn evaluate_switch(
        &self,
        input: &Value,
        support: &TaskSupport<'_>,
        task_name: &str,
        switch_task: &SwitchTaskDefinition,
    ) -> WorkflowResult<String> {
        let mut default_then: Option<String> = None;

        for (_case_name, case_def) in &switch_task.switch.entries {
            match &case_def.when {
                None => {
                    // Default case
                    if let Some(ref then) = case_def.then {
                        default_then = Some(then.clone());
                    }
                }
                Some(when_expr) => {
                    let result = support
                        .eval_bool(when_expr, input)
                        .map_err(|e| WorkflowError::expression(format!("{}", e), task_name))?;
                    if result {
                        return case_def.then.clone().ok_or_else(|| {
                            WorkflowError::expression(
                                "missing 'then' directive in matched switch case",
                                task_name,
                            )
                        });
                    }
                }
            }
        }

        // No matching case and no default: pass through (continue to next task)
        Ok(default_then.unwrap_or_else(|| "continue".to_string()))
    }
}

#[async_trait::async_trait]
impl TaskRunner for DoTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        self.run_tasks(input, support).await
    }

    task_name_impl!();
}

/// Extracts the `if` condition from a TaskDefinition
fn get_if_condition(task: &TaskDefinition) -> Option<&str> {
    task.common_fields().if_.as_deref()
}

/// Returns the task type strings that this task matches for extension purposes.
/// Every task matches "all"; composite tasks also match "composite".
fn task_type_str(task: &TaskDefinition) -> &'static str {
    match task {
        TaskDefinition::Call(_) => "call",
        TaskDefinition::Set(_) => "set",
        TaskDefinition::Wait(_) => "wait",
        TaskDefinition::Raise(_) => "raise",
        TaskDefinition::Emit(_) => "emit",
        TaskDefinition::Listen(_) => "listen",
        TaskDefinition::Run(_) => "run",
        TaskDefinition::Switch(_) => "switch",
        TaskDefinition::Try(_) => "try",
        TaskDefinition::For(_) => "for",
        TaskDefinition::Do(_) => "do",
        TaskDefinition::Fork(_) => "fork",
        TaskDefinition::Custom(_) => "custom",
    }
}

/// Returns the task type strings that this task matches for extension purposes.
/// Every task matches "all"; composite tasks also match "composite".
fn task_extension_types(task: &TaskDefinition) -> Vec<String> {
    let mut types = match task {
        TaskDefinition::Call(_) => vec!["call".to_string()],
        TaskDefinition::Set(_) => vec!["set".to_string()],
        TaskDefinition::Wait(_) => vec!["wait".to_string()],
        TaskDefinition::Raise(_) => vec!["raise".to_string()],
        TaskDefinition::Emit(_) => vec!["emit".to_string()],
        TaskDefinition::Listen(_) => vec!["listen".to_string()],
        TaskDefinition::Run(_) => vec!["run".to_string()],
        TaskDefinition::Switch(_) => vec!["switch".to_string(), "composite".to_string()],
        TaskDefinition::Try(_) => vec!["try".to_string(), "composite".to_string()],
        TaskDefinition::For(_) => vec!["for".to_string(), "composite".to_string()],
        TaskDefinition::Do(_) => vec!["do".to_string(), "composite".to_string()],
        TaskDefinition::Fork(_) => vec!["fork".to_string(), "composite".to_string()],
        TaskDefinition::Custom(t) => {
            let mut v = vec!["custom".to_string()];
            if let Some(ref type_name) = t.type_ {
                v.push(type_name.clone());
            }
            v
        }
    };
    types.push("all".to_string());
    types
}

/// Runs a list of extension tasks sequentially, preserving side effects (export, events)
/// but discarding their output so the main data flow is not affected.
async fn run_extension_task_list(
    tasks: &[HashMap<String, TaskDefinition>],
    input: &Value,
    support: &mut TaskSupport<'_>,
) -> WorkflowResult<()> {
    for task_map in tasks {
        for (name, task_def) in task_map {
            let runner = create_task_runner(name, task_def, support.workflow)?;
            let common = task_def.common_fields();
            let raw_output = support
                .run_task_with_input_and_timeout(name, common, input, &*runner)
                .await?;
            let _ = support
                .execute_task_lifecycle(name, common, input, raw_output)
                .await?;
        }
    }
    Ok(())
}

/// Runs extension before tasks for matching extensions.
/// Before tasks' output is discarded — the main task receives the original input.
async fn run_extension_before_tasks(
    task: &TaskDefinition,
    input: &Value,
    support: &mut TaskSupport<'_>,
) -> WorkflowResult<()> {
    let extensions = match support
        .workflow
        .use_
        .as_ref()
        .and_then(|u| u.extensions.as_ref())
    {
        Some(exts) => exts,
        None => return Ok(()),
    };

    let task_types = task_extension_types(task);

    for ext_map in extensions {
        for ext in ext_map.values() {
            if !task_types.iter().any(|t| t == &ext.extend) {
                continue;
            }
            if let Some(ref when) = ext.when {
                if !support.eval_bool(when, input).unwrap_or(false) {
                    continue;
                }
            }
            if let Some(ref before) = ext.before {
                run_extension_task_list(before, input, support).await?;
            }
        }
    }
    Ok(())
}

/// Runs extension after tasks for matching extensions.
/// After tasks receive the main task's output but their output is discarded.
async fn run_extension_after_tasks(
    task: &TaskDefinition,
    output: &Value,
    support: &mut TaskSupport<'_>,
) -> WorkflowResult<()> {
    let extensions = match support
        .workflow
        .use_
        .as_ref()
        .and_then(|u| u.extensions.as_ref())
    {
        Some(exts) => exts,
        None => return Ok(()),
    };

    let task_types = task_extension_types(task);

    for ext_map in extensions {
        for ext in ext_map.values() {
            if !task_types.iter().any(|t| t == &ext.extend) {
                continue;
            }
            if let Some(ref when) = ext.when {
                if !support.eval_bool(when, output).unwrap_or(false) {
                    continue;
                }
            }
            if let Some(ref after) = ext.after {
                run_extension_task_list(after, output, support).await?;
            }
        }
    }
    Ok(())
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
        SetTaskDefinition, SetValue, SwitchCaseDefinition, SwitchTaskDefinition,
        TaskDefinitionFields,
    };
    use swf_core::models::workflow::WorkflowDefinition;

    fn make_do_runner(tasks: Vec<(&str, TaskDefinition)>) -> DoTaskRunner {
        let entries: Vec<(String, TaskDefinition)> = tasks
            .into_iter()
            .map(|(name, task)| (name.to_string(), task))
            .collect();
        let do_def = swf_core::models::task::DoTaskDefinition::new(Map { entries });
        DoTaskRunner::new("testDo", &do_def).unwrap()
    }

    #[tokio::test]
    async fn test_do_sequential_execution() {
        // Set tasks replace the output; chain via expressions referencing prior values
        let runner = make_do_runner(vec![
            ("task1", make_set_task("a", json!(1))),
            ("task2", make_set_task("b", json!("${ .a + 1 }"))),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // task2 output replaces task1 output, but references .a from task1
        assert_eq!(output["b"], json!(2));
    }

    #[tokio::test]
    async fn test_do_with_if_condition_skip() {
        let mut skip_task = make_set_task("skipped", json!(true));
        if let TaskDefinition::Set(ref mut s) = skip_task {
            s.common.if_ = Some("${ .run == true }".to_string());
        }

        let runner = make_do_runner(vec![
            ("task1", make_set_task("a", json!(1))),
            ("task2", skip_task),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // run=false (in input), so task2 should be skipped; output stays from task1
        let output = runner
            .run(json!({"run": false}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["a"], json!(1));
        assert!(output.get("skipped").is_none());
    }

    #[tokio::test]
    async fn test_do_with_if_condition_execute() {
        let mut exec_task = make_set_task("executed", json!(true));
        if let TaskDefinition::Set(ref mut s) = exec_task {
            s.common.if_ = Some("${ .run == true }".to_string());
        }

        let runner = make_do_runner(vec![
            ("task1", make_set_task("run", json!(true))),
            ("task2", exec_task),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // task1 sets run=true, so task2 should execute
        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["executed"], json!(true));
    }

    #[tokio::test]
    async fn test_do_with_then_end() {
        let mut end_task = make_set_task("final", json!(42));
        if let TaskDefinition::Set(ref mut s) = end_task {
            s.common.then = Some("end".to_string());
        }

        let runner = make_do_runner(vec![
            ("task1", end_task),
            ("task2", make_set_task("skipped", json!(true))),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["final"], json!(42));
        assert!(output.get("skipped").is_none());
    }

    #[tokio::test]
    async fn test_do_with_then_goto() {
        let mut goto_task = make_set_task("start", json!(1));
        if let TaskDefinition::Set(ref mut s) = goto_task {
            s.common.then = Some("task3".to_string());
        }

        let runner = make_do_runner(vec![
            ("task1", goto_task),
            ("task2", make_set_task("skipped", json!(true))),
            ("task3", make_set_task("end", json!(99))),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // Set replaces output, so task3's set overwrites task1's set
        assert!(output.get("skipped").is_none());
        assert_eq!(output["end"], json!(99));
    }

    // --- Switch tests matching Java/Go SDK patterns ---

    #[tokio::test]
    async fn test_switch_then_loop() {
        // Matches Java SDK's switch-then-loop.yaml
        // inc: set count+1, then goto looping
        // looping: switch - if count<6 goto inc, else "exit" (flow directive)
        let mut inc_task = make_set_task("count", json!("${ .count + 1 }"));
        if let TaskDefinition::Set(ref mut s) = inc_task {
            s.common.then = Some("looping".to_string());
        }

        let switch_task = TaskDefinition::Switch(SwitchTaskDefinition {
            switch: Map {
                entries: vec![
                    (
                        "loopCount".to_string(),
                        SwitchCaseDefinition {
                            when: Some(".count < 6".to_string()),
                            then: Some("inc".to_string()),
                        },
                    ),
                    (
                        "default".to_string(),
                        SwitchCaseDefinition {
                            when: None,
                            then: Some("exit".to_string()),
                        },
                    ),
                ],
            },
            common: TaskDefinitionFields::new(),
        });

        let runner = make_do_runner(vec![("inc", inc_task), ("looping", switch_task)]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({"count": 0}), &mut support).await.unwrap();
        // Loop runs: count 0→1→2→3→4→5→6 (count<6 fails at 6, exits)
        assert_eq!(output["count"], json!(6));
    }

    #[tokio::test]
    async fn test_switch_then_string() {
        // Matches Java SDK's switch-then-string.yaml
        // processOrder switch → processElectronicOrder / processPhysicalOrder / handleUnknownOrderType
        // "exit" and "end" are flow directives, not task names
        let switch_task = TaskDefinition::Switch(SwitchTaskDefinition {
            switch: Map {
                entries: vec![
                    (
                        "case1".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".orderType == "electronic""#.to_string()),
                            then: Some("processElectronicOrder".to_string()),
                        },
                    ),
                    (
                        "case2".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".orderType == "physical""#.to_string()),
                            then: Some("processPhysicalOrder".to_string()),
                        },
                    ),
                    (
                        "default".to_string(),
                        SwitchCaseDefinition {
                            when: None,
                            then: Some("handleUnknownOrderType".to_string()),
                        },
                    ),
                ],
            },
            common: TaskDefinitionFields::new(),
        });

        // processElectronicOrder: set validate=true, status=fulfilled, then: exit (flow directive)
        let mut electronic_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("validate".to_string(), json!(true));
                m.insert("status".to_string(), json!("fulfilled"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });
        if let TaskDefinition::Set(ref mut s) = electronic_task {
            s.common.then = Some("exit".to_string());
        }

        // processPhysicalOrder: set inventory, items, address, then: exit
        let mut physical_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("inventory".to_string(), json!("clear"));
                m.insert("items".to_string(), json!(1));
                m.insert("address".to_string(), json!("Elmer St"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });
        if let TaskDefinition::Set(ref mut s) = physical_task {
            s.common.then = Some("exit".to_string());
        }

        // handleUnknownOrderType: set log=warn, message (no then, continues sequentially)
        let unknown_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("log".to_string(), json!("warn"));
                m.insert("message".to_string(), json!("something's wrong"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });

        let runner = make_do_runner(vec![
            ("processOrder", switch_task),
            ("processElectronicOrder", electronic_task),
            ("processPhysicalOrder", physical_task),
            ("handleUnknownOrderType", unknown_task),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // Test electronic order path
        let output = runner
            .run(json!({"orderType": "electronic"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["validate"], json!(true));
        assert_eq!(output["status"], json!("fulfilled"));
        assert!(output.get("inventory").is_none());
    }

    #[tokio::test]
    async fn test_switch_then_string_physical() {
        // Same workflow as above, testing physical order path
        let switch_task = TaskDefinition::Switch(SwitchTaskDefinition {
            switch: Map {
                entries: vec![
                    (
                        "case1".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".orderType == "electronic""#.to_string()),
                            then: Some("processElectronicOrder".to_string()),
                        },
                    ),
                    (
                        "case2".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".orderType == "physical""#.to_string()),
                            then: Some("processPhysicalOrder".to_string()),
                        },
                    ),
                    (
                        "default".to_string(),
                        SwitchCaseDefinition {
                            when: None,
                            then: Some("handleUnknownOrderType".to_string()),
                        },
                    ),
                ],
            },
            common: TaskDefinitionFields::new(),
        });

        let mut electronic_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("validate".to_string(), json!(true));
                m.insert("status".to_string(), json!("fulfilled"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });
        if let TaskDefinition::Set(ref mut s) = electronic_task {
            s.common.then = Some("exit".to_string());
        }

        let mut physical_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("inventory".to_string(), json!("clear"));
                m.insert("items".to_string(), json!(1));
                m.insert("address".to_string(), json!("Elmer St"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });
        if let TaskDefinition::Set(ref mut s) = physical_task {
            s.common.then = Some("exit".to_string());
        }

        let unknown_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("log".to_string(), json!("warn"));
                m.insert("message".to_string(), json!("something's wrong"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });

        let runner = make_do_runner(vec![
            ("processOrder", switch_task),
            ("processElectronicOrder", electronic_task),
            ("processPhysicalOrder", physical_task),
            ("handleUnknownOrderType", unknown_task),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // Test physical order path
        let output = runner
            .run(json!({"orderType": "physical"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["inventory"], json!("clear"));
        assert_eq!(output["items"], json!(1));
        assert_eq!(output["address"], json!("Elmer St"));
        assert!(output.get("validate").is_none());
    }

    #[tokio::test]
    async fn test_switch_then_string_default() {
        // Same workflow, testing default/unknown order path
        let switch_task = TaskDefinition::Switch(SwitchTaskDefinition {
            switch: Map {
                entries: vec![
                    (
                        "case1".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".orderType == "electronic""#.to_string()),
                            then: Some("processElectronicOrder".to_string()),
                        },
                    ),
                    (
                        "case2".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".orderType == "physical""#.to_string()),
                            then: Some("processPhysicalOrder".to_string()),
                        },
                    ),
                    (
                        "default".to_string(),
                        SwitchCaseDefinition {
                            when: None,
                            then: Some("handleUnknownOrderType".to_string()),
                        },
                    ),
                ],
            },
            common: TaskDefinitionFields::new(),
        });

        let mut electronic_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("validate".to_string(), json!(true));
                m.insert("status".to_string(), json!("fulfilled"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });
        if let TaskDefinition::Set(ref mut s) = electronic_task {
            s.common.then = Some("exit".to_string());
        }

        let mut physical_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("inventory".to_string(), json!("clear"));
                m.insert("items".to_string(), json!(1));
                m.insert("address".to_string(), json!("Elmer St"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });
        if let TaskDefinition::Set(ref mut s) = physical_task {
            s.common.then = Some("exit".to_string());
        }

        let unknown_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("log".to_string(), json!("warn"));
                m.insert("message".to_string(), json!("something's wrong"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });

        let runner = make_do_runner(vec![
            ("processOrder", switch_task),
            ("processElectronicOrder", electronic_task),
            ("processPhysicalOrder", physical_task),
            ("handleUnknownOrderType", unknown_task),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // Test unknown order type path (falls through to default)
        let output = runner
            .run(json!({"orderType": "digital"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["log"], json!("warn"));
        assert_eq!(output["message"], json!("something's wrong"));
    }

    #[tokio::test]
    async fn test_switch_match_color() {
        // Matches Go SDK's switch_match.yaml
        // switchColor → red/green/blue cases, each appends to colors array, then end
        let switch_task = TaskDefinition::Switch(SwitchTaskDefinition {
            switch: Map {
                entries: vec![
                    (
                        "red".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".color == "red""#.to_string()),
                            then: Some("setRed".to_string()),
                        },
                    ),
                    (
                        "green".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".color == "green""#.to_string()),
                            then: Some("setGreen".to_string()),
                        },
                    ),
                    (
                        "blue".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".color == "blue""#.to_string()),
                            then: Some("setBlue".to_string()),
                        },
                    ),
                ],
            },
            common: TaskDefinitionFields::new(),
        });

        let mut set_red = make_set_task("colors", json!("${ .colors + [\"red\"] }"));
        if let TaskDefinition::Set(ref mut s) = set_red {
            s.common.then = Some("end".to_string());
        }
        let mut set_green = make_set_task("colors", json!("${ .colors + [\"green\"] }"));
        if let TaskDefinition::Set(ref mut s) = set_green {
            s.common.then = Some("end".to_string());
        }
        let mut set_blue = make_set_task("colors", json!("${ .colors + [\"blue\"] }"));
        if let TaskDefinition::Set(ref mut s) = set_blue {
            s.common.then = Some("end".to_string());
        }

        let runner = make_do_runner(vec![
            ("switchColor", switch_task),
            ("setRed", set_red),
            ("setGreen", set_green),
            ("setBlue", set_blue),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // Test red path
        let output = runner
            .run(json!({"color": "red", "colors": []}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["colors"], json!(["red"]));
    }

    #[tokio::test]
    async fn test_switch_with_default_fallback() {
        // Matches Go SDK's switch_with_default.yaml
        // switchColor → red/green/fallback cases
        let switch_task = TaskDefinition::Switch(SwitchTaskDefinition {
            switch: Map {
                entries: vec![
                    (
                        "red".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".color == "red""#.to_string()),
                            then: Some("setRed".to_string()),
                        },
                    ),
                    (
                        "green".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".color == "green""#.to_string()),
                            then: Some("setGreen".to_string()),
                        },
                    ),
                    (
                        "fallback".to_string(),
                        SwitchCaseDefinition {
                            when: None,
                            then: Some("setDefault".to_string()),
                        },
                    ),
                ],
            },
            common: TaskDefinitionFields::new(),
        });

        let mut set_red = make_set_task("colors", json!("${ .colors + [\"red\"] }"));
        if let TaskDefinition::Set(ref mut s) = set_red {
            s.common.then = Some("end".to_string());
        }
        let mut set_green = make_set_task("colors", json!("${ .colors + [\"green\"] }"));
        if let TaskDefinition::Set(ref mut s) = set_green {
            s.common.then = Some("end".to_string());
        }
        let mut set_default = make_set_task("colors", json!("${ .colors + [\"default\"] }"));
        if let TaskDefinition::Set(ref mut s) = set_default {
            s.common.then = Some("end".to_string());
        }

        let runner = make_do_runner(vec![
            ("switchColor", switch_task),
            ("setRed", set_red),
            ("setGreen", set_green),
            ("setDefault", set_default),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // Test fallback (no matching case)
        let output = runner
            .run(json!({"color": "yellow", "colors": []}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["colors"], json!(["default"]));
    }

    #[tokio::test]
    async fn test_switch_no_match_continues() {
        // Switch with no matching case and no default should continue to next task
        let switch_task = TaskDefinition::Switch(SwitchTaskDefinition {
            switch: Map {
                entries: vec![(
                    "red".to_string(),
                    SwitchCaseDefinition {
                        when: Some(r#".color == "red""#.to_string()),
                        then: Some("setRed".to_string()),
                    },
                )],
            },
            common: TaskDefinitionFields::new(),
        });

        let set_red = make_set_task("isRed", json!(true));

        let runner = make_do_runner(vec![("switchColor", switch_task), ("setRed", set_red)]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // Color is "green" which doesn't match "red" - no default, so continue
        let output = runner
            .run(json!({"color": "green"}), &mut support)
            .await
            .unwrap();
        // setRed task runs because switch continues flow (no match, no default)
        // But setRed was the goto target - switch returns "continue" so the next task in list runs
        // which happens to be setRed, so it executes
        assert_eq!(output["isRed"], json!(true));
    }

    // --- Go SDK pattern tests ---

    #[tokio::test]
    async fn test_chained_set_tasks() {
        // Matches Go SDK's chained_set_tasks.yaml
        // task1: set baseValue=10, task2: set doubled=baseValue*2, task3: set tripled=doubled*3
        // Each set replaces the output; the next task receives the previous output as input
        let runner = make_do_runner(vec![
            ("task1", make_set_task("baseValue", json!(10))),
            (
                "task2",
                make_set_task("doubled", json!("${ .baseValue * 2 }")),
            ),
            (
                "task3",
                make_set_task("tripled", json!("${ .doubled * 3 }")),
            ),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // Set replaces output, so only the last task's output remains
        assert_eq!(output["tripled"], json!(60));
    }

    #[tokio::test]
    async fn test_sequential_set_colors() {
        // Matches Go SDK's sequential_set_colors.yaml
        // Sequentially append colors to array
        let runner = make_do_runner(vec![
            (
                "setRed",
                make_set_task("colors", json!("${ .colors + [\"red\"] }")),
            ),
            (
                "setGreen",
                make_set_task("colors", json!("${ .colors + [\"green\"] }")),
            ),
            (
                "setBlue",
                make_set_task("colors", json!("${ .colors + [\"blue\"] }")),
            ),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"colors": []}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["colors"], json!(["red", "green", "blue"]));
    }

    #[tokio::test]
    async fn test_set_with_then_goto_and_expression() {
        // Matches Go SDK's set_tasks_with_then.yaml
        // task1: set value=30, then: task3; task3: set result=value*3
        let mut task1 = make_set_task("value", json!(30));
        if let TaskDefinition::Set(ref mut s) = task1 {
            s.common.then = Some("task3".to_string());
        }

        let runner = make_do_runner(vec![
            ("task1", task1),
            ("task2", make_set_task("skipped", json!(true))),
            ("task3", make_set_task("result", json!("${ .value * 3 }"))),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // Set replaces output; task1 sets {value: 30}, task3 uses that as input
        assert_eq!(output["result"], json!(90));
        assert!(output.get("skipped").is_none());
    }

    #[tokio::test]
    async fn test_conditional_raise_skipped() {
        // Matches Go SDK's raise_conditional.yaml - raise with if condition
        // When condition is false, raise is skipped and next task runs
        let mut raise_task = TaskDefinition::Raise(swf_core::models::task::RaiseTaskDefinition {
            raise: swf_core::models::task::RaiseErrorDefinition::new(
                swf_core::models::error::OneOfErrorDefinitionOrReference::Error(
                    swf_core::models::error::ErrorDefinition::new(
                        "authorization",
                        "Authorization Error",
                        json!(403),
                        Some("User is under the required age".to_string()),
                        None,
                    ),
                ),
            ),
            common: TaskDefinitionFields::new(),
        });
        if let TaskDefinition::Raise(ref mut r) = raise_task {
            r.common.if_ = Some("${ .user.age < 18 }".to_string());
        }

        let runner = make_do_runner(vec![
            ("underageError", raise_task),
            (
                "continueProcess",
                make_set_task("message", json!("User is allowed")),
            ),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // User is 25 (>= 18), so raise is skipped, continueProcess runs
        let output = runner
            .run(json!({"user": {"age": 25}}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["message"], json!("User is allowed"));
    }

    #[tokio::test]
    async fn test_conditional_raise_triggered() {
        // Same workflow, but user is underage so raise fires
        let mut raise_task = TaskDefinition::Raise(swf_core::models::task::RaiseTaskDefinition {
            raise: swf_core::models::task::RaiseErrorDefinition::new(
                swf_core::models::error::OneOfErrorDefinitionOrReference::Error(
                    swf_core::models::error::ErrorDefinition::new(
                        "authorization",
                        "Authorization Error",
                        json!(403),
                        Some("User is under the required age".to_string()),
                        None,
                    ),
                ),
            ),
            common: TaskDefinitionFields::new(),
        });
        if let TaskDefinition::Raise(ref mut r) = raise_task {
            r.common.if_ = Some("${ .user.age < 18 }".to_string());
        }

        let runner = make_do_runner(vec![
            ("underageError", raise_task),
            (
                "continueProcess",
                make_set_task("message", json!("User is allowed")),
            ),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // User is 15 (< 18), so raise fires
        let result = runner.run(json!({"user": {"age": 15}}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wait_then_set_with_iso8601() {
        // Matches Go SDK's wait_duration_iso8601.yaml
        // set phase=started, wait PT0.01S, set phase=completed
        use swf_core::models::duration::OneOfDurationOrIso8601Expression;
        use swf_core::models::task::WaitTaskDefinition;

        let wait_task = TaskDefinition::Wait(WaitTaskDefinition {
            wait: OneOfDurationOrIso8601Expression::Iso8601Expression("PT0.01S".to_string()),
            common: TaskDefinitionFields::new(),
        });

        let runner = make_do_runner(vec![
            (
                "prepareWaitExample",
                make_set_task("phase", json!("started")),
            ),
            ("waitOneSecond", wait_task),
            (
                "completeWaitExample",
                make_set_task("phase", json!("completed")),
            ),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["phase"], json!("completed"));
    }

    #[tokio::test]
    async fn test_concatenating_strings() {
        // Matches Go SDK's concatenating_strings.yaml
        // task1: set firstName/lastName, task2: update, task3: concatenate
        let runner = make_do_runner(vec![
            (
                "task1",
                TaskDefinition::Set(SetTaskDefinition {
                    set: SetValue::Map({
                        let mut m = HashMap::new();
                        m.insert("firstName".to_string(), json!("John"));
                        m.insert("lastName".to_string(), json!(""));
                        m
                    }),
                    common: TaskDefinitionFields::new(),
                }),
            ),
            (
                "task2",
                TaskDefinition::Set(SetTaskDefinition {
                    set: SetValue::Map({
                        let mut m = HashMap::new();
                        m.insert("firstName".to_string(), json!("${ .firstName }"));
                        m.insert("lastName".to_string(), json!("Doe"));
                        m
                    }),
                    common: TaskDefinitionFields::new(),
                }),
            ),
            (
                "task3",
                make_set_task("fullName", json!("${ .firstName + \" \" + .lastName }")),
            ),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // Set replaces output, so only last task's output remains
        assert_eq!(output["fullName"], json!("John Doe"));
    }

    #[tokio::test]
    async fn test_conditional_logic() {
        // Matches Go SDK's conditional_logic.yaml
        // set temperature, then set weather based on condition
        let runner = make_do_runner(vec![
            (
                "task1",
                TaskDefinition::Set(SetTaskDefinition {
                    set: SetValue::Map({
                        let mut m = HashMap::new();
                        m.insert("temperature".to_string(), json!(35));
                        m
                    }),
                    common: TaskDefinitionFields::new(),
                }),
            ),
            (
                "task2",
                make_set_task(
                    "weather",
                    json!("${ if .temperature > 25 then \"hot\" else \"cold\" end }"),
                ),
            ),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["weather"], json!("hot"));
    }

    #[tokio::test]
    async fn test_set_tasks_with_termination() {
        // Matches Go SDK's set_tasks_with_termination.yaml
        // task1: set finalValue=20, then: end
        // task2: set skipped=true (should be skipped because then: end)
        let mut task1 = make_set_task("finalValue", json!(20));
        if let TaskDefinition::Set(ref mut s) = task1 {
            s.common.then = Some("end".to_string());
        }

        let runner = make_do_runner(vec![
            ("task1", task1),
            ("task2", make_set_task("skipped", json!(true))),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["finalValue"], json!(20));
        assert!(output.get("skipped").is_none());
    }

    #[tokio::test]
    async fn test_set_tasks_invalid_then_goto() {
        // Matches Go SDK's set_tasks_invalid_then.yaml
        // task1: set partialResult=15, then: nonExistentTask
        // task2: set skipped=true
        // Invalid goto target should result in an error
        let mut task1 = make_set_task("partialResult", json!(15));
        if let TaskDefinition::Set(ref mut s) = task1 {
            s.common.then = Some("nonExistentTask".to_string());
        }

        let runner = make_do_runner(vec![
            ("task1", task1),
            ("task2", make_set_task("skipped", json!(true))),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // Invalid goto target should produce an error
        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonExistentTask"));
    }

    #[tokio::test]
    async fn test_conditional_set_enabled() {
        // Matches Java SDK's conditional-set.yaml - set with if condition
        // When if condition is true, set executes
        let mut set_task = make_set_task("name", json!("javierito"));
        if let TaskDefinition::Set(ref mut s) = set_task {
            s.common.if_ = Some(".enabled".to_string());
        }

        let runner = make_do_runner(vec![("conditionalExpression", set_task)]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // enabled=true, so set should execute
        let output = runner
            .run(json!({"enabled": true}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["name"], json!("javierito"));
    }

    #[tokio::test]
    async fn test_conditional_set_disabled() {
        // Same workflow, but enabled=false so set is skipped
        let mut set_task = make_set_task("name", json!("javierito"));
        if let TaskDefinition::Set(ref mut s) = set_task {
            s.common.if_ = Some(".enabled".to_string());
        }

        let runner = make_do_runner(vec![("conditionalExpression", set_task)]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        // enabled=false, so set is skipped, output is unchanged input
        let output = runner
            .run(json!({"enabled": false, "original": "data"}), &mut support)
            .await
            .unwrap();
        assert!(output.get("name").is_none());
        assert_eq!(output["original"], json!("data"));
    }

    #[tokio::test]
    async fn test_sequential_set_colors_with_output_as() {
        // Matches Go SDK's sequential_set_colors.yaml - with output.as transformation
        // on the last task
        let mut set_blue = make_set_task("colors", json!("${ .colors + [\"blue\"] }"));
        if let TaskDefinition::Set(ref mut s) = set_blue {
            s.common.output = Some(swf_core::models::output::OutputDataModelDefinition {
                as_: Some(json!("${ { resultColors: .colors } }")),
                schema: None,
            });
        }

        let runner = make_do_runner(vec![
            (
                "setRed",
                make_set_task("colors", json!("${ .colors + [\"red\"] }")),
            ),
            (
                "setGreen",
                make_set_task("colors", json!("${ .colors + [\"green\"] }")),
            ),
            ("setBlue", set_blue),
        ]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"colors": []}), &mut support)
            .await
            .unwrap();
        // Output.as transforms the last task's output
        assert_eq!(output["resultColors"], json!(["red", "green", "blue"]));
    }

    #[tokio::test]
    async fn test_fork_with_join_result() {
        // Matches Go SDK's fork_simple.yaml - fork non-compete with join set
        // branchColors: fork (compete: false) with setRed/setBlue branches
        // joinResult: set colors: ${ [.[] | .[]] }
        use swf_core::models::task::{BranchingDefinition, ForkTaskDefinition};

        let set_red = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("color1".to_string(), json!("red"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });
        let set_blue = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("color2".to_string(), json!("blue"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });

        let branch_entries = vec![
            ("setRed".to_string(), set_red),
            ("setBlue".to_string(), set_blue),
        ];

        let fork_task = TaskDefinition::Fork(ForkTaskDefinition {
            fork: BranchingDefinition {
                branches: Map {
                    entries: branch_entries,
                },
                compete: false,
            },
            common: TaskDefinitionFields::new(),
        });

        let join_set = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("colors".to_string(), json!("${ [to_entries[].value[]] }"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });

        let runner = make_do_runner(vec![("branchColors", fork_task), ("joinResult", join_set)]);

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // Fork returns object: {setRed: {color1:"red"}, setBlue: {color2:"blue"}}
        // joinResult: to_entries[].value[] → ["red", "blue"] (order may vary)
        let colors = output["colors"].as_array().unwrap();
        assert_eq!(colors.len(), 2);
        assert!(colors.contains(&json!("red")));
        assert!(colors.contains(&json!("blue")));
    }

    // --- Extension before/after tests ---

    use swf_core::models::extension::ExtensionDefinition;
    use swf_core::models::workflow::ComponentDefinitionCollection;

    fn make_workflow_with_extensions(
        tasks: Vec<(&str, TaskDefinition)>,
        extensions: Vec<HashMap<String, ExtensionDefinition>>,
    ) -> WorkflowDefinition {
        let mut workflow = WorkflowDefinition::default();
        let entries: Vec<(String, TaskDefinition)> = tasks
            .into_iter()
            .map(|(name, task)| (name.to_string(), task))
            .collect();
        workflow.do_ = Map { entries };
        workflow.use_ = Some(ComponentDefinitionCollection {
            extensions: Some(extensions),
            ..Default::default()
        });
        workflow
    }

    #[tokio::test]
    async fn test_extension_before_set() {
        // Extension with before task that exports to $context
        let before_task = make_set_task("preKey", json!("preValue"));
        let before_task_with_export = {
            let mut t = before_task;
            if let TaskDefinition::Set(ref mut s) = t {
                s.common.export = Some(swf_core::models::output::OutputDataModelDefinition {
                    as_: Some(json!("${ {preKey: .preKey} }")),
                    schema: None,
                });
            }
            t
        };

        let ext = ExtensionDefinition {
            extend: "set".to_string(),
            when: None,
            before: Some(vec![HashMap::from([(
                "preTask".to_string(),
                before_task_with_export,
            )])]),
            after: None,
        };

        let workflow = make_workflow_with_extensions(
            vec![("mainSet", make_set_task("result", json!(42)))],
            vec![HashMap::from([("myExt".to_string(), ext)])],
        );

        let do_runner = DoTaskRunner::new_from_workflow(&workflow).unwrap();
        default_support!(workflow, context, support);

        let output = do_runner.run(json!({}), &mut support).await.unwrap();
        // Main task output is preserved (before task output is discarded)
        assert_eq!(output["result"], json!(42));
        // Before task's export updated $context
        let ctx = support.context.get_instance_ctx().unwrap();
        assert_eq!(ctx["preKey"], json!("preValue"));
    }

    #[tokio::test]
    async fn test_extension_after_set() {
        // Extension with after task that exports to $context
        let after_task = make_set_task("postKey", json!("postValue"));
        let after_task_with_export = {
            let mut t = after_task;
            if let TaskDefinition::Set(ref mut s) = t {
                s.common.export = Some(swf_core::models::output::OutputDataModelDefinition {
                    as_: Some(json!("${ {postKey: .postKey} }")),
                    schema: None,
                });
            }
            t
        };

        let ext = ExtensionDefinition {
            extend: "set".to_string(),
            when: None,
            before: None,
            after: Some(vec![HashMap::from([(
                "postTask".to_string(),
                after_task_with_export,
            )])]),
        };

        let workflow = make_workflow_with_extensions(
            vec![("mainSet", make_set_task("result", json!(42)))],
            vec![HashMap::from([("myExt".to_string(), ext)])],
        );

        let do_runner = DoTaskRunner::new_from_workflow(&workflow).unwrap();
        default_support!(workflow, context, support);

        let output = do_runner.run(json!({}), &mut support).await.unwrap();
        // Main task output is preserved
        assert_eq!(output["result"], json!(42));
        // After task's export updated $context
        let ctx = support.context.get_instance_ctx().unwrap();
        assert_eq!(ctx["postKey"], json!("postValue"));
    }

    #[tokio::test]
    async fn test_extension_before_and_after() {
        let before_task = {
            let mut t = make_set_task("preKey", json!("before"));
            if let TaskDefinition::Set(ref mut s) = t {
                s.common.export = Some(swf_core::models::output::OutputDataModelDefinition {
                    as_: Some(json!("${ {preKey: .preKey} }")),
                    schema: None,
                });
            }
            t
        };
        let after_task = {
            let mut t = make_set_task("postKey", json!("after"));
            if let TaskDefinition::Set(ref mut s) = t {
                s.common.export = Some(swf_core::models::output::OutputDataModelDefinition {
                    as_: Some(json!("${ {postKey: .postKey} }")),
                    schema: None,
                });
            }
            t
        };

        let ext = ExtensionDefinition {
            extend: "set".to_string(),
            when: None,
            before: Some(vec![HashMap::from([(
                "preTask".to_string(),
                before_task,
            )])]),
            after: Some(vec![HashMap::from([(
                "postTask".to_string(),
                after_task,
            )])]),
        };

        let workflow = make_workflow_with_extensions(
            vec![("mainSet", make_set_task("result", json!(42)))],
            vec![HashMap::from([("myExt".to_string(), ext)])],
        );

        let do_runner = DoTaskRunner::new_from_workflow(&workflow).unwrap();
        default_support!(workflow, context, support);

        let output = do_runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["result"], json!(42));
        // Before export then after export — last export wins
        let ctx = support.context.get_instance_ctx().unwrap();
        assert_eq!(ctx["postKey"], json!("after"));
    }

    #[tokio::test]
    async fn test_extension_when_condition_true() {
        let before_task = {
            let mut t = make_set_task("touched", json!(true));
            if let TaskDefinition::Set(ref mut s) = t {
                s.common.export = Some(swf_core::models::output::OutputDataModelDefinition {
                    as_: Some(json!("${ {touched: .touched} }")),
                    schema: None,
                });
            }
            t
        };

        let ext = ExtensionDefinition {
            extend: "set".to_string(),
            when: Some(".enabled".to_string()),
            before: Some(vec![HashMap::from([(
                "condTask".to_string(),
                before_task,
            )])]),
            after: None,
        };

        let workflow = make_workflow_with_extensions(
            vec![("mainSet", make_set_task("result", json!(1)))],
            vec![HashMap::from([("condExt".to_string(), ext)])],
        );

        let do_runner = DoTaskRunner::new_from_workflow(&workflow).unwrap();
        default_support!(workflow, context, support);

        let output = do_runner
            .run(json!({"enabled": true}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["result"], json!(1));
        let ctx = support.context.get_instance_ctx().unwrap();
        assert_eq!(ctx["touched"], json!(true));
    }

    #[tokio::test]
    async fn test_extension_when_condition_false() {
        let before_task = {
            let mut t = make_set_task("touched", json!(true));
            if let TaskDefinition::Set(ref mut s) = t {
                s.common.export = Some(swf_core::models::output::OutputDataModelDefinition {
                    as_: Some(json!("${ {touched: .touched} }")),
                    schema: None,
                });
            }
            t
        };

        let ext = ExtensionDefinition {
            extend: "set".to_string(),
            when: Some(".enabled".to_string()),
            before: Some(vec![HashMap::from([(
                "condTask".to_string(),
                before_task,
            )])]),
            after: None,
        };

        let workflow = make_workflow_with_extensions(
            vec![("mainSet", make_set_task("result", json!(1)))],
            vec![HashMap::from([("condExt".to_string(), ext)])],
        );

        let do_runner = DoTaskRunner::new_from_workflow(&workflow).unwrap();
        default_support!(workflow, context, support);

        let output = do_runner
            .run(json!({"enabled": false}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["result"], json!(1));
        // Extension was skipped because when condition is false
        let ctx = support.context.get_instance_ctx();
        assert!(ctx.is_none() || ctx.unwrap().get("touched").is_none());
    }

    #[tokio::test]
    async fn test_extension_extend_all() {
        let after_task = {
            let mut t = make_set_task("logMsg", json!("logged"));
            if let TaskDefinition::Set(ref mut s) = t {
                s.common.export = Some(swf_core::models::output::OutputDataModelDefinition {
                    as_: Some(json!("${ {logMsg: .logMsg} }")),
                    schema: None,
                });
            }
            t
        };

        let ext = ExtensionDefinition {
            extend: "all".to_string(),
            when: None,
            before: None,
            after: Some(vec![HashMap::from([(
                "logAfter".to_string(),
                after_task,
            )])]),
        };

        let workflow = make_workflow_with_extensions(
            vec![
                ("task1", make_set_task("a", json!(1))),
                ("task2", make_set_task("b", json!(2))),
            ],
            vec![HashMap::from([("logExt".to_string(), ext)])],
        );

        let do_runner = DoTaskRunner::new_from_workflow(&workflow).unwrap();
        default_support!(workflow, context, support);

        let output = do_runner.run(json!({}), &mut support).await.unwrap();
        // Both tasks run, "all" extension runs after each
        assert_eq!(output["b"], json!(2));
        let ctx = support.context.get_instance_ctx().unwrap();
        assert_eq!(ctx["logMsg"], json!("logged"));
    }

    #[tokio::test]
    async fn test_extension_no_match() {
        let before_task = {
            let mut t = make_set_task("touched", json!(true));
            if let TaskDefinition::Set(ref mut s) = t {
                s.common.export = Some(swf_core::models::output::OutputDataModelDefinition {
                    as_: Some(json!("${ {touched: .touched} }")),
                    schema: None,
                });
            }
            t
        };

        // Extension targets "call" but we only have "set" tasks
        let ext = ExtensionDefinition {
            extend: "call".to_string(),
            when: None,
            before: Some(vec![HashMap::from([(
                "callBefore".to_string(),
                before_task,
            )])]),
            after: None,
        };

        let workflow = make_workflow_with_extensions(
            vec![("mainSet", make_set_task("result", json!(1)))],
            vec![HashMap::from([("callExt".to_string(), ext)])],
        );

        let do_runner = DoTaskRunner::new_from_workflow(&workflow).unwrap();
        default_support!(workflow, context, support);

        let output = do_runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["result"], json!(1));
        // Extension didn't match, so no side effects
        let ctx = support.context.get_instance_ctx();
        assert!(ctx.is_none() || ctx.unwrap().get("touched").is_none());
    }
}
