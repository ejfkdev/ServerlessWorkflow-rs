use serde::{Deserialize, Serialize};

use super::{Map, TaskDefinition, TaskDefinitionFields};
use crate::models::input::InputDataModelDefinition;

/// Represents the definition of a task that executes a set of subtasks iteratively for each element in a collection
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForTaskDefinition {
    /// Gets/sets the definition of the loop that iterates over a range of values
    #[serde(rename = "for")]
    pub for_: ForLoopDefinition,

    /// Gets/sets a runtime expression that represents the condition, if any, that must be met for the iteration to continue
    #[serde(rename = "while", skip_serializing_if = "Option::is_none")]
    pub while_: Option<String>,

    /// Gets/sets the tasks to perform for each item in the collection
    #[serde(rename = "do")]
    pub do_: Map<String, TaskDefinition>,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
impl ForTaskDefinition {
    /// Initializes a new ForTaskDefinition
    pub fn new(
        for_: ForLoopDefinition,
        do_: Map<String, TaskDefinition>,
        while_: Option<String>,
    ) -> Self {
        Self {
            for_,
            while_,
            do_,
            common: TaskDefinitionFields::new(),
        }
    }
}

/// Represents the definition of a loop that iterates over a range of values
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForLoopDefinition {
    /// Gets/sets the name of the variable that represents each element in the collection during iteration
    #[serde(rename = "each")]
    pub each: String,

    /// Gets/sets the runtime expression used to get the collection to iterate over
    #[serde(rename = "in")]
    pub in_: String,

    /// Gets/sets the name of the variable used to hold the index of each element in the collection during iteration
    #[serde(rename = "at", skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,

    /// Gets/sets the definition of the data, if any, to pass to iterations to run
    #[serde(rename = "input", skip_serializing_if = "Option::is_none")]
    pub input: Option<InputDataModelDefinition>,
}
impl ForLoopDefinition {
    pub fn new(
        each: &str,
        in_: &str,
        at: Option<String>,
        input: Option<InputDataModelDefinition>,
    ) -> Self {
        Self {
            each: each.to_string(),
            in_: in_.to_string(),
            at,
            input,
        }
    }
}
