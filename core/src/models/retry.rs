use crate::models::duration::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Represents the definition of a retry policy
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryPolicyDefinition {
    /// Gets/sets a runtime expression used to determine whether or not to retry running the task, in a given context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,

    /// Gets/sets a runtime expression used to determine whether or not to retry running the task, in a given context
    #[serde(rename = "exceptWhen", skip_serializing_if = "Option::is_none")]
    pub except_when: Option<String>,

    /// Gets/sets the limits, if any, of the retry policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<RetryPolicyLimitDefinition>,

    /// Gets/sets the delay duration between retry attempts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<OneOfDurationOrIso8601Expression>,

    /// Gets/sets the backoff strategy to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff: Option<BackoffStrategyDefinition>,

    /// Gets/sets the parameters, if any, that control the randomness or variability of the delay between retry attempts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jitter: Option<JitterDefinition>,
}

/// Represents the configuration of the limits of a retry policy
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicyLimitDefinition {
    /// Gets/sets the definition of the limits for all retry attempts of a given policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<RetryAttemptLimitDefinition>,

    /// Gets/sets the maximum duration, if any, during which to retry a given task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<OneOfDurationOrIso8601Expression>,
}

/// Represents the definition of the limits for all retry attempts of a given policy
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryAttemptLimitDefinition {
    /// Gets/sets the maximum attempts count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u16>,

    /// Gets/sets the duration limit, if any, for all retry attempts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<OneOfDurationOrIso8601Expression>,
}

/// Represents the definition of a retry backoff strategy
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackoffStrategyDefinition {
    /// Gets/sets the definition of the constant backoff to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constant: Option<ConstantBackoffDefinition>,

    /// Gets/sets the definition of the exponential backoff to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exponential: Option<ExponentialBackoffDefinition>,

    /// Gets/sets the definition of the linear backoff to use, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear: Option<LinearBackoffDefinition>,
}

/// Represents the definition of a constant backoff
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstantBackoffDefinition {
    /// Gets/sets the definition of the constant backoff parameters (e.g., {"factor": 2})
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub definition: Option<HashMap<String, Value>>,
}
impl ConstantBackoffDefinition {
    /// Creates a constant backoff with a delay duration
    pub fn with_delay(delay: &str) -> Self {
        let mut map = HashMap::new();
        map.insert("delay".to_string(), Value::String(delay.to_string()));
        Self {
            definition: Some(map),
        }
    }

    /// Gets the delay value, if set
    pub fn delay(&self) -> Option<&str> {
        self.definition.as_ref()?.get("delay")?.as_str()
    }
}

/// Represents the definition of an exponential backoff
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExponentialBackoffDefinition {
    /// Gets/sets the definition of the exponential backoff parameters (e.g., {"factor": 2, "maxDelay": "PT30S"})
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub definition: Option<HashMap<String, Value>>,
}
impl ExponentialBackoffDefinition {
    /// Creates an exponential backoff with a factor
    pub fn with_factor(factor: f64) -> Self {
        let mut map = HashMap::new();
        map.insert("factor".to_string(), Value::from(factor));
        Self {
            definition: Some(map),
        }
    }

    /// Creates an exponential backoff with a factor and max delay
    pub fn with_factor_and_max_delay(factor: f64, max_delay: &str) -> Self {
        let mut map = HashMap::new();
        map.insert("factor".to_string(), Value::from(factor));
        map.insert("maxDelay".to_string(), Value::String(max_delay.to_string()));
        Self {
            definition: Some(map),
        }
    }

    /// Gets the factor value, if set
    pub fn factor(&self) -> Option<f64> {
        self.definition.as_ref()?.get("factor")?.as_f64()
    }

    /// Gets the maxDelay value, if set
    pub fn max_delay(&self) -> Option<&str> {
        self.definition.as_ref()?.get("maxDelay")?.as_str()
    }
}

/// Represents the definition of a linear backoff
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinearBackoffDefinition {
    /// Gets/sets the linear incrementation to the delay between retry attempts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub increment: Option<Duration>,

    /// Gets/sets the definition of additional linear backoff parameters (e.g., {"maxDelay": "PT30S"})
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub definition: Option<HashMap<String, Value>>,
}

/// Represents the definition of the parameters that control the randomness or variability of a delay, typically between retry attempts
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JitterDefinition {
    /// Gets/sets the minimum duration of the jitter range
    pub from: Duration,

    /// Gets/sets the maximum duration of the jitter range
    pub to: Duration,
}

define_one_of_or_reference!(
    /// Represents a value that can be either a RetryPolicyDefinition or a reference to a RetryPolicyDefinition
    OneOfRetryPolicyDefinitionOrReference, Retry(Box<RetryPolicyDefinition>)
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_serialize() {
        let policy = RetryPolicyDefinition {
            when: Some("${someCondition}".to_string()),
            except_when: Some("${someOtherCondition}".to_string()),
            delay: Some(OneOfDurationOrIso8601Expression::Duration(
                Duration::from_seconds(5),
            )),
            backoff: Some(BackoffStrategyDefinition {
                exponential: Some(ExponentialBackoffDefinition::with_factor(2.0)),
                constant: None,
                linear: None,
            }),
            limit: Some(RetryPolicyLimitDefinition {
                attempt: Some(RetryAttemptLimitDefinition {
                    count: Some(3),
                    duration: Some(OneOfDurationOrIso8601Expression::Duration(
                        Duration::from_minutes(1),
                    )),
                }),
                duration: Some(OneOfDurationOrIso8601Expression::Duration(
                    Duration::from_minutes(10),
                )),
            }),
            jitter: Some(JitterDefinition {
                from: Duration::from_seconds(1),
                to: Duration::from_seconds(3),
            }),
        };

        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("\"when\":\"${someCondition}\""));
        assert!(json.contains("\"exceptWhen\":\"${someOtherCondition}\""));
        assert!(json.contains("\"factor\":2"));
        assert!(json.contains("\"count\":3"));
    }

    #[test]
    fn test_retry_policy_deserialize_inline() {
        let json = r#"{
            "when": "${someCondition}",
            "exceptWhen": "${someOtherCondition}",
            "delay": {"seconds": 5},
            "backoff": {"exponential": {"factor": 2}},
            "limit": {
                "attempt": {"count": 3, "duration": {"minutes": 1}},
                "duration": {"minutes": 10}
            },
            "jitter": {"from": {"seconds": 1}, "to": {"seconds": 3}}
        }"#;

        let policy: RetryPolicyDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(policy.when, Some("${someCondition}".to_string()));
        assert_eq!(
            policy.except_when,
            Some("${someOtherCondition}".to_string())
        );
        assert!(policy.delay.is_some());
        assert!(policy.backoff.is_some());
        assert!(policy.backoff.as_ref().unwrap().exponential.is_some());
        assert_eq!(
            policy
                .backoff
                .as_ref()
                .unwrap()
                .exponential
                .as_ref()
                .unwrap()
                .factor(),
            Some(2.0)
        );
        assert!(policy.limit.is_some());
        assert_eq!(
            policy
                .limit
                .as_ref()
                .unwrap()
                .attempt
                .as_ref()
                .unwrap()
                .count,
            Some(3)
        );
        assert!(policy.jitter.is_some());
    }

    #[test]
    fn test_oneof_retry_reference_deserialize() {
        let json = r#""default""#;
        let oneof: OneOfRetryPolicyDefinitionOrReference = serde_json::from_str(json).unwrap();
        match oneof {
            OneOfRetryPolicyDefinitionOrReference::Reference(name) => {
                assert_eq!(name, "default");
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn test_oneof_retry_inline_deserialize() {
        let json = r#"{"delay": {"seconds": 3}, "limit": {"attempt": {"count": 5}}}"#;
        let oneof: OneOfRetryPolicyDefinitionOrReference = serde_json::from_str(json).unwrap();
        match oneof {
            OneOfRetryPolicyDefinitionOrReference::Retry(policy) => {
                assert!(policy.delay.is_some());
                assert_eq!(
                    policy
                        .limit
                        .as_ref()
                        .unwrap()
                        .attempt
                        .as_ref()
                        .unwrap()
                        .count,
                    Some(5)
                );
            }
            _ => panic!("Expected Retry variant"),
        }
    }

    #[test]
    fn test_constant_backoff_definition() {
        let constant = ConstantBackoffDefinition::with_delay("PT5S");
        assert_eq!(constant.delay(), Some("PT5S"));

        let empty = ConstantBackoffDefinition::default();
        assert_eq!(empty.delay(), None);
    }

    #[test]
    fn test_exponential_backoff_definition() {
        let exp = ExponentialBackoffDefinition::with_factor(2.5);
        assert_eq!(exp.factor(), Some(2.5));
        assert_eq!(exp.max_delay(), None);

        let exp_with_max = ExponentialBackoffDefinition::with_factor_and_max_delay(2.0, "PT30S");
        assert_eq!(exp_with_max.factor(), Some(2.0));
        assert_eq!(exp_with_max.max_delay(), Some("PT30S"));
    }

    #[test]
    fn test_backoff_strategy_serialize() {
        let backoff = BackoffStrategyDefinition {
            constant: None,
            exponential: Some(ExponentialBackoffDefinition::with_factor(2.0)),
            linear: Some(LinearBackoffDefinition {
                increment: Some(Duration::from_milliseconds(500)),
                definition: None,
            }),
        };

        let json = serde_json::to_string(&backoff).unwrap();
        assert!(json.contains("\"exponential\""));
        assert!(json.contains("\"factor\":2"));
        assert!(json.contains("\"linear\""));
        assert!(json.contains("\"increment\""));
    }

    #[test]
    fn test_backoff_strategy_deserialize() {
        let json = r#"{"exponential": {"factor": 2.5, "maxDelay": "PT30S"}}"#;
        let backoff: BackoffStrategyDefinition = serde_json::from_str(json).unwrap();
        assert!(backoff.exponential.is_some());
        assert_eq!(backoff.exponential.as_ref().unwrap().factor(), Some(2.5));
        assert_eq!(
            backoff.exponential.as_ref().unwrap().max_delay(),
            Some("PT30S")
        );
    }

    #[test]
    fn test_retry_attempt_limit_default() {
        let limit = RetryAttemptLimitDefinition::default();
        assert_eq!(limit.count, None);
        assert_eq!(limit.duration, None);
    }

    #[test]
    fn test_jitter_definition() {
        let jitter = JitterDefinition {
            from: Duration::from_seconds(1),
            to: Duration::from_seconds(3),
        };
        assert_eq!(jitter.from.total_milliseconds(), 1000);
        assert_eq!(jitter.to.total_milliseconds(), 3000);
    }

    // Additional tests matching Go SDK's task_try_test.go

    #[test]
    fn test_retry_policy_roundtrip_with_iso8601() {
        // Matches Go SDK's TestRetryPolicy_MarshalJSON/UnmarshalJSON
        let json = r#"{
            "when": "${someCondition}",
            "exceptWhen": "${someOtherCondition}",
            "delay": {"seconds": 5},
            "backoff": {"exponential": {"factor": 2}},
            "limit": {
                "attempt": {"count": 3, "duration": {"minutes": 1}},
                "duration": {"minutes": 10}
            },
            "jitter": {"from": {"seconds": 1}, "to": {"seconds": 3}}
        }"#;
        let policy: RetryPolicyDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(policy.when, Some("${someCondition}".to_string()));
        assert_eq!(
            policy.except_when,
            Some("${someOtherCondition}".to_string())
        );
        assert!(policy.delay.is_some());
        assert!(policy.delay.as_ref().unwrap().is_duration());
        match policy.delay.as_ref().unwrap() {
            OneOfDurationOrIso8601Expression::Duration(d) => assert_eq!(d.seconds, Some(5)),
            _ => panic!("Expected Duration variant"),
        }
        // roundtrip
        let serialized = serde_json::to_string(&policy).unwrap();
        let deserialized: RetryPolicyDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(policy, deserialized);
    }

    #[test]
    fn test_retry_policy_constant_backoff_roundtrip() {
        let json = r#"{
            "delay": {"seconds": 3},
            "backoff": {"constant": {"delay": "PT5S"}},
            "limit": {"attempt": {"count": 5}}
        }"#;
        let policy: RetryPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.backoff.as_ref().unwrap().constant.is_some());
        let serialized = serde_json::to_string(&policy).unwrap();
        let deserialized: RetryPolicyDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(policy, deserialized);
    }

    #[test]
    fn test_retry_policy_linear_backoff_deserialize() {
        let json = r#"{
            "delay": {"seconds": 1},
            "backoff": {"linear": {"increment": {"seconds": 2}}},
            "limit": {"attempt": {"count": 3}}
        }"#;
        let policy: RetryPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.backoff.as_ref().unwrap().linear.is_some());
        let linear = policy.backoff.as_ref().unwrap().linear.as_ref().unwrap();
        assert!(linear.increment.is_some());
    }

    #[test]
    fn test_oneof_retry_reference_roundtrip() {
        let ref_val = OneOfRetryPolicyDefinitionOrReference::Reference("default".to_string());
        let serialized = serde_json::to_string(&ref_val).unwrap();
        assert_eq!(serialized, r#""default""#);
        let deserialized: OneOfRetryPolicyDefinitionOrReference =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(ref_val, deserialized);
    }

    #[test]
    fn test_oneof_retry_inline_roundtrip() {
        let json = r#"{"delay": {"seconds": 3}, "backoff": {"exponential": {}}, "limit": {"attempt": {"count": 5}}}"#;
        let oneof: OneOfRetryPolicyDefinitionOrReference = serde_json::from_str(json).unwrap();
        match &oneof {
            OneOfRetryPolicyDefinitionOrReference::Retry(policy) => {
                assert!(policy.delay.is_some());
                assert!(policy.backoff.is_some());
            }
            _ => panic!("Expected Retry variant"),
        }
        let serialized = serde_json::to_string(&oneof).unwrap();
        let deserialized: OneOfRetryPolicyDefinitionOrReference =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(oneof, deserialized);
    }

    #[test]
    fn test_retry_limit_duration_only() {
        let json = r#"{"limit": {"duration": {"minutes": 5}}}"#;
        let policy: RetryPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.limit.is_some());
        assert!(policy.limit.as_ref().unwrap().duration.is_some());
        assert!(policy.limit.as_ref().unwrap().attempt.is_none());
    }

    #[test]
    fn test_retry_policy_minimal() {
        // Minimal retry with just delay
        let json = r#"{"delay": {"seconds": 1}}"#;
        let policy: RetryPolicyDefinition = serde_json::from_str(json).unwrap();
        assert!(policy.delay.is_some());
        assert!(policy.when.is_none());
        assert!(policy.except_when.is_none());
        assert!(policy.backoff.is_none());
        assert!(policy.limit.is_none());
        assert!(policy.jitter.is_none());
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicyDefinition::default();
        assert!(policy.when.is_none());
        assert!(policy.except_when.is_none());
        assert!(policy.delay.is_none());
        assert!(policy.backoff.is_none());
        assert!(policy.limit.is_none());
        assert!(policy.jitter.is_none());
    }
}
