use serde_json::{json, Value};
use std::fmt;

/// Common fields shared by all workflow error types
#[derive(Debug, Clone)]
pub struct ErrorFields {
    pub message: String,
    pub task: String,
    pub instance: String,
    pub status: Option<Value>,
    pub title: Option<String>,
    pub detail: Option<String>,
    pub original_type: Option<String>,
}

impl ErrorFields {
    fn new(
        message: impl Into<String>,
        task: impl Into<String>,
        instance: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            task: task.into(),
            instance: instance.into(),
            status: None,
            title: None,
            detail: None,
            original_type: None,
        }
    }

    fn with_status(mut self, status: Option<Value>) -> Self {
        self.status = status;
        self
    }

    fn with_title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    fn with_detail(mut self, detail: Option<String>) -> Self {
        self.detail = detail;
        self
    }

    fn with_original_type(mut self, original_type: Option<String>) -> Self {
        self.original_type = original_type;
        self
    }

    fn instance_opt(&self) -> Option<&str> {
        if self.instance.is_empty() {
            None
        } else {
            Some(&self.instance)
        }
    }
}

/// Error kind discriminator for WorkflowError
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Validation,
    Expression,
    Runtime,
    Timeout,
    Communication,
    Authentication,
    Authorization,
    Configuration,
}

impl ErrorKind {
    /// Returns the short type name (e.g., "validation", "runtime")
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::Validation => "validation",
            ErrorKind::Expression => "expression",
            ErrorKind::Runtime => "runtime",
            ErrorKind::Timeout => "timeout",
            ErrorKind::Communication => "communication",
            ErrorKind::Authentication => "authentication",
            ErrorKind::Authorization => "authorization",
            ErrorKind::Configuration => "configuration",
        }
    }

    /// Returns the full error type URI per the Serverless Workflow spec
    pub fn type_uri(&self) -> &'static str {
        match self {
            ErrorKind::Validation => "https://serverlessworkflow.io/spec/1.0.0/errors/validation",
            ErrorKind::Expression => "https://serverlessworkflow.io/spec/1.0.0/errors/expression",
            ErrorKind::Runtime => "https://serverlessworkflow.io/spec/1.0.0/errors/runtime",
            ErrorKind::Timeout => "https://serverlessworkflow.io/spec/1.0.0/errors/timeout",
            ErrorKind::Communication => {
                "https://serverlessworkflow.io/spec/1.0.0/errors/communication"
            }
            ErrorKind::Authentication => {
                "https://serverlessworkflow.io/spec/1.0.0/errors/authentication"
            }
            ErrorKind::Authorization => {
                "https://serverlessworkflow.io/spec/1.0.0/errors/authorization"
            }
            ErrorKind::Configuration => {
                "https://serverlessworkflow.io/spec/1.0.0/errors/configuration"
            }
        }
    }

    /// Resolves an error type string to an ErrorKind.
    /// Matches both the full URI (from ErrorTypes constants) and short names (suffix after last '/').
    /// Returns ErrorKind::Runtime as the default for unknown types.
    pub fn from_type_str(error_type: &str) -> Self {
        const TYPE_MAP: &[(&str, ErrorKind)] = &[
            ("validation", ErrorKind::Validation),
            ("expression", ErrorKind::Expression),
            ("timeout", ErrorKind::Timeout),
            ("communication", ErrorKind::Communication),
            ("authentication", ErrorKind::Authentication),
            ("authorization", ErrorKind::Authorization),
            ("configuration", ErrorKind::Configuration),
        ];
        TYPE_MAP
            .iter()
            .find(|(suffix, _)| {
                error_type.ends_with(suffix)
                    && (error_type.len() == suffix.len()
                        || error_type
                            .as_bytes()
                            .get(error_type.len() - suffix.len() - 1)
                            == Some(&b'/'))
            })
            .map(|(_, kind)| *kind)
            .unwrap_or(ErrorKind::Runtime)
    }
}

/// Runtime error for the Serverless Workflow engine
#[derive(Debug, Clone)]
pub struct WorkflowError {
    kind: ErrorKind,
    fields: ErrorFields,
}

impl std::error::Error for WorkflowError {}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} error in task '{}': {}",
            self.kind.as_str(),
            self.fields.task,
            self.fields.message
        )
    }
}

impl WorkflowError {
    /// Returns the error kind
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns a reference to the error fields
    pub fn fields(&self) -> &ErrorFields {
        &self.fields
    }

    /// Creates a validation error
    pub fn validation(message: impl Into<String>, task: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Validation,
            fields: ErrorFields::new(message, task, ""),
        }
    }

    /// Creates an expression error
    pub fn expression(message: impl Into<String>, task: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Expression,
            fields: ErrorFields::new(message, task, ""),
        }
    }

    /// Creates a runtime error
    pub fn runtime(
        message: impl Into<String>,
        task: impl Into<String>,
        instance: impl Into<String>,
    ) -> Self {
        Self {
            kind: ErrorKind::Runtime,
            fields: ErrorFields::new(message, task, instance),
        }
    }

    /// Creates a runtime error without an instance (defaults to empty string)
    pub fn runtime_simple(message: impl Into<String>, task: impl Into<String>) -> Self {
        Self::runtime(message, task, "")
    }

    /// Creates a timeout error
    /// Per the Serverless Workflow spec, timeout errors have status 408
    pub fn timeout(message: impl Into<String>, task: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Timeout,
            fields: ErrorFields::new(message, task, "").with_status(Some(json!(408))),
        }
    }

    /// Creates a communication error
    pub fn communication(message: impl Into<String>, task: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Communication,
            fields: ErrorFields::new(message, task, ""),
        }
    }

    /// Creates a communication error with an HTTP status code
    pub fn communication_with_status(
        message: impl Into<String>,
        task: impl Into<String>,
        status_code: u16,
    ) -> Self {
        Self {
            kind: ErrorKind::Communication,
            fields: ErrorFields::new(message, task, "").with_status(Some(Value::from(status_code))),
        }
    }

    /// Creates a typed error from DSL error definition fields
    pub fn typed(
        error_type: &str,
        detail: String,
        task: String,
        instance: String,
        status: Option<Value>,
        title: Option<String>,
    ) -> Self {
        let details = if detail.is_empty() {
            None
        } else {
            Some(detail)
        };
        let fields = ErrorFields::new(details.clone().unwrap_or_default(), task, instance)
            .with_status(status)
            .with_title(title)
            .with_detail(details)
            .with_original_type(Some(error_type.to_string()));

        let kind = ErrorKind::from_type_str(error_type);

        Self { kind, fields }
    }

    /// Returns the error type as a full URI (prefers original type from DSL if available)
    pub fn error_type(&self) -> &str {
        self.fields
            .original_type
            .as_deref()
            .unwrap_or(self.kind.type_uri())
    }

    /// Returns the short error type name (last segment of URI)
    pub fn error_type_short(&self) -> &str {
        if let Some(ot) = &self.fields.original_type {
            if let Some(short) = ot.rsplit('/').next() {
                return short;
            }
        }
        self.kind.as_str()
    }

    /// Returns the task name associated with this error
    pub fn task(&self) -> &str {
        &self.fields.task
    }

    /// Returns the instance reference, if available
    pub fn instance(&self) -> Option<&str> {
        self.fields.instance_opt()
    }

    /// Returns the status code, if available
    pub fn status(&self) -> Option<&Value> {
        self.fields.status.as_ref()
    }

    /// Returns the title, if available
    pub fn title(&self) -> Option<&str> {
        self.fields.title.as_deref()
    }

    /// Returns the detail, if available
    pub fn detail(&self) -> Option<&str> {
        self.fields.detail.as_deref()
    }

    /// Converts the error to a JSON Value for use in expressions (e.g., $caughtError)
    pub fn to_value(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert(
            "type".to_string(),
            Value::String(self.error_type().to_string()),
        );
        if let Some(status) = self.status() {
            map.insert("status".to_string(), status.clone());
        }
        if let Some(title) = self.title() {
            map.insert("title".to_string(), Value::String(title.to_string()));
        }
        if let Some(detail) = self.detail() {
            map.insert("details".to_string(), Value::String(detail.to_string()));
        }
        if let Some(instance) = self.instance() {
            map.insert("instance".to_string(), Value::String(instance.to_string()));
        }
        Value::Object(map)
    }

    /// Sets the instance reference on the error if not already set
    pub fn with_instance(self, instance: impl Into<String>) -> Self {
        let new_instance = instance.into();
        let inst = if self.fields.instance.is_empty() || self.fields.instance == "/" {
            new_instance
        } else {
            self.fields.instance.clone()
        };

        Self {
            kind: self.kind,
            fields: ErrorFields {
                message: self.fields.message,
                task: self.fields.task,
                instance: inst,
                status: self.fields.status,
                title: self.fields.title,
                detail: self.fields.detail,
                original_type: self.fields.original_type,
            },
        }
    }
}

/// Result type alias for workflow operations
pub type WorkflowResult<T> = Result<T, WorkflowError>;

/// Serializes a value to JSON, mapping serialization errors to WorkflowError::runtime.
/// This is a common pattern used across task runners.
pub fn serialize_to_value<T: serde::Serialize>(
    value: &T,
    label: &str,
    task_name: &str,
) -> WorkflowResult<Value> {
    serde_json::to_value(value).map_err(|e| {
        WorkflowError::runtime(
            format!("failed to serialize {}: {}", label, e),
            task_name,
            "",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_type_validation() {
        let err = WorkflowError::validation("invalid input", "task1");
        assert_eq!(err.error_type_short(), "validation");
        assert!(err.error_type().ends_with("/validation"));
        assert_eq!(err.task(), "task1");
    }

    #[test]
    fn test_error_type_expression() {
        let err = WorkflowError::expression("bad jq", "task2");
        assert_eq!(err.error_type_short(), "expression");
    }

    #[test]
    fn test_error_type_runtime() {
        let err = WorkflowError::runtime("something failed", "task3", "/ref");
        assert_eq!(err.error_type_short(), "runtime");
        assert_eq!(err.instance(), Some("/ref"));
    }

    #[test]
    fn test_error_type_timeout() {
        let err = WorkflowError::timeout("timed out", "task4");
        assert_eq!(err.error_type_short(), "timeout");
        assert!(err.instance().is_none());
    }

    #[test]
    fn test_error_type_communication() {
        let err = WorkflowError::communication("connection refused", "task5");
        assert_eq!(err.error_type_short(), "communication");
    }

    #[test]
    fn test_error_with_instance() {
        let err = WorkflowError::runtime("invalid", "task1", "").with_instance("/ref/task1");
        assert_eq!(err.error_type_short(), "runtime");
        assert_eq!(err.instance(), Some("/ref/task1"));
    }

    #[test]
    fn test_error_with_instance_preserves_type() {
        let err = WorkflowError::timeout("timed out", "task1").with_instance("/ref/task1");
        assert_eq!(err.error_type_short(), "timeout");
        assert_eq!(err.instance(), Some("/ref/task1"));
    }

    #[test]
    fn test_error_task_name() {
        let err = WorkflowError::timeout("timeout", "myTask");
        assert_eq!(err.task(), "myTask");
    }

    #[test]
    fn test_error_display() {
        let err = WorkflowError::validation("bad input", "task1");
        let msg = format!("{}", err);
        assert!(msg.contains("bad input"));
        assert!(msg.contains("task1"));
    }

    #[test]
    fn test_error_typed_with_status() {
        let err = WorkflowError::typed(
            "https://serverlessworkflow.io/spec/1.0.0/errors/transient",
            "Something went wrong".to_string(),
            "testTask".to_string(),
            "/do/0/testTask".to_string(),
            Some(Value::from(503)),
            Some("Transient Error".to_string()),
        );
        assert_eq!(err.error_type_short(), "transient");
        assert_eq!(err.status(), Some(&Value::from(503)));
        assert_eq!(err.title(), Some("Transient Error"));
        assert_eq!(err.detail(), Some("Something went wrong"));
    }

    #[test]
    fn test_error_to_value() {
        let err = WorkflowError::typed(
            "https://serverlessworkflow.io/spec/1.0.0/errors/authentication",
            "Auth failed".to_string(),
            "authTask".to_string(),
            "".to_string(),
            Some(Value::from(401)),
            Some("Auth Error".to_string()),
        );
        let val = err.to_value();
        assert_eq!(
            val["type"],
            "https://serverlessworkflow.io/spec/1.0.0/errors/authentication"
        );
        assert_eq!(val["status"], 401);
        assert_eq!(val["title"], "Auth Error");
        assert_eq!(val["details"], "Auth failed");
    }

    #[test]
    fn test_error_kind() {
        let err = WorkflowError::timeout("timed out", "task1");
        assert_eq!(err.kind(), ErrorKind::Timeout);
    }
}
