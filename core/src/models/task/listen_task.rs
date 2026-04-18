use serde::{Deserialize, Serialize};

use super::{Map, TaskDefinition, TaskDefinitionFields};
use crate::models::event::EventConsumptionStrategyDefinition;
use crate::models::output::OutputDataModelDefinition;

/// Represents the configuration of a task used to listen to specific events
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListenTaskDefinition {
    /// Gets/sets the configuration of the listener to use
    #[serde(rename = "listen")]
    pub listen: ListenerDefinition,

    ///Gets/sets the configuration of the iterator, if any, for processing each consumed event
    #[serde(rename = "foreach")]
    pub foreach: Option<SubscriptionIteratorDefinition>,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
impl ListenTaskDefinition {
    /// Initializes a new ListenTaskDefinition
    pub fn new(listen: ListenerDefinition) -> Self {
        Self {
            listen,
            foreach: None,
            common: TaskDefinitionFields::new(),
        }
    }
}

/// Represents the configuration of an event listener
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListenerDefinition {
    /// Gets/sets the listener's target
    #[serde(rename = "to")]
    pub to: EventConsumptionStrategyDefinition,

    /// Gets/sets a string that specifies how events are read during the listen operation
    #[serde(rename = "read", skip_serializing_if = "Option::is_none")]
    pub read: Option<String>,
}
impl ListenerDefinition {
    pub fn new(to: EventConsumptionStrategyDefinition) -> Self {
        Self { to, read: None }
    }
}

/// Represents the definition of the iterator used to process each event or message consumed by a subscription
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionIteratorDefinition {
    /// Gets the name of the variable used to store the current item being enumerated
    #[serde(rename = "item", skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,

    /// Gets the name of the variable used to store the index of the current item being enumerated
    #[serde(rename = "at", skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,

    /// Gets the tasks to perform for each consumed item
    #[serde(rename = "do", skip_serializing_if = "Option::is_none")]
    pub do_: Option<Map<String, TaskDefinition>>,

    /// Gets/sets an object, if any, used to customize the item's output and to document its schema.
    #[serde(rename = "output", skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputDataModelDefinition>,

    /// Gets/sets an object, if any, used to customize the content of the workflow context.
    #[serde(rename = "export", skip_serializing_if = "Option::is_none")]
    pub export: Option<OutputDataModelDefinition>,
}
