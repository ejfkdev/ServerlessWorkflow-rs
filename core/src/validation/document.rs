use crate::models::duration::OneOfDurationOrIso8601Expression;
use crate::models::workflow::*;
use super::{ValidationResult, ValidationRule, is_valid_hostname, is_valid_semver, validate_required_hostname, validate_required_semver};
use super::one_of_validators::validate_schedule_one_of;

/// Validates a complete workflow definition
pub fn validate_workflow(workflow: &WorkflowDefinition) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Validate document
    validate_document(&workflow.document, &mut result);

    // Validate input if present
    if let Some(ref input) = workflow.input {
        validate_input(input, "input", &mut result);
    }

    // Validate timeout if present
    if let Some(ref timeout) = workflow.timeout {
        validate_timeout(timeout, "timeout", &mut result);
    }

    // Validate schedule if present
    if let Some(ref schedule) = workflow.schedule {
        validate_schedule_one_of(schedule, "schedule", &mut result);
    }

    // Validate tasks
    super::task::validate_task_map(&workflow.do_, "do", &mut result);

    result
}

/// Validates workflow document metadata
pub(crate) fn validate_document(doc: &WorkflowDefinitionMetadata, result: &mut ValidationResult) {
    validate_required_hostname(&doc.name, "document.name", result);
    validate_required_semver(&doc.version, "document.version", result);
    if !doc.dsl.is_empty() && !is_valid_semver(&doc.dsl) {
        result.add_error(
            "document.dsl",
            ValidationRule::Semver,
            "DSL version must be a valid semantic version",
        );
    }
    if !doc.namespace.is_empty() && !is_valid_hostname(&doc.namespace) {
        result.add_error(
            "document.namespace",
            ValidationRule::Hostname,
            "namespace must be a valid RFC 1123 hostname",
        );
    }
}

/// Validates an input data model definition
pub(crate) fn validate_input(
    input: &crate::models::input::InputDataModelDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    if let Some(ref from) = input.from {
        if from.is_null() {
            result.add_error(
                &format!("{}.from", prefix),
                ValidationRule::Required,
                "input 'from' must not be null",
            );
        }
    }
}

/// Validates a timeout definition or reference
pub(crate) fn validate_timeout(
    timeout: &crate::models::timeout::OneOfTimeoutDefinitionOrReference,
    prefix: &str,
    result: &mut ValidationResult,
) {
    match timeout {
        crate::models::timeout::OneOfTimeoutDefinitionOrReference::Timeout(t) => {
            validate_duration(&t.after, &format!("{}.after", prefix), result);
        }
        crate::models::timeout::OneOfTimeoutDefinitionOrReference::Reference(_) => {
            // References are assumed valid at this level
        }
    }
}

/// Validates a duration value
pub(crate) fn validate_duration(
    duration: &OneOfDurationOrIso8601Expression,
    prefix: &str,
    result: &mut ValidationResult,
) {
    match duration {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            if !crate::models::duration::is_iso8601_duration_valid(expr) {
                result.add_error(
                    prefix,
                    ValidationRule::Iso8601Duration,
                    &format!("'{}' is not a valid ISO 8601 duration", expr),
                );
            }
        }
        OneOfDurationOrIso8601Expression::Duration(_) => {
            // Structured duration is always valid
        }
    }
}
