use crate::context::WorkflowContext;
use crate::error::{WorkflowError, WorkflowResult};
use crate::expression::{traverse_and_evaluate, traverse_and_evaluate_obj};
use crate::handler::HandlerRegistry;
use crate::json_schema::validate_schema;
use crate::listener::WorkflowEvent;
use crate::status::StatusPhase;
use crate::tasks::*;
use serde_json::Value;
use std::collections::HashMap;
use swf_core::models::input::InputDataModelDefinition;
use swf_core::models::output::OutputDataModelDefinition;
use swf_core::models::task::{TaskDefinition, TaskDefinitionFields};
use swf_core::models::workflow::WorkflowDefinition;

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
        let reference = self
            .context
            .get_workflow_json()
            .and_then(|json| crate::json_pointer::generate_json_pointer_from_value(json, name).ok())
            .unwrap_or_else(|| format!("/{}", name));
        self.context.set_task_reference(&reference);
        Ok(())
    }

    /// Increments the iteration counter for a task and returns the new value.
    /// Called each time a task starts execution to track how many times it has run.
    pub fn inc_iteration(&mut self, task_name: &str) -> u32 {
        self.context.inc_iteration(task_name)
    }

    /// Sets the retry attempt count for the current task
    pub fn set_retry_attempt(&mut self, attempt: u32) {
        self.context.set_retry_attempt(attempt)
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

    /// Gets all variables for JQ expression evaluation (cached internally)
    pub fn get_vars(&self) -> HashMap<String, Value> {
        self.context.get_vars()
    }

    /// Evaluates a JQ expression against the given input using current context vars
    pub fn eval_jq(&self, expr: &str, input: &Value, task_name: &str) -> WorkflowResult<Value> {
        let vars = self.get_vars();
        crate::expression::evaluate_jq(expr, input, &vars)
            .map_err(|e| crate::error::WorkflowError::expression(format!("{}", e), task_name))
    }

    /// Evaluates a raw JQ expression string (with `${...}` wrapper) after sanitization.
    /// This combines `prepare_expression()` + `eval_jq()` into one call.
    pub fn eval_jq_expr(
        &self,
        raw_expr: &str,
        input: &Value,
        task_name: &str,
    ) -> WorkflowResult<Value> {
        let sanitized = crate::expression::prepare_expression(raw_expr);
        self.eval_jq(&sanitized, input, task_name)
    }

    /// Evaluates a boolean expression (e.g., when/if conditions)
    pub fn eval_bool(&self, expr: &str, input: &Value) -> WorkflowResult<bool> {
        let vars = self.get_vars();
        crate::expression::traverse_and_evaluate_bool(expr, input, &vars)
    }

    /// Evaluates an expression string, resolving runtime expressions
    pub fn eval_str(&self, expr: &str, input: &Value, task_name: &str) -> WorkflowResult<String> {
        let vars = self.get_vars();
        crate::expression::evaluate_expression_str(expr, input, &vars, task_name)
    }

    /// Recursively traverses a JSON structure and evaluates all runtime expressions in-place
    pub fn eval_traverse(&self, node: &mut Value, input: &Value) -> WorkflowResult<()> {
        let vars = self.get_vars();
        traverse_and_evaluate(node, input, &vars)
    }

    /// Evaluates an optional input `from` expression into a Value (for task input processing)
    pub fn eval_obj(
        &self,
        from: Option<&Value>,
        input: &Value,
        task_name: &str,
    ) -> WorkflowResult<Value> {
        let vars = self.get_vars();
        traverse_and_evaluate_obj(from, input, &vars, task_name)
    }

    /// Resolves a duration expression with current context vars
    pub fn eval_duration(
        &self,
        expr: &swf_core::models::duration::OneOfDurationOrIso8601Expression,
        input: &Value,
        task_name: &str,
    ) -> WorkflowResult<std::time::Duration> {
        let vars = self.get_vars();
        crate::utils::resolve_duration_expr(expr, input, &vars, task_name)
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
            Some(condition) => self.eval_bool(condition, input),
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
        match input_def.from {
            Some(ref from_val) => {
                crate::expression::evaluate_value_expr(from_val, input, &self.get_vars(), task_name)
            }
            None => Ok(input.clone()),
        }
    }

    /// Processes task output: expression transformation and schema validation.
    /// Accepts pre-computed vars to avoid redundant `get_vars()` calls in hot paths.
    fn process_task_output_with_vars(
        &self,
        output_def: Option<&OutputDataModelDefinition>,
        output: &Value,
        task_name: &str,
        vars: &HashMap<String, Value>,
    ) -> WorkflowResult<Value> {
        let output_def = match output_def {
            Some(def) => def,
            None => return Ok(output.clone()),
        };

        let result = match output_def.as_ {
            Some(ref as_val) => {
                crate::expression::evaluate_value_expr(as_val, output, vars, task_name)?
            }
            None => output.clone(),
        };

        if let Some(ref schema) = output_def.schema {
            validate_schema(&result, schema, task_name)?;
        }

        Ok(result)
    }

    /// Processes task output: expression transformation and schema validation
    pub fn process_task_output(
        &self,
        output_def: Option<&OutputDataModelDefinition>,
        output: &Value,
        task_name: &str,
    ) -> WorkflowResult<Value> {
        let vars = self.get_vars();
        self.process_task_output_with_vars(output_def, output, task_name, &vars)
    }

    /// Processes task export: expression transformation, schema validation, and instance context update.
    /// Reuses `process_task_output` for the expression evaluation and schema validation.
    pub fn process_task_export(
        &mut self,
        export_def: Option<&OutputDataModelDefinition>,
        output: &Value,
        task_name: &str,
    ) -> WorkflowResult<()> {
        if export_def.is_none() {
            return Ok(());
        }
        let result = self.process_task_output(export_def, output, task_name)?;
        self.set_instance_ctx(result);
        Ok(())
    }

    /// Completes the task lifecycle after execution: output/export processing and cleanup.
    /// Must be called after `run_task_with_input_and_timeout` or equivalent execution.
    pub async fn execute_task_lifecycle(
        &mut self,
        task_name: &str,
        common: &TaskDefinitionFields,
        _input: &Value,
        raw_output: Value,
    ) -> WorkflowResult<Value> {
        self.set_task_raw_output(&raw_output);

        // Compute vars once for both output and export processing
        let vars = self.get_vars();

        // Process task output
        let output = self.process_task_output_with_vars(
            common.output.as_ref(),
            &raw_output,
            task_name,
            &vars,
        )?;

        // Process task export (same expression evaluation as output)
        if common.export.is_some() {
            let export_result = self.process_task_output_with_vars(
                common.export.as_ref(),
                &output,
                task_name,
                &vars,
            )?;
            self.set_instance_ctx(export_result);
        }

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
        self.inc_iteration(task_name);

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
