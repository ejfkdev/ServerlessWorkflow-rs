use crate::models::resource::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Provides the default schema format
fn default_schema_format() -> String {
    SchemaFormat::JSON.to_string()
}

/// Enumerates all supported schema formats
pub struct SchemaFormat;
impl SchemaFormat {
    /// Gets the Avro schema format
    pub const AVRO: &'static str = "avro";
    /// Gets the JSON schema format
    pub const JSON: &'static str = "json";
    /// Gets the XML schema format
    pub const XML: &'static str = "xml";
}

/// Represents the definition of a schema
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaDefinition {
    /// Gets/sets the schema's format. Defaults to 'json'. The (optional) version of the format can be set using `{format}:{version}`.
    #[serde(rename = "format", default = "default_schema_format")]
    pub format: String,

    /// Gets/sets the schema's external resource, if any. Required if `document` has not been set.
    #[serde(rename = "resource", skip_serializing_if = "Option::is_none")]
    pub resource: Option<ExternalResourceDefinition>,

    /// Gets/sets the inline definition of the schema to use. Required if `resource` has not been set.
    #[serde(rename = "document", skip_serializing_if = "Option::is_none")]
    pub document: Option<Value>,
}

impl Default for SchemaDefinition {
    fn default() -> Self {
        SchemaDefinition {
            format: default_schema_format(),
            resource: None,
            document: None,
        }
    }
}

impl SchemaDefinition {
    /// Creates a new schema definition with an inline document
    pub fn with_document(format: &str, document: Value) -> Self {
        Self {
            format: format.to_string(),
            resource: None,
            document: Some(document),
        }
    }

    /// Creates a new schema definition with an external resource
    pub fn with_resource(format: &str, resource: ExternalResourceDefinition) -> Self {
        Self {
            format: format.to_string(),
            resource: Some(resource),
            document: None,
        }
    }

    /// Validates that the schema definition is valid (document XOR resource)
    pub fn validate(&self) -> Result<(), SchemaValidationError> {
        match (&self.document, &self.resource) {
            (Some(_), Some(_)) => Err(SchemaValidationError::BothSet),
            (None, None) => Err(SchemaValidationError::NeitherSet),
            _ => Ok(()),
        }
    }

    /// Gets whether this schema uses an inline document
    pub fn is_document(&self) -> bool {
        self.document.is_some() && self.resource.is_none()
    }

    /// Gets whether this schema uses an external resource
    pub fn is_resource(&self) -> bool {
        self.resource.is_some() && self.document.is_none()
    }
}

/// Represents validation errors for schema definitions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaValidationError {
    /// Both document and resource are set (should be mutually exclusive)
    BothSet,
    /// Neither document nor resource is set (one is required)
    NeitherSet,
}

impl std::fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaValidationError::BothSet => write!(
                f,
                "Schema 'document' and 'resource' are mutually exclusive; only one can be set"
            ),
            SchemaValidationError::NeitherSet => {
                write!(f, "Schema must have either 'document' or 'resource' set")
            }
        }
    }
}

impl std::error::Error for SchemaValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_default_format() {
        let schema = SchemaDefinition::default();
        assert_eq!(schema.format, "json");
    }

    #[test]
    fn test_schema_with_document() {
        let doc = serde_json::json!({"type": "object", "properties": {"key": {"type": "string"}}});
        let schema = SchemaDefinition::with_document("json", doc.clone());
        assert!(schema.is_document());
        assert!(!schema.is_resource());
        assert_eq!(schema.document, Some(doc));
    }

    #[test]
    fn test_schema_with_resource() {
        let resource = ExternalResourceDefinition {
            name: Some("mySchema".to_string()),
            endpoint: crate::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                "http://example.com/schema".to_string(),
            ),
        };
        let schema = SchemaDefinition::with_resource("json", resource);
        assert!(schema.is_resource());
        assert!(!schema.is_document());
    }

    #[test]
    fn test_schema_validate_document() {
        let schema = SchemaDefinition::with_document("json", serde_json::json!({}));
        assert!(schema.validate().is_ok());
    }

    #[test]
    fn test_schema_validate_neither() {
        let schema = SchemaDefinition::default();
        assert!(matches!(
            schema.validate(),
            Err(SchemaValidationError::NeitherSet)
        ));
    }

    #[test]
    fn test_schema_deserialize_with_document() {
        let json = r#"{
            "format": "json",
            "document": {"type": "object"}
        }"#;
        let schema: SchemaDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(schema.format, "json");
        assert!(schema.document.is_some());
    }

    #[test]
    fn test_schema_deserialize_with_resource() {
        let json = r#"{
            "format": "avro",
            "resource": {
                "name": "myAvroSchema",
                "endpoint": "http://example.com/schema.avsc"
            }
        }"#;
        let schema: SchemaDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(schema.format, "avro");
        assert!(schema.resource.is_some());
    }

    #[test]
    fn test_schema_roundtrip() {
        let json = r#"{"format": "json", "document": {"type": "object"}}"#;
        let schema: SchemaDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&schema).unwrap();
        let deserialized: SchemaDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(schema, deserialized);
    }

    // Additional tests matching Go SDK patterns

    #[test]
    fn test_schema_default_format_on_deserialize() {
        // When format is omitted, it should default to "json"
        let json = r#"{"document": {"type": "object"}}"#;
        let schema: SchemaDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(schema.format, "json");
    }

    #[test]
    fn test_schema_format_with_version() {
        // Format can include version: "json:2020-12"
        let json = r#"{"format": "json:2020-12", "document": {"type": "object"}}"#;
        let schema: SchemaDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(schema.format, "json:2020-12");
    }

    #[test]
    fn test_schema_avro_format() {
        let json = r#"{"format": "avro", "document": {"type": "record", "name": "Test"}}"#;
        let schema: SchemaDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(schema.format, "avro");
    }

    #[test]
    fn test_schema_validate_both_set() {
        // Both document and resource set should be invalid
        let mut schema = SchemaDefinition::with_document("json", serde_json::json!({}));
        schema.resource = Some(ExternalResourceDefinition {
            name: Some("test".to_string()),
            endpoint: crate::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                "http://example.com".to_string(),
            ),
        });
        assert!(matches!(
            schema.validate(),
            Err(SchemaValidationError::BothSet)
        ));
    }

    #[test]
    fn test_schema_with_resource_roundtrip() {
        let json = r#"{
            "format": "json",
            "resource": {
                "name": "mySchema",
                "endpoint": "http://example.com/schema.json"
            }
        }"#;
        let schema: SchemaDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&schema).unwrap();
        let deserialized: SchemaDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(schema, deserialized);
    }

    #[test]
    fn test_schema_validation_error_display() {
        let both = SchemaValidationError::BothSet;
        assert!(both.to_string().contains("mutually exclusive"));

        let neither = SchemaValidationError::NeitherSet;
        assert!(neither.to_string().contains("must have either"));
    }
}
