use serde::{Deserialize, Serialize};

use super::{Map, TaskDefinition, TaskDefinitionFields};

/// Represents the configuration of a task that is composed of multiple subtasks to run concurrently
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForkTaskDefinition {
    /// Gets/sets the configuration of the branches to perform concurrently
    #[serde(rename = "fork")]
    pub fork: BranchingDefinition,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
impl ForkTaskDefinition {
    /// Initializes a new ForkTaskDefinition
    pub fn new(fork: BranchingDefinition) -> Self {
        Self {
            fork,
            common: TaskDefinitionFields::new(),
        }
    }
}

/// Represents an object used to configure branches to perform concurrently
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchingDefinition {
    /// Gets/sets a name/definition mapping of the subtasks to perform concurrently
    #[serde(rename = "branches")]
    pub branches: Map<String, TaskDefinition>,

    /// Gets/sets a boolean indicating whether or not the branches should compete each other. If `true` and if a branch completes, it will cancel all other branches then it will return its output as the task's output
    #[serde(rename = "compete")]
    pub compete: bool,
}
impl BranchingDefinition {
    pub fn new(branches: Map<String, TaskDefinition>, compete: bool) -> Self {
        Self { branches, compete }
    }
}
