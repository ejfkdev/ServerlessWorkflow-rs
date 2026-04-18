use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Map, TaskDefinition, TaskDefinitionFields};
use crate::models::retry::OneOfRetryPolicyDefinitionOrReference;

/// Represents the definition of a task used to try one or more subtasks, and to catch/handle the errors that can potentially be raised during execution
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct TryTaskDefinition {
    /// Gets/sets a name/definition map of the tasks to try running
    #[serde(rename = "try")]
    pub try_: Map<String, TaskDefinition>,

    /// Gets/sets the object used to define the errors to catch
    #[serde(rename = "catch")]
    pub catch: ErrorCatcherDefinition,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
impl TryTaskDefinition {
    /// Initializes a new TryTaskDefintion
    pub fn new(try_: Map<String, TaskDefinition>, catch: ErrorCatcherDefinition) -> Self {
        Self {
            try_,
            catch,
            common: TaskDefinitionFields::new(),
        }
    }
}

/// Represents the configuration of a concept used to catch errors
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorCatcherDefinition {
    /// Gets/sets the definition of the errors to catch
    #[serde(rename = "errors", skip_serializing_if = "Option::is_none")]
    pub errors: Option<ErrorFilterDefinition>,

    /// Gets/sets the name of the runtime expression variable to save the error as. Defaults to 'error'.
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    pub as_: Option<String>,

    /// Gets/sets a runtime expression used to determine whether or not to catch the filtered error
    #[serde(rename = "when", skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,

    /// Gets/sets a runtime expression used to determine when NOT to catch the filtered error
    #[serde(rename = "exceptWhen", skip_serializing_if = "Option::is_none")]
    pub except_when: Option<String>,

    /// Gets/sets the retry policy to use, if any
    #[serde(rename = "retry", skip_serializing_if = "Option::is_none")]
    pub retry: Option<OneOfRetryPolicyDefinitionOrReference>,

    /// Gets/sets a name/definition map of the tasks, if any, to run when catching an error
    #[serde(rename = "do", skip_serializing_if = "Option::is_none")]
    pub do_: Option<Map<String, TaskDefinition>>,
}

/// Represents the definition of an error filter
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorFilterDefinition {
    /// Gets/sets the properties that errors to filter must define
    #[serde(rename = "with", skip_serializing_if = "Option::is_none")]
    pub with: Option<ErrorFilterProperties>,
}

/// Represents the specific properties used to filter errors
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorFilterProperties {
    /// Gets/sets the error type to filter by
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    /// Gets/sets the error status to filter by
    #[serde(rename = "status", skip_serializing_if = "Option::is_none")]
    pub status: Option<Value>,

    /// Gets/sets the error instance to filter by
    #[serde(rename = "instance", skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,

    /// Gets/sets the error title to filter by
    #[serde(rename = "title", skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Gets/sets the error detail to filter by
    #[serde(rename = "detail", skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
