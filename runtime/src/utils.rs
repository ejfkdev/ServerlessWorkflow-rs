use crate::error::{WorkflowError, WorkflowResult};
use serde_json::Value;
use serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression;
use serverless_workflow_core::models::timeout::OneOfTimeoutDefinitionOrReference;

/// Ensures a variable name has the `$` prefix used in JQ expressions.
/// If the name already starts with `$`, returns it unchanged.
/// If empty, returns the provided default.
pub fn ensure_dollar_prefix(name: &str, default: &str) -> String {
    if name.is_empty() {
        default.to_string()
    } else if name.starts_with('$') {
        name.to_string()
    } else {
        format!("${}", name)
    }
}

/// Resolves a OneOfDurationOrIso8601Expression into a std::time::Duration
pub fn resolve_duration_expr(
    after: &OneOfDurationOrIso8601Expression,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<std::time::Duration> {
    match after {
        OneOfDurationOrIso8601Expression::Duration(d) => {
            Ok(std::time::Duration::from_millis(d.total_milliseconds()))
        }
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            let resolved = resolve_duration_expression(expr, input, vars, task_name)?;
            parse_iso8601_duration(&resolved).ok_or_else(|| {
                WorkflowError::validation(
                    format!("invalid ISO 8601 duration: {}", resolved),
                    task_name,
                )
            })
        }
    }
}

/// Parses a timeout definition into a std::time::Duration with expression context support.
/// If the ISO8601 string contains a JQ expression (${...}), it will be evaluated first.
/// Supports both inline timeout definitions and references to reusable timeouts in workflow.use.timeouts.
pub fn parse_duration_with_context(
    timeout: &OneOfTimeoutDefinitionOrReference,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
    workflow: Option<&serverless_workflow_core::models::workflow::WorkflowDefinition>,
) -> WorkflowResult<std::time::Duration> {
    match timeout {
        OneOfTimeoutDefinitionOrReference::Timeout(def) => {
            resolve_duration_expr(&def.after, input, vars, task_name)
        }
        OneOfTimeoutDefinitionOrReference::Reference(ref_name) => {
            let workflow = workflow.ok_or_else(|| {
                WorkflowError::runtime_simple(
                    "referenced timeout requires workflow context",
                    task_name,
                )
            })?;
            let use_ = workflow.use_.as_ref().ok_or_else(|| {
                WorkflowError::runtime_simple(
                    format!(
                        "referenced timeout '{}' not found: no use definitions",
                        ref_name
                    ),
                    task_name,
                )
            })?;
            let timeouts = use_.timeouts.as_ref().ok_or_else(|| {
                WorkflowError::runtime_simple(
                    format!(
                        "referenced timeout '{}' not found: no timeouts defined",
                        ref_name
                    ),
                    task_name,
                )
            })?;
            let timeout_def = timeouts.get(ref_name).ok_or_else(|| {
                WorkflowError::runtime_simple(
                    format!(
                        "referenced timeout '{}' not found in workflow.use.timeouts",
                        ref_name
                    ),
                    task_name,
                )
            })?;
            resolve_duration_expr(&timeout_def.after, input, vars, task_name)
        }
    }
}

/// Resolves a JQ expression within a duration string.
/// If the string contains ${...}, evaluates it; otherwise returns as-is.
pub fn resolve_duration_expression(
    expr: &str,
    input: &Value,
    vars: &std::collections::HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<String> {
    if expr.contains("${") {
        let sanitized = crate::expression::prepare_expression(expr);
        let result = crate::expression::evaluate_jq(&sanitized, input, vars)
            .map_err(|e| WorkflowError::expression(format!("{}", e), task_name))?;
        match result {
            Value::String(s) => Ok(s),
            other => Ok(other.to_string()),
        }
    } else {
        Ok(expr.to_string())
    }
}

/// Parses an ISO 8601 duration string (e.g., "PT30S", "PT5M", "PT1H", "PT0.1S")
pub fn parse_iso8601_duration(s: &str) -> Option<std::time::Duration> {
    let s = s.strip_prefix('P')?;
    let mut total_ms: u64 = 0;
    let mut current_buf = String::new();
    let mut in_time = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            'T' => in_time = true,
            '0'..='9' | '.' => {
                current_buf.push(chars[i]);
            }
            'D' if !in_time => {
                let val: u64 = current_buf.parse().ok()?;
                total_ms += val * 24 * 60 * 60 * 1000;
                current_buf.clear();
            }
            'H' if in_time => {
                let val: f64 = current_buf.parse().ok()?;
                total_ms += (val * 60.0 * 60.0 * 1000.0) as u64;
                current_buf.clear();
            }
            'M' if in_time => {
                // Check if next char is 'S' (milliseconds: MS suffix)
                if i + 1 < chars.len() && chars[i + 1] == 'S' {
                    let val: u64 = current_buf.parse().ok()?;
                    total_ms += val;
                    i += 1; // skip the 'S' after 'M'
                } else {
                    let val: f64 = current_buf.parse().ok()?;
                    total_ms += (val * 60.0 * 1000.0) as u64;
                }
                current_buf.clear();
            }
            'S' if in_time => {
                let val: f64 = current_buf.parse().ok()?;
                total_ms += (val * 1000.0) as u64;
                current_buf.clear();
            }
            _ => return None,
        }
        i += 1;
    }

    Some(std::time::Duration::from_millis(total_ms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serverless_workflow_core::models::duration::Duration;
    use serverless_workflow_core::models::timeout::TimeoutDefinition;
    use std::collections::HashMap;

    #[test]
    fn test_parse_duration_with_context_struct() {
        let timeout = OneOfTimeoutDefinitionOrReference::Timeout(TimeoutDefinition {
            after: OneOfDurationOrIso8601Expression::Duration(Duration::from_seconds(30)),
        });
        let vars = HashMap::new();
        let dur = parse_duration_with_context(&timeout, &Value::Null, &vars, "test", None).unwrap();
        assert_eq!(dur, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_parse_duration_with_context_iso8601() {
        let timeout = OneOfTimeoutDefinitionOrReference::Timeout(TimeoutDefinition {
            after: OneOfDurationOrIso8601Expression::Iso8601Expression("PT5M".to_string()),
        });
        let vars = HashMap::new();
        let dur = parse_duration_with_context(&timeout, &Value::Null, &vars, "test", None).unwrap();
        assert_eq!(dur, std::time::Duration::from_secs(300));
    }

    #[test]
    fn test_parse_iso8601_various() {
        assert_eq!(
            parse_iso8601_duration("PT30S"),
            Some(std::time::Duration::from_secs(30))
        );
        assert_eq!(
            parse_iso8601_duration("PT1H"),
            Some(std::time::Duration::from_secs(3600))
        );
        assert_eq!(
            parse_iso8601_duration("PT1M30S"),
            Some(std::time::Duration::from_secs(90))
        );
        assert_eq!(
            parse_iso8601_duration("P1D"),
            Some(std::time::Duration::from_secs(86400))
        );
        assert_eq!(
            parse_iso8601_duration("P1DT2H30M"),
            Some(std::time::Duration::from_millis(
                86400000 + 7200000 + 1800000
            ))
        );
        assert_eq!(
            parse_iso8601_duration("PT0.1S"),
            Some(std::time::Duration::from_millis(100))
        );
        assert_eq!(
            parse_iso8601_duration("PT1.5S"),
            Some(std::time::Duration::from_millis(1500))
        );
    }

    #[test]
    fn test_parse_iso8601_milliseconds_suffix() {
        // PT250MS = 250 milliseconds
        assert_eq!(
            parse_iso8601_duration("PT250MS"),
            Some(std::time::Duration::from_millis(250))
        );
        // Combined with MS
        assert_eq!(
            parse_iso8601_duration("P3DT4H5M6S250MS"),
            Some(std::time::Duration::from_millis(
                3 * 86400000 + 4 * 3600000 + 5 * 60000 + 6 * 1000 + 250
            ))
        );
        // Just MS
        assert_eq!(
            parse_iso8601_duration("PT100MS"),
            Some(std::time::Duration::from_millis(100))
        );
    }
}
