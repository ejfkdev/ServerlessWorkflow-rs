use crate::context::WorkflowContext;
use crate::error::{WorkflowError, WorkflowResult};
use crate::expression::{
    evaluate_optional_expr, traverse_and_evaluate_bool, traverse_and_evaluate_obj,
};
use crate::handler::HandlerRegistry;
use crate::json_schema::validate_schema;
use crate::listener::WorkflowEvent;
use crate::status::StatusPhase;
use crate::tasks::*;
use serde_json::Value;
use serverless_workflow_core::models::input::InputDataModelDefinition;
use serverless_workflow_core::models::output::OutputDataModelDefinition;
use serverless_workflow_core::models::task::{TaskDefinition, TaskDefinitionFields};
use serverless_workflow_core::models::timeout::OneOfTimeoutDefinitionOrReference;
use serverless_workflow_core::models::workflow::WorkflowDefinition;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Owned task support for concurrent branch execution (e.g., fork)
/// Unlike `TaskSupport` which borrows its context, this owns both
/// the workflow definition and context, making it `'static + Send`.
pub struct OwnedTaskSupport {
    /// Owned workflow definition
    pub workflow: WorkflowDefinition,
    /// Owned runtime context
    pub context: WorkflowContext,
}

impl OwnedTaskSupport {
    /// Creates an owned task support by cloning from a borrowed one
    pub fn from_support(support: &TaskSupport<'_>) -> Self {
        Self {
            workflow: support.workflow.clone(),
            context: support.context.clone(),
        }
    }

    /// Creates a temporary `TaskSupport` borrowing from this owned data
    pub fn as_task_support(&mut self) -> TaskSupport<'_> {
        TaskSupport::new(&self.workflow, &mut self.context)
    }
}

/// Asynchronous trait for executing a workflow task
#[async_trait::async_trait]
pub trait TaskRunner: Send + Sync {
    /// Executes the task with the given input and context support
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value>;

    /// Returns the name of this task
    fn task_name(&self) -> &str;
}

/// Provides shared context and workflow access for task runners
pub struct TaskSupport<'a> {
    /// The workflow definition being executed
    pub workflow: &'a WorkflowDefinition,
    /// The runtime context
    pub context: &'a mut WorkflowContext,
}

impl<'a> TaskSupport<'a> {
    /// Creates a new TaskSupport
    pub fn new(workflow: &'a WorkflowDefinition, context: &'a mut WorkflowContext) -> Self {
        Self { workflow, context }
    }

    /// Sets the status for a task
    pub fn set_task_status(&mut self, task: &str, status: StatusPhase) {
        self.context.set_task_status(task, status);
    }

    /// Sets the task name in the context
    pub fn set_task_name(&mut self, name: &str) {
        self.context.set_task_name(name);
    }

    /// Sets the task raw input in the context
    pub fn set_task_raw_input(&mut self, input: &Value) {
        self.context.set_task_raw_input(input);
    }

    /// Sets the task raw output in the context
    pub fn set_task_raw_output(&mut self, output: &Value) {
        self.context.set_task_raw_output(output);
    }

    /// Sets the task definition in the context
    pub fn set_task_def(&mut self, task: &Value) {
        self.context.set_task_def(task);
    }

    /// Sets the task startedAt timestamp
    pub fn set_task_started_at(&mut self) {
        self.context.set_task_started_at();
    }

    /// Sets the task reference from a task name using JSON Pointer
    pub fn set_task_reference_from_name(&mut self, name: &str) -> WorkflowResult<()> {
        let reference = self.context.get_workflow_json()
            .and_then(|json| crate::json_pointer::generate_json_pointer_from_value(json, name).ok())
            .unwrap_or_else(|| format!("/{}", name));
        self.context.set_task_reference(&reference);
        Ok(())
    }

    /// Gets the current task reference
    pub fn get_task_reference(&self) -> Option<&str> {
        self.context.get_task_reference()
    }

    /// Adds local expression variables
    pub fn add_local_expr_vars(&mut self, vars: HashMap<String, Value>) {
        self.context.add_local_expr_vars(vars);
    }

    /// Removes local expression variables
    pub fn remove_local_expr_vars(&mut self, keys: &[&str]) {
        self.context.remove_local_expr_vars(keys);
    }

    /// Emits an event to the listener if configured
    pub fn emit_event(&self, event: WorkflowEvent) {
        self.context.emit_event(event);
    }

    /// Sets the instance context ($context)
    pub fn set_instance_ctx(&mut self, value: Value) {
        self.context.set_instance_ctx(value);
    }

    /// Gets all variables for JQ expression evaluation
    pub fn get_vars(&self) -> HashMap<String, Value> {
        self.context.get_vars()
    }

    /// Gets the handler registry for custom call/run handlers
    pub fn get_handler_registry(&self) -> &HandlerRegistry {
        self.context.get_handler_registry()
    }

    /// Gets a cloned Arc to the event bus (for emit/listen tasks)
    pub fn clone_event_bus(&self) -> Option<crate::events::SharedEventBus> {
        self.context.clone_event_bus()
    }

    /// Checks if a task should run based on its `if` condition
    pub fn should_run_task(
        &self,
        if_condition: Option<&str>,
        input: &Value,
    ) -> WorkflowResult<bool> {
        match if_condition {
            None => Ok(true),
            Some(condition) => {
                let vars = self.get_vars();
                traverse_and_evaluate_bool(condition, input, &vars)
            }
        }
    }

    /// Processes task input: schema validation and expression transformation
    pub fn process_task_input(
        &self,
        input_def: Option<&InputDataModelDefinition>,
        input: &Value,
        task_name: &str,
    ) -> WorkflowResult<Value> {
        let input_def = match input_def {
            Some(def) => def,
            None => return Ok(input.clone()),
        };

        // Validate input schema
        if let Some(ref schema) = input_def.schema {
            validate_schema(input, schema, task_name)?;
        }

        // Transform input via from expression
        let vars = self.get_vars();
        traverse_and_evaluate_obj(input_def.from.as_ref(), input, &vars, task_name)
    }

    /// Processes task output: expression transformation and schema validation
    pub fn process_task_output(
        &self,
        output_def: Option<&OutputDataModelDefinition>,
        output: &Value,
        task_name: &str,
    ) -> WorkflowResult<Value> {
        let output_def = match output_def {
            Some(def) => def,
            None => return Ok(output.clone()),
        };

        // Transform output via as expression
        let vars = self.get_vars();
        let result = evaluate_optional_expr(output_def.as_.as_ref(), output, &vars, task_name)?;

        // Validate output schema
        if let Some(ref schema) = output_def.schema {
            validate_schema(&result, schema, task_name)?;
        }

        Ok(result)
    }

    /// Processes task export: sets the instance context
    pub fn process_task_export(
        &mut self,
        export_def: Option<&OutputDataModelDefinition>,
        output: &Value,
        task_name: &str,
    ) -> WorkflowResult<()> {
        let export_def = match export_def {
            Some(def) => def,
            None => return Ok(()),
        };

        let vars = self.get_vars();
        let result = evaluate_optional_expr(export_def.as_.as_ref(), output, &vars, task_name)?;

        if let Some(ref schema) = export_def.schema {
            validate_schema(&result, schema, task_name)?;
        }

        self.set_instance_ctx(result);
        Ok(())
    }

    /// Completes the task lifecycle after execution: output/export processing and cleanup.
    /// Must be called after `run_task_with_input_and_timeout` or equivalent execution.
    pub async fn execute_task_lifecycle(
        &mut self,
        task_name: &str,
        common: &TaskDefinitionFields,
        input: &Value,
        raw_output: Value,
    ) -> WorkflowResult<Value> {
        self.set_task_raw_output(&raw_output);

        // Process task output
        let output = self.process_task_output(common.output.as_ref(), &raw_output, task_name)?;

        // Process task export
        self.process_task_export(common.export.as_ref(), &output, task_name)?;

        // Clear per-task authorization context after export
        self.context.clear_authorization();

        self.emit_event(WorkflowEvent::TaskCompleted {
            instance_id: self.context.instance_id().to_string(),
            task_name: task_name.to_string(),
            output: output.clone(),
        });

        Ok(output)
    }

    /// Processes task input and handles timeout-wrapped execution.
    /// Returns the raw task output (before output/export processing).
    pub async fn run_task_with_input_and_timeout(
        &mut self,
        task_name: &str,
        common: &TaskDefinitionFields,
        input: &Value,
        runner: &dyn TaskRunner,
    ) -> WorkflowResult<Value> {
        // Set context before execution (needed for $task.name etc.)
        self.set_task_started_at();
        self.set_task_raw_input(input);
        self.set_task_name(task_name);

        self.emit_event(WorkflowEvent::TaskStarted {
            instance_id: self.context.instance_id().to_string(),
            task_name: task_name.to_string(),
        });

        // Process task input
        let task_input = self.process_task_input(common.input.as_ref(), input, task_name)?;

        // Execute the task (with optional timeout)
        if let Some(timeout) = common.timeout.as_ref() {
            let vars = self.get_vars();
            let duration = crate::utils::parse_duration_with_context(
                timeout,
                &task_input,
                &vars,
                task_name,
                Some(self.workflow),
            )?;
            match tokio::time::timeout(duration, runner.run(task_input, self)).await {
                Ok(result) => result,
                Err(_) => Err(WorkflowError::timeout(
                    format!("task '{}' timed out after {:?}", task_name, duration),
                    task_name,
                )),
            }
        } else {
            runner.run(task_input, self).await
        }
    }
}

/// Creates the appropriate TaskRunner for a given TaskDefinition
pub fn create_task_runner(
    name: &str,
    task: &TaskDefinition,
    workflow: &WorkflowDefinition,
) -> WorkflowResult<Box<dyn TaskRunner>> {
    match task {
        TaskDefinition::Do(t) => Ok(Box::new(DoTaskRunner::new(name, t)?)),
        TaskDefinition::Set(t) => Ok(Box::new(SetTaskRunner::new(name, t)?)),
        TaskDefinition::Wait(t) => Ok(Box::new(WaitTaskRunner::new(name, t)?)),
        TaskDefinition::Raise(t) => Ok(Box::new(RaiseTaskRunner::new(name, t, workflow)?)),
        TaskDefinition::For(t) => Ok(Box::new(ForTaskRunner::new(name, t)?)),
        TaskDefinition::Switch(t) => Ok(Box::new(SwitchTaskRunner::new(name, t)?)),
        TaskDefinition::Fork(t) => Ok(Box::new(ForkTaskRunner::new(name, t, workflow)?)),
        TaskDefinition::Try(t) => Ok(Box::new(TryTaskRunner::new(name, t, workflow)?)),
        TaskDefinition::Emit(t) => Ok(Box::new(EmitTaskRunner::new(name, t)?)),
        TaskDefinition::Listen(t) => Ok(Box::new(ListenTaskRunner::new(name, t)?)),
        TaskDefinition::Call(t) => Ok(Box::new(CallTaskRunner::new(name, t)?)),
        TaskDefinition::Run(t) => Ok(Box::new(RunTaskRunner::new(name, t)?)),
        TaskDefinition::Custom(t) => Ok(Box::new(CustomTaskRunner::new(name, t)?)),
    }
}
