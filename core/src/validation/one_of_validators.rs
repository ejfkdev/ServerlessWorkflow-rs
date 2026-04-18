use crate::models::authentication::AuthenticationPolicyDefinition;
use crate::models::retry::BackoffStrategyDefinition;
use crate::models::schema::SchemaDefinition;
use crate::models::task::ProcessTypeDefinition;
use crate::models::workflow::WorkflowScheduleDefinition;
use super::{ValidationResult, ValidationRule};

/// Validates an authentication policy for oneOf constraint
/// Only one authentication scheme (basic/bearer/digest/oauth2/oidc/certificate) should be set
/// Matches Go SDK's AuthenticationPolicy.UnmarshalJSON which enforces exactly one
pub fn validate_auth_policy_one_of(
    policy: &AuthenticationPolicyDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let count = [
        policy.basic.is_some(),
        policy.bearer.is_some(),
        policy.certificate.is_some(),
        policy.digest.is_some(),
        policy.oauth2.is_some(),
        policy.oidc.is_some(),
    ]
    .iter()
    .filter(|&&f| f)
    .count();

    if count > 1 {
        result.add_error(
            prefix,
            ValidationRule::MutualExclusion,
            "authentication policy: only one authentication type must be specified",
        );
    }
}

/// Validates a workflow schedule definition for oneOf constraint
/// Only one of 'every', 'cron', 'after', 'on' should be set
/// Matches Go SDK's Schedule validation which enforces mutual exclusivity
pub fn validate_schedule_one_of(
    schedule: &WorkflowScheduleDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let count = [
        schedule.every.is_some(),
        schedule.cron.as_ref().is_some_and(|s| !s.is_empty()),
        schedule.after.is_some(),
        schedule.on.is_some(),
    ]
    .iter()
    .filter(|&&f| f)
    .count();

    if count > 1 {
        result.add_error(
            prefix,
            ValidationRule::MutualExclusion,
            "schedule: only one of 'every', 'cron', 'after', or 'on' must be specified",
        );
    }
}

/// Validates a process type definition for oneOf constraint
/// Only one of 'container', 'script', 'shell', 'workflow' should be set
/// Matches Go SDK's RunTaskConfiguration.UnmarshalJSON which enforces exactly one
pub fn validate_process_type_one_of(
    process: &ProcessTypeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let count = [
        process.container.is_some(),
        process.script.is_some(),
        process.shell.is_some(),
        process.workflow.is_some(),
    ]
    .iter()
    .filter(|&&f| f)
    .count();

    if count != 1 {
        result.add_error(
            &format!("{}.run", prefix),
            ValidationRule::MutualExclusion,
            "run task: exactly one of 'container', 'script', 'shell', or 'workflow' must be specified",
        );
    }
}

/// Validates a backoff strategy definition for oneOf constraint
/// Only one of 'constant', 'exponential', or 'linear' should be set
/// Matches Go SDK's RetryBackoff.UnmarshalJSON which enforces oneOf
pub fn validate_backoff_one_of(
    backoff: &BackoffStrategyDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let count = [
        backoff.constant.is_some(),
        backoff.exponential.is_some(),
        backoff.linear.is_some(),
    ]
    .iter()
    .filter(|&&f| f)
    .count();

    if count > 1 {
        result.add_error(
            prefix,
            ValidationRule::MutualExclusion,
            "backoff: only one of 'constant', 'exponential', or 'linear' must be specified",
        );
    }
}

/// Validates a schema definition for oneOf constraint
/// Must specify either 'document' or 'resource', but not both
/// Matches Go SDK's Schema.UnmarshalJSON which enforces exactly one
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
