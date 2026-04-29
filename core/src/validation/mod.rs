use crate::models::expression::is_strict_expr;
use regex::Regex;
use std::sync::LazyLock;

mod authentication;
mod document;
mod enum_validators;
mod one_of_validators;
mod task;
#[cfg(test)]
mod tests;

// Re-export all pub items from sub-modules
pub use authentication::{
    validate_basic_auth, validate_bearer_auth, validate_digest_auth, validate_oauth2_auth,
    validate_oidc_auth, validate_auth_policy,
};
pub use document::validate_workflow;
pub use enum_validators::{
    validate_asyncapi_protocol, validate_http_output, validate_pull_policy,
    validate_container_cleanup, validate_script_language, validate_oauth2_client_auth_method,
    validate_oauth2_request_encoding, validate_oauth2_grant_type, validate_extension_task_type,
    validate_http_method, validate_container_lifetime,
};
pub use one_of_validators::{
    validate_auth_policy_one_of, validate_schedule_one_of, validate_process_type_one_of,
    validate_backoff_one_of, validate_schema_one_of,
};
pub use task::{
    validate_task_map, validate_set_task, validate_workflow_process, validate_switch_task,
};

/// Represents a validation error
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// The field path that failed validation (e.g., "document.name", "do.task1.call")
    pub field: String,
    /// The validation rule that failed
    pub rule: ValidationRule,
    /// A human-readable error message
    pub message: String,
}

/// Enumerates the types of validation rules
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationRule {
    Required,
    Semver,
    Hostname,
    Uri,
    Iso8601Duration,
    MutualExclusion,
    InvalidValue,
    Custom(String),
}

/// Represents the result of a validation operation
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    /// The list of validation errors found
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Creates a new empty validation result
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Adds an error to the validation result
    pub fn add_error(&mut self, field: &str, rule: ValidationRule, message: &str) {
        self.errors.push(ValidationError {
            field: field.to_string(),
            rule,
            message: message.to_string(),
        });
    }

    /// Returns true if no validation errors were found
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Merges another validation result into this one, prefixing field paths
    pub fn merge_with_prefix(&mut self, prefix: &str, other: ValidationResult) {
        for error in other.errors {
            self.errors.push(ValidationError {
                field: format!("{}.{}", prefix, error.field),
                rule: error.rule,
                message: error.message,
            });
        }
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

static SEMVER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$").expect("static semver regex is valid")
});

static HOSTNAME_RFC1123_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(([a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)*[a-zA-Z]{2,63}|[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)$").expect("static hostname regex is valid")
});

// URI/URI-template patterns (matching Go SDK's LiteralUriPattern/LiteralUriTemplatePattern)
static URI_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z][A-Za-z0-9+\-.]*://[^{}\s]+$").expect("static URI regex is valid")
});

static URI_TEMPLATE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z][A-Za-z0-9+\-.]*://.*\{.*}.*$")
        .expect("static URI template regex is valid")
});

// RFC 6901 JSON Pointer pattern (matching Go SDK's JSONPointerPattern)
static JSON_POINTER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(/([^/~]|~[01])*)*$").expect("static JSON pointer regex is valid")
});

/// Validates a semantic version string
pub fn is_valid_semver(version: &str) -> bool {
    SEMVER_PATTERN.is_match(version)
}

/// Validates an RFC 1123 hostname
pub fn is_valid_hostname(hostname: &str) -> bool {
    HOSTNAME_RFC1123_PATTERN.is_match(hostname)
}

/// Validates a required string field as an RFC 1123 hostname.
/// Adds a Required error if empty, or a Hostname error if invalid format.
pub fn validate_required_hostname(value: &str, field: &str, result: &mut ValidationResult) {
    if value.is_empty() {
        result.add_error(field, ValidationRule::Required, &format!("{} is required", field));
    } else if !is_valid_hostname(value) {
        result.add_error(field, ValidationRule::Hostname, &format!("{} must be a valid RFC 1123 hostname", field));
    }
}

/// Validates a required string field as a semantic version.
/// Adds a Required error if empty, or a Semver error if invalid format.
pub fn validate_required_semver(value: &str, field: &str, result: &mut ValidationResult) {
    if value.is_empty() {
        result.add_error(field, ValidationRule::Required, &format!("{} is required", field));
    } else if !is_valid_semver(value) {
        result.add_error(field, ValidationRule::Semver, &format!("{} must be a valid semantic version", field));
    }
}

/// Checks if a value is a non-empty string.
/// Per the Go SDK's `string_or_runtime_expr` validator, any non-empty string is valid
/// (either a plain string or a runtime expression).
pub fn is_non_empty_string(value: &str) -> bool {
    !value.is_empty()
}

/// Checks if a string value is a valid URI or a valid runtime expression
/// Matches Go SDK's uri_template_or_runtime_expr validator
/// Uses Go SDK's LiteralUriPattern and LiteralUriTemplatePattern for validation
pub fn is_uri_or_runtime_expr(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    // Runtime expressions are valid
    if is_strict_expr(value) {
        return true;
    }
    // Match Go SDK's URI/URI-template patterns
    URI_PATTERN.is_match(value) || URI_TEMPLATE_PATTERN.is_match(value)
}

/// Checks if a string value is a valid JSON Pointer or a valid runtime expression
/// Matches Go SDK's json_pointer_or_runtime_expr validator
/// RFC 6901: "" references the whole document, "/foo/bar" references a path
/// Escape sequences: ~0 = ~, ~1 = /
pub fn is_json_pointer_or_runtime_expr(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    // Runtime expressions are valid
    if is_strict_expr(value) {
        return true;
    }
    // RFC 6901 JSON Pointer pattern (matches Go SDK's JSONPointerPattern)
    JSON_POINTER_PATTERN.is_match(value)
}

/// Checks if a string value is a valid literal URI (no placeholders)
/// Matches Go SDK's LiteralUriPattern: `^[A-Za-z][A-Za-z0-9+\-.]*://[^{}\s]+$`
pub fn is_valid_uri(value: &str) -> bool {
    URI_PATTERN.is_match(value)
}

/// Checks if a string value is a valid URI template (with placeholders)
/// Matches Go SDK's LiteralUriTemplatePattern: `^[A-Za-z][A-Za-z0-9+\-.]*://.*\{.*}.*$`
pub fn is_valid_uri_template(value: &str) -> bool {
    URI_TEMPLATE_PATTERN.is_match(value)
}

/// Checks if a string value is a valid JSON Pointer (RFC 6901)
/// Matches Go SDK's JSONPointerPattern: `^(/([^/~]|~[01])*)*$`
pub fn is_valid_json_pointer(value: &str) -> bool {
    JSON_POINTER_PATTERN.is_match(value)
}
