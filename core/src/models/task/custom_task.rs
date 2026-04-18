use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::TaskDefinitionFields;

/// Represents a custom/extension task definition not covered by built-in types.
///
/// Matches Go SDK's TaskRegistry pattern — allows users to define custom task types
/// that are preserved during deserialization and can be handled by a CustomTaskHandler
/// at runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomTaskDefinition {
    /// The task type name (e.g., "myCustomTask")
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// The raw JSON configuration for the custom task
    #[serde(flatten)]
    pub config: Value,
    /// Common task fields (if, input, output, export, timeout, then, metadata)
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
