use serde::{Deserialize, Serialize};
use serde_json::Value;

string_constants! {
    /// Standard error type URIs based on the Serverless Workflow specification
    ErrorTypes {
        CONFIGURATION => "https://serverlessworkflow.io/spec/1.0.0/errors/configuration",
        VALIDATION => "https://serverlessworkflow.io/spec/1.0.0/errors/validation",
        EXPRESSION => "https://serverlessworkflow.io/spec/1.0.0/errors/expression",
        AUTHENTICATION => "https://serverlessworkflow.io/spec/1.0.0/errors/authentication",
        AUTHORIZATION => "https://serverlessworkflow.io/spec/1.0.0/errors/authorization",
        TIMEOUT => "https://serverlessworkflow.io/spec/1.0.0/errors/timeout",
        COMMUNICATION => "https://serverlessworkflow.io/spec/1.0.0/errors/communication",
        RUNTIME => "https://serverlessworkflow.io/spec/1.0.0/errors/runtime",
    }
}

/// Represents the type of an error, which can be either a URI template or a runtime expression.
///
/// Runtime expressions are detected by the `${` prefix in the string value.
/// Since serde(untagged) cannot distinguish between the two forms at deserialization,
/// this is stored as a simple newtype wrapper rather than an enum.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ErrorType(String);

impl ErrorType {
    /// Creates a new URI template error type
    pub fn uri_template(template: &str) -> Self {
        ErrorType(template.to_string())
    }

    /// Creates a new runtime expression error type
    pub fn runtime_expression(expression: &str) -> Self {
        ErrorType(expression.to_string())
    }

    /// Checks if this is a runtime expression (starts with '${')
    pub fn is_runtime_expression(&self) -> bool {
        self.0.starts_with("${")
    }

    /// Gets the string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ErrorType {
    fn from(s: String) -> Self {
        ErrorType(s)
    }
}

impl From<&str> for ErrorType {
    fn from(s: &str) -> Self {
        ErrorType(s.to_string())
    }
}

/// Represents the definition an error to raise
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorDefinition {
    /// Gets/sets an uri that reference the type of the described error
    #[serde(rename = "type")]
    pub type_: ErrorType,

    /// Gets/sets a short, human-readable summary of the error type.It SHOULD NOT change from occurrence to occurrence of the error, except for purposes of localization
    #[serde(rename = "title", skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,

    /// Gets/sets the status code produced by the described error
    #[serde(rename = "status")]
    pub status: Value,

    /// Gets/sets a human-readable explanation specific to this occurrence of the error.
    #[serde(rename = "detail", skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// Gets/sets a URI reference that identifies the specific occurrence of the error.
    /// It may or may not yield further information if dereferenced.
    #[serde(rename = "instance", skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}
macro_rules! define_error_type {
    ($factory:ident, $is:ident, $const:ident, $title:literal, $status:expr) => {
        #[doc = concat!("Creates a new ", stringify!($factory))]
        pub fn $factory(detail: Option<String>, instance: Option<String>) -> Self {
            Self::new(
                ErrorTypes::$const,
                $title,
                serde_json::json!($status),
                detail,
                instance,
            )
        }

        #[doc = concat!("Checks if this error is a ", stringify!($factory))]
        pub fn $is(&self) -> bool {
            self.type_.as_str() == ErrorTypes::$const
        }
    };
}

impl ErrorDefinition {
    /// Initializes a new ErrorDefinition
    pub fn new(
        type_: &str,
        title: &str,
        status: Value,
        detail: Option<String>,
        instance: Option<String>,
    ) -> Self {
        Self {
            type_: ErrorType::uri_template(type_),
            title: Some(title.to_string()),
            status,
            detail,
            instance,
        }
    }

    define_error_type!(
        configuration_error,
        is_configuration_error,
        CONFIGURATION,
        "Configuration Error",
        400
    );
    define_error_type!(
        validation_error,
        is_validation_error,
        VALIDATION,
        "Validation Error",
        400
    );
    define_error_type!(
        expression_error,
        is_expression_error,
        EXPRESSION,
        "Expression Error",
        400
    );
    define_error_type!(
        authentication_error,
        is_authentication_error,
        AUTHENTICATION,
        "Authentication Error",
        401
    );
    define_error_type!(
        authorization_error,
        is_authorization_error,
        AUTHORIZATION,
        "Authorization Error",
        403
    );
    define_error_type!(
        timeout_error,
        is_timeout_error,
        TIMEOUT,
        "Timeout Error",
        408
    );
    define_error_type!(
        communication_error,
        is_communication_error,
        COMMUNICATION,
        "Communication Error",
        500
    );
    define_error_type!(
        runtime_error,
        is_runtime_error,
        RUNTIME,
        "Runtime Error",
        500
    );
}

define_one_of_or_reference!(
    /// A error definition or a reference to one
    OneOfErrorDefinitionOrReference, Error(ErrorDefinition)
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_error_definition_new() {
        let err = ErrorDefinition::new(
            ErrorTypes::RUNTIME,
            "Runtime Error",
            json!(500),
            Some("Something went wrong".to_string()),
            Some("/task/1".to_string()),
        );
        assert_eq!(err.type_.as_str(), ErrorTypes::RUNTIME);
        assert_eq!(err.title, Some("Runtime Error".to_string()));
        assert_eq!(err.status, json!(500));
        assert_eq!(err.detail, Some("Something went wrong".to_string()));
        assert_eq!(err.instance, Some("/task/1".to_string()));
    }

    #[test]
    fn test_error_type_check_methods() {
        let err = ErrorDefinition::validation_error(None, None);
        assert!(err.is_validation_error());
        assert!(!err.is_runtime_error());

        let err = ErrorDefinition::runtime_error(None, None);
        assert!(err.is_runtime_error());
        assert!(!err.is_communication_error());

        let err = ErrorDefinition::authentication_error(None, None);
        assert!(err.is_authentication_error());

        let err = ErrorDefinition::authorization_error(None, None);
        assert!(err.is_authorization_error());

        let err = ErrorDefinition::timeout_error(None, None);
        assert!(err.is_timeout_error());

        let err = ErrorDefinition::communication_error(None, None);
        assert!(err.is_communication_error());

        let err = ErrorDefinition::configuration_error(None, None);
        assert!(err.is_configuration_error());

        let err = ErrorDefinition::expression_error(None, None);
        assert!(err.is_expression_error());
    }

    #[test]
    fn test_error_type_enum() {
        let uri =
            ErrorType::uri_template("https://serverlessworkflow.io/spec/1.0.0/errors/runtime");
        assert_eq!(
            uri.as_str(),
            "https://serverlessworkflow.io/spec/1.0.0/errors/runtime"
        );
        assert!(!uri.is_runtime_expression());

        let expr = ErrorType::runtime_expression("${ .errorType }");
        assert_eq!(expr.as_str(), "${ .errorType }");
        assert!(expr.is_runtime_expression());
    }

    #[test]
    fn test_error_definition_serialize() {
        let err = ErrorDefinition::new(
            ErrorTypes::COMMUNICATION,
            "Communication Error",
            json!(500),
            None,
            None,
        );
        let json_str = serde_json::to_string(&err).unwrap();
        assert!(json_str.contains(
            "\"type\":\"https://serverlessworkflow.io/spec/1.0.0/errors/communication\""
        ));
        assert!(json_str.contains("\"title\":\"Communication Error\""));
        assert!(json_str.contains("\"status\":500"));
        assert!(!json_str.contains("detail"));
        assert!(!json_str.contains("instance"));
    }

    #[test]
    fn test_error_definition_deserialize() {
        let json = r#"{
            "type": "https://serverlessworkflow.io/spec/1.0.0/errors/runtime",
            "title": "Runtime Error",
            "status": 500,
            "detail": "Something failed",
            "instance": "/task/step1"
        }"#;
        let err: ErrorDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(
            err.type_.as_str(),
            "https://serverlessworkflow.io/spec/1.0.0/errors/runtime"
        );
        assert_eq!(err.title, Some("Runtime Error".to_string()));
        assert_eq!(err.detail, Some("Something failed".to_string()));
    }

    #[test]
    fn test_oneof_error_reference_deserialize() {
        let json = r#""someErrorRef""#;
        let oneof: OneOfErrorDefinitionOrReference = serde_json::from_str(json).unwrap();
        match oneof {
            OneOfErrorDefinitionOrReference::Reference(name) => {
                assert_eq!(name, "someErrorRef");
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn test_oneof_error_inline_deserialize() {
        let json = r#"{
            "type": "https://serverlessworkflow.io/spec/1.0.0/errors/timeout",
            "title": "Timeout Error",
            "status": 408
        }"#;
        let oneof: OneOfErrorDefinitionOrReference = serde_json::from_str(json).unwrap();
        match oneof {
            OneOfErrorDefinitionOrReference::Error(err) => {
                assert_eq!(
                    err.type_.as_str(),
                    "https://serverlessworkflow.io/spec/1.0.0/errors/timeout"
                );
            }
            _ => panic!("Expected Error variant"),
        }
    }

    // Additional tests matching Go SDK patterns

    #[test]
    fn test_error_definition_roundtrip() {
        let json = r#"{
            "type": "https://serverlessworkflow.io/spec/1.0.0/errors/communication",
            "title": "Communication Error",
            "status": 500,
            "detail": "Connection refused",
            "instance": "/task/step2"
        }"#;
        let err: ErrorDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&err).unwrap();
        let deserialized: ErrorDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(err, deserialized);
    }

    #[test]
    fn test_oneof_error_reference_roundtrip() {
        let oneof = OneOfErrorDefinitionOrReference::Reference("myErrorRef".to_string());
        let serialized = serde_json::to_string(&oneof).unwrap();
        assert_eq!(serialized, r#""myErrorRef""#);
        let deserialized: OneOfErrorDefinitionOrReference =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(oneof, deserialized);
    }

    #[test]
    fn test_oneof_error_inline_roundtrip() {
        let json = r#"{
            "type": "https://serverlessworkflow.io/spec/1.0.0/errors/authentication",
            "title": "Auth Error",
            "status": 401
        }"#;
        let oneof: OneOfErrorDefinitionOrReference = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&oneof).unwrap();
        let deserialized: OneOfErrorDefinitionOrReference =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(oneof, deserialized);
    }

    #[test]
    fn test_error_type_runtime_expression_detection() {
        // URI-based error types should not be detected as runtime expressions
        let uri_type =
            ErrorType::uri_template("https://serverlessworkflow.io/spec/1.0.0/errors/runtime");
        assert!(!uri_type.is_runtime_expression());

        // Runtime expression types should be detected
        let expr_type = ErrorType::runtime_expression("${ .errorType }");
        assert!(expr_type.is_runtime_expression());

        // A URI that starts with ${ should be detected as runtime expression
        let uri_with_expr = ErrorType::uri_template("${ .dynamicError }");
        assert!(uri_with_expr.is_runtime_expression());
    }

    #[test]
    fn test_error_definition_with_runtime_type() {
        // Error with runtime expression as type
        let json = r#"{
            "type": "${ .error.type }",
            "title": "Dynamic Error",
            "status": 500
        }"#;
        let err: ErrorDefinition = serde_json::from_str(json).unwrap();
        assert!(err.type_.is_runtime_expression());
    }

    #[test]
    fn test_standard_error_factory_methods() {
        // Test all standard error factory methods produce correct types
        let config = ErrorDefinition::configuration_error(Some("bad config".to_string()), None);
        assert!(config.is_configuration_error());
        assert_eq!(config.status, json!(400));

        let validation = ErrorDefinition::validation_error(None, None);
        assert!(validation.is_validation_error());
        assert_eq!(validation.status, json!(400));

        let expr = ErrorDefinition::expression_error(None, None);
        assert!(expr.is_expression_error());
        assert_eq!(expr.status, json!(400));

        let authn = ErrorDefinition::authentication_error(None, None);
        assert!(authn.is_authentication_error());
        assert_eq!(authn.status, json!(401));

        let authz = ErrorDefinition::authorization_error(None, None);
        assert!(authz.is_authorization_error());
        assert_eq!(authz.status, json!(403));

        let timeout = ErrorDefinition::timeout_error(None, None);
        assert!(timeout.is_timeout_error());
        assert_eq!(timeout.status, json!(408));

        let comm = ErrorDefinition::communication_error(None, None);
        assert!(comm.is_communication_error());
        assert_eq!(comm.status, json!(500));

        let runtime = ErrorDefinition::runtime_error(None, None);
        assert!(runtime.is_runtime_error());
        assert_eq!(runtime.status, json!(500));
    }

    #[test]
    fn test_error_definition_without_optional_title() {
        // Matches Go SDK pattern where title is optional (omitempty)
        let json = r#"{
            "type": "https://serverlessworkflow.io/spec/1.0.0/errors/timeout",
            "status": 408,
            "detail": "Request took too long"
        }"#;
        let err: ErrorDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(
            err.type_.as_str(),
            "https://serverlessworkflow.io/spec/1.0.0/errors/timeout"
        );
        assert_eq!(err.title, None);
        assert_eq!(err.status, json!(408));
        assert_eq!(err.detail, Some("Request took too long".to_string()));
    }

    #[test]
    fn test_error_definition_serialize_skips_none_title() {
        let err = ErrorDefinition {
            type_: ErrorType::uri_template(
                "https://serverlessworkflow.io/spec/1.0.0/errors/timeout",
            ),
            title: None,
            status: json!(408),
            detail: Some("Timed out".to_string()),
            instance: None,
        };
        let json_str = serde_json::to_string(&err).unwrap();
        assert!(!json_str.contains("title"));
        assert!(json_str.contains("\"detail\":\"Timed out\""));
    }
}
