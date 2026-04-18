use super::*;

#[test]
fn test_retry_policy_serialization() {
    // Test RetryPolicyDefinition serialization
    let retry_json = json!({
        "when": "${someCondition}",
        "exceptWhen": "${someOtherCondition}",
        "delay": {"seconds": 5},
        "backoff": {"exponential": {}},
        "limit": {
            "attempt": {"count": 3, "duration": {"minutes": 1}},
            "duration": {"minutes": 10}
        },
        "jitter": {"from": {"seconds": 1}, "to": {"seconds": 3}}
    });
    let result: Result<RetryPolicyDefinition, _> = serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry policy: {:?}",
        result.err()
    );
    let retry_policy = result.unwrap();
    assert_eq!(retry_policy.when, Some("${someCondition}".to_string()));
    assert_eq!(
        retry_policy.except_when,
        Some("${someOtherCondition}".to_string())
    );
    assert!(retry_policy.delay.is_some());
    assert!(retry_policy.backoff.is_some());
    assert!(retry_policy.limit.is_some());
    assert!(retry_policy.jitter.is_some());
}

#[test]
fn test_retry_policy_roundtrip_serialization() {
    // Test RetryPolicyDefinition roundtrip serialization
    let retry_policy = RetryPolicyDefinition {
        when: Some("${condition}".to_string()),
        except_when: Some("${exceptCondition}".to_string()),
        delay: Some(OneOfDurationOrIso8601Expression::Duration(
            Duration::from_seconds(5),
        )),
        backoff: Some(BackoffStrategyDefinition {
            constant: None,
            exponential: Some(ExponentialBackoffDefinition::default()),
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

    let json_str = serde_json::to_string(&retry_policy).expect("Failed to serialize retry policy");
    let deserialized: RetryPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(deserialized.when, Some("${condition}".to_string()));
    assert_eq!(
        deserialized.except_when,
        Some("${exceptCondition}".to_string())
    );
    assert!(deserialized.backoff.is_some());
    assert!(deserialized.limit.is_some());
    assert!(deserialized.jitter.is_some());
}

#[test]
fn test_retry_policy_with_exponential_backoff() {
    // Test RetryPolicyDefinition with exponential backoff
    let retry_json = json!({
        "backoff": {"exponential": {}},
        "limit": {"attempt": {"count": 5}}
    });
    let result: Result<RetryPolicyDefinition, _> = serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry policy: {:?}",
        result.err()
    );
    let retry_policy = result.unwrap();
    assert!(retry_policy.backoff.is_some());
    let backoff = retry_policy.backoff.unwrap();
    assert!(backoff.exponential.is_some());
}

#[test]
fn test_retry_policy_with_linear_backoff() {
    // Test RetryPolicyDefinition with linear backoff
    let retry_json = json!({
        "backoff": {"linear": {"increment": {"seconds": 5}}},
        "limit": {"attempt": {"count": 3}}
    });
    let result: Result<RetryPolicyDefinition, _> = serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry policy with linear backoff: {:?}",
        result.err()
    );
    let retry_policy = result.unwrap();
    assert!(retry_policy.backoff.is_some());
    let backoff = retry_policy.backoff.unwrap();
    assert!(backoff.linear.is_some());
    if let Some(linear) = &backoff.linear {
        assert!(linear.increment.is_some());
    }
}

#[test]
fn test_retry_policy_with_constant_backoff() {
    // Test RetryPolicyDefinition with constant backoff
    let retry_json = json!({
        "backoff": {"constant": {}},
        "limit": {"attempt": {"count": 3}}
    });
    let result: Result<RetryPolicyDefinition, _> = serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry policy with constant backoff: {:?}",
        result.err()
    );
    let retry_policy = result.unwrap();
    assert!(retry_policy.backoff.is_some());
    let backoff = retry_policy.backoff.unwrap();
    assert!(backoff.constant.is_some());
}

#[test]
fn test_retry_policy_limit_attempt() {
    // Test RetryAttemptLimitDefinition
    let limit_json = json!({
        "attempt": {"count": 10, "duration": {"seconds": 30}}
    });
    let result: Result<RetryPolicyLimitDefinition, _> = serde_json::from_value(limit_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry limit: {:?}",
        result.err()
    );
    let limit = result.unwrap();
    assert!(limit.attempt.is_some());
    let attempt = limit.attempt.unwrap();
    assert_eq!(attempt.count, Some(10));
    assert!(attempt.duration.is_some());
}

#[test]
fn test_jitter_definition_serialization() {
    // Test JitterDefinition serialization
    let jitter_json = json!({
        "from": {"seconds": 1},
        "to": {"seconds": 5}
    });
    let result: Result<JitterDefinition, _> = serde_json::from_value(jitter_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize jitter: {:?}",
        result.err()
    );
    let jitter = result.unwrap();
    assert_eq!(jitter.from.seconds, Some(1));
    assert_eq!(jitter.to.seconds, Some(5));
}

#[test]
fn test_retry_policy_with_backoff() {
    use serverless_workflow_core::models::retry::{
        BackoffStrategyDefinition, ExponentialBackoffDefinition, RetryPolicyDefinition,
    };
    let retry = RetryPolicyDefinition {
        when: Some("${ .retryable }".to_string()),
        delay: Some(OneOfDurationOrIso8601Expression::Duration(
            Duration::from_milliseconds(1000),
        )),
        backoff: Some(BackoffStrategyDefinition {
            exponential: Some(ExponentialBackoffDefinition::default()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&retry).expect("Failed to serialize");
    assert!(json_str.contains("exponential"));
}

#[test]
fn test_retry_policy_roundtrip() {
    use serverless_workflow_core::models::retry::{LinearBackoffDefinition, RetryPolicyDefinition};
    let retry = RetryPolicyDefinition {
        when: Some("${ .shouldRetry }".to_string()),
        delay: Some(OneOfDurationOrIso8601Expression::Duration(
            Duration::from_milliseconds(500),
        )),
        backoff: Some(BackoffStrategyDefinition {
            linear: Some(LinearBackoffDefinition::default()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&retry).expect("Failed to serialize");
    let deserialized: RetryPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(retry.when, deserialized.when);
}

#[test]
fn test_retry_policy_with_all_fields() {
    use serverless_workflow_core::models::retry::{
        BackoffStrategyDefinition, ExponentialBackoffDefinition, JitterDefinition,
        RetryAttemptLimitDefinition, RetryPolicyDefinition, RetryPolicyLimitDefinition,
    };
    let retry = RetryPolicyDefinition {
        when: Some("${ .shouldRetry }".to_string()),
        except_when: Some("${ .shouldNotRetry }".to_string()),
        delay: Some(OneOfDurationOrIso8601Expression::Duration(
            Duration::from_seconds(5),
        )),
        backoff: Some(BackoffStrategyDefinition {
            exponential: Some(ExponentialBackoffDefinition::default()),
            ..Default::default()
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
    let json_str = serde_json::to_string(&retry).expect("Failed to serialize");
    assert!(json_str.contains("when"));
    assert!(json_str.contains("exceptWhen"));
    assert!(json_str.contains("exponential"));
    assert!(json_str.contains("jitter"));
}

#[test]
fn test_retry_policy_roundtrip_with_all_fields() {
    use serverless_workflow_core::models::retry::{
        BackoffStrategyDefinition, ConstantBackoffDefinition, JitterDefinition,
        RetryPolicyDefinition,
    };
    let retry = RetryPolicyDefinition {
        when: Some("${ .retryable }".to_string()),
        delay: Some(OneOfDurationOrIso8601Expression::Duration(
            Duration::from_milliseconds(500),
        )),
        backoff: Some(BackoffStrategyDefinition {
            constant: Some(ConstantBackoffDefinition::default()),
            ..Default::default()
        }),
        jitter: Some(JitterDefinition {
            from: Duration::from_milliseconds(100),
            to: Duration::from_milliseconds(300),
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&retry).expect("Failed to serialize");
    let deserialized: RetryPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(retry.when, deserialized.when);
    assert!(json_str.contains("constant"));
}

#[test]
fn test_retry_with_linear_backoff() {
    // Test retry policy with linear backoff
    let retry_json = json!({
        "delay": {
            "seconds": 5
        },
        "backoff": {
            "linear": {
                "wait": "PT1S"
            }
        },
        "limit": {
            "attempt": {
                "count": 3
            }
        }
    });

    let result: Result<serverless_workflow_core::models::retry::RetryPolicyDefinition, _> =
        serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry with linear backoff: {:?}",
        result.err()
    );
}

#[test]
fn test_retry_with_constant_backoff() {
    // Test retry policy with constant backoff
    let retry_json = json!({
        "delay": {
            "seconds": 5
        },
        "backoff": {
            "constant": {
                "wait": "PT1S"
            }
        },
        "limit": {
            "attempt": {
                "count": 3
            }
        }
    });

    let result: Result<serverless_workflow_core::models::retry::RetryPolicyDefinition, _> =
        serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry with constant backoff: {:?}",
        result.err()
    );
}

#[test]
fn test_retry_with_exponential_backoff() {
    // Test retry policy with exponential backoff
    let retry_json = json!({
        "delay": {
            "seconds": 5
        },
        "backoff": {
            "exponential": {}
        },
        "limit": {
            "attempt": {
                "count": 3
            }
        }
    });

    let result: Result<serverless_workflow_core::models::retry::RetryPolicyDefinition, _> =
        serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry with exponential backoff: {:?}",
        result.err()
    );
}

#[test]
fn test_backoff_definition_with_parameters() {
    use serverless_workflow_core::models::retry::*;

    // Test constant backoff with definition
    let constant = ConstantBackoffDefinition {
        definition: Some({
            let mut map = std::collections::HashMap::new();
            map.insert("factor".to_string(), json!(2));
            map
        }),
    };
    let json_str = serde_json::to_string(&constant).expect("Failed to serialize constant backoff");
    assert!(json_str.contains("\"factor\":2"));

    let deserialized: ConstantBackoffDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.definition.is_some());

    // Test exponential backoff with definition
    let exponential = ExponentialBackoffDefinition {
        definition: Some({
            let mut map = std::collections::HashMap::new();
            map.insert("factor".to_string(), json!(2));
            map.insert("maxDelay".to_string(), json!("PT30S"));
            map
        }),
    };
    let json_str =
        serde_json::to_string(&exponential).expect("Failed to serialize exponential backoff");
    assert!(json_str.contains("\"factor\":2"));
    assert!(json_str.contains("\"maxDelay\":\"PT30S\""));

    // Test linear backoff with increment and definition
    let linear = LinearBackoffDefinition {
        increment: Some(Duration::from_milliseconds(1000)),
        definition: Some({
            let mut map = std::collections::HashMap::new();
            map.insert("maxDelay".to_string(), json!("PT30S"));
            map
        }),
    };
    let json_str = serde_json::to_string(&linear).expect("Failed to serialize linear backoff");
    assert!(json_str.contains("PT1S") || json_str.contains("\"increment\""));
    assert!(json_str.contains("\"maxDelay\":\"PT30S\""));
}

#[test]
fn test_backoff_strong_typed_accessors() {
    let backoff = ExponentialBackoffDefinition::with_factor_and_max_delay(2.0, "PT30S");
    assert_eq!(backoff.factor(), Some(2.0));
    assert_eq!(backoff.max_delay(), Some("PT30S"));

    let constant = ConstantBackoffDefinition::with_delay("PT5S");
    assert_eq!(constant.delay(), Some("PT5S"));
}
