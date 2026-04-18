use super::*;

// ============== TaskDefinitionBuilder ==============
/// Primary entry point for building workflow task definitions.
///
/// Supports all task types: call, do, emit, for, fork, listen, raise, run, set, switch, try, and wait.
#[derive(Default)]
pub struct TaskDefinitionBuilder {
    builder: Option<TaskBuilderVariant>,
}

impl TaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    task_variant_method!(call, Call, CallFunctionTaskDefinitionBuilder, function: &str);
    task_variant_method!(do_, Do, DoTaskDefinitionBuilder);
    task_variant_method!(for_, For, ForTaskDefinitionBuilder);
    task_variant_method!(fork, Fork, ForkTaskDefinitionBuilder);
    task_variant_method!(listen, Listen, ListenTaskDefinitionBuilder);
    task_variant_method!(raise, Raise, RaiseTaskDefinitionBuilder);
    task_variant_method!(run, Run, RunTaskDefinitionBuilder);
    task_variant_method!(set, Set, SetTaskDefinitionBuilder);
    task_variant_method!(switch, Switch, SwitchTaskDefinitionBuilder);
    task_variant_method!(try_, Try, TryTaskDefinitionBuilder);
    task_variant_method!(wait, Wait, WaitTaskDefinitionBuilder, duration: OneOfDurationOrIso8601Expression);

    /// Creates an emit task that produces a workflow event.
    pub fn emit<F>(&mut self, setup: F) -> &mut EmitTaskDefinitionBuilder
    where
        F: FnOnce(&mut EventDefinitionBuilder),
    {
        let mut event_builder = EventDefinitionBuilder::new();
        setup(&mut event_builder);
        let event = event_builder.build();
        let builder = EmitTaskDefinitionBuilder::new(event);
        self.builder = Some(TaskBuilderVariant::Emit(Box::new(builder)));
        match &mut self.builder {
            Some(TaskBuilderVariant::Emit(ref mut builder)) => builder,
            _ => unreachable!("Builder should always be set to Emit"),
        }
    }

    /// Builds the final `TaskDefinition`, panicking if no task type was configured.
    pub fn build(self) -> TaskDefinition {
        if let Some(builder) = self.builder {
            match builder {
                TaskBuilderVariant::Call(builder) => builder.build(),
                TaskBuilderVariant::Do(builder) => builder.build(),
                TaskBuilderVariant::Emit(builder) => builder.build(),
                TaskBuilderVariant::For(builder) => builder.build(),
                TaskBuilderVariant::Fork(builder) => builder.build(),
                TaskBuilderVariant::Listen(builder) => builder.build(),
                TaskBuilderVariant::Raise(builder) => builder.build(),
                TaskBuilderVariant::Run(builder) => builder.build(),
                TaskBuilderVariant::Set(builder) => builder.build(),
                TaskBuilderVariant::Switch(builder) => builder.build(),
                TaskBuilderVariant::Try(builder) => builder.build(),
                TaskBuilderVariant::Wait(builder) => builder.build(),
            }
        } else {
            panic!("The task must be configured");
        }
    }
}

/// Internal enum tracking which task type variant is being built.
pub(crate) enum TaskBuilderVariant {
    /// Call function task builder.
    Call(Box<CallFunctionTaskDefinitionBuilder>),
    /// Do (sequential) task builder.
    Do(Box<DoTaskDefinitionBuilder>),
    /// Emit event task builder.
    Emit(Box<EmitTaskDefinitionBuilder>),
    /// For loop task builder.
    For(Box<ForTaskDefinitionBuilder>),
    /// Fork (parallel) task builder.
    Fork(Box<ForkTaskDefinitionBuilder>),
    /// Listen event task builder.
    Listen(Box<ListenTaskDefinitionBuilder>),
    /// Raise error task builder.
    Raise(Box<RaiseTaskDefinitionBuilder>),
    /// Run process task builder.
    Run(Box<RunTaskDefinitionBuilder>),
    /// Set variables task builder.
    Set(Box<SetTaskDefinitionBuilder>),
    /// Switch conditional task builder.
    Switch(Box<SwitchTaskDefinitionBuilder>),
    /// Try-catch task builder.
    Try(Box<TryTaskDefinitionBuilder>),
    /// Wait delay task builder.
    Wait(Box<WaitTaskDefinitionBuilder>),
}

/// Common trait shared by all task definition builders, providing
/// conditional execution, timeout, input/output, export, and flow control methods.
#[allow(dead_code)]
pub(crate) trait TaskDefinitionBuilderBase {
    /// Sets a runtime condition that must be true for the task to execute.
    fn if_(&mut self, condition: &str) -> &mut Self;
    /// References a named timeout definition by name.
    fn with_timeout_reference(&mut self, reference: &str) -> &mut Self;
    /// Configures an inline timeout definition using a builder callback.
    fn with_timeout<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TimeoutDefinitionBuilder);
    /// Configures input data transformation using a builder callback.
    fn with_input<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut InputDataModelDefinitionBuilder);
    /// Configures output data transformation using a builder callback.
    fn with_output<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut OutputDataModelDefinitionBuilder);
    /// Configures exported output data using a builder callback.
    fn with_export<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut OutputDataModelDefinitionBuilder);
    /// Sets a flow directive to execute after this task completes (e.g., "continue", "end").
    fn then(&mut self, directive: &str) -> &mut Self;
    /// Builds the final `TaskDefinition`.
    fn build(self) -> TaskDefinition;
}
