use serde::{Deserialize, Serialize};

use super::TaskDefinitionFields;
use crate::models::duration::OneOfDurationOrIso8601Expression;

/// Represents the definition of a task used to wait a certain amount of time
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaitTaskDefinition {
    /// Gets/sets the amount of time to wait before resuming workflow
    #[serde(rename = "wait")]
    pub wait: OneOfDurationOrIso8601Expression,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
impl WaitTaskDefinition {
    /// Initializes a new WaitTaskDefinition
    pub fn new(wait: OneOfDurationOrIso8601Expression) -> Self {
        Self {
            wait,
            common: TaskDefinitionFields::new(),
        }
    }
}
