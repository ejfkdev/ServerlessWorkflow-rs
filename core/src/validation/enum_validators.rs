use super::{ValidationResult, ValidationRule};
use crate::models::authentication::{OAuth2ClientAuthenticationMethod, OAuth2RequestEncoding};
use crate::models::call::AsyncApiProtocol;
use crate::models::task::ContainerLifetimeDefinition;
use crate::models::task::{
    ContainerCleanupPolicy, ExtensionTarget, HttpMethod, HttpOutputFormat, OAuth2GrantType,
    PullPolicy, ScriptLanguage,
};

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
            &format!(
                "{} must be one of {}, got '{}'",
                field_name, valid_list, value
            ),
        );
    }
}

/// Validates an AsyncAPI protocol value
pub fn validate_asyncapi_protocol(protocol: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(
        protocol,
        AsyncApiProtocol::ALL_VALUES,
        "protocol",
        prefix,
        true,
        false,
        result,
    );
}

/// Validates an HTTP call output format value
pub fn validate_http_output(output: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(
        output,
        HttpOutputFormat::ALL_VALUES,
        "output",
        prefix,
        true,
        false,
        result,
    );
}

/// Validates a container's pull policy value
pub fn validate_pull_policy(pull_policy: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(
        pull_policy,
        PullPolicy::ALL_VALUES,
        "pullPolicy",
        prefix,
        false,
        false,
        result,
    );
}

/// Validates a container's cleanup policy value
pub fn validate_container_cleanup(cleanup: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(
        cleanup,
        ContainerCleanupPolicy::ALL_VALUES,
        "lifetime.cleanup",
        prefix,
        false,
        false,
        result,
    );
}

/// Validates a script's language value
pub fn validate_script_language(language: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(
        language,
        ScriptLanguage::ALL_VALUES,
        "language",
        prefix,
        false,
        false,
        result,
    );
}

/// Validates an OAuth2 client authentication method value
pub fn validate_oauth2_client_auth_method(
    method: &str,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_enum_value(
        method,
        OAuth2ClientAuthenticationMethod::ALL_VALUES,
        "client.authentication",
        prefix,
        true,
        false,
        result,
    );
}

/// Validates an OAuth2 token request encoding value
pub fn validate_oauth2_request_encoding(
    encoding: &str,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_enum_value(
        encoding,
        OAuth2RequestEncoding::ALL_VALUES,
        "request.encoding",
        prefix,
        true,
        false,
        result,
    );
}

/// Validates an OAuth2 grant type value
pub fn validate_oauth2_grant_type(grant: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(
        grant,
        OAuth2GrantType::ALL_VALUES,
        "grant",
        prefix,
        true,
        false,
        result,
    );
}

/// Validates an extension's task type value
pub fn validate_extension_task_type(extend: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(
        extend,
        ExtensionTarget::ALL_VALUES,
        "extend",
        prefix,
        false,
        false,
        result,
    );
}

/// Validates an HTTP method value (case-insensitive)
pub fn validate_http_method(method: &str, prefix: &str, result: &mut ValidationResult) {
    validate_enum_value(
        method,
        HttpMethod::ALL_VALUES,
        "with.method",
        prefix,
        false,
        true,
        result,
    );
}

/// Validates container lifetime: when cleanup is "eventually", the 'after' field is required
pub fn validate_container_lifetime(
    lifetime: &ContainerLifetimeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    validate_container_cleanup(&lifetime.cleanup, prefix, result);
    if lifetime.cleanup == "eventually" && lifetime.after.is_none() {
        result.add_error(
            &format!("{}.lifetime.after", prefix),
            ValidationRule::Required,
            "lifetime.after is required when cleanup is 'eventually'",
        );
    }
}
