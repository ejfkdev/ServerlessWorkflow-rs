use serde::{Deserialize, Serialize};

use super::TaskDefinitionFields;
use crate::models::error::OneOfErrorDefinitionOrReference;

/// Represents the configuration of a task used to listen to specific events
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RaiseTaskDefinition {
    /// Gets/sets the definition of the error to raise
    #[serde(rename = "raise")]
    pub raise: RaiseErrorDefinition,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
impl RaiseTaskDefinition {
    /// Initializes a new RaiseTaskDefinition
    pub fn new(raise: RaiseErrorDefinition) -> Self {
        Self {
            raise,
            common: TaskDefinitionFields::new(),
        }
    }
}

/// Represents the definition of the error to raise
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RaiseErrorDefinition {
    /// Gets/sets the error to raise
    #[serde(rename = "error")]
    pub error: OneOfErrorDefinitionOrReference,
}
impl RaiseErrorDefinition {
    /// Initializes a new RaiseErrorDefinition
    pub fn new(error: OneOfErrorDefinitionOrReference) -> Self {
        Self { error }
    }
}
