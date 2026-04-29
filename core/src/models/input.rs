use crate::models::schema::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents the definition of an input data model
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputDataModelDefinition {
    /// Gets/sets the schema, if any, that defines and describes the input data of a workflow or task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaDefinition>,

    /// Gets/sets a runtime expression, if any, used to build the workflow or task input data based on both input and scope data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_with_from() {
        let json = r#"{"from": {"key": "value"}}"#;
        let input: InputDataModelDefinition = serde_json::from_str(json).unwrap();
        assert!(input.from.is_some());
        assert!(input.schema.is_none());
    }

    #[test]
    fn test_input_with_schema() {
        let json = r#"{"schema": {"format": "json", "document": {"type": "object"}}}"#;
        let input: InputDataModelDefinition = serde_json::from_str(json).unwrap();
        assert!(input.schema.is_some());
        assert!(input.from.is_none());
    }

    #[test]
    fn test_input_roundtrip() {
        let json = r#"{"from": {"key": "value"}}"#;
        let input: InputDataModelDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&input).unwrap();
        let deserialized: InputDataModelDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(input, deserialized);
    }

    // Additional tests matching Go SDK patterns

    #[test]
    fn test_input_with_from_expression() {
        // Input with a runtime expression as the "from" value
        let json = r#"{"from": "${ .inputData }"}"#;
        let input: InputDataModelDefinition = serde_json::from_str(json).unwrap();
        assert!(input.from.is_some());
        assert_eq!(input.from.unwrap(), serde_json::json!("${ .inputData }"));
    }

    #[test]
    fn test_input_with_schema_and_from() {
        // Both schema and from set
        let json = r#"{
            "schema": {"format": "json", "document": {"type": "object"}},
            "from": {"key": "value"}
        }"#;
        let input: InputDataModelDefinition = serde_json::from_str(json).unwrap();
        assert!(input.schema.is_some());
        assert!(input.from.is_some());
    }

    #[test]
    fn test_input_default() {
        let input = InputDataModelDefinition::default();
        assert!(input.schema.is_none());
        assert!(input.from.is_none());
    }

    #[test]
    fn test_input_empty_object() {
        let json = r#"{}"#;
        let input: InputDataModelDefinition = serde_json::from_str(json).unwrap();
        assert!(input.schema.is_none());
        assert!(input.from.is_none());
    }
}
