use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::TaskDefinitionFields;

/// Represents the value that can be set in a Set task - either a map or a runtime expression string
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SetValue {
    /// A map of key-value pairs to set
    Map(HashMap<String, Value>),
    /// A runtime expression string that evaluates to the data to set
    Expression(String),
}

impl Default for SetValue {
    fn default() -> Self {
        SetValue::Map(HashMap::new())
    }
}

/// Represents the definition of a task used to set data
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetTaskDefinition {
    /// Gets/sets the data to set
    #[serde(rename = "set")]
    pub set: SetValue,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
