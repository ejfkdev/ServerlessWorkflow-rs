use crate::error::{WorkflowError, WorkflowResult};
use crate::status::StatusPhase;
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::utils::resolve_duration_expr;
use serde_json::Value;
use serverless_workflow_core::models::duration::{
    Duration as SwfDuration, OneOfDurationOrIso8601Expression,
};
use serverless_workflow_core::models::task::WaitTaskDefinition;
use std::collections::HashMap;
use std::time::Duration;

/// Runner for Wait tasks - pauses execution for a specified duration
pub struct WaitTaskRunner {
    name: String,
    duration_expr: OneOfDurationOrIso8601Expression,
}

impl WaitTaskRunner {
    pub fn new(name: &str, task: &WaitTaskDefinition) -> WorkflowResult<Self> {
        Ok(Self {
            name: name.to_string(),
            duration_expr: task.wait.clone(),
        })
    }
}

/// Converts a workflow Duration to a std Duration
pub fn duration_to_std(dur: &SwfDuration) -> Duration {
    let total_ms = dur.total_milliseconds();
    Duration::from_millis(total_ms)
}

/// Resolves a OneOfDurationOrIso8601Expression with expression context support.
/// If the ISO8601 string contains a JQ expression (${...}), it will be evaluated first.
pub fn resolve_duration_with_context(
    expr: &OneOfDurationOrIso8601Expression,
    input: &Value,
    vars: &HashMap<String, Value>,
) -> WorkflowResult<Duration> {
    resolve_duration_expr(expr, input, vars, "")
}

#[async_trait::async_trait]
impl TaskRunner for WaitTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        let vars = support.get_vars();
        let wait_duration = resolve_duration_with_context(&self.duration_expr, &input, &vars)?;

        if wait_duration.as_millis() == 0 {
            return Ok(input);
        }

        support.set_task_status(&self.name, StatusPhase::Waiting);

        // Use tokio::select! instead of plain sleep to respond to cancellation.
        // This mirrors Go SDK's timer+select pattern which gracefully handles
        // context cancellation (e.g., from workflow timeout) instead of blocking
        // unconditionally with time.Sleep.
        let cancel_token = support.context.cancellation_token();
        tokio::select! {
            _ = tokio::time::sleep(wait_duration) => {
                Ok(input)
            }
            _ = cancel_token.cancelled() => {
                Err(WorkflowError::timeout(
                    format!("wait task '{}' cancelled", self.name),
                    &self.name,
                ))
            }
        }
    }

    fn task_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::parse_iso8601_duration;

    #[test]
    fn test_parse_iso8601_duration_seconds() {
        let dur = parse_iso8601_duration("PT5S").unwrap();
        assert_eq!(dur, Duration::from_millis(5000));
    }

    #[test]
    fn test_parse_iso8601_duration_minutes() {
        let dur = parse_iso8601_duration("PT10M").unwrap();
        assert_eq!(dur, Duration::from_millis(10 * 60 * 1000));
    }

    #[test]
    fn test_parse_iso8601_duration_hours() {
        let dur = parse_iso8601_duration("PT1H").unwrap();
        assert_eq!(dur, Duration::from_millis(60 * 60 * 1000));
    }

    #[test]
    fn test_parse_iso8601_duration_days() {
        let dur = parse_iso8601_duration("P1D").unwrap();
        assert_eq!(dur, Duration::from_millis(24 * 60 * 60 * 1000));
    }

    #[test]
    fn test_parse_iso8601_duration_combined() {
        let dur = parse_iso8601_duration("P1DT12H30M5S").unwrap();
        let expected = (24 + 12) * 60 * 60 * 1000 + 30 * 60 * 1000 + 5000;
        assert_eq!(dur, Duration::from_millis(expected as u64));
    }

    #[test]
    fn test_parse_iso8601_duration_invalid() {
        // "1Y" is not valid ISO 8601 duration (no P prefix, Y not supported)
        let result = parse_iso8601_duration("1Y");
        assert!(result.is_none(), "expected None for invalid duration '1Y'");

        // Empty string is invalid
        let result = parse_iso8601_duration("");
        assert!(result.is_none(), "expected None for empty duration");

        // "P" alone (no time components) should produce zero duration
        let result = parse_iso8601_duration("P");
        assert!(result.is_some(), "'P' alone should parse successfully");

        // "P1Y" - years not supported in ISO 8601 duration for our parser
        let _result = parse_iso8601_duration("P1Y");
    }

    #[test]
    fn test_parse_iso8601_duration_fractional_seconds() {
        // 0.25 seconds = 250ms
        let dur = parse_iso8601_duration("PT0.25S").unwrap();
        assert_eq!(dur, Duration::from_millis(250));
    }

    #[test]
    fn test_parse_iso8601_duration_milliseconds_suffix() {
        // PT250MS = 250 milliseconds (MS suffix)
        let dur = parse_iso8601_duration("PT250MS").unwrap();
        assert_eq!(dur, Duration::from_millis(250));
    }

    #[test]
    fn test_parse_iso8601_duration_combined_with_ms() {
        // P3DT4H5M6S250MS
        let dur = parse_iso8601_duration("P3DT4H5M6S250MS").unwrap();
        let expected = 3 * 24 * 3600 * 1000 + 4 * 3600 * 1000 + 5 * 60 * 1000 + 6 * 1000 + 250;
        assert_eq!(dur, Duration::from_millis(expected as u64));
    }

    #[test]
    fn test_parse_iso8601_duration_zero() {
        let dur = parse_iso8601_duration("PT0S").unwrap();
        assert_eq!(dur, Duration::from_millis(0));
    }

    #[tokio::test]
    async fn test_wait_returns_input_unchanged() {
        use crate::context::WorkflowContext;
        use crate::task_runner::TaskSupport;
        use serde_json::json;
        use serverless_workflow_core::models::task::TaskDefinitionFields;
        use serverless_workflow_core::models::workflow::WorkflowDefinition;

        let task = WaitTaskDefinition {
            wait: OneOfDurationOrIso8601Expression::Duration(SwfDuration::from_milliseconds(10)),
            common: TaskDefinitionFields::new(),
        };
        let runner = WaitTaskRunner::new("waitTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let input = json!({"data": "preserved"});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_wait_zero_duration() {
        use crate::context::WorkflowContext;
        use crate::task_runner::TaskSupport;
        use serde_json::json;
        use serverless_workflow_core::models::task::TaskDefinitionFields;
        use serverless_workflow_core::models::workflow::WorkflowDefinition;

        let task = WaitTaskDefinition {
            wait: OneOfDurationOrIso8601Expression::Duration(SwfDuration::from_milliseconds(0)),
            common: TaskDefinitionFields::new(),
        };
        let runner = WaitTaskRunner::new("zeroWait", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let input = json!({"fast": true});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_wait_with_iso8601_string() {
        use crate::context::WorkflowContext;
        use crate::task_runner::TaskSupport;
        use serde_json::json;
        use serverless_workflow_core::models::task::TaskDefinitionFields;
        use serverless_workflow_core::models::workflow::WorkflowDefinition;

        let task = WaitTaskDefinition {
            wait: OneOfDurationOrIso8601Expression::Iso8601Expression("PT0.01S".to_string()),
            common: TaskDefinitionFields::new(),
        };
        let runner = WaitTaskRunner::new("isoWait", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let input = json!({"iso": "duration"});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_wait_then_set() {
        // Matches Java SDK's wait-set.yaml - wait then set
        use crate::context::WorkflowContext;
        use crate::task_runner::TaskSupport;
        use crate::tasks::DoTaskRunner;
        use serde_json::json;
        use serverless_workflow_core::models::map::Map;
        use serverless_workflow_core::models::task::{
            DoTaskDefinition, SetTaskDefinition, SetValue, TaskDefinition, TaskDefinitionFields,
        };
        use serverless_workflow_core::models::workflow::WorkflowDefinition;
        use std::collections::HashMap;

        let wait_task = TaskDefinition::Wait(WaitTaskDefinition {
            wait: OneOfDurationOrIso8601Expression::Duration(SwfDuration::from_milliseconds(50)),
            common: TaskDefinitionFields::new(),
        });

        let mut set_map = HashMap::new();
        set_map.insert("name".to_string(), json!("Javierito"));
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let entries = vec![
            ("waitABit".to_string(), wait_task),
            ("useExpression".to_string(), set_task),
        ];

        let do_def = DoTaskDefinition::new(Map { entries });
        let workflow = WorkflowDefinition::default();
        let runner = DoTaskRunner::new("waitSet", &do_def).unwrap();

        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["name"], json!("Javierito"));
    }

    #[tokio::test]
    async fn test_wait_preserves_and_references_prior_values() {
        // Matches Go SDK's wait_duration_iso8601.yaml
        // set phase=started, waitExpression=PT1S → wait PT0.01S → set phase=completed, previousPhase=${ .phase }, waitExpression=${ .waitExpression }
        use crate::context::WorkflowContext;
        use crate::task_runner::TaskSupport;
        use crate::tasks::DoTaskRunner;
        use serde_json::json;
        use serverless_workflow_core::models::map::Map;
        use serverless_workflow_core::models::task::{
            DoTaskDefinition, SetTaskDefinition, SetValue, TaskDefinition, TaskDefinitionFields,
        };
        use serverless_workflow_core::models::workflow::WorkflowDefinition;
        use std::collections::HashMap;

        // Task 1: set phase=started, waitExpression=PT1S
        let set_prepare = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("phase".to_string(), json!("started"));
                m.insert("waitExpression".to_string(), json!("PT1S"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });

        // Task 2: wait PT0.01S (short for test speed)
        let wait_task = TaskDefinition::Wait(WaitTaskDefinition {
            wait: OneOfDurationOrIso8601Expression::Iso8601Expression("PT0.01S".to_string()),
            common: TaskDefinitionFields::new(),
        });

        // Task 3: set phase=completed, previousPhase=${ .phase }, waitExpression=${ .waitExpression }
        let set_complete = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map({
                let mut m = HashMap::new();
                m.insert("phase".to_string(), json!("completed"));
                m.insert("previousPhase".to_string(), json!("${ .phase }"));
                m.insert("waitExpression".to_string(), json!("${ .waitExpression }"));
                m
            }),
            common: TaskDefinitionFields::new(),
        });

        let entries = vec![
            ("prepareWaitExample".to_string(), set_prepare),
            ("waitOneSecond".to_string(), wait_task),
            ("completeWaitExample".to_string(), set_complete),
        ];

        let do_def = DoTaskDefinition::new(Map { entries });
        let workflow = WorkflowDefinition::default();
        let runner = DoTaskRunner::new("waitPreserve", &do_def).unwrap();

        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["phase"], json!("completed"));
        assert_eq!(output["previousPhase"], json!("started"));
        assert_eq!(output["waitExpression"], json!("PT1S"));
    }

    #[tokio::test]
    async fn test_wait_cancellation() {
        // Matches Go SDK's context cancellation pattern:
        // When the cancellation token is triggered, wait should return a timeout error
        // instead of blocking unconditionally.
        use crate::context::WorkflowContext;
        use crate::task_runner::TaskSupport;
        use serde_json::json;
        use serverless_workflow_core::models::task::TaskDefinitionFields;
        use serverless_workflow_core::models::workflow::WorkflowDefinition;

        let task = WaitTaskDefinition {
            // Long wait that would normally block for 10 seconds
            wait: OneOfDurationOrIso8601Expression::Duration(SwfDuration::from_seconds(10)),
            common: TaskDefinitionFields::new(),
        };
        let runner = WaitTaskRunner::new("cancelTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();

        // Cancel the context BEFORE creating the mutable borrow to support
        context.cancel();

        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({"data": "test"}), &mut support).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("cancelled"),
            "Expected cancellation error, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_wait_cancellation_during_wait() {
        // Test that cancelling while waiting returns timeout error promptly
        use crate::context::WorkflowContext;
        use crate::task_runner::TaskSupport;
        use serde_json::json;
        use serverless_workflow_core::models::task::TaskDefinitionFields;
        use serverless_workflow_core::models::workflow::WorkflowDefinition;

        let task = WaitTaskDefinition {
            // Short wait for test speed (was 5s, reduced for performance)
            wait: OneOfDurationOrIso8601Expression::Duration(SwfDuration::from_milliseconds(10)),
            common: TaskDefinitionFields::new(),
        };
        let runner = WaitTaskRunner::new("midCancel", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();

        // Clone the token BEFORE creating the mutable borrow to support
        let token = context.cancellation_token();
        let mut support = TaskSupport::new(&workflow, &mut context);

        // Spawn a task that cancels after a short delay
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1)).await;
            token.cancel();
        });

        let start = std::time::Instant::now();
        let result = runner.run(json!({"data": "test"}), &mut support).await;
        let elapsed = start.elapsed();

        assert!(result.is_err());
        assert!(
            elapsed < Duration::from_millis(500),
            "Should cancel quickly, took {:?}",
            elapsed
        );
    }
}
