use crate::error::{WorkflowError, WorkflowResult};
use serde_json::Value;
use swf_core::models::resource::OneOfEndpointDefinitionOrUri;
use swf_core::models::schema::SchemaDefinition;

/// Validates JSON data against a SchemaDefinition.
///
/// Supports both inline `document` schemas and external `resource` schemas.
/// For external resources, the schema is fetched via HTTP GET from the
/// resource's endpoint URI.
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
    } else if let Some(ref resource) = schema.resource {
        fetch_external_schema(&resource.endpoint)?
    } else {
        return Ok(());
    };

    validate_json_schema(data, &schema_json, task_name)
}

/// Fetches an external JSON Schema document from a remote endpoint.
fn fetch_external_schema(endpoint: &OneOfEndpointDefinitionOrUri) -> WorkflowResult<Value> {
    let uri = match endpoint {
        OneOfEndpointDefinitionOrUri::Uri(uri) => uri.clone(),
        OneOfEndpointDefinitionOrUri::Endpoint(ep) => ep.uri.clone(),
    };

    let rt = tokio::runtime::Handle::try_current().map_err(|_| {
        WorkflowError::runtime_simple(
            "no tokio runtime available for fetching external schema",
            "schema",
        )
    })?;

    rt.block_on(async {
        let response = reqwest::get(&uri).await.map_err(|e| {
            WorkflowError::runtime_simple(
                format!("failed to fetch external schema from '{}': {}", uri, e),
                "schema",
            )
        })?;

        let schema_json: Value = response.json().await.map_err(|e| {
            WorkflowError::runtime_simple(
                format!(
                    "failed to parse external schema response from '{}': {}",
                    uri, e
                ),
                "schema",
            )
        })?;

        Ok(schema_json)
    })
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
