pub mod constants;
pub mod custom_task;
pub mod do_task;
pub mod emit_task;
pub mod flow_directive;
pub mod for_task;
pub mod fork_task;
pub mod listen_task;
pub mod raise_task;
pub mod run_task;
pub mod set_task;
pub mod switch_task;
pub mod try_task;
pub mod wait_task;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::input::InputDataModelDefinition;
use super::map::Map;
use super::output::OutputDataModelDefinition;
use super::timeout::OneOfTimeoutDefinitionOrReference;

// Re-export CallTaskDefinition for convenience
pub use crate::models::call::CallTaskDefinition;

// Re-export all public types from sub-modules
pub use constants::{
    ContainerCleanupPolicy, EventReadMode, ExtensionTarget, FlowDirective, HttpMethod,
    HttpOutputFormat, OAuth2GrantType, ProcessReturnType, ProcessType, PullPolicy,
    ScriptLanguage, TaskType,
};
pub use custom_task::CustomTaskDefinition;
pub use do_task::DoTaskDefinition;
pub use emit_task::{EmitTaskDefinition, EventEmissionDefinition};
pub use flow_directive::{FlowDirectiveType, FlowDirectiveValue};
pub use for_task::{ForLoopDefinition, ForTaskDefinition};
pub use fork_task::{BranchingDefinition, ForkTaskDefinition};
pub use listen_task::{ListenerDefinition, ListenTaskDefinition, SubscriptionIteratorDefinition};
pub use raise_task::{RaiseErrorDefinition, RaiseTaskDefinition};
pub use run_task::{
    ContainerLifetimeDefinition, ContainerProcessDefinition, OneOfRunArguments,
    ProcessTypeDefinition, RunTaskDefinition, ScriptProcessDefinition, ShellProcessDefinition,
    WorkflowProcessDefinition,
};
pub use set_task::{SetTaskDefinition, SetValue};
pub use switch_task::{SwitchCaseDefinition, SwitchTaskDefinition};
pub use try_task::{
    ErrorCatcherDefinition, ErrorFilterDefinition, ErrorFilterProperties, TryTaskDefinition,
};
pub use wait_task::WaitTaskDefinition;

/// Represents a value that can be any of the supported task definitions
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TaskDefinition {
    /// Variant holding the definition of a 'call' task
    Call(Box<CallTaskDefinition>),
    /// Variant holding the definition of a 'do' task
    Do(DoTaskDefinition),
    /// Variant holding the definition of an 'emit' task
    Emit(EmitTaskDefinition),
    /// Variant holding the definition of a 'for' task
    For(ForTaskDefinition),
    /// Variant holding the definition of a 'fork' task
    Fork(ForkTaskDefinition),
    /// Variant holding the definition of a 'listen' task
    Listen(Box<ListenTaskDefinition>),
    /// Variant holding the definition of a 'raise' task
    Raise(RaiseTaskDefinition),
    /// Variant holding the definition of a 'run' task
    Run(Box<RunTaskDefinition>),
    /// Variant holding the definition of a 'set' task
    Set(SetTaskDefinition),
    /// Variant holding the definition of a 'switch' task
    Switch(SwitchTaskDefinition),
    /// Variant holding the definition of a 'try' task
    Try(TryTaskDefinition),
    /// Variant holding the definition of a 'wait' task
    Wait(WaitTaskDefinition),
    /// Variant holding a custom/extension task definition (raw JSON value)
    Custom(CustomTaskDefinition),
}

impl TaskDefinition {
    /// Returns the common fields (if, input, output, export, timeout, then, metadata)
    /// shared by all task definition variants.
    pub fn common_fields(&self) -> &TaskDefinitionFields {
        match self {
            TaskDefinition::Do(t) => &t.common,
            TaskDefinition::Set(t) => &t.common,
            TaskDefinition::Wait(t) => &t.common,
            TaskDefinition::Raise(t) => &t.common,
            TaskDefinition::For(t) => &t.common,
            TaskDefinition::Switch(t) => &t.common,
            TaskDefinition::Fork(t) => &t.common,
            TaskDefinition::Try(t) => &t.common,
            TaskDefinition::Emit(t) => &t.common,
            TaskDefinition::Listen(t) => &t.common,
            TaskDefinition::Call(call_def) => call_def.common_fields(),
            TaskDefinition::Run(t) => &t.common,
            TaskDefinition::Custom(t) => &t.common,
        }
    }
}

// Custom deserializer that uses field-based detection to determine task type.
//
// Priority order matters because some task types share fields:
// - `for` MUST be checked before `do`: For tasks contain a `do` sub-field,
//   so a YAML with both `for` and `do` is a For task, not a Do task.
// - All other task types have unique discriminant fields (`call`, `set`, etc.).
// - `do` is checked last as a fallback for plain sequential task lists.
impl<'de> serde::Deserialize<'de> for TaskDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        macro_rules! try_deserialize {
            ($key:expr, $variant:ident, $ty:ty) => {
                if value.get($key).is_some() {
                    return <$ty>::deserialize(value)
                        .map(TaskDefinition::$variant)
                        .map_err(serde::de::Error::custom);
                }
            };
        }
        macro_rules! try_deserialize_boxed {
            ($key:expr, $variant:ident, $ty:ty) => {
                if value.get($key).is_some() {
                    return <$ty>::deserialize(value)
                        .map(|v| TaskDefinition::$variant(Box::new(v)))
                        .map_err(serde::de::Error::custom);
                }
            };
        }

        // Check for 'for' field first - if present, it's a For task
        try_deserialize!("for", For, ForTaskDefinition);
        // Try other variants in priority order
        try_deserialize_boxed!("call", Call, CallTaskDefinition);
        try_deserialize!("set", Set, SetTaskDefinition);
        try_deserialize!("fork", Fork, ForkTaskDefinition);
        try_deserialize_boxed!("run", Run, RunTaskDefinition);
        try_deserialize!("switch", Switch, SwitchTaskDefinition);
        try_deserialize!("try", Try, TryTaskDefinition);
        try_deserialize!("emit", Emit, EmitTaskDefinition);
        try_deserialize!("raise", Raise, RaiseTaskDefinition);
        try_deserialize!("wait", Wait, WaitTaskDefinition);
        try_deserialize_boxed!("listen", Listen, ListenTaskDefinition);
        // If we get here and there's a 'do' field, it's a Do task (not a For task)
        try_deserialize!("do", Do, DoTaskDefinition);

        // Unrecognized task type: store as Custom for extensibility
        // Matches Go SDK's TaskRegistry pattern — custom tasks are preserved
        CustomTaskDefinition::deserialize(value)
            .map(TaskDefinition::Custom)
            .map_err(serde::de::Error::custom)
    }
}

/// Holds the fields common to all tasks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskDefinitionFields {
    /// Gets/sets a runtime expression, if any, used to determine whether or not the execute the task in the current context
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub if_: Option<String>,

    /// Gets/sets the definition, if any, of the task's input data
    #[serde(rename = "input", skip_serializing_if = "Option::is_none")]
    pub input: Option<InputDataModelDefinition>,

    /// Gets/sets the definition, if any, of the task's output data
    #[serde(rename = "output", skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputDataModelDefinition>,

    /// Gets/sets the optional configuration for exporting data within the task's context
    #[serde(rename = "export", skip_serializing_if = "Option::is_none")]
    pub export: Option<OutputDataModelDefinition>,

    /// Gets/sets the task's timeout, if any
    #[serde(rename = "timeout", skip_serializing_if = "Option::is_none")]
    pub timeout: Option<OneOfTimeoutDefinitionOrReference>,

    /// Gets/sets the flow directive to be performed upon completion of the task
    #[serde(rename = "then", skip_serializing_if = "Option::is_none")]
    pub then: Option<String>,

    /// Gets/sets a key/value mapping of additional information associated with the task
    #[serde(rename = "metadata", skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}
impl Default for TaskDefinitionFields {
    fn default() -> Self {
        TaskDefinitionFields::new()
    }
}
impl TaskDefinitionFields {
    /// Initializes a new TaskDefinitionFields
    pub fn new() -> Self {
        Self {
            if_: None,
            input: None,
            output: None,
            export: None,
            timeout: None,
            then: None,
            metadata: None,
        }
    }
}
