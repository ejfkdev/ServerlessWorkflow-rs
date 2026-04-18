use crate::models::task::ContainerLifetimeDefinition;
use super::{ValidationResult, ValidationRule};

/// Valid HTTP output formats (matching Go SDK's oneof validation)
const VALID_HTTP_OUTPUT_FORMATS: &[&str] = &["raw", "content", "response"];

/// Valid AsyncAPI protocols (matching Go SDK's oneof validation)
const VALID_ASYNCAPI_PROTOCOLS: &[&str] = &[
    "amqp",
    "amqp1",
    "anypointmq",
    "googlepubsub",
    "http",
    "ibmmq",
    "jms",
    "kafka",
    "mercure",
    "mqtt",
    "mqtt5",
    "nats",
    "pulsar",
    "redis",
    "sns",
    "solace",
    "sqs",
    "stomp",
    "ws",
];

/// Helper to validate a string value against a list of allowed enum values.
pub(crate) fn validate_enum_value(
    value: &str,
    valid_values: &[&str],
    field_name: &str,
    prefix: &str,
    skip_empty: bool,
    case_insensitive: bool,
    result: &mut ValidationResult,
) {
    if skip_empty && value.is_empty() {
        return;
    }
    let is_valid = if case_insensitive {
        valid_values.iter().any(|v| v.eq_ignore_ascii_case(value))
    } else {
        valid_values.contains(&value)
    };
    if !is_valid {
        let valid_list = valid_values
            .iter()
            .map(|v| format!("'{}'", v))
            .collect::<Vec<_>>()
            .join(", ");
        result.add_error(
            &format!("{}.{}", prefix, field_name),
            ValidationRule::InvalidValue,
            &format!("{} must be one of {}, got '{}'", field_name, valid_list, value),
        );
    }
}

/// Validates an AsyncAPI protocol value
/// Must be one of: amqp, amqp1, anypointmq, googlepubsub, http, ibmmq, jms, kafka, mercure, mqtt, mqtt5, nats, pulsar, redis, sns, solace, sqs, stomp, ws
pub fn validate_asyncapi_protocol(protocol: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(protocol, VALID_ASYNCAPI_PROTOCOLS, "protocol", prefix, true, false, result);
}

/// Validates an HTTP call output format value
/// Must be one of: "raw", "content", "response"
pub fn validate_http_output(output: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(output, VALID_HTTP_OUTPUT_FORMATS, "output", prefix, true, false, result);
}

/// Valid pull policies
const VALID_PULL_POLICIES: &[&str] = &["ifNotPresent", "always", "never"];

/// Valid container cleanup policies
const VALID_CONTAINER_CLEANUPS: &[&str] = &["always", "never", "eventually"];

/// Valid script languages
const VALID_SCRIPT_LANGUAGES: &[&str] = &["javascript", "js", "python"];

/// Validates a container's pull policy value
/// Must be one of: "ifNotPresent", "always", "never"
pub fn validate_pull_policy(pull_policy: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(pull_policy, VALID_PULL_POLICIES, "pullPolicy", prefix, false, false, result);
}

/// Validates a container's cleanup policy value
/// Must be one of: "always", "never", "eventually"
pub fn validate_container_cleanup(cleanup: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(cleanup, VALID_CONTAINER_CLEANUPS, "lifetime.cleanup", prefix, false, false, result);
}

/// Validates a script's language value
/// Must be one of: "javascript", "js", "python"
pub fn validate_script_language(language: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(language, VALID_SCRIPT_LANGUAGES, "language", prefix, false, false, result);
}

/// Valid OAuth2 client authentication methods
const VALID_OAUTH2_CLIENT_AUTH_METHODS: &[&str] = &[
    "client_secret_basic",
    "client_secret_post",
    "client_secret_jwt",
    "private_key_jwt",
    "none",
];

/// Valid OAuth2 request encodings
const VALID_OAUTH2_REQUEST_ENCODINGS: &[&str] = &[
    "application/x-www-form-urlencoded",
    "application/json",
];

/// Valid OAuth2 grant types
const VALID_OAUTH2_GRANT_TYPES: &[&str] = &[
    "authorization_code",
    "client_credentials",
    "password",
    "refresh_token",
    "urn:ietf:params:oauth:grant-type:token-exchange",
];

/// Validates an OAuth2 client authentication method value
/// Empty string is allowed (optional field)
pub fn validate_oauth2_client_auth_method(
    method: &str,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_enum_value(method, VALID_OAUTH2_CLIENT_AUTH_METHODS, "client.authentication", prefix, true, false, result);
}

/// Validates an OAuth2 token request encoding value
/// Empty string is allowed (optional field)
pub fn validate_oauth2_request_encoding(
    encoding: &str,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_enum_value(encoding, VALID_OAUTH2_REQUEST_ENCODINGS, "request.encoding", prefix, true, false, result);
}

/// Validates an OAuth2 grant type value
/// Empty string is allowed (optional field)
pub fn validate_oauth2_grant_type(grant: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(grant, VALID_OAUTH2_GRANT_TYPES, "grant", prefix, true, false, result);
}

/// Valid extension task types (matching Go SDK's oneof validation)
const VALID_EXTENSION_TASK_TYPES: &[&str] = &[
    "call",
    "composite",
    "emit",
    "for",
    "listen",
    "raise",
    "run",
    "set",
    "switch",
    "try",
    "wait",
    "all",
];

/// Validates an extension's task type value
/// Must be one of: call, composite, emit, for, listen, raise, run, set, switch, try, wait, all
pub fn validate_extension_task_type(extend: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(extend, VALID_EXTENSION_TASK_TYPES, "extend", prefix, false, false, result);
}

/// Valid HTTP methods (case-insensitive)
const VALID_HTTP_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

/// Validates an HTTP method value (case-insensitive)
pub fn validate_http_method(method: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(method, VALID_HTTP_METHODS, "with.method", prefix, false, true, result);
}

/// Validates container lifetime: when cleanup is "eventually", the 'after' field is required
/// Matches Go SDK's validate:"required_if=Cleanup eventually"
pub fn validate_container_lifetime(
    lifetime: &ContainerLifetimeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    // First validate the cleanup value itself
    validate_container_cleanup(&lifetime.cleanup, prefix, result);
    // Then check: if cleanup is "eventually", after must be set
    if lifetime.cleanup == "eventually" && lifetime.after.is_none() {
        result.add_error(
            &format!("{}.lifetime.after", prefix),
            ValidationRule::Required,
            "lifetime.after is required when cleanup is 'eventually'",
        );
    }
}
