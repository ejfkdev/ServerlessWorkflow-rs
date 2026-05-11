use crate::error::{WorkflowError, WorkflowResult};
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::tasks::task_name_impl;

use serde_json::Value;
use swf_core::models::task::SwitchTaskDefinition;

/// Runner for Switch tasks - evaluates conditions and returns matched case info
pub struct SwitchTaskRunner {
    name: String,
    task: SwitchTaskDefinition,
}

impl SwitchTaskRunner {
    pub fn new(name: &str, task: &SwitchTaskDefinition) -> WorkflowResult<Self> {
        Ok(Self {
            name: name.to_string(),
            task: task.clone(),
        })
    }

    /// Evaluates switch conditions and returns (matched_case_name, then_directive)
    fn evaluate(
        &self,
        input: &Value,
        support: &TaskSupport<'_>,
    ) -> WorkflowResult<(Option<String>, String)> {
        let mut default_then: Option<(String, String)> = None;

        for (case_name, case_def) in &self.task.switch.entries {
            match &case_def.when {
                None => {
                    // Default case
                    if let Some(ref then) = case_def.then {
                        default_then = Some((case_name.clone(), then.clone()));
                    }
                }
                Some(when_expr) => {
                    let result = support
                        .eval_bool(when_expr, input)
                        .map_err(|e| WorkflowError::expression(format!("{}", e), &self.name))?;
                    if result {
                        let then = case_def
                            .then
                            .clone()
                            .unwrap_or_else(|| "continue".to_string());
                        return Ok((Some(case_name.clone()), then));
                    }
                }
            }
        }

        // No matching case — use default or continue
        match default_then {
            Some((name, then)) => Ok((Some(name), then)),
            None => Ok((None, "continue".to_string())),
        }
    }
}

#[async_trait::async_trait]
impl TaskRunner for SwitchTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        let (matched_case, then) = self.evaluate(&input, support)?;

        // When run standalone (outside Do context), return the input with matched case metadata
        // The DoTaskRunner handles flow directives (goto/end/exit) when switch is inside a Do block
        match then.as_str() {
            "end" | "exit" => {
                // Return input unchanged — the caller (DoTaskRunner or workflow runner) handles termination
                Ok(input)
            }
            _ => {
                // For "continue" or task-name targets, return input with switch result metadata
                // so callers can inspect what matched
                let mut result = match input {
                    Value::Object(map) => map,
                    other => {
                        let mut map = serde_json::Map::new();
                        map.insert("input".to_string(), other);
                        map
                    }
                };
                if let Some(case_name) = matched_case {
                    result.insert("switchMatched".to_string(), Value::String(case_name));
                }
                result.insert("switchThen".to_string(), Value::String(then));
                Ok(Value::Object(result))
            }
        }
    }

    task_name_impl!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_support;
    use serde_json::json;
    use swf_core::models::map::Map;
    use swf_core::models::task::{
        SwitchCaseDefinition, SwitchTaskDefinition, TaskDefinitionFields,
    };
    use swf_core::models::workflow::WorkflowDefinition;

    #[test]
    fn test_switch_runner_new() {
        let switch = SwitchTaskDefinition {
            switch: Map::default(),
            common: TaskDefinitionFields::new(),
        };
        let runner = SwitchTaskRunner::new("testSwitch", &switch);
        assert!(runner.is_ok());
        assert_eq!(runner.unwrap().task_name(), "testSwitch");
    }

    #[tokio::test]
    async fn test_switch_match_condition() {
        let switch = SwitchTaskDefinition {
            switch: Map {
                entries: vec![
                    (
                        "red".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".color == "red""#.to_string()),
                            then: Some("handleRed".to_string()),
                        },
                    ),
                    (
                        "default".to_string(),
                        SwitchCaseDefinition {
                            when: None,
                            then: Some("handleDefault".to_string()),
                        },
                    ),
                ],
            },
            common: TaskDefinitionFields::new(),
        };
        let runner = SwitchTaskRunner::new("colorSwitch", &switch).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"color": "red"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["switchMatched"], "red");
        assert_eq!(output["switchThen"], "handleRed");
    }

    #[tokio::test]
    async fn test_switch_default_case() {
        let switch = SwitchTaskDefinition {
            switch: Map {
                entries: vec![
                    (
                        "red".to_string(),
                        SwitchCaseDefinition {
                            when: Some(r#".color == "red""#.to_string()),
                            then: Some("handleRed".to_string()),
                        },
                    ),
                    (
                        "fallback".to_string(),
                        SwitchCaseDefinition {
                            when: None,
                            then: Some("handleDefault".to_string()),
                        },
                    ),
                ],
            },
            common: TaskDefinitionFields::new(),
        };
        let runner = SwitchTaskRunner::new("colorSwitch", &switch).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"color": "green"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["switchMatched"], "fallback");
        assert_eq!(output["switchThen"], "handleDefault");
    }

    #[tokio::test]
    async fn test_switch_no_match_no_default() {
        let switch = SwitchTaskDefinition {
            switch: Map {
                entries: vec![(
                    "red".to_string(),
                    SwitchCaseDefinition {
                        when: Some(r#".color == "red""#.to_string()),
                        then: Some("handleRed".to_string()),
                    },
                )],
            },
            common: TaskDefinitionFields::new(),
        };
        let runner = SwitchTaskRunner::new("colorSwitch", &switch).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"color": "green"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["switchThen"], "continue");
        assert!(output.get("switchMatched").is_none());
    }

    #[tokio::test]
    async fn test_switch_then_end() {
        let switch = SwitchTaskDefinition {
            switch: Map {
                entries: vec![(
                    "stop".to_string(),
                    SwitchCaseDefinition {
                        when: Some(".stop == true".to_string()),
                        then: Some("end".to_string()),
                    },
                )],
            },
            common: TaskDefinitionFields::new(),
        };
        let runner = SwitchTaskRunner::new("stopSwitch", &switch).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let input = json!({"stop": true});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        // "end" returns input unchanged (no metadata added)
        assert_eq!(output, input);
    }
}
