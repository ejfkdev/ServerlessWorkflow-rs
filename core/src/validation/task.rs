use crate::models::map::Map;
use crate::models::retry::OneOfRetryPolicyDefinitionOrReference;
use crate::models::task::*;
use super::{ValidationResult, ValidationRule, is_valid_hostname, validate_required_hostname, validate_required_semver};
use super::document::validate_timeout;
use super::enum_validators::*;
use super::one_of_validators::{validate_backoff_one_of, validate_process_type_one_of};

/// Validates a map of task definitions
pub fn validate_task_map(
    tasks: &Map<String, TaskDefinition>,
    prefix: &str,
    result: &mut ValidationResult,
) {
    for (name, task) in &tasks.entries {
        validate_task(task, &format!("{}.{}", prefix, name), result);
    }
}

/// Validates a single task definition
pub(crate) fn validate_task(task: &TaskDefinition, prefix: &str, result: &mut ValidationResult) {
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
pub(crate) fn validate_run_task(
    run: &ProcessTypeDefinition,
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
pub(crate) fn validate_call_task(call: &CallTaskDefinition, prefix: &str, result: &mut ValidationResult) {
    validate_common_fields(call.common_fields(), prefix, result);
    match call {
        CallTaskDefinition::HTTP(http) => {
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
            if let Some(ref protocol) = asyncapi.with.protocol {
                validate_asyncapi_protocol(protocol, &format!("{}.with", prefix), result);
            }
        }
        CallTaskDefinition::A2A(a2a) => {
            if a2a.with.method.is_empty() {
                result.add_error(
                    &format!("{}.with.method", prefix),
                    ValidationRule::Required,
                    "A2A method is required",
                );
            }
        }
        CallTaskDefinition::Function(func) => {
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
pub(crate) fn validate_common_fields(
    fields: &TaskDefinitionFields,
    prefix: &str,
    result: &mut ValidationResult,
) {
    if let Some(ref timeout) = fields.timeout {
        validate_timeout(timeout, &format!("{}.timeout", prefix), result);
    }
}

/// Validates a set task's value
/// Matches Go SDK's validate:"required,min=1,dive"
/// Set must have at least 1 key-value pair
pub fn validate_set_task(
    set: &SetValue,
    prefix: &str,
    result: &mut ValidationResult,
) {
    match set {
        SetValue::Map(map) => {
            if map.is_empty() {
                result.add_error(
                    &format!("{}.set", prefix),
                    ValidationRule::Required,
                    "set task must have at least one key-value pair",
                );
            }
        }
        SetValue::Expression(expr) => {
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
    workflow: &WorkflowProcessDefinition,
    prefix: &str,
    result: &mut ValidationResult,
) {
    let p = &format!("{}.run.workflow", prefix);
    validate_required_hostname(&workflow.namespace, &format!("{}.namespace", p), result);
    validate_required_hostname(&workflow.name, &format!("{}.name", p), result);
    validate_required_semver(&workflow.version, &format!("{}.version", p), result);
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
