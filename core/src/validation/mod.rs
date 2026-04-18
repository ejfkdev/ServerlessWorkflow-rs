use crate::models::authentication::*;
use crate::models::call::*;
use crate::models::duration::*;
use crate::models::expression::is_strict_expr;
use crate::models::retry::*;
use crate::models::schema::*;
use crate::models::task::*;
use crate::models::workflow::*;
use regex::Regex;
use std::sync::LazyLock;

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
    validate_task_map(&workflow.do_, "do", &mut result);

    result
}

/// Validates workflow document metadata
fn validate_document(doc: &WorkflowDefinitionMetadata, result: &mut ValidationResult) {
    if doc.name.is_empty() {
        result.add_error(
            "document.name",
            ValidationRule::Required,
            "workflow name is required",
        );
    } else if !is_valid_hostname(&doc.name) {
        // Go SDK: validate:"required,hostname_rfc1123"
        result.add_error(
            "document.name",
            ValidationRule::Hostname,
            "workflow name must be a valid RFC 1123 hostname",
        );
    }
    if doc.version.is_empty() {
        result.add_error(
            "document.version",
            ValidationRule::Required,
            "workflow version is required",
        );
    } else if !is_valid_semver(&doc.version) {
        // Go SDK: validate:"required,semver_pattern"
        result.add_error(
            "document.version",
            ValidationRule::Semver,
            "workflow version must be a valid semantic version",
        );
    }
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
fn validate_input(
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
fn validate_timeout(
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
fn validate_duration(
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

/// Validates a map of task definitions
fn validate_task_map(
    tasks: &crate::models::map::Map<String, TaskDefinition>,
    prefix: &str,
    result: &mut ValidationResult,
) {
    for (name, task) in &tasks.entries {
        validate_task(task, &format!("{}.{}", prefix, name), result);
    }
}

/// Validates a single task definition
fn validate_task(task: &TaskDefinition, prefix: &str, result: &mut ValidationResult) {
    match task {
        TaskDefinition::Call(call) => validate_call_task(call, prefix, result),
        TaskDefinition::Do(do_task) => {
            validate_common_fields(&do_task.common, prefix, result);
            validate_task_map(&do_task.do_, &format!("{}.do", prefix), result);
        }
        TaskDefinition::Emit(emit) => {
            validate_common_fields(&emit.common, prefix, result);
        }
        TaskDefinition::For(for_task) => {
            validate_common_fields(&for_task.common, prefix, result);
            if for_task.for_.each.is_empty() {
                result.add_error(
                    &format!("{}.for.each", prefix),
                    ValidationRule::Required,
                    "'each' is required in for loop",
                );
            }
            if for_task.for_.in_.is_empty() {
                result.add_error(
                    &format!("{}.for.in", prefix),
                    ValidationRule::Required,
                    "'in' is required in for loop",
                );
            }
            validate_task_map(&for_task.do_, &format!("{}.do", prefix), result);
        }
        TaskDefinition::Fork(fork) => {
            validate_common_fields(&fork.common, prefix, result);
            validate_task_map(
                &fork.fork.branches,
                &format!("{}.fork.branches", prefix),
                result,
            );
        }
        TaskDefinition::Listen(listen) => {
            validate_common_fields(&listen.common, prefix, result);
        }
        TaskDefinition::Raise(raise) => {
            validate_common_fields(&raise.common, prefix, result);
        }
        TaskDefinition::Run(run) => {
            validate_common_fields(&run.common, prefix, result);
            validate_run_task(&run.run, prefix, result);
        }
        TaskDefinition::Set(set) => {
            validate_common_fields(&set.common, prefix, result);
            validate_set_task(&set.set, prefix, result);
        }
        TaskDefinition::Switch(switch) => {
            validate_common_fields(&switch.common, prefix, result);
            validate_switch_task(switch, prefix, result);
        }
        TaskDefinition::Try(try_task) => {
            validate_common_fields(&try_task.common, prefix, result);
            validate_task_map(&try_task.try_, &format!("{}.try", prefix), result);
            // Validate retry backoff oneOf if retry is set
            if let Some(OneOfRetryPolicyDefinitionOrReference::Retry(ref policy)) =
                try_task.catch.retry
            {
                if let Some(ref backoff) = policy.backoff {
                    validate_backoff_one_of(
                        backoff,
                        &format!("{}.catch.retry.backoff", prefix),
                        result,
                    );
                }
            }
        }
        TaskDefinition::Wait(wait) => {
            validate_common_fields(&wait.common, prefix, result);
        }
        TaskDefinition::Custom(custom) => {
            validate_common_fields(&custom.common, prefix, result);
        }
    }
}

/// Validates a run task configuration
fn validate_run_task(
    run: &crate::models::task::ProcessTypeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    // Enforce oneOf: exactly one of container/script/shell/workflow must be set
    validate_process_type_one_of(run, prefix, result);

    if let Some(ref container) = run.container {
        if let Some(ref pull_policy) = container.pull_policy {
            validate_pull_policy(pull_policy, prefix, result);
        }
        if let Some(ref lifetime) = container.lifetime {
            validate_container_lifetime(lifetime, prefix, result);
        }
    }
    if let Some(ref script) = run.script {
        validate_script_language(&script.language, prefix, result);
    }
    if let Some(ref workflow) = run.workflow {
        validate_workflow_process(workflow, prefix, result);
    }
}

/// Validates a call task definition
fn validate_call_task(call: &CallTaskDefinition, prefix: &str, result: &mut ValidationResult) {
    match call {
        CallTaskDefinition::HTTP(http) => {
            validate_common_fields(&http.common, prefix, result);
            if http.with.method.is_empty() {
                result.add_error(
                    &format!("{}.with.method", prefix),
                    ValidationRule::Required,
                    "HTTP method is required",
                );
            } else {
                validate_http_method(&http.with.method, prefix, result);
            }
            if let Some(ref output) = http.with.output {
                validate_http_output(output, &format!("{}.with", prefix), result);
            }
        }
        CallTaskDefinition::GRPC(grpc) => {
            validate_common_fields(&grpc.common, prefix, result);
            if grpc.with.method.is_empty() {
                result.add_error(
                    &format!("{}.with.method", prefix),
                    ValidationRule::Required,
                    "GRPC method is required",
                );
            }
            if grpc.with.service.name.is_empty() {
                result.add_error(
                    &format!("{}.with.service.name", prefix),
                    ValidationRule::Required,
                    "GRPC service name is required",
                );
            }
            // Go SDK: validate:"required,hostname_rfc1123"
            if grpc.with.service.host.is_empty() {
                result.add_error(
                    &format!("{}.with.service.host", prefix),
                    ValidationRule::Required,
                    "GRPC service host is required",
                );
            } else if !is_valid_hostname(&grpc.with.service.host) {
                result.add_error(
                    &format!("{}.with.service.host", prefix),
                    ValidationRule::Hostname,
                    "GRPC service host must be a valid RFC 1123 hostname",
                );
            }
        }
        CallTaskDefinition::OpenAPI(openapi) => {
            validate_common_fields(&openapi.common, prefix, result);
            if openapi.with.operation_id.is_empty() {
                result.add_error(
                    &format!("{}.with.operationId", prefix),
                    ValidationRule::Required,
                    "OpenAPI operationId is required",
                );
            }
            if let Some(ref output) = openapi.with.output {
                validate_http_output(output, &format!("{}.with", prefix), result);
            }
        }
        CallTaskDefinition::AsyncAPI(asyncapi) => {
            validate_common_fields(&asyncapi.common, prefix, result);
            if let Some(ref protocol) = asyncapi.with.protocol {
                validate_asyncapi_protocol(protocol, &format!("{}.with", prefix), result);
            }
        }
        CallTaskDefinition::A2A(a2a) => {
            validate_common_fields(&a2a.common, prefix, result);
            if a2a.with.method.is_empty() {
                result.add_error(
                    &format!("{}.with.method", prefix),
                    ValidationRule::Required,
                    "A2A method is required",
                );
            }
        }
        CallTaskDefinition::Function(func) => {
            validate_common_fields(&func.common, prefix, result);
            if func.call.is_empty() {
                result.add_error(
                    &format!("{}.call", prefix),
                    ValidationRule::Required,
                    "function name is required",
                );
            }
        }
    }
}

/// Validates common task definition fields
fn validate_common_fields(
    fields: &TaskDefinitionFields,
    prefix: &str,
    result: &mut ValidationResult,
) {
    if let Some(ref timeout) = fields.timeout {
        validate_timeout(timeout, &format!("{}.timeout", prefix), result);
    }
}

/// Validates a Basic authentication scheme for mutual exclusivity
/// Basic auth must have either `use` (secret reference) OR username/password, not both
pub fn validate_basic_auth(
    basic: &BasicAuthenticationSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = basic.use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_credentials = basic.username.as_ref().is_some_and(|s| !s.is_empty())
        || basic.password.as_ref().is_some_and(|s| !s.is_empty());
    if has_use && has_credentials {
        result.add_error(
            &format!("{}.basic", prefix),
            ValidationRule::MutualExclusion,
            "basic auth: 'use' and username/password are mutually exclusive",
        );
    }
}

/// Validates a Bearer authentication scheme for mutual exclusivity
/// Bearer auth must have either `use` (secret reference) OR token, not both
pub fn validate_bearer_auth(
    bearer: &BearerAuthenticationSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = bearer.use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_token = bearer.token.as_ref().is_some_and(|s| !s.is_empty());
    if has_use && has_token {
        result.add_error(
            &format!("{}.bearer", prefix),
            ValidationRule::MutualExclusion,
            "bearer auth: 'use' and token are mutually exclusive",
        );
    }
}

/// Validates a Digest authentication scheme for mutual exclusivity
/// Digest auth must have either `use` (secret reference) OR username/password, not both
pub fn validate_digest_auth(
    digest: &DigestAuthenticationSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = digest.use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_credentials = digest.username.as_ref().is_some_and(|s| !s.is_empty())
        || digest.password.as_ref().is_some_and(|s| !s.is_empty());
    if has_use && has_credentials {
        result.add_error(
            &format!("{}.digest", prefix),
            ValidationRule::MutualExclusion,
            "digest auth: 'use' and username/password are mutually exclusive",
        );
    }
}

/// Validates an OAuth2 authentication scheme for mutual exclusivity
/// OAuth2 must have either `use` (secret reference) OR properties (authority/grant/client/etc.), not both
/// At least one of `use` or properties must be set
pub fn validate_oauth2_auth(
    oauth2: &OAuth2AuthenticationSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = oauth2.use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_properties = oauth2.authority.as_ref().is_some_and(|s| !s.is_empty())
        || oauth2.grant.as_ref().is_some_and(|s| !s.is_empty())
        || oauth2.client.is_some()
        || oauth2.endpoints.is_some()
        || oauth2.scopes.is_some()
        || oauth2.audiences.is_some()
        || oauth2.issuers.is_some();

    if has_use && has_properties {
        result.add_error(
            &format!("{}.oauth2", prefix),
            ValidationRule::MutualExclusion,
            "oauth2 auth: 'use' and inline properties are mutually exclusive",
        );
    }
    if !has_use && !has_properties {
        result.add_error(
            &format!("{}.oauth2", prefix),
            ValidationRule::Required,
            "oauth2 auth: either 'use' or inline properties must be set",
        );
    }
    // Validate grant type if set (matches Go SDK's oneof validation)
    if let Some(ref grant) = oauth2.grant {
        validate_oauth2_grant_type(grant, &format!("{}.oauth2", prefix), result);
    }
    // Validate client authentication method if set
    if let Some(ref client) = oauth2.client {
        if let Some(ref auth_method) = client.authentication {
            validate_oauth2_client_auth_method(auth_method, &format!("{}.oauth2", prefix), result);
        }
    }
    // Validate request encoding if set
    if let Some(ref request) = oauth2.request {
        validate_oauth2_request_encoding(&request.encoding, &format!("{}.oauth2", prefix), result);
    }
}

/// Validates an OIDC authentication scheme for mutual exclusivity
/// OIDC must have either `use` (secret reference) OR properties (authority/grant/client/etc.), not both
/// At least one of `use` or properties must be set
pub fn validate_oidc_auth(
    oidc: &OpenIDConnectSchemeDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let has_use = oidc.use_.as_ref().is_some_and(|s| !s.is_empty());
    let has_properties = oidc.authority.as_ref().is_some_and(|s| !s.is_empty())
        || oidc.grant.as_ref().is_some_and(|s| !s.is_empty())
        || oidc.client.is_some()
        || oidc.scopes.is_some()
        || oidc.audiences.is_some()
        || oidc.issuers.is_some();

    if has_use && has_properties {
        result.add_error(
            &format!("{}.oidc", prefix),
            ValidationRule::MutualExclusion,
            "oidc auth: 'use' and inline properties are mutually exclusive",
        );
    }
    if !has_use && !has_properties {
        result.add_error(
            &format!("{}.oidc", prefix),
            ValidationRule::Required,
            "oidc auth: either 'use' or inline properties must be set",
        );
    }
    // Validate grant type if set (matches Go SDK's oneof validation)
    if let Some(ref grant) = oidc.grant {
        validate_oauth2_grant_type(grant, &format!("{}.oidc", prefix), result);
    }
    // Validate client authentication method if set
    if let Some(ref client) = oidc.client {
        if let Some(ref auth_method) = client.authentication {
            validate_oauth2_client_auth_method(auth_method, &format!("{}.oidc", prefix), result);
        }
    }
    // Validate request encoding if set
    if let Some(ref request) = oidc.request {
        validate_oauth2_request_encoding(&request.encoding, &format!("{}.oidc", prefix), result);
    }
}

/// Validates an authentication policy definition
pub fn validate_auth_policy(
    policy: &AuthenticationPolicyDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    // Enforce oneOf constraint: only one authentication scheme should be set
    validate_auth_policy_one_of(policy, prefix, result);

    if let Some(ref basic) = policy.basic {
        validate_basic_auth(basic, prefix, result);
    }
    if let Some(ref bearer) = policy.bearer {
        validate_bearer_auth(bearer, prefix, result);
    }
    if let Some(ref digest) = policy.digest {
        validate_digest_auth(digest, prefix, result);
    }
    if let Some(ref oauth2) = policy.oauth2 {
        validate_oauth2_auth(oauth2, prefix, result);
    }
    if let Some(ref oidc) = policy.oidc {
        validate_oidc_auth(oidc, prefix, result);
    }
}

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
fn validate_enum_value(
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
/// Validates a set task's value
/// Matches Go SDK's validate:"required,min=1,dive"
/// Set must have at least 1 key-value pair
pub fn validate_set_task(
    set: &crate::models::task::SetValue,
    prefix: &str,
    result: &mut ValidationResult,
) {
    match set {
        crate::models::task::SetValue::Map(map) => {
            if map.is_empty() {
                result.add_error(
                    &format!("{}.set", prefix),
                    ValidationRule::Required,
                    "set task must have at least one key-value pair",
                );
            }
        }
        crate::models::task::SetValue::Expression(expr) => {
            if expr.is_empty() {
                result.add_error(
                    &format!("{}.set", prefix),
                    ValidationRule::Required,
                    "set task expression must not be empty",
                );
            }
        }
    }
}

/// Validates a WorkflowProcessDefinition (sub-workflow reference)
/// Matches Go SDK's validation tags:
/// - namespace: validate:"required,hostname_rfc1123"
/// - name: validate:"required,hostname_rfc1123"
/// - version: validate:"required,semver_pattern"
pub fn validate_workflow_process(
    workflow: &crate::models::task::WorkflowProcessDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    if workflow.namespace.is_empty() {
        result.add_error(
            &format!("{}.run.workflow.namespace", prefix),
            ValidationRule::Required,
            "workflow namespace is required",
        );
    } else if !is_valid_hostname(&workflow.namespace) {
        result.add_error(
            &format!("{}.run.workflow.namespace", prefix),
            ValidationRule::Hostname,
            "workflow namespace must be a valid RFC 1123 hostname",
        );
    }
    if workflow.name.is_empty() {
        result.add_error(
            &format!("{}.run.workflow.name", prefix),
            ValidationRule::Required,
            "workflow name is required",
        );
    } else if !is_valid_hostname(&workflow.name) {
        result.add_error(
            &format!("{}.run.workflow.name", prefix),
            ValidationRule::Hostname,
            "workflow name must be a valid RFC 1123 hostname",
        );
    }
    if workflow.version.is_empty() {
        result.add_error(
            &format!("{}.run.workflow.version", prefix),
            ValidationRule::Required,
            "workflow version is required",
        );
    } else if !is_valid_semver(&workflow.version) {
        result.add_error(
            &format!("{}.run.workflow.version", prefix),
            ValidationRule::Semver,
            "workflow version must be a valid semantic version",
        );
    }
}

/// Validates a switch task definition
/// Matches Go SDK's SwitchTask validation:
/// - switch must have at least 1 case item (`validate:"required,min=1,dive,switch_item"`)
/// - each switch item must have exactly 1 key (`validate_switch_item` where `len(switchItem) == 1`)
/// - each case's `then` field is required (`validate:"required"`)
pub fn validate_switch_task(
    switch: &SwitchTaskDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    // Validate at least 1 switch case
    if switch.switch.is_empty() {
        result.add_error(
            &format!("{}.switch", prefix),
            ValidationRule::Required,
            "switch task must have at least one case",
        );
    }

    // Validate each switch item has exactly 1 key and then is required
    for (i, (name, case_def)) in switch.switch.entries.iter().enumerate() {
        let case_prefix = format!("{}.switch[{}]", prefix, i);
        // Validate each case's then field
        if case_def.then.is_none() {
            result.add_error(
                &format!("{}.{}.then", case_prefix, name),
                ValidationRule::Required,
                "switch case 'then' is required",
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::map::Map;
    use crate::models::resource::ExternalResourceDefinition;
    use crate::models::task::{SwitchCaseDefinition, SwitchTaskDefinition};
    use std::collections::HashMap;

    #[test]
    fn test_valid_semver() {
        // Valid cases
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("0.1.0"));
        assert!(is_valid_semver("1.2.3"));
        assert!(is_valid_semver("1.0.0-alpha"));
        assert!(is_valid_semver("1.0.0-alpha.1"));
        assert!(is_valid_semver("1.0.0+build.123"));
        assert!(is_valid_semver("1.2.3-beta.1+build.123"));
        // Invalid cases
        assert!(!is_valid_semver(""));
        assert!(!is_valid_semver("1"));
        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("v1.0.0"));
        assert!(!is_valid_semver("v1.2.3"));
    }

    #[test]
    fn test_valid_hostname() {
        // Valid cases
        assert!(is_valid_hostname("example"));
        assert!(is_valid_hostname("example.com"));
        assert!(is_valid_hostname("my-host"));
        assert!(is_valid_hostname("my-hostname"));
        assert!(is_valid_hostname("subdomain.example.com"));
        assert!(is_valid_hostname("default"));
        // Invalid cases
        assert!(!is_valid_hostname(""));
        assert!(!is_valid_hostname("-invalid"));
        assert!(!is_valid_hostname("invalid-"));
        assert!(!is_valid_hostname("127.0.0.1"));
        assert!(!is_valid_hostname("example.com."));
        assert!(!is_valid_hostname("example..com"));
        assert!(!is_valid_hostname("example.com-"));
    }

    #[test]
    fn test_valid_iso8601_duration() {
        // Valid cases
        assert!(crate::models::duration::is_iso8601_duration_valid("PT5S"));
        assert!(crate::models::duration::is_iso8601_duration_valid("PT10M"));
        assert!(crate::models::duration::is_iso8601_duration_valid("PT1H"));
        assert!(crate::models::duration::is_iso8601_duration_valid("P1D"));
        assert!(crate::models::duration::is_iso8601_duration_valid(
            "P1DT12H"
        ));
        assert!(crate::models::duration::is_iso8601_duration_valid(
            "P1DT12H30M"
        ));
        assert!(crate::models::duration::is_iso8601_duration_valid(
            "PT1H30M"
        ));
        assert!(crate::models::duration::is_iso8601_duration_valid(
            "PT250MS"
        ));
        assert!(crate::models::duration::is_iso8601_duration_valid(
            "P3DT4H5M6S250MS"
        ));
        // Invalid cases
        assert!(!crate::models::duration::is_iso8601_duration_valid(""));
        assert!(!crate::models::duration::is_iso8601_duration_valid("5S"));
        assert!(!crate::models::duration::is_iso8601_duration_valid("P1Y"));
        assert!(!crate::models::duration::is_iso8601_duration_valid(
            "P1Y2M3D"
        ));
        assert!(!crate::models::duration::is_iso8601_duration_valid("P1W"));
        assert!(!crate::models::duration::is_iso8601_duration_valid(
            "P1Y2M3D4H"
        ));
        assert!(!crate::models::duration::is_iso8601_duration_valid(
            "P1Y2M3D4H5M6S"
        ));
        assert!(!crate::models::duration::is_iso8601_duration_valid("P"));
        assert!(!crate::models::duration::is_iso8601_duration_valid("P1DT"));
        assert!(!crate::models::duration::is_iso8601_duration_valid("1Y"));
        assert!(!crate::models::duration::is_iso8601_duration_valid(
            "P1DT2H3M4S5MS7"
        )); // trailing garbage
    }

    #[test]
    fn test_validate_workflow_valid() {
        let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default",
            "test-workflow",
            "1.0.0",
            None,
            None,
            None,
        ));
        workflow.do_.add(
            "task1".to_string(),
            TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
                crate::models::call::CallFunctionDefinition {
                    call: "myFunction".to_string(),
                    with: None,
                    common: TaskDefinitionFields::new(),
                },
            ))),
        );

        let result = validate_workflow(&workflow);
        assert!(
            result.is_valid(),
            "Expected valid workflow, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_workflow_missing_name() {
        let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default", "", "1.0.0", None, None, None,
        ));
        let result = validate_workflow(&workflow);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.field == "document.name"));
    }

    #[test]
    fn test_validate_workflow_invalid_semver() {
        let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata {
            dsl: "not-semver".to_string(),
            namespace: "default".to_string(),
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            title: None,
            summary: None,
            tags: None,
            metadata: None,
        });
        let result = validate_workflow(&workflow);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.field == "document.dsl"));
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::new();
        result1.add_error("field1", ValidationRule::Required, "field1 is required");

        let mut result2 = ValidationResult::new();
        result2.add_error("field2", ValidationRule::Semver, "field2 is not semver");

        result1.merge_with_prefix("parent", result2);
        assert_eq!(result1.errors.len(), 2);
        assert_eq!(result1.errors[1].field, "parent.field2");
    }

    // Auth policy mutual exclusion tests (matching Go SDK's validator.go)

    #[test]
    fn test_validate_basic_auth_use_only() {
        let basic = BasicAuthenticationSchemeDefinition {
            use_: Some("mySecret".to_string()),
            username: None,
            password: None,
        };
        let mut result = ValidationResult::new();
        validate_basic_auth(&basic, "auth", &mut result);
        assert!(result.is_valid(), "use-only basic auth should be valid");
    }

    #[test]
    fn test_validate_basic_auth_credentials_only() {
        let basic = BasicAuthenticationSchemeDefinition {
            use_: None,
            username: Some("john".to_string()),
            password: Some("secret".to_string()),
        };
        let mut result = ValidationResult::new();
        validate_basic_auth(&basic, "auth", &mut result);
        assert!(
            result.is_valid(),
            "credentials-only basic auth should be valid"
        );
    }

    #[test]
    fn test_validate_basic_auth_mutual_exclusion() {
        let basic = BasicAuthenticationSchemeDefinition {
            use_: Some("mySecret".to_string()),
            username: Some("john".to_string()),
            password: Some("secret".to_string()),
        };
        let mut result = ValidationResult::new();
        validate_basic_auth(&basic, "auth", &mut result);
        assert!(!result.is_valid(), "use + credentials should be invalid");
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    #[test]
    fn test_validate_bearer_auth_use_only() {
        let bearer = BearerAuthenticationSchemeDefinition {
            use_: Some("bearerSecret".to_string()),
            token: None,
        };
        let mut result = ValidationResult::new();
        validate_bearer_auth(&bearer, "auth", &mut result);
        assert!(result.is_valid(), "use-only bearer auth should be valid");
    }

    #[test]
    fn test_validate_bearer_auth_token_only() {
        let bearer = BearerAuthenticationSchemeDefinition {
            use_: None,
            token: Some("mytoken123".to_string()),
        };
        let mut result = ValidationResult::new();
        validate_bearer_auth(&bearer, "auth", &mut result);
        assert!(result.is_valid(), "token-only bearer auth should be valid");
    }

    #[test]
    fn test_validate_bearer_auth_mutual_exclusion() {
        let bearer = BearerAuthenticationSchemeDefinition {
            use_: Some("bearerSecret".to_string()),
            token: Some("mytoken123".to_string()),
        };
        let mut result = ValidationResult::new();
        validate_bearer_auth(&bearer, "auth", &mut result);
        assert!(!result.is_valid(), "use + token should be invalid");
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    #[test]
    fn test_validate_digest_auth_use_only() {
        let digest = DigestAuthenticationSchemeDefinition {
            use_: Some("digestSecret".to_string()),
            username: None,
            password: None,
        };
        let mut result = ValidationResult::new();
        validate_digest_auth(&digest, "auth", &mut result);
        assert!(result.is_valid(), "use-only digest auth should be valid");
    }

    #[test]
    fn test_validate_digest_auth_credentials_only() {
        let digest = DigestAuthenticationSchemeDefinition {
            use_: None,
            username: Some("digestUser".to_string()),
            password: Some("digestPass".to_string()),
        };
        let mut result = ValidationResult::new();
        validate_digest_auth(&digest, "auth", &mut result);
        assert!(
            result.is_valid(),
            "credentials-only digest auth should be valid"
        );
    }

    #[test]
    fn test_validate_digest_auth_mutual_exclusion() {
        let digest = DigestAuthenticationSchemeDefinition {
            use_: Some("digestSecret".to_string()),
            username: Some("digestUser".to_string()),
            password: Some("digestPass".to_string()),
        };
        let mut result = ValidationResult::new();
        validate_digest_auth(&digest, "auth", &mut result);
        assert!(!result.is_valid(), "use + credentials should be invalid");
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    #[test]
    fn test_validate_oauth2_auth_use_only() {
        let oauth2 = OAuth2AuthenticationSchemeDefinition {
            use_: Some("oauth2Secret".to_string()),
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_oauth2_auth(&oauth2, "auth", &mut result);
        assert!(result.is_valid(), "use-only oauth2 should be valid");
    }

    #[test]
    fn test_validate_oauth2_auth_properties_only() {
        let oauth2 = OAuth2AuthenticationSchemeDefinition {
            use_: None,
            authority: Some("https://auth.example.com".to_string()),
            grant: Some("client_credentials".to_string()),
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_oauth2_auth(&oauth2, "auth", &mut result);
        assert!(result.is_valid(), "properties-only oauth2 should be valid");
    }

    #[test]
    fn test_validate_oauth2_auth_mutual_exclusion() {
        let oauth2 = OAuth2AuthenticationSchemeDefinition {
            use_: Some("oauth2Secret".to_string()),
            authority: Some("https://auth.example.com".to_string()),
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_oauth2_auth(&oauth2, "auth", &mut result);
        assert!(!result.is_valid(), "use + properties should be invalid");
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    #[test]
    fn test_validate_oauth2_auth_neither_set() {
        let oauth2 = OAuth2AuthenticationSchemeDefinition {
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_oauth2_auth(&oauth2, "auth", &mut result);
        assert!(!result.is_valid(), "empty oauth2 should be invalid");
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::Required));
    }

    #[test]
    fn test_validate_auth_policy_valid() {
        let policy = AuthenticationPolicyDefinition {
            use_: None,
            basic: Some(BasicAuthenticationSchemeDefinition {
                use_: None,
                username: Some("admin".to_string()),
                password: Some("secret".to_string()),
            }),
            bearer: None,
            certificate: None,
            digest: None,
            oauth2: None,
            oidc: None,
        };
        let mut result = ValidationResult::new();
        validate_auth_policy(&policy, "auth", &mut result);
        assert!(result.is_valid(), "valid auth policy should pass");
    }

    #[test]
    fn test_validate_auth_policy_multiple_violations() {
        let policy = AuthenticationPolicyDefinition {
            use_: None,
            basic: Some(BasicAuthenticationSchemeDefinition {
                use_: Some("secret".to_string()),
                username: Some("admin".to_string()),
                password: None,
            }),
            bearer: Some(BearerAuthenticationSchemeDefinition {
                use_: Some("bearerSecret".to_string()),
                token: Some("token".to_string()),
            }),
            certificate: None,
            digest: None,
            oauth2: None,
            oidc: None,
        };
        let mut result = ValidationResult::new();
        validate_auth_policy(&policy, "auth", &mut result);
        assert!(!result.is_valid());
        // 3 errors: 1 oneOf (multiple schemes) + 2 mutual exclusion (basic use+creds, bearer use+token)
        assert_eq!(
            result.errors.len(),
            3,
            "Should have 3 errors (1 oneOf + 2 mutual exclusion), got: {:?}",
            result.errors
        );
    }

    // Pull policy validation tests (matching Go SDK's oneof=ifNotPresent always never)

    #[test]
    fn test_validate_pull_policy_valid() {
        for policy in &["ifNotPresent", "always", "never"] {
            let mut result = ValidationResult::new();
            validate_pull_policy(policy, "container", &mut result);
            assert!(result.is_valid(), "'{}' should be valid pullPolicy", policy);
        }
    }

    #[test]
    fn test_validate_pull_policy_invalid() {
        let mut result = ValidationResult::new();
        validate_pull_policy("invalid", "container", &mut result);
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("invalid"));
    }

    // Container cleanup policy validation tests (matching Go SDK's oneof=always never eventually)

    #[test]
    fn test_validate_container_cleanup_valid() {
        for cleanup in &["always", "never", "eventually"] {
            let mut result = ValidationResult::new();
            validate_container_cleanup(cleanup, "container", &mut result);
            assert!(result.is_valid(), "'{}' should be valid cleanup", cleanup);
        }
    }

    #[test]
    fn test_validate_container_cleanup_invalid() {
        let mut result = ValidationResult::new();
        validate_container_cleanup("sometimes", "container", &mut result);
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("sometimes"));
    }

    // Script language validation tests (matching Go SDK's oneof=javascript js python)

    #[test]
    fn test_validate_script_language_valid() {
        for lang in &["javascript", "js", "python"] {
            let mut result = ValidationResult::new();
            validate_script_language(lang, "script", &mut result);
            assert!(result.is_valid(), "'{}' should be valid language", lang);
        }
    }

    #[test]
    fn test_validate_script_language_invalid() {
        let mut result = ValidationResult::new();
        validate_script_language("ruby", "script", &mut result);
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("ruby"));
    }

    // Run task validation integration tests

    #[test]
    fn test_validate_run_task_container_invalid_pull_policy() {
        let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default", "test-run", "1.0.0", None, None, None,
        ));
        workflow.do_.add(
            "runContainer".to_string(),
            TaskDefinition::Run(Box::new(RunTaskDefinition {
                run: ProcessTypeDefinition {
                    container: Some(ContainerProcessDefinition {
                        image: "nginx:latest".to_string(),
                        pull_policy: Some("invalid-policy".to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                common: TaskDefinitionFields::new(),
            })),
        );
        let result = validate_workflow(&workflow);
        assert!(
            !result.is_valid(),
            "invalid pullPolicy should fail validation"
        );
        assert!(result.errors.iter().any(|e| e.field.contains("pullPolicy")));
    }

    #[test]
    fn test_validate_run_task_container_invalid_cleanup() {
        let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default", "test-run", "1.0.0", None, None, None,
        ));
        workflow.do_.add(
            "runContainer".to_string(),
            TaskDefinition::Run(Box::new(RunTaskDefinition {
                run: ProcessTypeDefinition {
                    container: Some(ContainerProcessDefinition {
                        image: "nginx:latest".to_string(),
                        lifetime: Some(ContainerLifetimeDefinition {
                            cleanup: "sometimes".to_string(),
                            after: None,
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                common: TaskDefinitionFields::new(),
            })),
        );
        let result = validate_workflow(&workflow);
        assert!(!result.is_valid(), "invalid cleanup should fail validation");
        assert!(result.errors.iter().any(|e| e.field.contains("cleanup")));
    }

    #[test]
    fn test_validate_run_task_script_invalid_language() {
        let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default", "test-run", "1.0.0", None, None, None,
        ));
        workflow.do_.add(
            "runScript".to_string(),
            TaskDefinition::Run(Box::new(RunTaskDefinition {
                run: ProcessTypeDefinition {
                    script: Some(ScriptProcessDefinition {
                        language: "ruby".to_string(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                common: TaskDefinitionFields::new(),
            })),
        );
        let result = validate_workflow(&workflow);
        assert!(
            !result.is_valid(),
            "invalid language should fail validation"
        );
        assert!(result.errors.iter().any(|e| e.field.contains("language")));
    }

    #[test]
    fn test_validate_run_task_valid_container() {
        let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default", "test-run", "1.0.0", None, None, None,
        ));
        workflow.do_.add(
            "runContainer".to_string(),
            TaskDefinition::Run(Box::new(RunTaskDefinition {
                run: ProcessTypeDefinition {
                    container: Some(ContainerProcessDefinition {
                        image: "nginx:latest".to_string(),
                        pull_policy: Some("always".to_string()),
                        lifetime: Some(ContainerLifetimeDefinition {
                            cleanup: "eventually".to_string(),
                            after: Some(OneOfDurationOrIso8601Expression::Iso8601Expression(
                                "PT5M".to_string(),
                            )),
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                common: TaskDefinitionFields::new(),
            })),
        );
        let result = validate_workflow(&workflow);
        assert!(
            result.is_valid(),
            "valid container should pass, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_oauth2_client_auth_method_valid() {
        let mut result = ValidationResult::new();
        validate_oauth2_client_auth_method("client_secret_basic", "test", &mut result);
        assert!(result.is_valid());

        let mut result = ValidationResult::new();
        validate_oauth2_client_auth_method("client_secret_post", "test", &mut result);
        assert!(result.is_valid());

        let mut result = ValidationResult::new();
        validate_oauth2_client_auth_method("client_secret_jwt", "test", &mut result);
        assert!(result.is_valid());

        let mut result = ValidationResult::new();
        validate_oauth2_client_auth_method("private_key_jwt", "test", &mut result);
        assert!(result.is_valid());

        let mut result = ValidationResult::new();
        validate_oauth2_client_auth_method("none", "test", &mut result);
        assert!(result.is_valid());

        // Empty string is allowed (optional field)
        let mut result = ValidationResult::new();
        validate_oauth2_client_auth_method("", "test", &mut result);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_oauth2_client_auth_method_invalid() {
        let mut result = ValidationResult::new();
        validate_oauth2_client_auth_method("invalid_method", "test", &mut result);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.field.contains("client.authentication")));
    }

    #[test]
    fn test_validate_oauth2_request_encoding_valid() {
        let mut result = ValidationResult::new();
        validate_oauth2_request_encoding("application/x-www-form-urlencoded", "test", &mut result);
        assert!(result.is_valid());

        let mut result = ValidationResult::new();
        validate_oauth2_request_encoding("application/json", "test", &mut result);
        assert!(result.is_valid());

        // Empty string is allowed (optional field)
        let mut result = ValidationResult::new();
        validate_oauth2_request_encoding("", "test", &mut result);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_oauth2_request_encoding_invalid() {
        let mut result = ValidationResult::new();
        validate_oauth2_request_encoding("text/plain", "test", &mut result);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.field.contains("request.encoding")));
    }

    // OIDC authentication mutual exclusion tests (symmetric to OAuth2)

    #[test]
    fn test_validate_oidc_auth_use_only() {
        let oidc = OpenIDConnectSchemeDefinition {
            use_: Some("oidcSecret".to_string()),
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_oidc_auth(&oidc, "auth", &mut result);
        assert!(result.is_valid(), "use-only oidc should be valid");
    }

    #[test]
    fn test_validate_oidc_auth_properties_only() {
        let oidc = OpenIDConnectSchemeDefinition {
            use_: None,
            authority: Some("https://auth.example.com/token".to_string()),
            grant: Some("client_credentials".to_string()),
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_oidc_auth(&oidc, "auth", &mut result);
        assert!(result.is_valid(), "properties-only oidc should be valid");
    }

    #[test]
    fn test_validate_oidc_auth_mutual_exclusion() {
        let oidc = OpenIDConnectSchemeDefinition {
            use_: Some("oidcSecret".to_string()),
            authority: Some("https://auth.example.com/token".to_string()),
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_oidc_auth(&oidc, "auth", &mut result);
        assert!(!result.is_valid(), "use + properties should be invalid");
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    #[test]
    fn test_validate_oidc_auth_neither_set() {
        let oidc = OpenIDConnectSchemeDefinition {
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_oidc_auth(&oidc, "auth", &mut result);
        assert!(!result.is_valid(), "empty oidc should be invalid");
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::Required));
    }

    #[test]
    fn test_validate_auth_policy_with_oidc() {
        let policy = AuthenticationPolicyDefinition {
            use_: None,
            basic: None,
            bearer: None,
            certificate: None,
            digest: None,
            oauth2: None,
            oidc: Some(OpenIDConnectSchemeDefinition {
                use_: None,
                authority: Some("https://auth.example.com/token".to_string()),
                grant: Some("client_credentials".to_string()),
                ..Default::default()
            }),
        };
        let mut result = ValidationResult::new();
        validate_auth_policy(&policy, "auth", &mut result);
        assert!(result.is_valid(), "valid oidc auth policy should pass");
    }

    // OAuth2 grant type validation tests (matching Go SDK's oneof validation)

    #[test]
    fn test_validate_oauth2_grant_type_valid() {
        for grant in &[
            "authorization_code",
            "client_credentials",
            "password",
            "refresh_token",
            "urn:ietf:params:oauth:grant-type:token-exchange",
        ] {
            let mut result = ValidationResult::new();
            validate_oauth2_grant_type(grant, "auth.oauth2", &mut result);
            assert!(result.is_valid(), "'{}' should be valid grant type", grant);
        }
        // Empty string is allowed (optional field)
        let mut result = ValidationResult::new();
        validate_oauth2_grant_type("", "auth.oauth2", &mut result);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_oauth2_grant_type_invalid() {
        let mut result = ValidationResult::new();
        validate_oauth2_grant_type("invalid_grant", "auth.oauth2", &mut result);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.field.contains("grant")));
    }

    // Authentication policy oneOf validation tests (matching Go SDK's AuthenticationPolicy.UnmarshalJSON)

    #[test]
    fn test_validate_auth_policy_one_of_single_scheme() {
        let policy = AuthenticationPolicyDefinition {
            use_: None,
            basic: Some(BasicAuthenticationSchemeDefinition {
                use_: None,
                username: Some("admin".to_string()),
                password: Some("secret".to_string()),
            }),
            bearer: None,
            certificate: None,
            digest: None,
            oauth2: None,
            oidc: None,
        };
        let mut result = ValidationResult::new();
        validate_auth_policy_one_of(&policy, "auth", &mut result);
        assert!(result.is_valid(), "single scheme should be valid");
    }

    #[test]
    fn test_validate_auth_policy_one_of_multiple_schemes() {
        let policy = AuthenticationPolicyDefinition {
            use_: None,
            basic: Some(BasicAuthenticationSchemeDefinition {
                use_: None,
                username: Some("admin".to_string()),
                password: Some("secret".to_string()),
            }),
            bearer: Some(BearerAuthenticationSchemeDefinition {
                use_: None,
                token: Some("mytoken".to_string()),
            }),
            certificate: None,
            digest: None,
            oauth2: None,
            oidc: None,
        };
        let mut result = ValidationResult::new();
        validate_auth_policy_one_of(&policy, "auth", &mut result);
        assert!(!result.is_valid(), "multiple schemes should be invalid");
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    #[test]
    fn test_validate_auth_policy_one_of_no_schemes() {
        let policy = AuthenticationPolicyDefinition {
            use_: None,
            basic: None,
            bearer: None,
            certificate: None,
            digest: None,
            oauth2: None,
            oidc: None,
        };
        let mut result = ValidationResult::new();
        validate_auth_policy_one_of(&policy, "auth", &mut result);
        assert!(
            result.is_valid(),
            "no schemes is valid (use_ reference could be set separately)"
        );
    }

    // Runtime expression helper validation tests

    #[test]
    fn test_is_non_empty_string() {
        // Plain strings
        assert!(is_non_empty_string("hello"));
        assert!(is_non_empty_string("https://example.com"));
        // Runtime expressions
        assert!(is_non_empty_string("${.foo}"));
        assert!(is_non_empty_string("${ .bar }"));
        // Invalid
        assert!(!is_non_empty_string(""));
    }

    #[test]
    fn test_is_uri_or_runtime_expr() {
        // Literal URIs (no placeholders)
        assert!(is_uri_or_runtime_expr("https://example.com/api"));
        assert!(is_uri_or_runtime_expr("http://localhost:8080"));
        assert!(is_uri_or_runtime_expr("grpc://api.example.com:443"));
        // URI templates (with placeholders)
        assert!(is_uri_or_runtime_expr("http://example.com/{id}"));
        assert!(is_uri_or_runtime_expr(
            "https://api.example.com/v1/users/{userId}/orders/{orderId}"
        ));
        // Runtime expressions
        assert!(is_uri_or_runtime_expr("${.endpoint}"));
        // Invalid
        assert!(!is_uri_or_runtime_expr(""));
        assert!(!is_uri_or_runtime_expr("not-a-uri"));
        assert!(!is_uri_or_runtime_expr("example.com")); // missing scheme
    }

    #[test]
    fn test_is_json_pointer_or_runtime_expr() {
        // JSON Pointers (RFC 6901)
        // Note: empty string "" references whole document per RFC 6901, but function returns false for empty
        // (matching Go SDK behavior where empty = omitempty = not validated)
        assert!(!is_json_pointer_or_runtime_expr(""));
        assert!(is_json_pointer_or_runtime_expr("/"));
        assert!(is_json_pointer_or_runtime_expr("/foo/bar"));
        assert!(is_json_pointer_or_runtime_expr("/items/0/name"));
        assert!(is_json_pointer_or_runtime_expr("/a~1b")); // escaped / in token
        assert!(is_json_pointer_or_runtime_expr("/a~0b")); // escaped ~ in token
                                                           // Runtime expressions
        assert!(is_json_pointer_or_runtime_expr("${.pointer}"));
        // Invalid (doesn't start with /)
        assert!(!is_json_pointer_or_runtime_expr("foo"));
        // Invalid escape sequences
        assert!(!is_json_pointer_or_runtime_expr("/a~2b")); // ~2 is not a valid escape
    }

    // URI/URI-template pattern tests (matching Go SDK's LiteralUriPattern/LiteralUriTemplatePattern)

    #[test]
    fn test_is_valid_uri() {
        // Valid literal URIs
        assert!(is_valid_uri("https://example.com/api"));
        assert!(is_valid_uri("http://localhost:8080"));
        assert!(is_valid_uri("grpc://api.example.com:443"));
        assert!(is_valid_uri("ftp://files.example.com/document.pdf"));
        // URI templates are NOT valid literal URIs
        assert!(!is_valid_uri("http://example.com/{id}"));
        // Invalid
        assert!(!is_valid_uri(""));
        assert!(!is_valid_uri("example.com"));
        assert!(!is_valid_uri("not a uri"));
        assert!(!is_valid_uri("://missing-scheme.com"));
    }

    #[test]
    fn test_is_valid_uri_template() {
        // Valid URI templates
        assert!(is_valid_uri_template("http://example.com/{id}"));
        assert!(is_valid_uri_template(
            "https://api.example.com/v1/{resource}/{id}"
        ));
        // Literal URIs are NOT valid URI templates (no placeholders)
        assert!(!is_valid_uri_template("https://example.com/api"));
        // Invalid
        assert!(!is_valid_uri_template(""));
        assert!(!is_valid_uri_template("example.com/{id}"));
    }

    #[test]
    fn test_is_valid_json_pointer() {
        // RFC 6901 JSON Pointers
        assert!(is_valid_json_pointer("")); // whole document
        assert!(is_valid_json_pointer("/"));
        assert!(is_valid_json_pointer("/foo"));
        assert!(is_valid_json_pointer("/foo/bar"));
        assert!(is_valid_json_pointer("/items/0/name"));
        assert!(is_valid_json_pointer("/a~0b")); // escaped ~
        assert!(is_valid_json_pointer("/a~1b")); // escaped /
                                                 // Invalid
        assert!(!is_valid_json_pointer("foo")); // no leading /
        assert!(!is_valid_json_pointer("/a~2b")); // invalid escape
        assert!(!is_valid_json_pointer("/a~")); // incomplete escape
    }

    // Schedule oneOf validation tests (matching Go SDK's mutual exclusivity)

    #[test]
    fn test_validate_schedule_one_of_single_every() {
        let schedule = WorkflowScheduleDefinition {
            every: Some(Duration {
                hours: Some(1),
                ..Default::default()
            }),
            cron: None,
            after: None,
            on: None,
        };
        let mut result = ValidationResult::new();
        validate_schedule_one_of(&schedule, "schedule", &mut result);
        assert!(result.is_valid(), "single 'every' should be valid");
    }

    #[test]
    fn test_validate_schedule_one_of_single_cron() {
        let schedule = WorkflowScheduleDefinition {
            every: None,
            cron: Some("0 0 * * *".to_string()),
            after: None,
            on: None,
        };
        let mut result = ValidationResult::new();
        validate_schedule_one_of(&schedule, "schedule", &mut result);
        assert!(result.is_valid(), "single 'cron' should be valid");
    }

    #[test]
    fn test_validate_schedule_one_of_multiple() {
        let schedule = WorkflowScheduleDefinition {
            every: Some(Duration {
                hours: Some(1),
                ..Default::default()
            }),
            cron: Some("0 0 * * *".to_string()),
            after: None,
            on: None,
        };
        let mut result = ValidationResult::new();
        validate_schedule_one_of(&schedule, "schedule", &mut result);
        assert!(
            !result.is_valid(),
            "both 'every' and 'cron' should be invalid"
        );
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    #[test]
    fn test_validate_schedule_one_of_none() {
        let schedule = WorkflowScheduleDefinition {
            every: None,
            cron: None,
            after: None,
            on: None,
        };
        let mut result = ValidationResult::new();
        validate_schedule_one_of(&schedule, "schedule", &mut result);
        assert!(
            result.is_valid(),
            "empty schedule is valid (not required field)"
        );
    }

    // ProcessType oneOf validation tests (matching Go SDK's RunTaskConfiguration.UnmarshalJSON)

    #[test]
    fn test_validate_process_type_one_of_shell() {
        let process = ProcessTypeDefinition {
            shell: Some(ShellProcessDefinition {
                command: "echo hello".to_string(),
                ..Default::default()
            }),
            container: None,
            script: None,
            workflow: None,
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_process_type_one_of(&process, "task1", &mut result);
        assert!(result.is_valid(), "single 'shell' should be valid");
    }

    #[test]
    fn test_validate_process_type_one_of_multiple() {
        let process = ProcessTypeDefinition {
            shell: Some(ShellProcessDefinition {
                command: "echo hello".to_string(),
                ..Default::default()
            }),
            container: Some(ContainerProcessDefinition {
                image: "nginx:latest".to_string(),
                ..Default::default()
            }),
            script: None,
            workflow: None,
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_process_type_one_of(&process, "task1", &mut result);
        assert!(
            !result.is_valid(),
            "both 'shell' and 'container' should be invalid"
        );
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    #[test]
    fn test_validate_process_type_one_of_none() {
        let process = ProcessTypeDefinition {
            shell: None,
            container: None,
            script: None,
            workflow: None,
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_process_type_one_of(&process, "task1", &mut result);
        assert!(
            !result.is_valid(),
            "no process type should be invalid (exactly one required)"
        );
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    // Backoff strategy oneOf validation tests (matching Go SDK's RetryBackoff.UnmarshalJSON)

    #[test]
    fn test_validate_backoff_one_of_single_constant() {
        let backoff = BackoffStrategyDefinition {
            constant: Some(ConstantBackoffDefinition::with_delay("PT5S")),
            exponential: None,
            linear: None,
        };
        let mut result = ValidationResult::new();
        validate_backoff_one_of(&backoff, "backoff", &mut result);
        assert!(result.is_valid(), "single 'constant' should be valid");
    }

    #[test]
    fn test_validate_backoff_one_of_single_exponential() {
        let backoff = BackoffStrategyDefinition {
            constant: None,
            exponential: Some(ExponentialBackoffDefinition::with_factor(2.0)),
            linear: None,
        };
        let mut result = ValidationResult::new();
        validate_backoff_one_of(&backoff, "backoff", &mut result);
        assert!(result.is_valid(), "single 'exponential' should be valid");
    }

    #[test]
    fn test_validate_backoff_one_of_multiple() {
        let backoff = BackoffStrategyDefinition {
            constant: Some(ConstantBackoffDefinition::with_delay("PT5S")),
            exponential: Some(ExponentialBackoffDefinition::with_factor(2.0)),
            linear: None,
        };
        let mut result = ValidationResult::new();
        validate_backoff_one_of(&backoff, "backoff", &mut result);
        assert!(
            !result.is_valid(),
            "both 'constant' and 'exponential' should be invalid"
        );
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    #[test]
    fn test_validate_backoff_one_of_none() {
        let backoff = BackoffStrategyDefinition {
            constant: None,
            exponential: None,
            linear: None,
        };
        let mut result = ValidationResult::new();
        validate_backoff_one_of(&backoff, "backoff", &mut result);
        assert!(result.is_valid(), "empty backoff is valid (optional field)");
    }

    // Schema oneOf validation tests (matching Go SDK's Schema.UnmarshalJSON)

    #[test]
    fn test_validate_schema_one_of_document_only() {
        let schema = SchemaDefinition {
            document: Some(serde_json::json!({"type": "object"})),
            resource: None,
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_schema_one_of(&schema, "schema", &mut result);
        assert!(result.is_valid(), "document-only schema should be valid");
    }

    #[test]
    fn test_validate_schema_one_of_resource_only() {
        let schema = SchemaDefinition {
            document: None,
            resource: Some(ExternalResourceDefinition {
                name: Some("schema".to_string()),
                endpoint: crate::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                    "https://example.com/schema.json".to_string(),
                ),
            }),
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_schema_one_of(&schema, "schema", &mut result);
        assert!(result.is_valid(), "resource-only schema should be valid");
    }

    #[test]
    fn test_validate_schema_one_of_both_set() {
        let schema = SchemaDefinition {
            document: Some(serde_json::json!({"type": "object"})),
            resource: Some(ExternalResourceDefinition {
                name: Some("schema".to_string()),
                endpoint: crate::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                    "https://example.com/schema.json".to_string(),
                ),
            }),
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_schema_one_of(&schema, "schema", &mut result);
        assert!(
            !result.is_valid(),
            "both document and resource should be invalid"
        );
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::MutualExclusion));
    }

    #[test]
    fn test_validate_schema_one_of_neither_set() {
        let schema = SchemaDefinition {
            document: None,
            resource: None,
            ..Default::default()
        };
        let mut result = ValidationResult::new();
        validate_schema_one_of(&schema, "schema", &mut result);
        assert!(
            !result.is_valid(),
            "neither document nor resource should be invalid"
        );
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::Required));
    }

    // Extension task type validation tests (matching Go SDK's oneof=call composite emit for listen raise run set switch try wait all)

    #[test]
    fn test_validate_extension_task_type_valid() {
        for task_type in &[
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
        ] {
            let mut result = ValidationResult::new();
            validate_extension_task_type(task_type, "extension", &mut result);
            assert!(
                result.is_valid(),
                "'{}' should be valid extension task type",
                task_type
            );
        }
    }

    #[test]
    fn test_validate_extension_task_type_invalid() {
        let mut result = ValidationResult::new();
        validate_extension_task_type("invalid_type", "extension", &mut result);
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("invalid_type"));
    }

    // HTTP method validation tests (matching Go SDK's oneofci=GET POST PUT DELETE PATCH)

    #[test]
    fn test_validate_http_method_valid() {
        for method in &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] {
            let mut result = ValidationResult::new();
            validate_http_method(method, "task1", &mut result);
            assert!(
                result.is_valid(),
                "'{}' should be valid HTTP method",
                method
            );
        }
    }

    #[test]
    fn test_validate_http_method_case_insensitive() {
        // Go SDK's oneofci validation is case-insensitive
        let mut result = ValidationResult::new();
        validate_http_method("get", "task1", &mut result);
        assert!(
            result.is_valid(),
            "'get' (lowercase) should be valid HTTP method"
        );

        let mut result = ValidationResult::new();
        validate_http_method("Post", "task1", &mut result);
        assert!(
            result.is_valid(),
            "'Post' (mixed case) should be valid HTTP method"
        );
    }

    #[test]
    fn test_validate_http_method_invalid() {
        let mut result = ValidationResult::new();
        validate_http_method("CONNECT", "task1", &mut result);
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("CONNECT"));
    }

    // Container lifetime validation tests (matching Go SDK's required_if=Cleanup eventually)

    #[test]
    fn test_validate_container_lifetime_eventually_with_after() {
        let lifetime = ContainerLifetimeDefinition {
            cleanup: "eventually".to_string(),
            after: Some(OneOfDurationOrIso8601Expression::Iso8601Expression(
                "PT5M".to_string(),
            )),
        };
        let mut result = ValidationResult::new();
        validate_container_lifetime(&lifetime, "container", &mut result);
        assert!(result.is_valid(), "eventually with after should be valid");
    }

    #[test]
    fn test_validate_container_lifetime_eventually_without_after() {
        let lifetime = ContainerLifetimeDefinition {
            cleanup: "eventually".to_string(),
            after: None,
        };
        let mut result = ValidationResult::new();
        validate_container_lifetime(&lifetime, "container", &mut result);
        assert!(
            !result.is_valid(),
            "eventually without after should be invalid"
        );
        assert!(result
            .errors
            .iter()
            .any(|e| e.field.contains("lifetime.after")));
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::Required));
    }

    #[test]
    fn test_validate_container_lifetime_always_no_after_needed() {
        let lifetime = ContainerLifetimeDefinition {
            cleanup: "always".to_string(),
            after: None,
        };
        let mut result = ValidationResult::new();
        validate_container_lifetime(&lifetime, "container", &mut result);
        assert!(result.is_valid(), "always cleanup doesn't require after");
    }

    #[test]
    fn test_validate_container_lifetime_invalid_cleanup() {
        let lifetime = ContainerLifetimeDefinition {
            cleanup: "sometimes".to_string(),
            after: None,
        };
        let mut result = ValidationResult::new();
        validate_container_lifetime(&lifetime, "container", &mut result);
        assert!(!result.is_valid(), "invalid cleanup should fail");
        assert!(result.errors.iter().any(|e| e.field.contains("cleanup")));
    }

    // Document name/version format validation tests (matching Go SDK's validate tags)

    #[test]
    fn test_validate_document_name_hostname() {
        // Go SDK: validate:"required,hostname_rfc1123"
        let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata {
            dsl: "1.0.0".to_string(),
            namespace: "default".to_string(),
            name: "invalid name!".to_string(), // spaces and ! not valid in hostname
            version: "1.0.0".to_string(),
            title: None,
            summary: None,
            tags: None,
            metadata: None,
        });
        let result = validate_workflow(&workflow);
        assert!(!result.is_valid(), "invalid hostname name should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.field == "document.name" && e.rule == ValidationRule::Hostname));
    }

    #[test]
    fn test_validate_document_version_semver() {
        // Go SDK: validate:"required,semver_pattern"
        let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata {
            dsl: "1.0.0".to_string(),
            namespace: "default".to_string(),
            name: "test-workflow".to_string(),
            version: "not-semver".to_string(),
            title: None,
            summary: None,
            tags: None,
            metadata: None,
        });
        let result = validate_workflow(&workflow);
        assert!(!result.is_valid(), "invalid semver version should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.field == "document.version" && e.rule == ValidationRule::Semver));
    }

    #[test]
    fn test_validate_document_name_valid_hostname() {
        let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default",
            "my-workflow",
            "1.0.0",
            None,
            None,
            None,
        ));
        let result = validate_workflow(&workflow);
        assert!(
            result.errors.iter().all(|e| e.field != "document.name"),
            "valid hostname name should pass"
        );
    }

    #[test]
    fn test_validate_document_version_valid_semver() {
        let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default",
            "test-workflow",
            "2.3.4",
            None,
            None,
            None,
        ));
        let result = validate_workflow(&workflow);
        assert!(
            result.errors.iter().all(|e| e.field != "document.version"),
            "valid semver version should pass"
        );
    }

    // HTTP output format validation tests (matching Go SDK's oneof=raw content response)

    #[test]
    fn test_validate_http_output_valid() {
        for output in &["raw", "content", "response"] {
            let mut result = ValidationResult::new();
            validate_http_output(output, "task1.with", &mut result);
            assert!(
                result.is_valid(),
                "'{}' should be valid HTTP output format",
                output
            );
        }
    }

    #[test]
    fn test_validate_http_output_empty_valid() {
        // Empty string is valid (omitempty - optional field)
        let mut result = ValidationResult::new();
        validate_http_output("", "task1.with", &mut result);
        assert!(result.is_valid(), "empty output should be valid (optional)");
    }

    #[test]
    fn test_validate_http_output_invalid() {
        let mut result = ValidationResult::new();
        validate_http_output("invalid", "task1.with", &mut result);
        assert!(!result.is_valid());
        assert!(result.errors[0].message.contains("invalid"));
    }

    // AsyncAPI protocol validation tests (matching Go SDK's oneof validation)

    #[test]
    fn test_validate_asyncapi_protocol_valid() {
        for protocol in &[
            "amqp", "amqp1", "http", "kafka", "mqtt", "mqtt5", "nats", "redis", "ws",
        ] {
            let mut result = ValidationResult::new();
            validate_asyncapi_protocol(protocol, "task1.with", &mut result);
            assert!(
                result.is_valid(),
                "'{}' should be valid AsyncAPI protocol",
                protocol
            );
        }
    }

    #[test]
    fn test_validate_asyncapi_protocol_empty_valid() {
        let mut result = ValidationResult::new();
        validate_asyncapi_protocol("", "task1.with", &mut result);
        assert!(
            result.is_valid(),
            "empty protocol should be valid (optional)"
        );
    }

    #[test]
    fn test_validate_asyncapi_protocol_invalid() {
        let mut result = ValidationResult::new();
        validate_asyncapi_protocol("ftp", "task1.with", &mut result);
        assert!(!result.is_valid());
        assert!(result.errors[0].field.contains("protocol"));
    }

    // GRPC host hostname validation test (matching Go SDK's hostname_rfc1123)

    #[test]
    fn test_validate_grpc_host_hostname() {
        let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default",
            "test-grpc",
            "1.0.0",
            None,
            None,
            None,
        ));
        workflow.do_.add(
            "callGRPC".to_string(),
            TaskDefinition::Call(Box::new(CallTaskDefinition::GRPC(
                Box::new(crate::models::call::CallGRPCDefinition {
                    call: "grpc".to_string(),
                    with: crate::models::call::GRPCArguments {
                        proto: ExternalResourceDefinition {
                            name: Some("proto".to_string()),
                            endpoint: crate::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                                "https://example.com/proto.proto".to_string(),
                            ),
                        },
                        service: crate::models::call::GRPCServiceDefinition {
                            name: "myservice".to_string(),
                            host: "invalid host!".to_string(),
                            port: None,
                            authentication: None,
                        },
                        method: "MyMethod".to_string(),
                        arguments: None,
                        authentication: None,
                    },
                    common: TaskDefinitionFields::new(),
                })))));
        let result = validate_workflow(&workflow);
        assert!(!result.is_valid(), "invalid GRPC host should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.field.contains("service.host") && e.rule == ValidationRule::Hostname));
    }

    // Switch task validation tests (matching Go SDK's switch_item + then required validation)

    #[test]
    fn test_validate_switch_task_valid() {
        let mut switch_cases = Map::new();
        switch_cases.add(
            "case1".to_string(),
            SwitchCaseDefinition {
                when: Some(".status == 200".to_string()),
                then: Some("nextTask".to_string()),
            },
        );
        let switch_task = SwitchTaskDefinition {
            switch: switch_cases,
            common: TaskDefinitionFields::new(),
        };
        let mut result = ValidationResult::new();
        validate_switch_task(&switch_task, "task1", &mut result);
        assert!(
            result.is_valid(),
            "valid switch task should pass, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_switch_task_empty_cases() {
        let switch_task = SwitchTaskDefinition {
            switch: Map::new(),
            common: TaskDefinitionFields::new(),
        };
        let mut result = ValidationResult::new();
        validate_switch_task(&switch_task, "task1", &mut result);
        assert!(!result.is_valid(), "empty switch cases should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.field.contains("switch") && e.rule == ValidationRule::Required));
    }

    #[test]
    fn test_validate_switch_task_case_without_then() {
        let mut switch_cases = Map::new();
        switch_cases.add(
            "case1".to_string(),
            SwitchCaseDefinition {
                when: Some(".status == 200".to_string()),
                then: None, // Missing required 'then'
            },
        );
        let switch_task = SwitchTaskDefinition {
            switch: switch_cases,
            common: TaskDefinitionFields::new(),
        };
        let mut result = ValidationResult::new();
        validate_switch_task(&switch_task, "task1", &mut result);
        assert!(!result.is_valid(), "switch case without 'then' should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.field.contains("then") && e.rule == ValidationRule::Required));
    }

    #[test]
    fn test_validate_switch_task_default_case_with_then() {
        // Default case (no when) with then is valid
        let mut switch_cases = Map::new();
        switch_cases.add(
            "default".to_string(),
            SwitchCaseDefinition {
                when: None,
                then: Some("end".to_string()),
            },
        );
        let switch_task = SwitchTaskDefinition {
            switch: switch_cases,
            common: TaskDefinitionFields::new(),
        };
        let mut result = ValidationResult::new();
        validate_switch_task(&switch_task, "task1", &mut result);
        assert!(result.is_valid(), "default case with then should be valid");
    }

    #[test]
    fn test_validate_switch_task_multiple_cases() {
        let mut switch_cases = Map::new();
        switch_cases.add(
            "case1".to_string(),
            SwitchCaseDefinition {
                when: Some(".x == 1".to_string()),
                then: Some("task2".to_string()),
            },
        );
        switch_cases.add(
            "case2".to_string(),
            SwitchCaseDefinition {
                when: Some(".x == 2".to_string()),
                then: Some("task3".to_string()),
            },
        );
        switch_cases.add(
            "default".to_string(),
            SwitchCaseDefinition {
                when: None,
                then: Some("end".to_string()),
            },
        );
        let switch_task = SwitchTaskDefinition {
            switch: switch_cases,
            common: TaskDefinitionFields::new(),
        };
        let mut result = ValidationResult::new();
        validate_switch_task(&switch_task, "task1", &mut result);
        assert!(
            result.is_valid(),
            "multiple cases should be valid, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_switch_task_integration_empty_in_workflow() {
        let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default",
            "test-switch",
            "1.0.0",
            None,
            None,
            None,
        ));
        workflow.do_.add(
            "emptySwitch".to_string(),
            TaskDefinition::Switch(SwitchTaskDefinition {
                switch: Map::new(),
                common: TaskDefinitionFields::new(),
            }),
        );
        let result = validate_workflow(&workflow);
        assert!(!result.is_valid(), "empty switch should fail validation");
        assert!(result.errors.iter().any(|e| e.field.contains("switch")));
    }

    // WorkflowProcessDefinition validation tests (matching Go SDK's required,hostname_rfc1123,semver_pattern)

    #[test]
    fn test_validate_workflow_process_valid() {
        let workflow_process = crate::models::task::WorkflowProcessDefinition {
            namespace: "default".to_string(),
            name: "child-workflow".to_string(),
            version: "1.0.0".to_string(),
            input: None,
        };
        let mut result = ValidationResult::new();
        validate_workflow_process(&workflow_process, "task1", &mut result);
        assert!(
            result.is_valid(),
            "valid workflow process should pass, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_workflow_process_invalid_namespace() {
        let workflow_process = crate::models::task::WorkflowProcessDefinition {
            namespace: "invalid namespace!".to_string(),
            name: "child-workflow".to_string(),
            version: "1.0.0".to_string(),
            input: None,
        };
        let mut result = ValidationResult::new();
        validate_workflow_process(&workflow_process, "task1", &mut result);
        assert!(!result.is_valid(), "invalid namespace should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.field.contains("namespace") && e.rule == ValidationRule::Hostname));
    }

    #[test]
    fn test_validate_workflow_process_invalid_name() {
        let workflow_process = crate::models::task::WorkflowProcessDefinition {
            namespace: "default".to_string(),
            name: "invalid name!".to_string(),
            version: "1.0.0".to_string(),
            input: None,
        };
        let mut result = ValidationResult::new();
        validate_workflow_process(&workflow_process, "task1", &mut result);
        assert!(!result.is_valid(), "invalid name should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.field.contains("name") && e.rule == ValidationRule::Hostname));
    }

    #[test]
    fn test_validate_workflow_process_invalid_version() {
        let workflow_process = crate::models::task::WorkflowProcessDefinition {
            namespace: "default".to_string(),
            name: "child-workflow".to_string(),
            version: "not-semver".to_string(),
            input: None,
        };
        let mut result = ValidationResult::new();
        validate_workflow_process(&workflow_process, "task1", &mut result);
        assert!(!result.is_valid(), "invalid version should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.field.contains("version") && e.rule == ValidationRule::Semver));
    }

    #[test]
    fn test_validate_workflow_process_empty_fields() {
        let workflow_process = crate::models::task::WorkflowProcessDefinition {
            namespace: "".to_string(),
            name: "".to_string(),
            version: "".to_string(),
            input: None,
        };
        let mut result = ValidationResult::new();
        validate_workflow_process(&workflow_process, "task1", &mut result);
        assert!(!result.is_valid(), "empty fields should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::Required));
    }

    // GRPC host required validation test (matching Go SDK's validate:"required,hostname_rfc1123")

    #[test]
    fn test_validate_grpc_host_required() {
        let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
            "default",
            "test-grpc",
            "1.0.0",
            None,
            None,
            None,
        ));
        workflow.do_.add(
            "callGRPC".to_string(),
            TaskDefinition::Call(Box::new(CallTaskDefinition::GRPC(
                Box::new(crate::models::call::CallGRPCDefinition {
                    call: "grpc".to_string(),
                    with: crate::models::call::GRPCArguments {
                        proto: ExternalResourceDefinition {
                            name: Some("proto".to_string()),
                            endpoint: crate::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                                "https://example.com/proto.proto".to_string(),
                            ),
                        },
                        service: crate::models::call::GRPCServiceDefinition {
                            name: "myservice".to_string(),
                            host: "".to_string(), // Empty host should fail
                            port: None,
                            authentication: None,
                        },
                        method: "MyMethod".to_string(),
                        arguments: None,
                        authentication: None,
                    },
                    common: TaskDefinitionFields::new(),
                })))));
        let result = validate_workflow(&workflow);
        assert!(!result.is_valid(), "empty GRPC host should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.field.contains("service.host") && e.rule == ValidationRule::Required));
    }

    // Set task validation tests (matching Go SDK's validate:"required,min=1,dive")

    #[test]
    fn test_validate_set_task_map_with_values() {
        use crate::models::task::SetValue;
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("key".to_string(), serde_json::json!("value"));
        let set = SetValue::Map(map);
        let mut result = ValidationResult::new();
        validate_set_task(&set, "task1", &mut result);
        assert!(result.is_valid(), "set with values should pass");
    }

    #[test]
    fn test_validate_set_task_empty_map() {
        use crate::models::task::SetValue;
        let set = SetValue::Map(HashMap::new());
        let mut result = ValidationResult::new();
        validate_set_task(&set, "task1", &mut result);
        assert!(!result.is_valid(), "empty set map should fail");
        assert!(result
            .errors
            .iter()
            .any(|e| e.rule == ValidationRule::Required));
    }

    #[test]
    fn test_validate_set_task_expression() {
        use crate::models::task::SetValue;
        let set = SetValue::Expression("${ .data }".to_string());
        let mut result = ValidationResult::new();
        validate_set_task(&set, "task1", &mut result);
        assert!(result.is_valid(), "set with expression should pass");
    }

    #[test]
    fn test_validate_set_task_empty_expression() {
        use crate::models::task::SetValue;
        let set = SetValue::Expression("".to_string());
        let mut result = ValidationResult::new();
        validate_set_task(&set, "task1", &mut result);
        assert!(!result.is_valid(), "empty set expression should fail");
    }
}
