use serde::{Deserialize, Serialize};

/// Represents a typed flow directive that can be an enumerated value or a custom string
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlowDirectiveValue {
    /// One of the standard enumerated flow directives
    Enumerated(FlowDirectiveType),
    /// A custom/free-form flow directive string
    Custom(String),
}

/// Enumerates the standard flow directive types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowDirectiveType {
    /// Continues to the next task in the workflow
    #[serde(rename = "continue")]
    Continue,
    /// Exits the current composite task
    #[serde(rename = "exit")]
    Exit,
    /// Ends the workflow execution
    #[serde(rename = "end")]
    End,
}

impl FlowDirectiveValue {
    /// Checks if this is one of the standard enumerated values
    pub fn is_enumerated(&self) -> bool {
        matches!(self, FlowDirectiveValue::Enumerated(_))
    }

    /// Checks if this flow directive represents a termination (exit or end)
    pub fn is_termination(&self) -> bool {
        matches!(
            self,
            FlowDirectiveValue::Enumerated(FlowDirectiveType::Exit | FlowDirectiveType::End)
        )
    }
}

impl Default for FlowDirectiveValue {
    fn default() -> Self {
        FlowDirectiveValue::Enumerated(FlowDirectiveType::Continue)
    }
}
