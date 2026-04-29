use crate::models::authentication::*;
use crate::models::catalog::*;
use crate::models::duration::*;
use crate::models::error::*;
use crate::models::event::*;
use crate::models::extension::*;
use crate::models::input::*;
use crate::models::map::*;
use crate::models::output::*;
use crate::models::retry::*;
use crate::models::schema::SchemaDefinition;
use crate::models::task::*;
use crate::models::timeout::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Gets the namespace to use by default for workflow definitions
pub const DEFAULT_NAMESPACE: &str = "default";
// Provides the default namespace if not specified during deserialization
fn default_namespace() -> String {
    DEFAULT_NAMESPACE.to_string()
}

/// Gets the latest ServerlessWorkflow DSL version to use by default for workflow definitions
pub const LATEST_DSL_VERSION: &str = "1.0.1";
// Provides the latest ServerlessWorkflow DSL version
fn default_dsl_version() -> String {
    LATEST_DSL_VERSION.to_string()
}

// Provides the default runtime expression language
fn default_runtime_expression_language() -> String {
    RuntimeExpressionLanguage::JQ.to_string()
}

string_constants! {
    /// Enumerates all supported runtime expression languages
    RuntimeExpressionLanguage {
        JQ => "jq",
        JAVASCRIPT => "js",
    }
}

/// Represents the definition of a workflow
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Gets/sets an object used to document the defined workflow
    pub document: WorkflowDefinitionMetadata,

    /// Gets/sets the workflow's input definition, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<InputDataModelDefinition>,

    /// Gets/sets a collection that contains reusable components for the workflow definition
    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_: Option<ComponentDefinitionCollection>,

    /// Gets/sets the workflow's timeout, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<OneOfTimeoutDefinitionOrReference>,

    /// Gets/sets the workflow's output definition, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputDataModelDefinition>,

    /// Gets/sets the workflow's context data definition, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextDataModelDefinition>,

    /// Gets/sets the definition of the workflow's schedule, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<WorkflowScheduleDefinition>,

    /// Gets/sets the configuration of how the runtime expressions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluate: Option<RuntimeExpressionEvaluationConfiguration>,

    /// Gets/sets a name/value mapping of the tasks to perform
    #[serde(rename = "do")]
    pub do_: Map<String, TaskDefinition>,

    /// Gets/sets a key/value mapping, if any, of additional information associated with the workflow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}
impl WorkflowDefinition {
    /// Initializes a new workflow definition with the given document metadata
    pub fn new(document: WorkflowDefinitionMetadata) -> Self {
        Self {
            document,
            ..Default::default()
        }
    }
}

/// Represents the metadata of a workflow, including its name, version, and description.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDefinitionMetadata {
    /// Gets/sets the version of the DSL used to define the workflow
    pub dsl: String,

    /// Gets/sets the workflow's namespace
    ///
    /// Defaults to [`DEFAULT_NAMESPACE`] if not specified.
    #[serde(default = "default_namespace")]
    pub namespace: String,

    /// Gets/sets the workflow's name
    pub name: String,

    /// Gets/sets the workflow's semantic version
    pub version: String,

    /// Gets/sets the workflow's title, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Gets/sets the workflow's Markdown summary, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Gets/sets a key/value mapping of the workflow's tags, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,

    /// Gets/sets a key/value mapping, if any, of additional information associated with the workflow document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}
impl WorkflowDefinitionMetadata {
    // Initializes a new workflow definition metadata
    pub fn new(
        namespace: &str,
        name: &str,
        version: &str,
        title: Option<String>,
        summary: Option<String>,
        tags: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            dsl: default_dsl_version(),
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            version: version.to_owned(),
            title,
            summary,
            tags,
            metadata: None,
        }
    }
}

/// Represents the definition of a workflow's context data
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextDataModelDefinition {
    /// Gets/sets the schema, if any, that defines and describes the context data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaDefinition>,

    /// Gets/sets a runtime expression, if any, used to set the content of the workflow context
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    pub as_: Option<serde_json::Value>,
}

/// Represents the definition of a workflow's schedule
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowScheduleDefinition {
    /// Gets/sets an object used to document the defined workflow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub every: Option<Duration>,

    /// Gets/sets the schedule using a CRON expression, e.g., '0 0 * * *' for daily at midnight.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron: Option<String>,

    /// Gets/sets a delay duration, if any, that the workflow must wait before starting again after it completes. In other words, when this workflow completes, it should run again after the specified amount of time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<Duration>,

    /// Gets/sets the events that trigger the workflow execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<EventConsumptionStrategyDefinition>,
}

string_constants! {
    /// Enumerates all supported runtime expression evaluation modes
    RuntimeExpressionEvaluationMode {
        STRICT => "strict",
        LOOSE => "loose",
    }
}

string_constants! {
    /// Enumerates all supported runtime expression argument names
    RuntimeExpressions {
        RUNTIME => "runtime",
        WORKFLOW => "workflow",
        CONTEXT => "context",
        ITEM => "item",
        INDEX => "index",
        OUTPUT => "output",
        SECRET => "secret",
        TASK => "task",
        INPUT => "input",
        ERROR => "error",
        AUTHORIZATION => "authorization",
    }
}

/// Represents an object used to configure the workflow's runtime expression evaluation
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeExpressionEvaluationConfiguration {
    /// Gets/sets the language used for writing runtime expressions
    #[serde(default = "default_runtime_expression_language")]
    pub language: String,

    /// Gets/sets the evaluation mode used for runtime expressions. Defaults to 'loose'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// Represents a collection of workflow components
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDefinitionCollection {
    /// Gets/sets a name/value mapping of the workflow's reusable authentication policies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentications: Option<HashMap<String, ReferenceableAuthenticationPolicy>>,

    /// Gets/sets a name/value mapping of the catalogs, if any, from which to import reusable components used within the workflow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalogs: Option<HashMap<String, CatalogDefinition>>,

    /// Gets/sets a name/value mapping of the workflow's errors, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<HashMap<String, ErrorDefinition>>,

    /// Gets/sets a list containing the workflow's extensions, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<HashMap<String, ExtensionDefinition>>>,

    /// Gets/sets a name/value mapping of the workflow's reusable functions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<HashMap<String, TaskDefinition>>,

    /// Gets/sets a name/value mapping of the workflow's reusable retry policies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<HashMap<String, RetryPolicyDefinition>>,

    /// Gets/sets a list containing the workflow's secrets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<String>>,

    /// Gets/sets a name/value mapping of the workflow's reusable timeouts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<HashMap<String, TimeoutDefinition>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_metadata_deserialize() {
        let json = r#"{
            "dsl": "1.0.0",
            "namespace": "test-ns",
            "name": "test-workflow",
            "version": "1.0.0",
            "title": "Test Workflow",
            "summary": "A test workflow"
        }"#;
        let meta: WorkflowDefinitionMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.dsl, "1.0.0");
        assert_eq!(meta.namespace, "test-ns");
        assert_eq!(meta.name, "test-workflow");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.title, Some("Test Workflow".to_string()));
    }

    #[test]
    fn test_workflow_metadata_defaults() {
        let json = r#"{
            "dsl": "1.0.0",
            "name": "minimal",
            "version": "0.1.0"
        }"#;
        let meta: WorkflowDefinitionMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.dsl, "1.0.0");
        assert_eq!(meta.namespace, "default");
    }

    #[test]
    fn test_workflow_metadata_roundtrip() {
        let json = r#"{
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "myflow",
            "version": "1.0.0"
        }"#;
        let meta: WorkflowDefinitionMetadata = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&meta).unwrap();
        let deserialized: WorkflowDefinitionMetadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(meta, deserialized);
    }

    #[test]
    fn test_schedule_cron() {
        let json = r#"{"cron": "0 0 * * *"}"#;
        let schedule: WorkflowScheduleDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(schedule.cron, Some("0 0 * * *".to_string()));
        assert!(schedule.every.is_none());
    }

    #[test]
    fn test_schedule_every() {
        let json = r#"{"every": {"minutes": 30}}"#;
        let schedule: WorkflowScheduleDefinition = serde_json::from_str(json).unwrap();
        assert!(schedule.every.is_some());
        assert!(schedule.cron.is_none());
    }

    #[test]
    fn test_schedule_roundtrip() {
        let json = r#"{"cron": "0 0 * * *"}"#;
        let schedule: WorkflowScheduleDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&schedule).unwrap();
        let deserialized: WorkflowScheduleDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(schedule, deserialized);
    }

    #[test]
    fn test_component_collection_deserialize() {
        let json = r#"{
            "secrets": ["dbPassword", "apiKey"],
            "authentications": {
                "basicAuth": {"basic": {"username": "admin", "password": "secret"}}
            }
        }"#;
        let components: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        assert!(components.secrets.is_some());
        assert!(components.authentications.is_some());
    }

    #[test]
    fn test_component_collection_roundtrip() {
        let json = r#"{
            "secrets": ["dbPassword"]
        }"#;
        let components: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&components).unwrap();
        let deserialized: ComponentDefinitionCollection =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(components, deserialized);
    }

    #[test]
    fn test_context_data_model_deserialize() {
        let json = r#"{
            "schema": {"format": "json", "document": {"type": "object"}},
            "as": {"sessionId": "abc123"}
        }"#;
        let context: ContextDataModelDefinition = serde_json::from_str(json).unwrap();
        assert!(context.schema.is_some());
        assert!(context.as_.is_some());
    }

    #[test]
    fn test_runtime_expression_config() {
        let json = r#"{
            "language": "jq",
            "mode": "strict"
        }"#;
        let config: RuntimeExpressionEvaluationConfiguration = serde_json::from_str(json).unwrap();
        assert_eq!(config.language, "jq");
        assert_eq!(config.mode, Some("strict".to_string()));
    }

    #[test]
    fn test_runtime_expression_config_defaults() {
        let json = r#"{}"#;
        let config: RuntimeExpressionEvaluationConfiguration = serde_json::from_str(json).unwrap();
        assert_eq!(config.language, "jq");
        assert!(config.mode.is_none());
    }

    // Additional tests matching Go SDK's workflow_test.go Use definition tests

    #[test]
    fn test_use_definition_comprehensive_deserialize() {
        let json = r#"{
            "secrets": ["secret1", "secret2"],
            "timeouts": {"timeout1": {"after": "PT1M"}}
        }"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();

        // Secrets
        assert!(use_.secrets.is_some());
        let secrets = use_.secrets.as_ref().unwrap();
        assert_eq!(secrets.len(), 2);
        assert!(secrets.contains(&"secret1".to_string()));
        assert!(secrets.contains(&"secret2".to_string()));

        // Timeouts
        assert!(use_.timeouts.is_some());
        let timeouts = use_.timeouts.as_ref().unwrap();
        assert!(timeouts.contains_key("timeout1"));
    }

    #[test]
    fn test_use_definition_minimal() {
        let json = r#"{
            "secrets": ["mySecret"]
        }"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        assert!(use_.secrets.is_some());
        assert!(use_.authentications.is_none());
        assert!(use_.errors.is_none());
        assert!(use_.extensions.is_none());
        assert!(use_.retries.is_none());
        assert!(use_.timeouts.is_none());
        assert!(use_.catalogs.is_none());
    }

    #[test]
    fn test_use_definition_empty() {
        let json = r#"{}"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        assert!(use_.authentications.is_none());
        assert!(use_.secrets.is_none());
    }

    #[test]
    fn test_use_definition_with_catalogs_roundtrip() {
        let json = r#"{
            "catalogs": {"default": {"endpoint": "http://example.com/catalog"}}
        }"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&use_).unwrap();
        let deserialized: ComponentDefinitionCollection =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(use_, deserialized);
    }

    #[test]
    fn test_use_definition_with_timeouts_roundtrip() {
        let json = r#"{
            "timeouts": {"short": {"after": "PT10S"}, "long": {"after": "PT1H"}}
        }"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&use_).unwrap();
        let deserialized: ComponentDefinitionCollection =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(use_, deserialized);
    }

    #[test]
    fn test_document_tags_and_metadata() {
        // Matches Go SDK's Document_JSONMarshal/Unmarshal tests
        let json = r#"{
            "dsl": "1.0.0",
            "namespace": "example-namespace",
            "name": "example-name",
            "version": "1.0.0",
            "title": "Example Workflow",
            "summary": "This is a sample workflow document.",
            "tags": {"env": "prod", "team": "workflow"},
            "metadata": {"author": "John Doe", "created": "2025-01-01"}
        }"#;
        let meta: WorkflowDefinitionMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.dsl, "1.0.0");
        assert_eq!(meta.namespace, "example-namespace");
        assert_eq!(meta.name, "example-name");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.title, Some("Example Workflow".to_string()));
        assert_eq!(
            meta.summary,
            Some("This is a sample workflow document.".to_string())
        );
        assert!(meta.tags.is_some());
        let tags = meta.tags.as_ref().unwrap();
        assert_eq!(tags.get("env").map(|s| s.as_str()), Some("prod"));
        assert_eq!(tags.get("team").map(|s| s.as_str()), Some("workflow"));
        assert!(meta.metadata.is_some());
        let md = meta.metadata.as_ref().unwrap();
        assert!(md.contains_key("author"));
    }

    // Additional tests matching Go SDK's TestUse_UnmarshalJSON - individual sub-types

    #[test]
    fn test_use_authentications_deserialize() {
        let json = r#"{"authentications": {"auth1": {"basic": {"username": "alice", "password": "secret"}}}}"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        assert!(use_.authentications.is_some());
        let auths = use_.authentications.as_ref().unwrap();
        assert!(auths.contains_key("auth1"));
    }

    #[test]
    fn test_use_errors_deserialize() {
        let json = r#"{"errors": {"error1": {"type": "http://example.com/errors", "title": "Not Found", "status": 404}}}"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        assert!(use_.errors.is_some());
        let errors = use_.errors.as_ref().unwrap();
        assert!(errors.contains_key("error1"));
    }

    #[test]
    fn test_use_retries_deserialize() {
        let json = r#"{"retries": {"retry1": {"delay": {"seconds": 5}, "limit": {"attempt": {"count": 3}}}}}"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        assert!(use_.retries.is_some());
        let retries = use_.retries.as_ref().unwrap();
        assert!(retries.contains_key("retry1"));
    }

    #[test]
    fn test_use_extensions_deserialize() {
        let json = r#"{"extensions": [{"ext1": {"extend": "call"}}]}"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        assert!(use_.extensions.is_some());
    }

    #[test]
    fn test_use_functions_deserialize() {
        let json = r#"{"functions": {"func1": {"set": {"result": "ok"}}}}"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        assert!(use_.functions.is_some());
        let funcs = use_.functions.as_ref().unwrap();
        assert!(funcs.contains_key("func1"));
    }

    #[test]
    fn test_use_authentications_roundtrip() {
        let json = r#"{"authentications": {"auth1": {"basic": {"username": "alice", "password": "secret"}}}}"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&use_).unwrap();
        let deserialized: ComponentDefinitionCollection =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(use_, deserialized);
    }

    #[test]
    fn test_use_errors_roundtrip() {
        let json = r#"{"errors": {"error1": {"type": "http://example.com/errors", "title": "Not Found", "status": 404}}}"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&use_).unwrap();
        let deserialized: ComponentDefinitionCollection =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(use_, deserialized);
    }

    #[test]
    fn test_use_retries_roundtrip() {
        let json = r#"{"retries": {"retry1": {"delay": {"seconds": 5}, "limit": {"attempt": {"count": 3}}}}}"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&use_).unwrap();
        let deserialized: ComponentDefinitionCollection =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(use_, deserialized);
    }

    #[test]
    fn test_use_secrets_roundtrip() {
        let json = r#"{"secrets": ["secret1", "secret2"]}"#;
        let use_: ComponentDefinitionCollection = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&use_).unwrap();
        let deserialized: ComponentDefinitionCollection =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(use_, deserialized);
    }
}
