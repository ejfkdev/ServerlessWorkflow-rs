use crate::tasks::task_name_impl;
use crate::error::WorkflowResult;
use crate::task_runner::{TaskRunner, TaskSupport};

use serde_json::Value;
use serverless_workflow_core::models::task::SwitchTaskDefinition;

/// Runner for Switch tasks - evaluates conditions and branches
pub struct SwitchTaskRunner {
    name: String,
}

impl SwitchTaskRunner {
    pub fn new(name: &str, _task: &SwitchTaskDefinition) -> WorkflowResult<Self> {
        Ok(Self {
            name: name.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl TaskRunner for SwitchTaskRunner {
    async fn run(&self, input: Value, _support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        // The actual switch evaluation is handled by DoTaskRunner,
        // which processes flow directives. This runner is a fallback
        // that simply evaluates conditions and returns the input unchanged.
        // The real switching logic is in DoTaskRunner::evaluate_switch.
        Ok(input)
    }

    task_name_impl!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_support;
    use serde_json::json;
    use serverless_workflow_core::models::map::Map;
    use serverless_workflow_core::models::task::{
        SwitchCaseDefinition, SwitchTaskDefinition, TaskDefinitionFields,
    };
    use serverless_workflow_core::models::workflow::WorkflowDefinition;

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
    async fn test_switch_runner_returns_input() {
        let switch = SwitchTaskDefinition {
            switch: Map::default(),
            common: TaskDefinitionFields::new(),
        };
        let runner = SwitchTaskRunner::new("passThrough", &switch).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let input = json!({"data": "value"});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_switch_with_cases_returns_input() {
        let case1 = (
            "red".to_string(),
            SwitchCaseDefinition {
                when: Some(".color == \"red\"".to_string()),
                then: Some("setRed".to_string()),
            },
        );
        let case2 = (
            "default".to_string(),
            SwitchCaseDefinition {
                when: None,
                then: Some("setDefault".to_string()),
            },
        );

        let switch = SwitchTaskDefinition {
            switch: Map {
                entries: vec![case1, case2],
            },
            common: TaskDefinitionFields::new(),
        };
        let runner = SwitchTaskRunner::new("colorSwitch", &switch).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let input = json!({"color": "red"});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        // SwitchTaskRunner itself just passes through; DoTaskRunner handles the logic
        assert_eq!(output, input);
    }
}
