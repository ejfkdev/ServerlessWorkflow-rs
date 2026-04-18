use super::*;

// ============== WaitTaskDefinitionBuilder ==============
/// Builder for constructing a wait task that pauses execution for a duration.
pub struct WaitTaskDefinitionBuilder {
    task: WaitTaskDefinition,
}

impl WaitTaskDefinitionBuilder {
    pub fn new(duration: OneOfDurationOrIso8601Expression) -> Self {
        Self {
            task: WaitTaskDefinition::new(duration),
        }
    }
}

impl_task_definition_builder_base!(WaitTaskDefinitionBuilder, task, TaskDefinition::Wait);
