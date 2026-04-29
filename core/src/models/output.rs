use crate::models::schema::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents the definition of an output data model
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputDataModelDefinition {
    /// Gets/sets the schema, if any, that defines and describes the output data of a workflow or task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaDefinition>,

    /// Gets/sets a runtime expression, if any, used to output specific data to the scope data
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    pub as_: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_with_as() {
        let json = r#"{"as": {"result": "output"}}"#;
        let output: OutputDataModelDefinition = serde_json::from_str(json).unwrap();
        assert!(output.as_.is_some());
        assert!(output.schema.is_none());
    }

    #[test]
    fn test_output_with_schema() {
        let json = r#"{"schema": {"format": "json", "document": {"type": "string"}}}"#;
        let output: OutputDataModelDefinition = serde_json::from_str(json).unwrap();
        assert!(output.schema.is_some());
        assert!(output.as_.is_none());
    }

    #[test]
    fn test_output_roundtrip() {
        let json = r#"{"as": {"result": "output"}}"#;
        let output: OutputDataModelDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&output).unwrap();
        let deserialized: OutputDataModelDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(output, deserialized);
    }

    // Additional tests matching Go SDK patterns

    #[test]
    fn test_output_with_as_expression() {
        // Output with a runtime expression as the "as" value
        let json = r#"{"as": "${ .result }"}"#;
        let output: OutputDataModelDefinition = serde_json::from_str(json).unwrap();
        assert!(output.as_.is_some());
        assert_eq!(output.as_.unwrap(), serde_json::json!("${ .result }"));
    }

    #[test]
    fn test_output_with_schema_and_as() {
        // Both schema and as set
        let json = r#"{
            "schema": {"format": "json", "document": {"type": "object"}},
            "as": {"result": "output"}
        }"#;
        let output: OutputDataModelDefinition = serde_json::from_str(json).unwrap();
        assert!(output.schema.is_some());
        assert!(output.as_.is_some());
    }

    #[test]
    fn test_output_default() {
        let output = OutputDataModelDefinition::default();
        assert!(output.schema.is_none());
        assert!(output.as_.is_none());
    }

    #[test]
    fn test_output_schema_roundtrip() {
        let json = r#"{"schema": {"format": "json", "document": {"type": "string"}}}"#;
        let output: OutputDataModelDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&output).unwrap();
        let deserialized: OutputDataModelDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(output, deserialized);
    }
}
