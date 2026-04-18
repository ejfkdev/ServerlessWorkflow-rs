use super::*;

#[test]
fn test_error_definition_with_status() {
    // Test ErrorDefinition with numeric status
    use serverless_workflow_core::models::error::ErrorDefinition;

    let error_json = json!({
        "type": "http://example.com/error",
        "status": 404,
        "title": "Not Found"
    });
    let result: Result<ErrorDefinition, _> = serde_json::from_value(error_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize error definition: {:?}",
        result.err()
    );
    let error_def = result.unwrap();
    assert_eq!(error_def.type_.as_str(), "http://example.com/error");
    assert_eq!(error_def.title, Some("Not Found".to_string()));
    assert_eq!(error_def.status, serde_json::json!(404));
}

#[test]
fn test_error_definition_with_instance() {
    // Test ErrorDefinition with instance
    let error_json = json!({
        "type": "http://example.com/error",
        "status": 500,
        "title": "Internal Server Error",
        "detail": "An error occurred",
        "instance": "/errors/12345"
    });
    let result: Result<ErrorDefinition, _> = serde_json::from_value(error_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize error definition: {:?}",
        result.err()
    );
    let error_def = result.unwrap();
    assert_eq!(error_def.instance, Some("/errors/12345".to_string()));
}

#[test]
fn test_error_definition() {
    use serverless_workflow_core::models::error::ErrorDefinition;
    let error = ErrorDefinition::new(
        "https://example.com/errors",
        "Bad Request",
        serde_json::json!(400),
        Some("Error occurred".to_string()),
        None,
    );
    let json_str = serde_json::to_string(&error).expect("Failed to serialize");
    assert!(json_str.contains("Bad Request"));
    assert!(json_str.contains("Error occurred"));
}

#[test]
fn test_error_definition_roundtrip() {
    use serverless_workflow_core::models::error::ErrorDefinition;
    let error = ErrorDefinition::new(
        "https://example.com/errors",
        "Not Found",
        serde_json::json!(404),
        Some("Resource not found".to_string()),
        Some("/instance/123".to_string()),
    );
    let json_str = serde_json::to_string(&error).expect("Failed to serialize");
    let deserialized: ErrorDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(error.title, deserialized.title);
    assert_eq!(error.detail, deserialized.detail);
}

#[test]
fn test_error_type_constants() {
    // Test ErrorTypes constants
    use serverless_workflow_core::models::error::ErrorTypes;

    assert_eq!(
        ErrorTypes::CONFIGURATION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/configuration"
    );
    assert_eq!(
        ErrorTypes::VALIDATION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/validation"
    );
    assert_eq!(
        ErrorTypes::EXPRESSION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/expression"
    );
    assert_eq!(
        ErrorTypes::AUTHENTICATION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/authentication"
    );
    assert_eq!(
        ErrorTypes::AUTHORIZATION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/authorization"
    );
    assert_eq!(
        ErrorTypes::TIMEOUT,
        "https://serverlessworkflow.io/spec/1.0.0/errors/timeout"
    );
    assert_eq!(
        ErrorTypes::COMMUNICATION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/communication"
    );
    assert_eq!(
        ErrorTypes::RUNTIME,
        "https://serverlessworkflow.io/spec/1.0.0/errors/runtime"
    );
}

#[test]
fn test_error_convenience_constructors() {
    // Test ErrorDefinition convenience constructors
    use serverless_workflow_core::models::error::ErrorDefinition;

    let config_err = ErrorDefinition::configuration_error(
        Some("config detail".to_string()),
        Some("/instance/1".to_string()),
    );
    assert!(config_err.is_configuration_error());
    assert_eq!(config_err.title, Some("Configuration Error".to_string()));

    let auth_err = ErrorDefinition::authentication_error(None, None);
    assert!(auth_err.is_authentication_error());
    assert_eq!(auth_err.status, serde_json::json!(401));

    let timeout_err = ErrorDefinition::timeout_error(Some("timeout".to_string()), None);
    assert!(timeout_err.is_timeout_error());
    assert_eq!(timeout_err.status, serde_json::json!(408));
}

#[test]
fn test_error_classification_functions() {
    // Test ErrorDefinition classification functions
    use serverless_workflow_core::models::error::ErrorDefinition;

    let validation_err =
        ErrorDefinition::validation_error(Some("validation failed".to_string()), None);
    assert!(validation_err.is_validation_error());
    assert!(!validation_err.is_authentication_error());
    assert!(!validation_err.is_timeout_error());

    let runtime_err = ErrorDefinition::runtime_error(None, None);
    assert!(runtime_err.is_runtime_error());
    assert!(!runtime_err.is_configuration_error());
}

#[test]
fn test_error_definition_reference() {
    // Test ErrorDefinition with reference (using OneOfErrorDefinitionOrReference::Reference)
    use serverless_workflow_core::models::error::OneOfErrorDefinitionOrReference;

    let error_ref = OneOfErrorDefinitionOrReference::Reference("myError".to_string());
    let json_str = serde_json::to_string(&error_ref).expect("Failed to serialize error reference");
    assert!(json_str.contains("myError"));

    let deserialized: OneOfErrorDefinitionOrReference =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized {
        OneOfErrorDefinitionOrReference::Reference(s) => {
            assert_eq!(s, "myError");
        }
        _ => panic!("Expected Reference variant"),
    }
}

#[test]
fn test_error_definition_with_string_status() {
    // Test ErrorDefinition with string status
    use serverless_workflow_core::models::error::ErrorDefinition;

    let error_json = json!({
        "type": "https://example.com/errors",
        "status": "500",
        "title": "Server Error"
    });
    let result: Result<ErrorDefinition, _> = serde_json::from_value(error_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize error with string status: {:?}",
        result.err()
    );
}

#[test]
fn test_error_filter_with_status() {
    // Test error filter with status as string
    let error_filter_json = json!({
        "with": {
            "type": "https://example.com/errors",
            "status": 500,
            "title": "Server Error"
        },
        "use": {
            "set": {
                "error": "true"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ErrorFilterDefinition, _> =
        serde_json::from_value(error_filter_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize error filter: {:?}",
        result.err()
    );
}

#[test]
fn test_error_catcher_with_retry() {
    // Test error catcher with retry policy
    let error_catcher_json = json!({
        "errors": {
            "with": {
                "type": "https://example.com/errors/connection"
            }
        },
        "retry": {
            "delay": {
                "seconds": 1
            },
            "backoff": {
                "exponential": {}
            },
            "limit": {
                "attempt": {
                    "count": 5
                }
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ErrorCatcherDefinition, _> =
        serde_json::from_value(error_catcher_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize error catcher: {:?}",
        result.err()
    );
}

#[test]
fn test_error_definition_is_type_checks() {
    let config_error = ErrorDefinition::configuration_error(Some("bad config".to_string()), None);
    assert!(config_error.is_configuration_error());
    assert!(!config_error.is_runtime_error());

    let auth_error = ErrorDefinition::authentication_error(None, None);
    assert!(auth_error.is_authentication_error());

    let timeout_error = ErrorDefinition::timeout_error(None, None);
    assert!(timeout_error.is_timeout_error());

    let runtime_error = ErrorDefinition::runtime_error(Some("crash".to_string()), None);
    assert!(runtime_error.is_runtime_error());
}

#[test]
fn test_error_filter_properties() {
    use serverless_workflow_core::models::task::ErrorFilterProperties;
    let props = ErrorFilterProperties {
        type_: Some("https://serverlessworkflow.io/spec/1.0.0/errors/communication".to_string()),
        status: Some(json!(500)),
        instance: None,
        title: None,
        detail: None,
    };
    assert_eq!(
        props.type_,
        Some("https://serverlessworkflow.io/spec/1.0.0/errors/communication".to_string())
    );
    assert_eq!(props.status, Some(json!(500)));
}

#[test]
fn test_error_filter_definition_serde() {
    use serverless_workflow_core::models::task::ErrorFilterDefinition;
    let json_str = r#"{"with": {"type": "https://serverlessworkflow.io/spec/1.0.0/errors/communication", "status": 500}}"#;
    let filter: ErrorFilterDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize");
    assert!(filter.with.is_some());
    let props = filter.with.unwrap();
    assert_eq!(
        props.type_,
        Some("https://serverlessworkflow.io/spec/1.0.0/errors/communication".to_string())
    );
    assert_eq!(props.status, Some(json!(500)));
}

#[test]
fn test_error_type_enum() {
    use serverless_workflow_core::models::error::ErrorType;
    let uri = ErrorType::uri_template("https://serverlessworkflow.io/spec/1.0.0/errors/timeout");
    assert_eq!(
        uri.as_str(),
        "https://serverlessworkflow.io/spec/1.0.0/errors/timeout"
    );
    assert!(!uri.is_runtime_expression());

    let expr = ErrorType::runtime_expression("${ .error.type }");
    assert_eq!(expr.as_str(), "${ .error.type }");
    assert!(expr.is_runtime_expression());
}

#[test]
fn test_error_definition_with_error_type() {
    let err = ErrorDefinition::timeout_error(Some("timed out".to_string()), None);
    assert!(err.is_timeout_error());
    assert_eq!(
        err.type_.as_str(),
        "https://serverlessworkflow.io/spec/1.0.0/errors/timeout"
    );
}

#[test]
fn test_error_type_serde() {
    use serverless_workflow_core::models::error::ErrorType;
    // URI template
    let json_str = r#""https://serverlessworkflow.io/spec/1.0.0/errors/timeout""#;
    let et: ErrorType = serde_json::from_str(json_str).expect("Failed to deserialize");
    assert_eq!(
        et.as_str(),
        "https://serverlessworkflow.io/spec/1.0.0/errors/timeout"
    );
}
