use serde::{Deserialize, Serialize};

use super::{Map, TaskDefinition, TaskDefinitionFields};

/// Represents the configuration of a task that is composed of multiple subtasks to run sequentially
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoTaskDefinition {
    /// Gets/sets a name/definition mapping of the subtasks to perform sequentially
    #[serde(rename = "do")]
    pub do_: Map<String, TaskDefinition>,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
impl DoTaskDefinition {
    /// Initializes a new DoTaskDefinition
    pub fn new(do_: Map<String, TaskDefinition>) -> Self {
        Self {
            do_,
            common: TaskDefinitionFields::new(),
        }
    }
}
