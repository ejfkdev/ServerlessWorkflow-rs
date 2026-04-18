use serde::{Deserialize, Serialize};

use super::{Map, TaskDefinitionFields};

/// Represents the definition of a task that evaluates conditions and executes specific branches based on the result
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwitchTaskDefinition {
    /// Gets/sets the definition of the switch to use
    #[serde(rename = "switch")]
    pub switch: Map<String, SwitchCaseDefinition>,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}

/// Represents the definition of a case within a switch task, defining a condition and corresponding tasks to execute if the condition is met
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwitchCaseDefinition {
    /// Gets/sets the condition that determines whether or not the case should be executed in a switch task
    #[serde(rename = "when", skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,

    /// Gets/sets the transition to perform when the case matches
    #[serde(rename = "then", skip_serializing_if = "Option::is_none")]
    pub then: Option<String>,
}
