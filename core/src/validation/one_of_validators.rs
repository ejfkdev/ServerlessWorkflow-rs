use crate::models::authentication::AuthenticationPolicyDefinition;
use crate::models::retry::BackoffStrategyDefinition;
use crate::models::schema::SchemaDefinition;
use crate::models::task::ProcessTypeDefinition;
use crate::models::workflow::WorkflowScheduleDefinition;
use super::{ValidationResult, ValidationRule};

/// Validates that at most one of the given boolean flags is true.
/// Emits a MutualExclusion error if more than one is set.
fn validate_at_most_one(
    flags: &[bool],
    prefix: &str,
    message: &str,
    result: &mut ValidationResult,
) {
    let count = flags.iter().filter(|&&f| f).count();
    if count > 1 {
        result.add_error(prefix, ValidationRule::MutualExclusion, message);
    }
}

/// Validates that exactly one of the given boolean flags is true.
/// Emits a MutualExclusion error if zero or more than one is set.
fn validate_exactly_one(
    flags: &[bool],
    prefix: &str,
    message: &str,
    result: &mut ValidationResult,
) {
    let count = flags.iter().filter(|&&f| f).count();
    if count != 1 {
        result.add_error(prefix, ValidationRule::MutualExclusion, message);
    }
}

/// Validates an authentication policy for oneOf constraint
pub fn validate_auth_policy_one_of(
    policy: &AuthenticationPolicyDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_at_most_one(
        &[
            policy.basic.is_some(),
            policy.bearer.is_some(),
            policy.certificate.is_some(),
            policy.digest.is_some(),
            policy.oauth2.is_some(),
            policy.oidc.is_some(),
        ],
        prefix,
        "authentication policy: only one authentication type must be specified",
        result,
    );
}

/// Validates a workflow schedule definition for oneOf constraint
pub fn validate_schedule_one_of(
    schedule: &WorkflowScheduleDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_at_most_one(
        &[
            schedule.every.is_some(),
            schedule.cron.as_ref().is_some_and(|s| !s.is_empty()),
            schedule.after.is_some(),
            schedule.on.is_some(),
        ],
        prefix,
        "schedule: only one of 'every', 'cron', 'after', or 'on' must be specified",
        result,
    );
}

/// Validates a process type definition for oneOf constraint
pub fn validate_process_type_one_of(
    process: &ProcessTypeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_exactly_one(
        &[
            process.container.is_some(),
            process.script.is_some(),
            process.shell.is_some(),
            process.workflow.is_some(),
        ],
        &format!("{}.run", prefix),
        "run task: exactly one of 'container', 'script', 'shell', or 'workflow' must be specified",
        result,
    );
}

/// Validates a backoff strategy definition for oneOf constraint
pub fn validate_backoff_one_of(
    backoff: &BackoffStrategyDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_at_most_one(
        &[
            backoff.constant.is_some(),
            backoff.exponential.is_some(),
            backoff.linear.is_some(),
        ],
        prefix,
        "backoff: only one of 'constant', 'exponential', or 'linear' must be specified",
        result,
    );
}

/// Validates a schema definition for oneOf constraint
pub fn validate_schema_one_of(
    schema: &SchemaDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_document = schema.document.is_some();
    let has_resource = schema.resource.is_some();

    if has_document && has_resource {
        result.add_error(
            prefix,
            ValidationRule::MutualExclusion,
            "schema: 'document' and 'resource' are mutually exclusive",
        );
    }
    if !has_document && !has_resource {
        result.add_error(
            prefix,
            ValidationRule::Required,
            "schema: either 'document' or 'resource' must be specified",
        );
    }
}
