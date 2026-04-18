use super::*;

// ============== EmitTaskDefinitionBuilder ==============
/// Builder for constructing an emit task that produces a workflow event.
pub struct EmitTaskDefinitionBuilder {
    task: EmitTaskDefinition,
}

impl EmitTaskDefinitionBuilder {
    pub fn new(event: EventDefinition) -> Self {
        let mut task = EmitTaskDefinition::default();
        task.emit.event = event;
        Self { task }
    }

    /// Replaces all event attributes with the provided map.
    pub fn with_attributes(&mut self, attrs: HashMap<String, Value>) -> &mut Self {
        self.task.emit.event.with = attrs;
        self
    }
}

impl_task_definition_builder_base!(EmitTaskDefinitionBuilder, task, TaskDefinition::Emit);
