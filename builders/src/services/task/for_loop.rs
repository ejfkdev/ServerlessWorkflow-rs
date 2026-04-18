use super::*;

// ============== ForTaskDefinitionBuilder ==============
/// Builder for constructing a for task that iterates over a collection.
#[derive(Default)]
pub struct ForTaskDefinitionBuilder {
    task: ForTaskDefinition,
}

impl ForTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the iteration variable name (e.g., "item").
    pub fn each(&mut self, each: &str) -> &mut Self {
        self.task.for_.each = each.to_string();
        self
    }

    /// Sets the collection expression to iterate over.
    pub fn in_(&mut self, in_: &str) -> &mut Self {
        self.task.for_.in_ = in_.to_string();
        self
    }

    /// Sets the index variable name for the current iteration position.
    pub fn at(&mut self, at: &str) -> &mut Self {
        self.task.for_.at = Some(at.to_string());
        self
    }

    /// Adds a named sub-task to execute on each iteration.
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

    /// Sets a while condition to continue iteration.
    pub fn while_(&mut self, while_expr: &str) -> &mut Self {
        self.task.while_ = Some(while_expr.to_string());
        self
    }
}

impl_task_definition_builder_base!(ForTaskDefinitionBuilder, task, TaskDefinition::For);
