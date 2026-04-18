use serde::{Deserialize, Serialize};

use super::TaskDefinitionFields;
use crate::models::event::EventDefinition;

/// Represents the configuration of a task used to emit an event
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmitTaskDefinition {
    /// Gets/sets the configuration of an event's emission
    #[serde(rename = "emit")]
    pub emit: EventEmissionDefinition,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
impl EmitTaskDefinition {
    /// Initializes a new EmitTaskDefinition
    pub fn new(emit: EventEmissionDefinition) -> Self {
        Self {
            emit,
            common: TaskDefinitionFields::new(),
        }
    }
}

/// Represents the configuration of an event's emission
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEmissionDefinition {
    /// Gets/sets the definition of the event to emit
    #[serde(rename = "event")]
    pub event: EventDefinition,
}
impl EventEmissionDefinition {
    pub fn new(event: EventDefinition) -> Self {
        Self { event }
    }
}
