use super::*;

// ============== DoTaskDefinitionBuilder ==============
/// Builder for constructing a do task that executes a sequence of sub-tasks.
#[derive(Default)]
pub struct DoTaskDefinitionBuilder {
    task: DoTaskDefinition,
}

impl DoTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a named sub-task to the do block.
    pub fn do_<F>(&mut self, name: &str, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TaskDefinitionBuilder),
    {
        let mut builder = TaskDefinitionBuilder::new();
        setup(&mut builder);
        let task = builder.build();
        self.task.do_.add(name.to_string(), task);
        self
    }
}

impl_task_definition_builder_base!(DoTaskDefinitionBuilder, task, TaskDefinition::Do);
