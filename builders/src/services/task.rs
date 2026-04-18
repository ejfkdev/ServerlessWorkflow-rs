use serde_json::Value;
use serverless_workflow_core::models::authentication::{
    AuthenticationPolicyReference, ReferenceableAuthenticationPolicy,
};
use serverless_workflow_core::models::call::CallTaskDefinition;
use serverless_workflow_core::models::call::{
    A2AArguments, AsyncApiArguments, CallA2ADefinition, CallAsyncAPIDefinition,
    CallFunctionDefinition, CallGRPCDefinition, CallHTTPDefinition, CallOpenAPIDefinition,
    GRPCArguments, GRPCServiceDefinition, HTTPArguments, OneOfA2AParametersOrExpression,
    OneOfHeadersOrExpression, OneOfQueryOrExpression, OpenAPIArguments,
};
use serverless_workflow_core::models::duration::*;
use serverless_workflow_core::models::error::ErrorType;
use serverless_workflow_core::models::error::*;
use serverless_workflow_core::models::event::*;
use serverless_workflow_core::models::input::*;
use serverless_workflow_core::models::output::*;
use serverless_workflow_core::models::resource::{
    ExternalResourceDefinition, OneOfEndpointDefinitionOrUri,
};
use serverless_workflow_core::models::retry::*;
use serverless_workflow_core::models::task::*;
use serverless_workflow_core::models::timeout::*;
use std::collections::HashMap;

use super::timeout::TimeoutDefinitionBuilder;

// ============== InputDataModelDefinitionBuilder ==============
/// Builder for constructing an `InputDataModelDefinition` that configures task input data processing.
pub struct InputDataModelDefinitionBuilder {
    input: InputDataModelDefinition,
}

impl InputDataModelDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            input: InputDataModelDefinition::default(),
        }
    }

    /// Sets the `from` expression used to transform or filter task input data.
    pub fn from(&mut self, value: Value) -> &mut Self {
        self.input.from = Some(value);
        self
    }

    /// Builds the `InputDataModelDefinition`.
    pub fn build(self) -> InputDataModelDefinition {
        self.input
    }
}

impl Default for InputDataModelDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============== OutputDataModelDefinitionBuilder ==============
/// Builder for constructing an `OutputDataModelDefinition` that configures task output data processing.
pub struct OutputDataModelDefinitionBuilder {
    output: OutputDataModelDefinition,
}

impl OutputDataModelDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            output: OutputDataModelDefinition::default(),
        }
    }

    /// Sets the `as` expression used to transform task output data.
    pub fn r#as(&mut self, value: Value) -> &mut Self {
        self.output.as_ = Some(value);
        self
    }

    /// Builds the `OutputDataModelDefinition`.
    pub fn build(self) -> OutputDataModelDefinition {
        self.output
    }
}

impl Default for OutputDataModelDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============== EventDefinitionBuilder ==============
/// Builder for constructing an `EventDefinition` used in emit and listen tasks.
pub struct EventDefinitionBuilder {
    event: EventDefinition,
}

impl EventDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            event: EventDefinition::default(),
        }
    }

    /// Adds a single attribute to the event definition.
    pub fn with(&mut self, name: &str, value: Value) -> &mut Self {
        self.event.with.insert(name.to_string(), value);
        self
    }

    /// Replaces all event attributes with the provided map.
    pub fn with_attributes(&mut self, attrs: HashMap<String, Value>) -> &mut Self {
        self.event.with = attrs;
        self
    }

    /// Builds the `EventDefinition`.
    pub fn build(self) -> EventDefinition {
        self.event
    }
}

impl Default for EventDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============== TaskDefinitionBuilder ==============
/// Primary entry point for building workflow task definitions.
///
/// Supports all task types: call, do, emit, for, fork, listen, raise, run, set, switch, try, and wait.
#[derive(Default)]
pub struct TaskDefinitionBuilder {
    builder: Option<TaskBuilderVariant>,
}

/// Macro to generate the `pub fn variant()` method for TaskDefinitionBuilder,
/// creating the builder, storing it, and returning a mutable reference.
macro_rules! task_variant_method {
    ($method:ident, $variant:ident, $builder:ident $(, $arg:ident: $arg_ty:ty)*) => {
        pub fn $method(&mut self $(, $arg: $arg_ty)*) -> &mut $builder {
            let builder = $builder::new($($arg),*);
            self.builder = Some(TaskBuilderVariant::$variant(Box::new(builder)));
            match &mut self.builder {
                Some(TaskBuilderVariant::$variant(ref mut builder)) => builder,
                _ => unreachable!(concat!("Builder should always be set to ", stringify!($variant))),
            }
        }
    };
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

/// Implements `TaskDefinitionBuilderBase` for a builder struct, generating
/// the 7 common methods (if_, with_timeout_reference, with_timeout,
/// with_input, with_output, with_export, then) that all delegate to
/// `self.$field.common`. The `build()` method is unique per builder
/// and must be provided via the `$build_expr` expression, which will
/// be evaluated in a context where `v` holds the inner field value.
macro_rules! impl_task_definition_builder_base {
    ($builder:ident, $field:ident, $build_expr:expr) => {
        impl TaskDefinitionBuilderBase for $builder {
            fn if_(&mut self, condition: &str) -> &mut Self {
                self.$field.common.if_ = Some(condition.to_string());
                self
            }

            fn with_timeout_reference(&mut self, reference: &str) -> &mut Self {
                self.$field.common.timeout = Some(OneOfTimeoutDefinitionOrReference::Reference(
                    reference.to_string(),
                ));
                self
            }

            fn with_timeout<F>(&mut self, setup: F) -> &mut Self
            where
                F: FnOnce(&mut TimeoutDefinitionBuilder),
            {
                let mut builder = TimeoutDefinitionBuilder::new();
                setup(&mut builder);
                let timeout = builder.build();
                self.$field.common.timeout =
                    Some(OneOfTimeoutDefinitionOrReference::Timeout(timeout));
                self
            }

            fn with_input<F>(&mut self, setup: F) -> &mut Self
            where
                F: FnOnce(&mut InputDataModelDefinitionBuilder),
            {
                let mut builder = InputDataModelDefinitionBuilder::new();
                setup(&mut builder);
                self.$field.common.input = Some(builder.build());
                self
            }

            fn with_output<F>(&mut self, setup: F) -> &mut Self
            where
                F: FnOnce(&mut OutputDataModelDefinitionBuilder),
            {
                let mut builder = OutputDataModelDefinitionBuilder::new();
                setup(&mut builder);
                self.$field.common.output = Some(builder.build());
                self
            }

            fn with_export<F>(&mut self, setup: F) -> &mut Self
            where
                F: FnOnce(&mut OutputDataModelDefinitionBuilder),
            {
                let mut builder = OutputDataModelDefinitionBuilder::new();
                setup(&mut builder);
                self.$field.common.export = Some(builder.build());
                self
            }

            fn then(&mut self, directive: &str) -> &mut Self {
                self.$field.common.then = Some(directive.to_string());
                self
            }

            fn build(self) -> TaskDefinition {
                $build_expr(self.$field)
            }
        }
    };
}

// ============== CallFunctionTaskDefinitionBuilder ==============
/// Builder for constructing a call task that invokes a function.
pub struct CallFunctionTaskDefinitionBuilder {
    call_function: CallFunctionDefinition,
}

impl CallFunctionTaskDefinitionBuilder {
    pub fn new(function: &str) -> Self {
        Self {
            call_function: CallFunctionDefinition {
                call: function.to_string(),
                with: None,
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Adds a single named argument to the function call.
    pub fn with(&mut self, name: &str, value: Value) -> &mut Self {
        self.call_function
            .with
            .get_or_insert_with(HashMap::new)
            .insert(name.to_string(), value);
        self
    }

    /// Replaces all function call arguments with the provided map.
    pub fn with_arguments(&mut self, arguments: HashMap<String, Value>) -> &mut Self {
        self.call_function.with = Some(arguments);
        self
    }
}

impl_task_definition_builder_base!(CallFunctionTaskDefinitionBuilder, call_function, |v| {
    TaskDefinition::Call(Box::new(CallTaskDefinition::Function(v)))
});

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

// ============== ListenTaskDefinitionBuilder ==============
/// Builder for constructing a listen task that waits for events.
#[derive(Default)]
pub struct ListenTaskDefinitionBuilder {
    task: ListenTaskDefinition,
}

impl ListenTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the event filter to listen for using a builder callback.
    pub fn to<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut EventFilterDefinitionBuilder),
    {
        let mut builder = EventFilterDefinitionBuilder::new();
        setup(&mut builder);
        let filter = builder.build();
        let strategy = EventConsumptionStrategyDefinition {
            one: Some(filter),
            ..Default::default()
        };
        self.task.listen.to = strategy;
        self
    }
}

impl_task_definition_builder_base!(ListenTaskDefinitionBuilder, task, |v| {
    TaskDefinition::Listen(Box::new(v))
});

// ============== EventFilterDefinitionBuilder ==============
/// Builder for constructing an `EventFilterDefinition` used in listen tasks.
pub struct EventFilterDefinitionBuilder {
    filter: EventFilterDefinition,
}

impl EventFilterDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            filter: EventFilterDefinition::default(),
        }
    }

    /// Adds a single attribute to the event filter.
    pub fn with(&mut self, name: &str, value: Value) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(HashMap::new)
            .insert(name.to_string(), value);
        self
    }

    /// Builds the `EventFilterDefinition`.
    pub fn build(self) -> EventFilterDefinition {
        self.filter
    }
}

impl Default for EventFilterDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============== RaiseErrorDefinitionBuilder ==============
/// Builder for constructing an error definition used in raise tasks.
pub struct RaiseErrorDefinitionBuilder {
    error: ErrorDefinition,
}

impl RaiseErrorDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            error: ErrorDefinition::default(),
        }
    }

    /// Sets the error type URI template.
    pub fn with_type(&mut self, type_: &str) -> &mut Self {
        self.error.type_ = ErrorType::uri_template(type_);
        self
    }

    /// Sets the error status value.
    pub fn with_status(&mut self, status: Value) -> &mut Self {
        self.error.status = status;
        self
    }

    /// Sets the error title.
    pub fn with_title(&mut self, title: &str) -> &mut Self {
        self.error.title = Some(title.to_string());
        self
    }

    /// Sets the error detail message.
    pub fn with_detail(&mut self, detail: &str) -> &mut Self {
        self.error.detail = Some(detail.to_string());
        self
    }

    /// Sets the error instance URI.
    pub fn with_instance(&mut self, instance: &str) -> &mut Self {
        self.error.instance = Some(instance.to_string());
        self
    }

    /// Builds the error definition as an inline definition (not a reference).
    pub fn build(self) -> OneOfErrorDefinitionOrReference {
        OneOfErrorDefinitionOrReference::Error(self.error)
    }
}

impl Default for RaiseErrorDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============== RaiseTaskDefinitionBuilder ==============
/// Builder for constructing a raise task that produces an error.
#[derive(Default)]
pub struct RaiseTaskDefinitionBuilder {
    task: RaiseTaskDefinition,
    error_builder: Option<RaiseErrorDefinitionBuilder>,
}

impl RaiseTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an inline error definition using a builder.
    pub fn error(&mut self) -> &mut RaiseErrorDefinitionBuilder {
        self.error_builder = Some(RaiseErrorDefinitionBuilder::new());
        self.get_error_builder()
    }

    /// References a named error definition instead of defining one inline.
    pub fn error_reference(&mut self, ref_name: &str) -> &mut Self {
        self.task.raise.error = OneOfErrorDefinitionOrReference::Reference(ref_name.to_string());
        self.error_builder = None;
        self
    }

    fn get_error_builder(&mut self) -> &mut RaiseErrorDefinitionBuilder {
        if let Some(ref mut builder) = self.error_builder {
            builder
        } else {
            unreachable!("Error builder should be set")
        }
    }
}

impl TaskDefinitionBuilderBase for RaiseTaskDefinitionBuilder {
    fn if_(&mut self, condition: &str) -> &mut Self {
        self.task.common.if_ = Some(condition.to_string());
        self
    }

    fn with_timeout_reference(&mut self, reference: &str) -> &mut Self {
        self.task.common.timeout = Some(OneOfTimeoutDefinitionOrReference::Reference(
            reference.to_string(),
        ));
        self
    }

    fn with_timeout<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TimeoutDefinitionBuilder),
    {
        let mut builder = TimeoutDefinitionBuilder::new();
        setup(&mut builder);
        let timeout = builder.build();
        self.task.common.timeout = Some(OneOfTimeoutDefinitionOrReference::Timeout(timeout));
        self
    }

    fn with_input<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut InputDataModelDefinitionBuilder),
    {
        let mut builder = InputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.input = Some(builder.build());
        self
    }

    fn with_output<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut OutputDataModelDefinitionBuilder),
    {
        let mut builder = OutputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.output = Some(builder.build());
        self
    }

    fn with_export<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut OutputDataModelDefinitionBuilder),
    {
        let mut builder = OutputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.export = Some(builder.build());
        self
    }

    fn then(&mut self, directive: &str) -> &mut Self {
        self.task.common.then = Some(directive.to_string());
        self
    }

    fn build(mut self) -> TaskDefinition {
        if let Some(error_builder) = self.error_builder.take() {
            self.task.raise.error = error_builder.build();
        }
        TaskDefinition::Raise(self.task)
    }
}

// ============== RunTaskDefinitionBuilder ==============
/// Internal enum tracking which process type variant is being built for a run task.
pub(crate) enum ProcessDefinitionBuilder {
    /// Container process builder.
    Container(Box<ContainerProcessDefinitionBuilder>),
    /// Script process builder.
    Script(Box<ScriptProcessDefinitionBuilder>),
    /// Shell command process builder.
    Shell(Box<ShellProcessDefinitionBuilder>),
    /// Sub-workflow process builder.
    Workflow(Box<WorkflowProcessDefinitionBuilder>),
}

/// Builder for constructing a run task that executes a container, script, shell command, or sub-workflow.
#[derive(Default)]
pub struct RunTaskDefinitionBuilder {
    task: RunTaskDefinition,
    builder: Option<ProcessDefinitionBuilder>,
}

/// Macro to generate process variant methods for RunTaskDefinitionBuilder.
macro_rules! process_variant_method {
    ($method:ident, $variant:ident, $builder:ident) => {
        pub fn $method(&mut self) -> &mut $builder {
            self.builder = Some(ProcessDefinitionBuilder::$variant(Box::default()));
            match &mut self.builder {
                Some(ProcessDefinitionBuilder::$variant(ref mut builder)) => builder,
                _ => unreachable!(concat!("Builder should always be set to ", stringify!($variant))),
            }
        }
    };
}

impl RunTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    process_variant_method!(container, Container, ContainerProcessDefinitionBuilder);
    process_variant_method!(script, Script, ScriptProcessDefinitionBuilder);
    process_variant_method!(shell, Shell, ShellProcessDefinitionBuilder);
    process_variant_method!(workflow, Workflow, WorkflowProcessDefinitionBuilder);
}

impl TaskDefinitionBuilderBase for RunTaskDefinitionBuilder {
    fn if_(&mut self, condition: &str) -> &mut Self {
        self.task.common.if_ = Some(condition.to_string());
        self
    }

    fn with_timeout_reference(&mut self, reference: &str) -> &mut Self {
        self.task.common.timeout = Some(OneOfTimeoutDefinitionOrReference::Reference(
            reference.to_string(),
        ));
        self
    }

    fn with_timeout<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TimeoutDefinitionBuilder),
    {
        let mut builder = TimeoutDefinitionBuilder::new();
        setup(&mut builder);
        let timeout = builder.build();
        self.task.common.timeout = Some(OneOfTimeoutDefinitionOrReference::Timeout(timeout));
        self
    }

    fn with_input<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut InputDataModelDefinitionBuilder),
    {
        let mut builder = InputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.input = Some(builder.build());
        self
    }

    fn with_output<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut OutputDataModelDefinitionBuilder),
    {
        let mut builder = OutputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.output = Some(builder.build());
        self
    }

    fn with_export<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut OutputDataModelDefinitionBuilder),
    {
        let mut builder = OutputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.export = Some(builder.build());
        self
    }

    fn then(&mut self, directive: &str) -> &mut Self {
        self.task.common.then = Some(directive.to_string());
        self
    }

    fn build(mut self) -> TaskDefinition {
        if let Some(builder) = self.builder.take() {
            match builder {
                ProcessDefinitionBuilder::Container(b) => self.task.run.container = Some(b.build()),
                ProcessDefinitionBuilder::Script(b) => self.task.run.script = Some(b.build()),
                ProcessDefinitionBuilder::Shell(b) => self.task.run.shell = Some(b.build()),
                ProcessDefinitionBuilder::Workflow(b) => self.task.run.workflow = Some(b.build()),
            }
        }
        TaskDefinition::Run(Box::new(self.task))
    }
}

/// Builder for constructing a container process definition.
pub struct ContainerProcessDefinitionBuilder {
    container: ContainerProcessDefinition,
}

impl ContainerProcessDefinitionBuilder {
    fn new() -> Self {
        Self {
            container: ContainerProcessDefinition::default(),
        }
    }

    /// Sets the container image to run.
    pub fn with_image(&mut self, image: &str) -> &mut Self {
        self.container.image = image.to_string();
        self
    }

    /// Sets the command to execute inside the container.
    pub fn with_command(&mut self, command: &str) -> &mut Self {
        self.container.command = Some(command.to_string());
        self
    }

    /// Sets the port mappings for the container.
    pub fn with_ports(&mut self, ports: HashMap<String, serde_json::Value>) -> &mut Self {
        self.container.ports = Some(ports);
        self
    }

    /// Sets the volume mounts for the container.
    pub fn with_volumes(&mut self, volumes: HashMap<String, serde_json::Value>) -> &mut Self {
        self.container.volumes = Some(volumes);
        self
    }

    /// Replaces all environment variables with the provided map.
    pub fn with_environment_variables(&mut self, env: HashMap<String, String>) -> &mut Self {
        self.container.environment = Some(env);
        self
    }

    /// Adds a single environment variable to the container.
    pub fn with_env(&mut self, key: &str, value: &str) -> &mut Self {
        self.container
            .environment
            .get_or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Builds the `ContainerProcessDefinition`.
    pub fn build(self) -> ContainerProcessDefinition {
        self.container
    }
}

impl Default for ContainerProcessDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a script process definition.
pub struct ScriptProcessDefinitionBuilder {
    script: ScriptProcessDefinition,
}

impl ScriptProcessDefinitionBuilder {
    fn new() -> Self {
        Self {
            script: ScriptProcessDefinition::default(),
        }
    }

    /// Sets the script source code to execute.
    pub fn with_code(&mut self, code: &str) -> &mut Self {
        self.script.code = Some(code.to_string());
        self
    }

    /// Builds the `ScriptProcessDefinition`.
    pub fn build(self) -> ScriptProcessDefinition {
        self.script
    }
}

impl Default for ScriptProcessDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a shell process definition.
pub struct ShellProcessDefinitionBuilder {
    shell: ShellProcessDefinition,
}

impl ShellProcessDefinitionBuilder {
    fn new() -> Self {
        Self {
            shell: ShellProcessDefinition::default(),
        }
    }

    /// Sets the shell command to execute.
    pub fn with_command(&mut self, command: &str) -> &mut Self {
        self.shell.command = command.to_string();
        self
    }

    /// Sets the shell command arguments.
    pub fn with_arguments(&mut self, arguments: OneOfRunArguments) -> &mut Self {
        self.shell.arguments = Some(arguments);
        self
    }

    /// Sets the shell command arguments as a key-value map.
    pub fn with_map_arguments(&mut self, args: HashMap<String, Value>) -> &mut Self {
        self.shell.arguments = Some(OneOfRunArguments::Map(args));
        self
    }

    /// Sets the shell command arguments as an ordered array of strings.
    pub fn with_array_arguments(&mut self, args: Vec<String>) -> &mut Self {
        self.shell.arguments = Some(OneOfRunArguments::Array(args));
        self
    }

    /// Builds the `ShellProcessDefinition`.
    pub fn build(self) -> ShellProcessDefinition {
        self.shell
    }
}

impl Default for ShellProcessDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a sub-workflow process definition.
pub struct WorkflowProcessDefinitionBuilder {
    workflow: WorkflowProcessDefinition,
}

impl WorkflowProcessDefinitionBuilder {
    fn new() -> Self {
        Self {
            workflow: WorkflowProcessDefinition::default(),
        }
    }

    /// Sets the namespace of the target workflow.
    pub fn with_namespace(&mut self, namespace: &str) -> &mut Self {
        self.workflow.namespace = namespace.to_string();
        self
    }

    /// Sets the name of the target workflow.
    pub fn with_name(&mut self, name: &str) -> &mut Self {
        self.workflow.name = name.to_string();
        self
    }

    /// Sets the version of the target workflow.
    pub fn with_version(&mut self, version: &str) -> &mut Self {
        self.workflow.version = version.to_string();
        self
    }

    /// Sets the input data for the target workflow.
    pub fn with_input(&mut self, input: Value) -> &mut Self {
        self.workflow.input = Some(input);
        self
    }

    /// Builds the `WorkflowProcessDefinition`.
    pub fn build(self) -> WorkflowProcessDefinition {
        self.workflow
    }
}

impl Default for WorkflowProcessDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============== SetTaskDefinitionBuilder ==============
/// Builder for constructing a set task that assigns variables in the workflow state.
#[derive(Default)]
pub struct SetTaskDefinitionBuilder {
    task: SetTaskDefinition,
}

impl SetTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces all set variables with the provided map.
    pub fn variables(&mut self, vars: HashMap<String, Value>) -> &mut Self {
        self.task.set = SetValue::Map(vars);
        self
    }

    /// Adds or updates a single variable in the set task.
    pub fn put(&mut self, key: &str, value: Value) -> &mut Self {
        match &mut self.task.set {
            SetValue::Map(map) => {
                map.insert(key.to_string(), value);
            }
            _ => {
                let mut map = HashMap::new();
                map.insert(key.to_string(), value);
                self.task.set = SetValue::Map(map);
            }
        }
        self
    }
}

impl_task_definition_builder_base!(SetTaskDefinitionBuilder, task, TaskDefinition::Set);

// ============== SwitchTaskDefinitionBuilder ==============
/// Builder for constructing a switch task that selects a branch based on conditions.
#[derive(Default)]
pub struct SwitchTaskDefinitionBuilder {
    task: SwitchTaskDefinition,
}

impl SwitchTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a named switch case using a builder callback.
    pub fn case_<F>(&mut self, name: &str, setup: F) -> &mut Self
    where
        F: FnOnce(&mut SwitchCaseDefinitionBuilder),
    {
        let mut builder = SwitchCaseDefinitionBuilder::new();
        setup(&mut builder);
        let case_def = builder.build();
        self.task.switch.add(name.to_string(), case_def);
        self
    }
}

impl_task_definition_builder_base!(SwitchTaskDefinitionBuilder, task, |v| {
    TaskDefinition::Switch(v)
});

/// Builder for constructing a switch case definition.
pub struct SwitchCaseDefinitionBuilder {
    case_def: SwitchCaseDefinition,
}

impl SwitchCaseDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            case_def: SwitchCaseDefinition::default(),
        }
    }

    /// Sets the condition expression for this case.
    pub fn when(&mut self, when: &str) -> &mut Self {
        self.case_def.when = Some(when.to_string());
        self
    }

    /// Sets the flow directive to execute when this case matches.
    pub fn then(&mut self, then: &str) -> &mut Self {
        self.case_def.then = Some(then.to_string());
        self
    }

    /// Builds the `SwitchCaseDefinition`.
    pub fn build(self) -> SwitchCaseDefinition {
        self.case_def
    }
}

impl Default for SwitchCaseDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============== TryTaskDefinitionBuilder ==============
/// Builder for constructing a try task with error catching.
#[derive(Default)]
pub struct TryTaskDefinitionBuilder {
    task: TryTaskDefinition,
}

impl TryTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a named sub-task to the try block.
    pub fn do_<F>(&mut self, name: &str, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TaskDefinitionBuilder),
    {
        let mut builder = TaskDefinitionBuilder::new();
        setup(&mut builder);
        let task = builder.build();
        self.task.try_.add(name.to_string(), task);
        self
    }

    /// Configures the error catcher for this try task using a builder callback.
    pub fn catch<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut ErrorCatcherDefinitionBuilder),
    {
        let mut builder = ErrorCatcherDefinitionBuilder::new(&mut self.task);
        setup(&mut builder);
        self
    }
}

impl_task_definition_builder_base!(TryTaskDefinitionBuilder, task, TaskDefinition::Try);

/// Builder for constructing an error catcher within a try task.
pub struct ErrorCatcherDefinitionBuilder<'a> {
    parent: &'a mut TryTaskDefinition,
}

impl<'a> ErrorCatcherDefinitionBuilder<'a> {
    fn new(parent: &'a mut TryTaskDefinition) -> Self {
        parent.catch = ErrorCatcherDefinition::default();
        Self { parent }
    }

    /// Configures the error filter for this catcher using a builder callback.
    pub fn errors<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut ErrorFilterDefinitionBuilder),
    {
        let mut builder = ErrorFilterDefinitionBuilder::new();
        setup(&mut builder);
        self.parent.catch.errors = Some(builder.build());
        self
    }

    /// Sets the condition expression for when the catcher applies.
    pub fn when(&mut self, when: &str) -> &mut Self {
        self.parent.catch.when = Some(when.to_string());
        self
    }

    /// Configures the retry policy for this catcher using a builder callback.
    pub fn retry<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut RetryPolicyDefinitionBuilder),
    {
        let mut builder = RetryPolicyDefinitionBuilder::new();
        setup(&mut builder);
        self.parent.catch.retry = Some(OneOfRetryPolicyDefinitionOrReference::Retry(
            Box::new(builder.build()),
        ));
        self
    }

    /// Sets the variable name to store the caught error.
    pub fn as_(&mut self, as_var: &str) -> &mut Self {
        self.parent.catch.as_ = Some(as_var.to_string());
        self
    }

    /// Sets the condition expression for when the catcher should not apply.
    pub fn except_when(&mut self, except_when: &str) -> &mut Self {
        self.parent.catch.except_when = Some(except_when.to_string());
        self
    }

    /// Adds a named sub-task to execute when the error is caught.
    pub fn do_<F>(&mut self, name: &str, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TaskDefinitionBuilder),
    {
        let mut builder = TaskDefinitionBuilder::new();
        setup(&mut builder);
        let task = builder.build();
        self.parent
            .catch
            .do_
            .get_or_insert_with(serverless_workflow_core::models::map::Map::default)
            .add(name.to_string(), task);
        self
    }
}

/// Builder for constructing an error filter definition.
pub struct ErrorFilterDefinitionBuilder {
    filter: ErrorFilterDefinition,
}

impl ErrorFilterDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            filter: ErrorFilterDefinition::default(),
        }
    }

    /// Sets the error type to match.
    pub fn with_type(&mut self, type_: &str) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(ErrorFilterProperties::default)
            .type_ = Some(type_.to_string());
        self
    }

    /// Sets the error status to match.
    pub fn with_status(&mut self, status: Value) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(ErrorFilterProperties::default)
            .status = Some(status);
        self
    }

    /// Sets the error instance to match.
    pub fn with_instance(&mut self, instance: &str) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(ErrorFilterProperties::default)
            .instance = Some(instance.to_string());
        self
    }

    /// Sets the error title to match.
    pub fn with_title(&mut self, title: &str) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(ErrorFilterProperties::default)
            .title = Some(title.to_string());
        self
    }

    /// Sets the error detail to match.
    pub fn with_detail(&mut self, details: &str) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(ErrorFilterProperties::default)
            .detail = Some(details.to_string());
        self
    }

    /// Builds the `ErrorFilterDefinition`.
    pub fn build(self) -> ErrorFilterDefinition {
        self.filter
    }
}

impl Default for ErrorFilterDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a retry policy definition.
pub struct RetryPolicyDefinitionBuilder {
    policy: RetryPolicyDefinition,
}

impl RetryPolicyDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            policy: RetryPolicyDefinition::default(),
        }
    }

    /// Sets the condition for when the retry policy should not apply.
    pub fn except_when(&mut self, except_when: &str) -> &mut Self {
        self.policy.except_when = Some(except_when.to_string());
        self
    }

    /// Sets the condition for when the retry policy applies.
    pub fn when(&mut self, when: &str) -> &mut Self {
        self.policy.when = Some(when.to_string());
        self
    }

    /// Configures the retry limit using a builder callback.
    pub fn limit<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut RetryLimitDefinitionBuilder),
    {
        let mut builder = RetryLimitDefinitionBuilder::new();
        setup(&mut builder);
        self.policy.limit = Some(builder.build());
        self
    }

    /// Sets the initial delay before the first retry.
    pub fn delay(&mut self, delay: Duration) -> &mut Self {
        self.policy.delay = Some(OneOfDurationOrIso8601Expression::Duration(delay));
        self
    }

    /// Configures the backoff strategy using a builder callback.
    pub fn backoff<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut BackoffStrategyDefinitionBuilder),
    {
        let mut builder = BackoffStrategyDefinitionBuilder::new();
        setup(&mut builder);
        self.policy.backoff = Some(builder.build());
        self
    }

    /// Configures the jitter settings using a builder callback.
    pub fn jitter<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut JitterDefinitionBuilder),
    {
        let mut builder = JitterDefinitionBuilder::new();
        setup(&mut builder);
        self.policy.jitter = Some(builder.build());
        self
    }

    /// Builds the `RetryPolicyDefinition`.
    pub fn build(self) -> RetryPolicyDefinition {
        self.policy
    }
}

impl Default for RetryPolicyDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a retry policy limit definition.
pub struct RetryLimitDefinitionBuilder {
    limit: RetryPolicyLimitDefinition,
}

impl RetryLimitDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            limit: RetryPolicyLimitDefinition::default(),
        }
    }

    /// Sets the maximum number of retry attempts.
    pub fn attempt_count(&mut self, count: u16) -> &mut Self {
        self.limit
            .attempt
            .get_or_insert_with(RetryAttemptLimitDefinition::default)
            .count = Some(count);
        self
    }

    /// Sets the maximum duration for a single retry attempt.
    pub fn attempt_duration(&mut self, duration: Duration) -> &mut Self {
        self.limit
            .attempt
            .get_or_insert_with(RetryAttemptLimitDefinition::default)
            .duration = Some(OneOfDurationOrIso8601Expression::Duration(duration));
        self
    }

    /// Sets the overall maximum duration for all retries combined.
    pub fn duration(&mut self, duration: Duration) -> &mut Self {
        self.limit.duration = Some(OneOfDurationOrIso8601Expression::Duration(duration));
        self
    }

    /// Builds the `RetryPolicyLimitDefinition`.
    pub fn build(self) -> RetryPolicyLimitDefinition {
        self.limit
    }
}

impl Default for RetryLimitDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a backoff strategy definition.
pub struct BackoffStrategyDefinitionBuilder {
    strategy: BackoffStrategyDefinition,
    linear_builder: Option<LinearBackoffDefinitionBuilder>,
    constant_builder: Option<ConstantBackoffDefinitionBuilder>,
    exponential_builder: Option<ExponentialBackoffDefinitionBuilder>,
}

impl BackoffStrategyDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            strategy: BackoffStrategyDefinition::default(),
            linear_builder: None,
            constant_builder: None,
            exponential_builder: None,
        }
    }

    /// Configures a linear backoff strategy.
    pub fn linear(&mut self) -> &mut LinearBackoffDefinitionBuilder {
        self.linear_builder = Some(LinearBackoffDefinitionBuilder::new());
        self.get_linear_builder()
    }

    fn get_linear_builder(&mut self) -> &mut LinearBackoffDefinitionBuilder {
        if let Some(ref mut builder) = self.linear_builder {
            builder
        } else {
            unreachable!("Linear builder should be set")
        }
    }

    /// Configures a constant backoff strategy.
    pub fn constant(&mut self) -> &mut ConstantBackoffDefinitionBuilder {
        self.constant_builder = Some(ConstantBackoffDefinitionBuilder::new());
        self.get_constant_builder()
    }

    fn get_constant_builder(&mut self) -> &mut ConstantBackoffDefinitionBuilder {
        if let Some(ref mut builder) = self.constant_builder {
            builder
        } else {
            unreachable!("Constant builder should be set")
        }
    }

    /// Configures an exponential backoff strategy.
    pub fn exponential(&mut self) -> &mut ExponentialBackoffDefinitionBuilder {
        self.exponential_builder = Some(ExponentialBackoffDefinitionBuilder::new());
        self.get_exponential_builder()
    }

    fn get_exponential_builder(&mut self) -> &mut ExponentialBackoffDefinitionBuilder {
        if let Some(ref mut builder) = self.exponential_builder {
            builder
        } else {
            unreachable!("Exponential builder should be set")
        }
    }

    /// Builds the `BackoffStrategyDefinition`.
    pub fn build(mut self) -> BackoffStrategyDefinition {
        if let Some(builder) = self.linear_builder.take() {
            self.strategy.linear = Some(builder.build());
        }
        if let Some(builder) = self.constant_builder.take() {
            self.strategy.constant = Some(builder.build());
        }
        if let Some(builder) = self.exponential_builder.take() {
            self.strategy.exponential = Some(builder.build());
        }
        self.strategy
    }
}

impl Default for BackoffStrategyDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a linear backoff definition.
pub struct LinearBackoffDefinitionBuilder {
    linear: LinearBackoffDefinition,
}

impl LinearBackoffDefinitionBuilder {
    fn new() -> Self {
        Self {
            linear: LinearBackoffDefinition::default(),
        }
    }

    /// Sets the increment duration added on each retry.
    pub fn with_increment(&mut self, increment: Duration) -> &mut Self {
        self.linear.increment = Some(increment);
        self
    }

    /// Builds the `LinearBackoffDefinition`.
    pub fn build(self) -> LinearBackoffDefinition {
        self.linear
    }
}

impl Default for LinearBackoffDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a constant backoff definition.
pub struct ConstantBackoffDefinitionBuilder {
    constant: ConstantBackoffDefinition,
}

impl ConstantBackoffDefinitionBuilder {
    fn new() -> Self {
        Self {
            constant: ConstantBackoffDefinition::default(),
        }
    }

    /// Sets the constant delay duration between retries.
    pub fn with_delay(&mut self, delay: &str) -> &mut Self {
        self.constant = ConstantBackoffDefinition::with_delay(delay);
        self
    }

    /// Builds the `ConstantBackoffDefinition`.
    pub fn build(self) -> ConstantBackoffDefinition {
        self.constant
    }
}

impl Default for ConstantBackoffDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing an exponential backoff definition.
pub struct ExponentialBackoffDefinitionBuilder {
    exponential: ExponentialBackoffDefinition,
}

impl ExponentialBackoffDefinitionBuilder {
    fn new() -> Self {
        Self {
            exponential: ExponentialBackoffDefinition::default(),
        }
    }

    /// Sets the exponential growth factor.
    pub fn with_factor(&mut self, factor: f64) -> &mut Self {
        self.exponential = ExponentialBackoffDefinition::with_factor(factor);
        self
    }

    /// Sets the exponential growth factor and maximum delay cap.
    pub fn with_factor_and_max_delay(&mut self, factor: f64, max_delay: &str) -> &mut Self {
        self.exponential =
            ExponentialBackoffDefinition::with_factor_and_max_delay(factor, max_delay);
        self
    }

    /// Builds the `ExponentialBackoffDefinition`.
    pub fn build(self) -> ExponentialBackoffDefinition {
        self.exponential
    }
}

impl Default for ExponentialBackoffDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a jitter definition for retry delays.
pub struct JitterDefinitionBuilder {
    jitter: JitterDefinition,
}

impl JitterDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            jitter: JitterDefinition::default(),
        }
    }

    /// Sets the minimum jitter duration.
    pub fn from(&mut self, from: Duration) -> &mut Self {
        self.jitter.from = from;
        self
    }

    /// Sets the maximum jitter duration.
    pub fn to(&mut self, to: Duration) -> &mut Self {
        self.jitter.to = to;
        self
    }

    /// Builds the `JitterDefinition`.
    pub fn build(self) -> JitterDefinition {
        self.jitter
    }
}

impl Default for JitterDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

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

// ============== CallHTTPDefinitionBuilder ==============
/// Builder for constructing an HTTP call task.
pub struct CallHTTPDefinitionBuilder {
    task: CallHTTPDefinition,
}

impl CallHTTPDefinitionBuilder {
    pub fn new(method: &str, endpoint: &str) -> Self {
        Self {
            task: CallHTTPDefinition {
                call: "http".to_string(),
                with: HTTPArguments {
                    method: method.to_string(),
                    endpoint: OneOfEndpointDefinitionOrUri::Uri(endpoint.to_string()),
                    headers: None,
                    body: None,
                    query: None,
                    output: None,
                    redirect: None,
                },
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Adds an HTTP header to the request.
    pub fn with_header(&mut self, name: &str, value: &str) -> &mut Self {
        if let OneOfHeadersOrExpression::Map(headers) = self
            .task
            .with
            .headers
            .get_or_insert_with(|| OneOfHeadersOrExpression::Map(HashMap::new()))
        {
            headers.insert(name.to_string(), value.to_string());
        }
        self
    }

    /// Sets the HTTP request body.
    pub fn with_body(&mut self, body: Value) -> &mut Self {
        self.task.with.body = Some(body);
        self
    }

    /// Adds a query parameter to the request.
    pub fn with_query(&mut self, name: &str, value: &str) -> &mut Self {
        if let OneOfQueryOrExpression::Map(query) = self
            .task
            .with
            .query
            .get_or_insert_with(|| OneOfQueryOrExpression::Map(HashMap::new()))
        {
            query.insert(name.to_string(), value.to_string());
        }
        self
    }

    /// Adds multiple query parameters from a map.
    pub fn with_query_map(&mut self, map: HashMap<String, String>) -> &mut Self {
        if let OneOfQueryOrExpression::Map(query) = self
            .task
            .with
            .query
            .get_or_insert_with(|| OneOfQueryOrExpression::Map(HashMap::new()))
        {
            query.extend(map);
        }
        self
    }

    /// Sets query parameters from a runtime expression (e.g., `"${ .queryParams }"`).
    pub fn with_query_expression(&mut self, expr: &str) -> &mut Self {
        self.task.with.query = Some(OneOfQueryOrExpression::Expression(expr.to_string()));
        self
    }

    /// Sets the output format for the HTTP response.
    pub fn with_output_format(&mut self, output: &str) -> &mut Self {
        self.task.with.output = Some(output.to_string());
        self
    }

    /// Sets whether HTTP redirects should be followed.
    pub fn with_redirect(&mut self, redirect: bool) -> &mut Self {
        self.task.with.redirect = Some(redirect);
        self
    }
}

impl_task_definition_builder_base!(CallHTTPDefinitionBuilder, task, |v| TaskDefinition::Call(
    Box::new(CallTaskDefinition::HTTP(v))
));

// ============== CallGRPCDefinitionBuilder ==============
/// Builder for constructing a gRPC call task.
pub struct CallGRPCDefinitionBuilder {
    task: CallGRPCDefinition,
}

impl CallGRPCDefinitionBuilder {
    pub fn new(proto_url: &str, service_name: &str, method: &str) -> Self {
        Self {
            task: CallGRPCDefinition {
                call: "grpc".to_string(),
                with: GRPCArguments {
                    proto: ExternalResourceDefinition {
                        name: None,
                        endpoint: OneOfEndpointDefinitionOrUri::Uri(proto_url.to_string()),
                    },
                    service: GRPCServiceDefinition {
                        name: service_name.to_string(),
                        ..Default::default()
                    },
                    method: method.to_string(),
                    arguments: None,
                    authentication: None,
                },
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Sets the gRPC service host.
    pub fn with_host(&mut self, host: &str) -> &mut Self {
        self.task.with.service.host = host.to_string();
        self
    }

    /// Sets the gRPC service port.
    pub fn with_port(&mut self, port: u16) -> &mut Self {
        self.task.with.service.port = Some(port);
        self
    }

    /// Adds a single named argument to the gRPC call.
    pub fn with_argument(&mut self, name: &str, value: Value) -> &mut Self {
        self.task
            .with
            .arguments
            .get_or_insert_with(HashMap::new)
            .insert(name.to_string(), value);
        self
    }

    /// References a named authentication policy for the gRPC call.
    pub fn with_authentication_use(&mut self, auth_name: &str) -> &mut Self {
        self.task.with.authentication = Some(ReferenceableAuthenticationPolicy::Reference(
            AuthenticationPolicyReference {
                use_: auth_name.to_string(),
            },
        ));
        self
    }
}

impl_task_definition_builder_base!(CallGRPCDefinitionBuilder, task, |v| TaskDefinition::Call(
    Box::new(CallTaskDefinition::GRPC(Box::new(v)))
));

// ============== CallOpenAPIDefinitionBuilder ==============
/// Builder for constructing an OpenAPI call task.
pub struct CallOpenAPIDefinitionBuilder {
    task: CallOpenAPIDefinition,
}

impl CallOpenAPIDefinitionBuilder {
    pub fn new(document_url: &str, operation_id: &str) -> Self {
        Self {
            task: CallOpenAPIDefinition {
                call: "openapi".to_string(),
                with: OpenAPIArguments {
                    document: ExternalResourceDefinition {
                        name: None,
                        endpoint: OneOfEndpointDefinitionOrUri::Uri(document_url.to_string()),
                    },
                    operation_id: operation_id.to_string(),
                    parameters: None,
                    authentication: None,
                    output: None,
                    redirect: None,
                },
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Adds a single named parameter to the OpenAPI call.
    pub fn with_parameter(&mut self, name: &str, value: Value) -> &mut Self {
        self.task
            .with
            .parameters
            .get_or_insert_with(HashMap::new)
            .insert(name.to_string(), value);
        self
    }

    /// Sets the output format for the OpenAPI response.
    pub fn with_output_format(&mut self, output: &str) -> &mut Self {
        self.task.with.output = Some(output.to_string());
        self
    }
}

impl_task_definition_builder_base!(
    CallOpenAPIDefinitionBuilder,
    task,
    |v| TaskDefinition::Call(Box::new(CallTaskDefinition::OpenAPI(v)))
);

// ============== CallAsyncAPIDefinitionBuilder ==============
/// Builder for constructing an AsyncAPI call task.
pub struct CallAsyncAPIDefinitionBuilder {
    task: CallAsyncAPIDefinition,
}

impl CallAsyncAPIDefinitionBuilder {
    pub fn new(document_url: &str) -> Self {
        Self {
            task: CallAsyncAPIDefinition {
                call: "asyncapi".to_string(),
                with: AsyncApiArguments {
                    document: ExternalResourceDefinition {
                        name: None,
                        endpoint: OneOfEndpointDefinitionOrUri::Uri(document_url.to_string()),
                    },
                    channel: None,
                    operation: None,
                    server: None,
                    protocol: None,
                    message: None,
                    subscription: None,
                    authentication: None,
                },
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Sets the AsyncAPI channel name.
    pub fn with_channel(&mut self, channel: &str) -> &mut Self {
        self.task.with.channel = Some(channel.to_string());
        self
    }

    /// Sets the AsyncAPI operation.
    pub fn with_operation(&mut self, operation: &str) -> &mut Self {
        self.task.with.operation = Some(operation.to_string());
        self
    }

    /// Sets the AsyncAPI protocol (e.g., "kafka", "mqtt").
    pub fn with_protocol(&mut self, protocol: &str) -> &mut Self {
        self.task.with.protocol = Some(protocol.to_string());
        self
    }
}

impl_task_definition_builder_base!(CallAsyncAPIDefinitionBuilder, task, |v| {
    TaskDefinition::Call(Box::new(CallTaskDefinition::AsyncAPI(v)))
});

// ============== CallA2ADefinitionBuilder ==============
/// Builder for constructing an Agent-to-Agent (A2A) call task.
pub struct CallA2ADefinitionBuilder {
    task: CallA2ADefinition,
}

impl CallA2ADefinitionBuilder {
    pub fn new(method: &str) -> Self {
        Self {
            task: CallA2ADefinition {
                call: "a2a".to_string(),
                with: A2AArguments {
                    agent_card: None,
                    server: None,
                    method: method.to_string(),
                    parameters: None,
                },
                common: TaskDefinitionFields::default(),
            },
        }
    }

    /// Sets the agent card URL for the A2A call.
    pub fn with_agent_card(&mut self, url: &str) -> &mut Self {
        self.task.with.agent_card = Some(ExternalResourceDefinition {
            name: None,
            endpoint: OneOfEndpointDefinitionOrUri::Uri(url.to_string()),
        });
        self
    }

    /// Adds a single named parameter to the A2A call.
    pub fn with_parameter(&mut self, name: &str, value: Value) -> &mut Self {
        if let Some(ref mut params) = self.task.with.parameters {
            if let OneOfA2AParametersOrExpression::Map(ref mut map) = params {
                map.insert(name.to_string(), value);
            }
        } else {
            let mut map = HashMap::new();
            map.insert(name.to_string(), value);
            self.task.with.parameters = Some(OneOfA2AParametersOrExpression::Map(map));
        }
        self
    }
}

impl_task_definition_builder_base!(CallA2ADefinitionBuilder, task, |v| TaskDefinition::Call(
    Box::new(CallTaskDefinition::A2A(v))
));
