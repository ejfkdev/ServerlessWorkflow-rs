use crate::error::{WorkflowError, WorkflowResult};
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::tasks::task_name_impl;

use serde_json::Value;
use serverless_workflow_core::models::error::OneOfErrorDefinitionOrReference;
use serverless_workflow_core::models::expression::is_strict_expr;
use serverless_workflow_core::models::task::RaiseTaskDefinition;
use serverless_workflow_core::models::workflow::WorkflowDefinition;

/// Evaluates an optional strict expression string, returning the evaluated string result.
/// If the expression is a strict JQ expression (`${...}`), it is evaluated via `eval_jq_expr`;
/// on evaluation failure, falls back to the raw string. Non-strict strings are returned as-is.
fn eval_strict_expr(s: &str, input: &Value, support: &TaskSupport<'_>, task_name: &str) -> String {
    if is_strict_expr(s) {
        let val = support
            .eval_jq_expr(s, input, task_name)
            .unwrap_or_else(|_| Value::String(s.to_string()));
        val.as_str()
            .map(|v| v.to_string())
            .unwrap_or_else(|| format!("{}", val))
    } else {
        s.to_string()
    }
}

/// Runner for Raise tasks - raises structured errors
pub struct RaiseTaskRunner {
    name: String,
    error_def: OneOfErrorDefinitionOrReference,
}

impl RaiseTaskRunner {
    pub fn new(
        name: &str,
        task: &RaiseTaskDefinition,
        workflow: &WorkflowDefinition,
    ) -> WorkflowResult<Self> {
        let error_def = resolve_error_definition(&task.raise.error, workflow);
        Ok(Self {
            name: name.to_string(),
            error_def,
        })
    }
}

/// Resolves error references from workflow's use.errors collection
fn resolve_error_definition(
    error: &OneOfErrorDefinitionOrReference,
    workflow: &WorkflowDefinition,
) -> OneOfErrorDefinitionOrReference {
    match error {
        OneOfErrorDefinitionOrReference::Reference(ref_name) => {
            if let Some(ref use_) = workflow.use_ {
                if let Some(ref errors) = use_.errors {
                    if let Some(definition) = errors.get(ref_name) {
                        return OneOfErrorDefinitionOrReference::Error(definition.clone());
                    }
                }
            }
            error.clone()
        }
        _ => error.clone(),
    }
}

#[async_trait::async_trait]
impl TaskRunner for RaiseTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        let instance = support.get_task_reference().unwrap_or("/").to_string();

        match &self.error_def {
            OneOfErrorDefinitionOrReference::Reference(ref_name) => Err(WorkflowError::typed(
                "validation",
                format!("error definition '{}' not found in 'use.errors'", ref_name),
                self.name.clone(),
                instance,
                Some(Value::from(400)),
                Some("Validation Error".to_string()),
            )),
            OneOfErrorDefinitionOrReference::Error(def) => {
                // Evaluate detail expression if present
                let detail_str = def
                    .detail
                    .as_deref()
                    .map(|d| eval_strict_expr(d, &input, support, &self.name))
                    .unwrap_or_default();

                // Evaluate title expression if present
                let title_str = def
                    .title
                    .as_deref()
                    .map(|t| eval_strict_expr(t, &input, support, &self.name));

                // Use error definition's instance if set, otherwise task reference
                let instance = def
                    .instance
                    .as_deref()
                    .unwrap_or_else(|| support.get_task_reference().unwrap_or("/"))
                    .to_string();

                let err = WorkflowError::typed(
                    def.type_.as_str(),
                    detail_str,
                    self.name.clone(),
                    instance,
                    Some(def.status.clone()),
                    title_str,
                );

                Err(err)
            }
        }
    }

    task_name_impl!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::WorkflowContext;
    use crate::default_support;
    use serde_json::json;
    use serverless_workflow_core::models::error::{ErrorDefinition, ErrorTypes};
    use serverless_workflow_core::models::task::{RaiseErrorDefinition, TaskDefinitionFields};
    use serverless_workflow_core::models::workflow::WorkflowDefinitionMetadata;

    async fn run_raise(task: RaiseTaskDefinition) -> WorkflowResult<Value> {
        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow)?;
        let mut support = TaskSupport::new(&workflow, &mut context);
        let runner = RaiseTaskRunner::new("test_raise", &task, &workflow)?;
        runner.run(json!({}), &mut support).await
    }

    #[tokio::test]
    async fn test_raise_with_defined_validation_error() {
        let error_def = ErrorDefinition::new(
            ErrorTypes::VALIDATION,
            "Validation Error",
            json!(400),
            Some("Invalid input data".to_string()),
            None,
        );
        let task = RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(error_def)),
            common: TaskDefinitionFields::new(),
        };

        let result = run_raise(task).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "validation");
        assert!(err.to_string().contains("Invalid input data"));
    }

    #[tokio::test]
    async fn test_raise_with_timeout_error_and_expression() {
        let error_def = ErrorDefinition::new(
            ErrorTypes::TIMEOUT,
            "Timeout Error",
            json!(408),
            Some("${ .timeoutMessage }".to_string()),
            None,
        );
        let task = RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(error_def)),
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);
        let runner = RaiseTaskRunner::new("test_raise_timeout", &task, &workflow).unwrap();

        let input = json!({"timeoutMessage": "Request took too long"});
        let result = runner.run(input, &mut support).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "timeout");
        assert!(err.to_string().contains("Request took too long"));
    }

    #[tokio::test]
    async fn test_raise_with_referenced_error_not_found() {
        let task = RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Reference(
                "someErrorRef".to_string(),
            )),
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);
        let runner = RaiseTaskRunner::new("test_raise_ref", &task, &workflow).unwrap();

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("someErrorRef"));
    }

    #[tokio::test]
    async fn test_raise_with_communication_error() {
        let error_def = ErrorDefinition::new(
            ErrorTypes::COMMUNICATION,
            "Communication Error",
            json!(500),
            Some("Service unavailable".to_string()),
            None,
        );
        let task = RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(error_def)),
            common: TaskDefinitionFields::new(),
        };

        let result = run_raise(task).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "communication");
    }

    #[tokio::test]
    async fn test_raise_with_runtime_error() {
        let error_def = ErrorDefinition::new(
            ErrorTypes::RUNTIME,
            "Runtime Error",
            json!(500),
            Some("Unexpected failure".to_string()),
            Some("/task_runtime".to_string()),
        );
        let task = RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(error_def)),
            common: TaskDefinitionFields::new(),
        };

        let result = run_raise(task).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "runtime");
        assert_eq!(err.instance(), Some("/task_runtime"));
    }

    #[tokio::test]
    async fn test_raise_reusable_error_reference() {
        // Matches Java SDK's raise-reusable.yaml - error defined in use.errors, referenced by name
        let error_def = ErrorDefinition::new(
            "https://serverlessworkflow.io/errors/not-implemented",
            "Not Implemented",
            json!(500),
            Some("Feature not available".to_string()),
            None,
        );

        let mut errors = std::collections::HashMap::new();
        errors.insert("notImplemented".to_string(), error_def);
        let use_def = serverless_workflow_core::models::workflow::ComponentDefinitionCollection {
            errors: Some(errors),
            ..Default::default()
        };
        let workflow = WorkflowDefinition {
            use_: Some(use_def),
            ..Default::default()
        };

        let task = RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Reference(
                "notImplemented".to_string(),
            )),
            common: TaskDefinitionFields::new(),
        };

        default_support!(workflow, context, support);
        let runner = RaiseTaskRunner::new("testRaiseRef", &task, &workflow).unwrap();

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.error_type().contains("not-implemented"));
        assert!(err.to_string().contains("Feature not available"));
    }

    #[tokio::test]
    async fn test_raise_inline_full_error() {
        // Matches Java SDK's raise-inline.yaml - full inline error with all fields
        let error_def = ErrorDefinition::new(
            "https://serverlessworkflow.io/errors/not-implemented",
            "Not Implemented",
            json!(500),
            Some("The workflow is a work in progress".to_string()),
            None,
        );
        let task = RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(error_def)),
            common: TaskDefinitionFields::new(),
        };

        let result = run_raise(task).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.error_type().contains("not-implemented"));
        assert_eq!(err.status(), Some(&json!(500)));
        assert!(err.title().is_some());
    }

    #[tokio::test]
    async fn test_raise_error_with_dynamic_detail() {
        // Matches Go SDK's raise_error_with_input.yaml
        // Raise error with string interpolation in detail using \() syntax
        let error_def = ErrorDefinition::new(
            "https://serverlessworkflow.io/spec/1.0.0/errors/authentication",
            "Authentication Error",
            json!(401),
            Some("${ \"User authentication failed: \\(.reason)\" }".to_string()),
            None,
        );
        let task = RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(error_def)),
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);
        let runner = RaiseTaskRunner::new("dynamicError", &task, &workflow).unwrap();

        let input = json!({"reason": "User token expired"});
        let result = runner.run(input, &mut support).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.error_type().contains("authentication"));
        assert!(err
            .to_string()
            .contains("User authentication failed: User token expired"));
    }

    #[tokio::test]
    async fn test_raise_undefined_reference_returns_validation_error() {
        // Matches Go SDK's raise_undefined_reference.yaml
        // When referencing an undefined error, should return a validation error
        let task = RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Reference(
                "UndefinedError".to_string(),
            )),
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);
        let runner = RaiseTaskRunner::new("missingError", &task, &workflow).unwrap();

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Go SDK returns validation error type for undefined references
        assert!(
            err.error_type().contains("validation") || err.to_string().contains("UndefinedError")
        );
    }

    #[tokio::test]
    async fn test_raise_with_workflow_context_in_detail() {
        // Matches Go SDK's raise_inline.yaml
        // Raise error with $workflow.definition.document.name in detail
        let error_def = ErrorDefinition::new(
            "https://serverlessworkflow.io/spec/1.0.0/errors/validation",
            "Validation Error",
            json!(400),
            Some("${ \"Invalid input provided to workflow \\($workflow.definition.document.name)\" }".to_string()),
            None,
        );
        let task = RaiseTaskDefinition {
            raise: RaiseErrorDefinition::new(OneOfErrorDefinitionOrReference::Error(error_def)),
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition {
            document: WorkflowDefinitionMetadata {
                name: "raise-inline".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        default_support!(workflow, context, support);
        let runner = RaiseTaskRunner::new("inlineError", &task, &workflow).unwrap();

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.error_type().contains("validation"));
        // The detail should contain the workflow name
        assert!(err.to_string().contains("raise-inline"));
    }
}
