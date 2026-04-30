use crate::error::{WorkflowError, WorkflowResult};
use serde_json::Value;
use swf_core::models::schema::SchemaDefinition;

/// Validates JSON data against a SchemaDefinition
pub fn validate_schema(
    data: &Value,
    schema: &SchemaDefinition,
    task_name: &str,
) -> WorkflowResult<()> {
    if schema.document.is_none() && schema.resource.is_none() {
        return Ok(());
    }

    let schema_json = if let Some(ref doc) = schema.document {
        doc.clone()
    } else {
        // External resource references are not yet supported
        return Err(WorkflowError::validation(
            "external schema resources are not yet supported",
            task_name,
        ));
    };

    validate_json_schema(data, &schema_json, task_name)
}

/// Validates JSON data against a raw JSON Schema
pub fn validate_json_schema(data: &Value, schema: &Value, task_name: &str) -> WorkflowResult<()> {
    let validator = jsonschema::JSONSchema::compile(schema).map_err(|e| {
        WorkflowError::validation(format!("failed to compile JSON schema: {}", e), task_name)
    })?;

    let result = validator.validate(data);
    match result {
        Ok(_) => Ok(()),
        Err(errors) => {
            let msgs: Vec<String> = errors.map(|e| format!("{}", e)).collect();
            Err(WorkflowError::validation(
                format!("JSON schema validation failed: {}", msgs.join("; ")),
                task_name,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_json_schema_valid() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });
        let data = json!({"name": "test"});
        assert!(validate_json_schema(&data, &schema, "test").is_ok());
    }

    #[test]
    fn test_validate_json_schema_invalid() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });
        let data = json!({"age": 42});
        assert!(validate_json_schema(&data, &schema, "test").is_err());
    }
}
