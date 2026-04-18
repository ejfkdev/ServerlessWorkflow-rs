use super::*;

// ============== ForkTaskDefinitionBuilder ==============
/// Builder for constructing a fork task that executes branches in parallel.
#[derive(Default)]
pub struct ForkTaskDefinitionBuilder {
    task: ForkTaskDefinition,
    branch_counter: usize,
}

impl ForkTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether branches compete (first to complete cancels the rest).
    pub fn compete(&mut self, compete: bool) -> &mut Self {
        self.task.fork.compete = compete;
        self
    }

    /// Adds an auto-named branch (e.g., "branch-0", "branch-1").
    pub fn branch<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TaskDefinitionBuilder),
    {
        let mut builder = TaskDefinitionBuilder::new();
        setup(&mut builder);
        let task = builder.build();
        let name = format!("branch-{}", self.branch_counter);
        self.branch_counter += 1;
        self.task.fork.branches.add(name, task);
        self
    }

    /// Adds a named branch to the fork.
    pub fn named_branch<F>(&mut self, name: &str, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TaskDefinitionBuilder),
    {
        let mut builder = TaskDefinitionBuilder::new();
        setup(&mut builder);
        let task = builder.build();
        self.task.fork.branches.add(name.to_string(), task);
        self
    }
}

impl_task_definition_builder_base!(ForkTaskDefinitionBuilder, task, TaskDefinition::Fork);
