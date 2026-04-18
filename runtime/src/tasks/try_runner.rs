use crate::error::{WorkflowError, WorkflowResult};
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::tasks::DoTaskRunner;
use serde_json::Value;
use serverless_workflow_core::models::retry::{
    OneOfRetryPolicyDefinitionOrReference, RetryAttemptLimitDefinition, RetryPolicyDefinition,
    RetryPolicyLimitDefinition,
};
use serverless_workflow_core::models::task::{ErrorCatcherDefinition, TryTaskDefinition};
use serverless_workflow_core::models::workflow::WorkflowDefinition;
use std::collections::HashMap;
use std::time::Duration;

/// Runner for Try tasks - executes tasks with error catching and retry
pub struct TryTaskRunner {
    name: String,
    try_runner: DoTaskRunner,
    catch: ErrorCatcherDefinition,
    workflow: WorkflowDefinition,
}

impl TryTaskRunner {
    pub fn new(
        name: &str,
        task: &TryTaskDefinition,
        workflow: &WorkflowDefinition,
    ) -> WorkflowResult<Self> {
        let do_def =
            serverless_workflow_core::models::task::DoTaskDefinition::new(task.try_.clone());
        let try_runner = DoTaskRunner::new(name, &do_def)?;

        Ok(Self {
            name: name.to_string(),
            try_runner,
            catch: task.catch.clone(),
            workflow: workflow.clone(),
        })
    }
}

#[async_trait::async_trait]
impl TaskRunner for TryTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        // Try to execute the tasks
        match self.try_runner.run(input.clone(), support).await {
            Ok(output) => Ok(output),
            Err(error) => {
                // Check if the error matches the catch filter
                if !self.matches_catch_filter(&error) {
                    return Err(error);
                }

                // Build error context for when/exceptWhen evaluation
                let error_value = error.to_value();

                // Check when/exceptWhen conditions
                if !evaluates_when_allowed(support, self.catch.when.as_deref(), self.catch.except_when.as_deref(), &error_value) {
                    return Err(error);
                }

                // Handle retry if configured
                if let Some(ref retry) = self.catch.retry {
                    return self.handle_retry(input, support, retry).await;
                }

                // Set error variable if catch.as is configured
                let error_var_key = self
                    .catch
                    .as_
                    .as_deref()
                    .map(|name| crate::utils::ensure_dollar_prefix(name, name));

                if let Some(ref var_key) = error_var_key {
                    let mut error_vars = HashMap::new();
                    error_vars.insert(var_key.clone(), error_value);
                    support.add_local_expr_vars(error_vars);
                }

                // Execute catch.do tasks if configured
                let result = if let Some(ref catch_do) = self.catch.do_ {
                    let do_def = serverless_workflow_core::models::task::DoTaskDefinition::new(
                        catch_do.clone(),
                    );
                    let catch_runner = DoTaskRunner::new(&self.name, &do_def)?;
                    catch_runner.run(input, support).await
                } else {
                    // Swallow the error and return input
                    Ok(input)
                };

                // Clean up error variable
                if let Some(ref var_key) = error_var_key {
                    support.remove_local_expr_vars(&[var_key]);
                }

                result
            }
        }
    }

    fn task_name(&self) -> &str {
        &self.name
    }
}

impl TryTaskRunner {
    /// Checks if the error matches the catch filter
    fn matches_catch_filter(&self, error: &WorkflowError) -> bool {
        let filter = match &self.catch.errors {
            Some(f) => f,
            None => return true, // No filter means catch all
        };

        let props = match &filter.with {
            Some(p) => p,
            None => return true,
        };

        // Check type filter (supports both full URI and short name)
        if let Some(ref type_filter) = props.type_ {
            let error_type = error.error_type();
            let error_type_short = error.error_type_short();
            if error_type != type_filter.as_str() && error_type_short != type_filter.as_str() {
                return false;
            }
        }

        // Check status filter
        if let Some(ref status_filter) = props.status {
            match error.status() {
                Some(error_status) if error_status == status_filter => {}
                _ => return false,
            }
        }

        // Check title filter
        if let Some(ref title_filter) = props.title {
            match error.title() {
                Some(error_title) if error_title == title_filter => {}
                _ => return false,
            }
        }

        // Check details filter
        if let Some(ref detail_filter) = props.detail {
            match error.detail() {
                Some(error_details) if error_details == detail_filter => {}
                _ => return false,
            }
        }

        // Check instance filter
        if let Some(ref instance_filter) = props.instance {
            match error.instance() {
                Some(error_instance) if error_instance == instance_filter => {}
                _ => return false,
            }
        }

        true
    }

    /// Resolves a retry policy reference from workflow.use.retries
    fn resolve_retry_reference(&self, ref_name: &str) -> Option<RetryPolicyDefinition> {
        let use_ = self.workflow.use_.as_ref()?;
        let retries = use_.retries.as_ref()?;
        retries.get(ref_name).cloned()
    }

    /// Handles retry logic with backoff
    async fn handle_retry(
        &self,
        input: Value,
        support: &mut TaskSupport<'_>,
        retry: &OneOfRetryPolicyDefinitionOrReference,
    ) -> WorkflowResult<Value> {
        let policy = match retry {
            OneOfRetryPolicyDefinitionOrReference::Retry(p) => p.as_ref().clone(),
            OneOfRetryPolicyDefinitionOrReference::Reference(ref_name) => {
                // Resolve reference from workflow.use.retries
                let fallback = RetryPolicyDefinition {
                    delay: None,
                    backoff: None,
                    limit: Some(RetryPolicyLimitDefinition {
                        attempt: Some(RetryAttemptLimitDefinition {
                            count: Some(1),
                            duration: None,
                        }),
                        duration: None,
                    }),
                    when: None,
                    except_when: None,
                    jitter: None,
                };
                self.resolve_retry_reference(ref_name).unwrap_or(fallback)
            }
        };

        let max_attempts = policy
            .limit
            .as_ref()
            .and_then(|l| l.attempt.as_ref())
            .and_then(|a| a.count)
            .unwrap_or(3) as u32;

        let delay_ms = policy.delay
            .as_ref()
            .map(|d| {
                match d {
                    serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Duration(dur) => dur.total_milliseconds(),
                    serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
                        crate::utils::parse_iso8601_duration(expr)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(1000)
                    }
                }
            })
            .unwrap_or(1000);

        for attempt in 1..=max_attempts {
            // Apply backoff delay (skip first attempt)
            if attempt > 1 {
                let backoff_delay = self.calculate_backoff_delay(delay_ms, attempt - 1, &policy);
                tokio::time::sleep(Duration::from_millis(backoff_delay)).await;
            }

            match self.try_runner.run(input.clone(), support).await {
                Ok(output) => return Ok(output),
                Err(e) => {
                    // Check retry policy's when/except_when conditions against error context
                    let error_value = e.to_value();

                    if !evaluates_when_allowed(support, policy.when.as_deref(), policy.except_when.as_deref(), &error_value) {
                        return Err(e);
                    }

                    if attempt == max_attempts {
                        return Err(e);
                    }
                    // Continue to next attempt
                }
            }
        }

        Err(WorkflowError::runtime_simple("retry exhausted", &self.name))
    }

    /// Calculates delay with backoff strategy
    fn calculate_backoff_delay(
        &self,
        base_delay_ms: u64,
        attempt: u32,
        policy: &RetryPolicyDefinition,
    ) -> u64 {
        let backoff = match &policy.backoff {
            Some(b) => b,
            None => return base_delay_ms,
        };

        if let Some(ref constant) = backoff.constant {
            // Constant backoff: use the configured delay, or fall back to base_delay_ms
            if let Some(delay_str) = constant.delay() {
                if let Some(parsed) = crate::utils::parse_iso8601_duration(delay_str) {
                    return parsed.as_millis() as u64;
                }
            }
            return base_delay_ms;
        }

        if let Some(ref exponential) = backoff.exponential {
            let factor = exponential.factor().unwrap_or(2.0);
            let delay = (base_delay_ms as f64 * factor.powi(attempt as i32)) as u64;
            // Apply maxDelay if set
            if let Some(max_delay_str) = exponential.max_delay() {
                if let Some(parsed) = crate::utils::parse_iso8601_duration(max_delay_str) {
                    let max_ms = parsed.as_millis() as u64;
                    return delay.min(max_ms);
                }
            }
            return delay;
        }

        if let Some(ref linear) = backoff.linear {
            let increment_ms = linear
                .increment
                .as_ref()
                .map(|d| d.total_milliseconds())
                .unwrap_or(base_delay_ms);
            return base_delay_ms + (increment_ms * attempt as u64);
        }

        base_delay_ms
    }
}

/// Evaluates when/except_when conditions against error context.
/// Returns true if conditions allow proceeding (catch or retry is allowed),
/// false if conditions block it (should propagate error).
fn evaluates_when_allowed(
    support: &TaskSupport<'_>,
    when: Option<&str>,
    except_when: Option<&str>,
    error_value: &Value,
) -> bool {
    if let Some(when_expr) = when {
        let should_proceed = support.eval_bool(when_expr, error_value).unwrap_or(false);
        if !should_proceed {
            return false;
        }
    }
    if let Some(except_when_expr) = except_when {
        let should_except = support.eval_bool(except_when_expr, error_value).unwrap_or(false);
        if should_except {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::WorkflowContext;
    use crate::task_runner::TaskSupport;
    use crate::test_utils::test_helpers::make_set_task;
    use serde_json::json;
    use serverless_workflow_core::models::error::{
        ErrorDefinition, OneOfErrorDefinitionOrReference,
    };
    use serverless_workflow_core::models::map::Map;
    use serverless_workflow_core::models::retry::{
        RetryAttemptLimitDefinition, RetryPolicyDefinition, RetryPolicyLimitDefinition,
    };
    use serverless_workflow_core::models::task::{
        ErrorCatcherDefinition, ErrorFilterDefinition, ErrorFilterProperties, RaiseErrorDefinition,
        RaiseTaskDefinition, TaskDefinition, TaskDefinitionFields,
    };
    use serverless_workflow_core::models::workflow::WorkflowDefinition;
    use std::collections::HashMap;

    /// Helper: build a TryTaskRunner from try tasks + catch config
    fn make_try_runner(
        name: &str,
        try_tasks: Vec<(&str, TaskDefinition)>,
        catch: ErrorCatcherDefinition,
    ) -> TryTaskRunner {
        let workflow = WorkflowDefinition::default();
        make_try_runner_with_workflow(name, try_tasks, catch, &workflow)
    }

    fn make_try_runner_with_workflow(
        name: &str,
        try_tasks: Vec<(&str, TaskDefinition)>,
        catch: ErrorCatcherDefinition,
        workflow: &WorkflowDefinition,
    ) -> TryTaskRunner {
        let mut try_entries = Vec::new();
        for (task_name, task) in try_tasks {
            try_entries.push((task_name.to_string(), task));
        }

        let task = TryTaskDefinition {
            try_: Map {
                entries: try_entries,
            },
            catch,
            common: TaskDefinitionFields::new(),
        };

        TryTaskRunner::new(name, &task, workflow).unwrap()
    }

    fn make_raise_task(error_type: &str) -> TaskDefinition {
        make_raise_task_with_status(error_type, 500)
    }

    fn make_raise_task_with_status(error_type: &str, status: u16) -> TaskDefinition {
        TaskDefinition::Raise(RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(
                ErrorDefinition::new(error_type, "Test Error", json!(status), None, None),
            )),
            common: TaskDefinitionFields::new(),
        })
    }

    fn catch_all() -> ErrorCatcherDefinition {
        ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: None,
            retry: None,
            do_: None,
        }
    }

    fn catch_with_filter(type_filter: &str) -> ErrorCatcherDefinition {
        ErrorCatcherDefinition {
            errors: Some(ErrorFilterDefinition {
                with: Some(ErrorFilterProperties {
                    type_: Some(type_filter.to_string()),
                    status: None,
                    title: None,
                    detail: None,
                    instance: None,
                }),
            }),
            when: None,
            except_when: None,
            as_: None,
            retry: None,
            do_: None,
        }
    }

    #[tokio::test]
    async fn test_try_no_error() {
        let runner = make_try_runner(
            "safeTask",
            vec![("task1", make_set_task("result", json!(42)))],
            catch_all(),
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["result"], json!(42));
    }

    #[tokio::test]
    async fn test_try_catch_all() {
        let runner = make_try_runner(
            "riskyTask",
            vec![(
                "task1",
                make_raise_task("https://serverlessworkflow.io/spec/1.0.0/errors/runtime"),
            )],
            catch_all(),
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        // Should catch the error and return input (swallowed)
        let output = runner
            .run(json!({"original": "data"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["original"], json!("data"));
    }

    #[tokio::test]
    async fn test_try_catch_match_type() {
        let runner = make_try_runner(
            "filteredCatch",
            vec![(
                "task1",
                make_raise_task("https://serverlessworkflow.io/spec/1.0.0/errors/authentication"),
            )],
            catch_with_filter("authentication"),
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        // Should catch because type matches
        let output = runner
            .run(json!({"data": "safe"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["data"], json!("safe"));
    }

    #[tokio::test]
    async fn test_try_catch_no_match() {
        let runner = make_try_runner(
            "missedCatch",
            vec![("task1", make_raise_task("runtime"))],
            catch_with_filter("authentication"),
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        // Should NOT catch because type doesn't match
        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_catch_with_do() {
        let catch_do_tasks = vec![("compensate", make_set_task("compensated", json!(true)))];
        let mut catch_do_entries = Vec::new();
        for (name, task) in catch_do_tasks {
            catch_do_entries.push((name.to_string(), task));
        }

        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: None,
            retry: None,
            do_: Some(Map {
                entries: catch_do_entries,
            }),
        };

        let runner = make_try_runner(
            "tryWithCompensation",
            vec![("task1", make_raise_task("runtime"))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["compensated"], json!(true));
    }

    #[tokio::test]
    async fn test_try_retry_constant() {
        let retry = RetryPolicyDefinition {
            delay: Some(serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Duration(serverless_workflow_core::models::duration::Duration::from_milliseconds(10))),
            backoff: None,
            limit: Some(RetryPolicyLimitDefinition {
                attempt: Some(RetryAttemptLimitDefinition { count: Some(3), duration: None }),
                duration: None,
            }),
            when: None,
            except_when: None,
            jitter: None,
        };

        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: None,
            retry: Some(OneOfRetryPolicyDefinitionOrReference::Retry(Box::new(retry))),
            do_: None,
        };

        let runner = make_try_runner(
            "retryAlwaysFail",
            vec![("task1", make_raise_task("runtime"))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_catch_filter_by_status() {
        // Test error filter matching by status code
        let catch = ErrorCatcherDefinition {
            errors: Some(ErrorFilterDefinition {
                with: Some(ErrorFilterProperties {
                    type_: None,
                    status: Some(json!(401)),
                    title: None,
                    detail: None,
                    instance: None,
                }),
            }),
            when: None,
            except_when: None,
            as_: None,
            retry: None,
            do_: None,
        };

        // Raise an authentication error with status 401
        let runner = make_try_runner(
            "statusFilterTask",
            vec![("task1", make_raise_task_with_status("authentication", 401))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner
            .run(json!({"data": "safe"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["data"], json!("safe"));
    }

    #[tokio::test]
    async fn test_try_catch_filter_by_title() {
        // Test error filter matching by title
        let catch = ErrorCatcherDefinition {
            errors: Some(ErrorFilterDefinition {
                with: Some(ErrorFilterProperties {
                    type_: None,
                    status: None,
                    title: Some("Test Error".to_string()),
                    detail: None,
                    instance: None,
                }),
            }),
            when: None,
            except_when: None,
            as_: None,
            retry: None,
            do_: None,
        };

        let runner = make_try_runner(
            "titleFilterTask",
            vec![("task1", make_raise_task("runtime"))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({"ok": true}), &mut support).await.unwrap();
        assert_eq!(output["ok"], json!(true));
    }

    #[tokio::test]
    async fn test_try_catch_with_when_expression() {
        // Test catch.when expression - only catch when expression evaluates to true
        let catch = ErrorCatcherDefinition {
            errors: None,
            when: Some("${ .status == 401 }".to_string()),
            except_when: None,
            as_: None,
            retry: None,
            do_: Some({
                let entries = vec![(
                    "handleAuth".to_string(),
                    make_set_task("handled", json!("auth_error")),
                )];
                Map { entries }
            }),
        };

        // Raise an authentication error (status 401) - should be caught
        let runner = make_try_runner(
            "whenExprTask",
            vec![("task1", make_raise_task_with_status("authentication", 401))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["handled"], json!("auth_error"));
    }

    #[tokio::test]
    async fn test_try_catch_with_except_when_expression() {
        // Test catch.except_when expression - do NOT catch when expression evaluates to true
        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: Some("${ .status == 500 }".to_string()),
            as_: None,
            retry: None,
            do_: Some({
                let entries = vec![(
                    "handleNon500".to_string(),
                    make_set_task("caught", json!(true)),
                )];
                Map { entries }
            }),
        };

        // Raise an authentication error (status 401) - should be caught (except_when is false for 401)
        let runner = make_try_runner(
            "exceptWhenTask",
            vec![("task1", make_raise_task_with_status("authentication", 401))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["caught"], json!(true));
    }

    #[tokio::test]
    async fn test_try_catch_with_as_variable() {
        // Test catch.as - error should be available as the named variable
        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: Some("myError".to_string()),
            retry: None,
            do_: Some({
                let entries = vec![(
                    "inspectError".to_string(),
                    make_set_task("errorType", json!("${ $myError.type }")),
                )];
                Map { entries }
            }),
        };

        let runner = make_try_runner(
            "asVarTask",
            vec![("task1", make_raise_task_with_status("authentication", 401))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert!(output["errorType"].is_string());
        assert!(output["errorType"]
            .as_str()
            .unwrap()
            .contains("authentication"));
    }

    #[tokio::test]
    async fn test_try_catch_filter_by_details() {
        // Matches Java SDK's try-catch-match-details - filter by error details
        let catch = ErrorCatcherDefinition {
            errors: Some(ErrorFilterDefinition {
                with: Some(ErrorFilterProperties {
                    type_: Some("runtime".to_string()),
                    status: Some(json!(500)),
                    detail: Some("Enforcement Failure".to_string()),
                    title: None,
                    instance: None,
                }),
            }),
            when: None,
            except_when: None,
            as_: None,
            retry: None,
            do_: Some({
                let entries = vec![(
                    "handleError".to_string(),
                    make_set_task("recovered", json!(true)),
                )];
                Map { entries }
            }),
        };

        // Raise error with matching details
        let runner = make_try_runner(
            "detailsMatchTask",
            vec![(
                "task1",
                TaskDefinition::Raise(RaiseTaskDefinition {
                    raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(
                        ErrorDefinition::new(
                            "runtime",
                            "Test Error",
                            json!(500),
                            Some("Enforcement Failure".to_string()),
                            None,
                        ),
                    )),
                    common: TaskDefinitionFields::new(),
                }),
            )],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["recovered"], json!(true));
    }

    #[tokio::test]
    async fn test_try_catch_not_match_details() {
        // Matches Java SDK's try-catch-not-match-details - details mismatch propagates error
        let catch = ErrorCatcherDefinition {
            errors: Some(ErrorFilterDefinition {
                with: Some(ErrorFilterProperties {
                    type_: Some("runtime".to_string()),
                    status: None,
                    detail: Some("User not found".to_string()),
                    title: None,
                    instance: None,
                }),
            }),
            when: None,
            except_when: None,
            as_: None,
            retry: None,
            do_: None,
        };

        // Raise error with NON-matching details
        let runner = make_try_runner(
            "detailsMismatchTask",
            vec![(
                "task1",
                TaskDefinition::Raise(RaiseTaskDefinition {
                    raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(
                        ErrorDefinition::new(
                            "runtime",
                            "Test Error",
                            json!(500),
                            Some("Enforcement Failure".to_string()),
                            None,
                        ),
                    )),
                    common: TaskDefinitionFields::new(),
                }),
            )],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        // Should NOT catch because details don't match
        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_catch_not_match_status() {
        // Matches Java SDK's try-catch-not-match-status - status mismatch propagates error
        let catch = ErrorCatcherDefinition {
            errors: Some(ErrorFilterDefinition {
                with: Some(ErrorFilterProperties {
                    type_: Some("runtime".to_string()),
                    status: Some(json!(403)),
                    detail: None,
                    title: None,
                    instance: None,
                }),
            }),
            when: None,
            except_when: None,
            as_: None,
            retry: None,
            do_: None,
        };

        // Raise error with status 500, but filter expects 403
        let runner = make_try_runner(
            "statusMismatchTask",
            vec![("task1", make_raise_task("runtime"))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_catch_match_when_not_matching() {
        // Matches Java SDK's try-catch-not-match-when - when expression evaluates to false
        let catch = ErrorCatcherDefinition {
            errors: None,
            when: Some("${ .status == 400 }".to_string()),
            except_when: None,
            as_: None,
            retry: None,
            do_: Some({
                let entries = vec![(
                    "handleError".to_string(),
                    make_set_task("recovered", json!(true)),
                )];
                Map { entries }
            }),
        };

        // Raise error with status 503, when expects 400
        let runner = make_try_runner(
            "whenNotMatchTask",
            vec![("task1", make_raise_task_with_status("runtime", 503))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        // Should NOT catch because when expression is false
        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_catch_with_as_variable_details() {
        // Matches Java SDK's try-catch-error-variable - using $caughtError.details
        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: Some("caughtError".to_string()),
            retry: None,
            do_: Some({
                let entries = vec![(
                    "handleError".to_string(),
                    make_set_task("errorMessage", json!("${ $caughtError.detail }")),
                )];
                Map { entries }
            }),
        };

        let runner = make_try_runner(
            "asVarDetailsTask",
            vec![(
                "task1",
                TaskDefinition::Raise(RaiseTaskDefinition {
                    raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(
                        ErrorDefinition::new(
                            "runtime",
                            "Test Error",
                            json!(503),
                            Some("Javierito was here!".to_string()),
                            None,
                        ),
                    )),
                    common: TaskDefinitionFields::new(),
                }),
            )],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert!(output["errorMessage"].is_string());
        assert!(output["errorMessage"]
            .as_str()
            .unwrap()
            .contains("Javierito was here!"));
    }

    #[tokio::test]
    async fn test_try_catch_type_and_status_combo_filter() {
        // Matches Java SDK's try-catch-match-status - filter by type AND status together
        let catch = ErrorCatcherDefinition {
            errors: Some(ErrorFilterDefinition {
                with: Some(ErrorFilterProperties {
                    type_: Some("communication".to_string()),
                    status: Some(json!(404)),
                    detail: None,
                    title: None,
                    instance: None,
                }),
            }),
            when: None,
            except_when: None,
            as_: None,
            retry: None,
            do_: Some({
                let entries = vec![(
                    "handleError".to_string(),
                    make_set_task("recovered", json!(true)),
                )];
                Map { entries }
            }),
        };

        // Raise error matching both type (short name: communication) and status (404)
        let runner = make_try_runner(
            "comboFilterTask",
            vec![("task1", make_raise_task_with_status("communication", 404))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["recovered"], json!(true));
    }

    #[tokio::test]
    async fn test_try_catch_retry_reference() {
        // Matches Java SDK's try-catch-retry-reusable - retry using reference from workflow.use.retries
        let retry = RetryPolicyDefinition {
            delay: Some(serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Duration(
                serverless_workflow_core::models::duration::Duration::from_milliseconds(10)
            )),
            backoff: None,
            limit: Some(RetryPolicyLimitDefinition {
                attempt: Some(RetryAttemptLimitDefinition { count: Some(2), duration: None }),
                duration: None,
            }),
            when: None,
            except_when: None,
            jitter: None,
        };

        let mut retries = HashMap::new();
        retries.insert("default".to_string(), retry);
        let use_def = serverless_workflow_core::models::workflow::ComponentDefinitionCollection {
            retries: Some(retries),
            ..Default::default()
        };
        let workflow = WorkflowDefinition {
            use_: Some(use_def),
            ..Default::default()
        };

        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: None,
            retry: Some(OneOfRetryPolicyDefinitionOrReference::Reference(
                "default".to_string(),
            )),
            do_: None,
        };

        let runner = make_try_runner_with_workflow(
            "retryRefTask",
            vec![("task1", make_raise_task("runtime"))],
            catch,
            &workflow,
        );

        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        // Should retry 2 times then fail
        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_retry_with_when_condition() {
        // Retry policy with when condition - only retry if error status is 5xx
        let retry = RetryPolicyDefinition {
            delay: Some(serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Duration(
                serverless_workflow_core::models::duration::Duration::from_milliseconds(10)
            )),
            backoff: None,
            limit: Some(RetryPolicyLimitDefinition {
                attempt: Some(RetryAttemptLimitDefinition { count: Some(3), duration: None }),
                duration: None,
            }),
            when: Some("${ .status >= 500 }".to_string()),
            except_when: None,
            jitter: None,
        };

        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: None,
            retry: Some(OneOfRetryPolicyDefinitionOrReference::Retry(Box::new(retry))),
            do_: None,
        };

        // Raise error with status 401 (not >= 500) - should NOT retry, error propagates immediately
        let runner = make_try_runner(
            "retryWhenTask",
            vec![("task1", make_raise_task_with_status("authentication", 401))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Error should propagate without retry since status 401 < 500
        assert!(err.status().is_some());
        assert_eq!(err.status().unwrap(), &json!(401));
    }

    #[tokio::test]
    async fn test_try_retry_with_except_when_condition() {
        // Retry policy with except_when condition - do NOT retry if error type is authentication
        let retry = RetryPolicyDefinition {
            delay: Some(serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Duration(
                serverless_workflow_core::models::duration::Duration::from_milliseconds(10)
            )),
            backoff: None,
            limit: Some(RetryPolicyLimitDefinition {
                attempt: Some(RetryAttemptLimitDefinition { count: Some(3), duration: None }),
                duration: None,
            }),
            when: None,
            except_when: Some("${ .type == \"authentication\" }".to_string()),
            jitter: None,
        };

        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: None,
            retry: Some(OneOfRetryPolicyDefinitionOrReference::Retry(Box::new(retry))),
            do_: None,
        };

        // Raise authentication error - should NOT retry because except_when matches
        let runner = make_try_runner(
            "retryExceptWhenTask",
            vec![("task1", make_raise_task_with_status("authentication", 401))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_retry_when_condition_allows_retry() {
        // Retry policy with when condition - retry allowed for 5xx errors
        let retry = RetryPolicyDefinition {
            delay: Some(serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Duration(
                serverless_workflow_core::models::duration::Duration::from_milliseconds(10)
            )),
            backoff: None,
            limit: Some(RetryPolicyLimitDefinition {
                attempt: Some(RetryAttemptLimitDefinition { count: Some(2), duration: None }),
                duration: None,
            }),
            when: Some("${ .status >= 500 }".to_string()),
            except_when: None,
            jitter: None,
        };

        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: None,
            retry: Some(OneOfRetryPolicyDefinitionOrReference::Retry(Box::new(retry))),
            do_: None,
        };

        // Raise runtime error with status 500 - should retry (when condition is true)
        // Will still fail after retries exhausted
        let runner = make_try_runner(
            "retryWhenAllowsTask",
            vec![("task1", make_raise_task_with_status("runtime", 500))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_retry_exponential_backoff() {
        // Matches Java SDK's try-catch-retry-inline.yaml - retry with exponential backoff
        use serverless_workflow_core::models::retry::{
            BackoffStrategyDefinition, ExponentialBackoffDefinition,
        };

        let retry = RetryPolicyDefinition {
            delay: Some(serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Duration(
                serverless_workflow_core::models::duration::Duration::from_milliseconds(10)
            )),
            backoff: Some(BackoffStrategyDefinition {
                constant: None,
                exponential: Some(ExponentialBackoffDefinition::default()),
                linear: None,
            }),
            limit: Some(RetryPolicyLimitDefinition {
                attempt: Some(RetryAttemptLimitDefinition { count: Some(3), duration: None }),
                duration: None,
            }),
            when: None,
            except_when: None,
            jitter: None,
        };

        let catch = ErrorCatcherDefinition {
            errors: Some(ErrorFilterDefinition {
                with: Some(ErrorFilterProperties {
                    type_: Some("runtime".to_string()),
                    status: None,
                    title: None,
                    detail: None,
                    instance: None,
                }),
            }),
            when: None,
            except_when: None,
            as_: None,
            retry: Some(OneOfRetryPolicyDefinitionOrReference::Retry(Box::new(retry))),
            do_: None,
        };

        // Task always fails, should retry 3 times with exponential backoff then error
        let runner = make_try_runner(
            "exponentialRetryTask",
            vec![("task1", make_raise_task("runtime"))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_retry_linear_backoff() {
        // Test retry with linear backoff strategy
        use serverless_workflow_core::models::retry::{
            BackoffStrategyDefinition, LinearBackoffDefinition,
        };

        let retry = RetryPolicyDefinition {
            delay: Some(serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Duration(
                serverless_workflow_core::models::duration::Duration::from_milliseconds(10)
            )),
            backoff: Some(BackoffStrategyDefinition {
                constant: None,
                exponential: None,
                linear: Some(LinearBackoffDefinition {
                    increment: Some(serverless_workflow_core::models::duration::Duration::from_milliseconds(5)),
                    definition: None,
                }),
            }),
            limit: Some(RetryPolicyLimitDefinition {
                attempt: Some(RetryAttemptLimitDefinition { count: Some(2), duration: None }),
                duration: None,
            }),
            when: None,
            except_when: None,
            jitter: None,
        };

        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: None,
            retry: Some(OneOfRetryPolicyDefinitionOrReference::Retry(Box::new(retry))),
            do_: None,
        };

        let runner = make_try_runner(
            "linearRetryTask",
            vec![("task1", make_raise_task("runtime"))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_catch_with_as_and_do_tasks() {
        // Matches Java SDK's try-catch-error-variable full pattern
        // catch.as + catch.do with a task that accesses $caughtError
        // Note: Set tasks replace the entire output, so use a single set task
        // that combines multiple fields using JQ object construction
        let catch = ErrorCatcherDefinition {
            errors: None,
            when: None,
            except_when: None,
            as_: Some("caughtError".to_string()),
            retry: None,
            do_: Some({
                let entries = vec![(
                    "handleError".to_string(),
                    make_set_task(
                        "errorInfo",
                        json!("${ {type: $caughtError.type, status: $caughtError.status} }"),
                    ),
                )];
                Map { entries }
            }),
        };

        let runner = make_try_runner(
            "asAndDoTask",
            vec![("task1", make_raise_task_with_status("communication", 503))],
            catch,
        );

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // The set task produces {errorInfo: {type: ..., status: ...}}
        assert!(
            output.get("errorInfo").is_some(),
            "expected errorInfo field, got: {:?}",
            output
        );
        let info = &output["errorInfo"];
        if let Some(obj) = info.as_object() {
            assert!(obj.get("type").is_some(), "expected type in errorInfo");
            assert!(obj.get("status").is_some(), "expected status in errorInfo");
            if let Some(type_val) = obj.get("type").and_then(|v| v.as_str()) {
                assert!(
                    type_val.contains("communication"),
                    "expected 'communication' in type, got: {}",
                    type_val
                );
            }
        }
    }
}
