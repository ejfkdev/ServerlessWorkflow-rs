use crate::context::{SuspendState, WorkflowContext};
use crate::error::{ErrorKind, WorkflowError, WorkflowResult};
use crate::events::SharedEventBus;
use crate::expression::{evaluate_value_expr, ExpressionEngine, ExpressionEngineRegistry};
use crate::handler::{CallHandler, CustomTaskHandler, HandlerRegistry, RunHandler};
use crate::json_schema::validate_schema;
use crate::listener::{WorkflowEvent, WorkflowExecutionListener};
use crate::secret::SecretManager;
use crate::status::StatusPhase;
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::tasks::DoTaskRunner;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use swf_core::models::task::TaskDefinition;
use swf_core::models::workflow::WorkflowDefinition;

/// A handle to control a running workflow from another task/thread
///
/// Allows suspending and resuming a workflow execution externally.
/// Obtain via `WorkflowRunner::handle()` before calling `run()`.
#[derive(Clone)]
pub struct WorkflowHandle {
    suspend_state: SuspendState,
}

/// A handle to a scheduled (recurring) workflow execution
///
/// Can be cancelled to stop the recurring schedule.
pub struct ScheduledWorkflow {
    join_handle: tokio::task::JoinHandle<()>,
    cancel_tx: tokio::sync::watch::Sender<bool>,
}

impl ScheduledWorkflow {
    /// Cancels the scheduled workflow execution
    pub fn cancel(&self) {
        let _ = self.cancel_tx.send(true);
    }

    /// Waits for the scheduled workflow to complete (after cancellation)
    pub async fn join(self) {
        let _ = self.join_handle.await;
    }
}

impl WorkflowHandle {
    /// Suspends the workflow. Returns true if suspended, false if already suspended.
    pub fn suspend(&self) -> bool {
        self.suspend_state.suspend()
    }

    /// Resumes a suspended workflow. Returns true if resumed, false if not suspended.
    pub fn resume(&self) -> bool {
        self.suspend_state.resume()
    }

    /// Checks if the workflow is currently suspended
    pub fn is_suspended(&self) -> bool {
        self.suspend_state.is_suspended()
    }
}

/// The main workflow runner that executes workflow definitions
pub struct WorkflowRunner {
    workflow: WorkflowDefinition,
    secret_manager: Option<Arc<dyn SecretManager>>,
    listener: Option<Arc<dyn WorkflowExecutionListener>>,
    event_bus: Option<SharedEventBus>,
    sub_workflows: HashMap<String, WorkflowDefinition>,
    functions: HashMap<String, TaskDefinition>,
    handler_registry: HandlerRegistry,
    expression_engines: ExpressionEngineRegistry,
    custom_vars: HashMap<String, Value>,
    suspend_state: SuspendState,
}

impl WorkflowRunner {
    /// Creates a new WorkflowRunner for the given workflow definition
    pub fn new(workflow: WorkflowDefinition) -> WorkflowResult<Self> {
        Ok(Self {
            workflow,
            secret_manager: None,
            listener: None,
            event_bus: None,
            sub_workflows: HashMap::new(),
            functions: HashMap::new(),
            handler_registry: HandlerRegistry::new(),
            expression_engines: ExpressionEngineRegistry::new(),
            custom_vars: HashMap::new(),
            suspend_state: SuspendState::new(),
        })
    }

    /// Sets the secret manager for $secret expression variable
    pub fn with_secret_manager(mut self, manager: Arc<dyn SecretManager>) -> Self {
        self.secret_manager = Some(manager);
        self
    }

    /// Sets the execution listener for workflow/task events
    pub fn with_listener(mut self, listener: Arc<dyn WorkflowExecutionListener>) -> Self {
        self.listener = Some(listener);
        self
    }

    /// Sets the event bus for emit/listen tasks
    pub fn with_event_bus(mut self, bus: SharedEventBus) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Registers a sub-workflow that can be invoked via `run: workflow`
    /// Keyed by "namespace/name/version"
    pub fn with_sub_workflow(mut self, workflow: WorkflowDefinition) -> Self {
        let doc = &workflow.document;
        let key = format!("{}/{}/{}", doc.namespace, doc.name, doc.version);
        self.sub_workflows.insert(key, workflow);
        self
    }

    /// Registers a custom call handler for a specific call type
    /// (e.g., "grpc", "openapi", "asyncapi", "a2a")
    pub fn with_call_handler(mut self, handler: Box<dyn CallHandler>) -> Self {
        self.handler_registry.register_call_handler(handler);
        self
    }

    /// Registers a custom run handler for a specific run type
    /// (e.g., "container", "script")
    pub fn with_run_handler(mut self, handler: Box<dyn RunHandler>) -> Self {
        self.handler_registry.register_run_handler(handler);
        self
    }

    /// Registers a named function definition for call.function resolution
    ///
    /// This allows registering external function definitions that can be
    /// referenced by `call: <functionName>` in workflows, similar to
    /// Java SDK's cataloged function mechanism.
    pub fn with_function(mut self, name: &str, task: TaskDefinition) -> Self {
        self.functions.insert(name.to_string(), task);
        self
    }

    /// Sets the entire handler registry (used for propagating handlers to child runners)
    pub fn with_handler_registry(mut self, registry: HandlerRegistry) -> Self {
        self.handler_registry = registry;
        self
    }

    /// Registers a custom task handler for a specific custom task type
    pub fn with_custom_task_handler(mut self, handler: Box<dyn CustomTaskHandler>) -> Self {
        self.handler_registry.register_custom_task_handler(handler);
        self
    }

    /// Registers a custom expression engine for a specific prefix (e.g., "cel", "js")
    ///
    /// Expressions prefixed with `engine_prefix:` will be routed to this engine.
    /// Unprefixed expressions default to JQ.
    pub fn with_expression_engine(mut self, engine: Arc<dyn ExpressionEngine>) -> Self {
        self.expression_engines.register(engine);
        self
    }

    /// Sets the expression engine registry (replaces all previously registered engines).
    /// Useful for propagating engines from a parent runner to a child runner.
    pub fn with_expression_engine_registry(mut self, registry: ExpressionEngineRegistry) -> Self {
        self.expression_engines = registry;
        self
    }

    /// Injects a custom variable into the JQ expression context.
    ///
    /// The variable will be available as `$name` in all expressions (input.from,
    /// output.as, switch conditions, etc.). This is useful for passing external
    /// configuration or environment information into the workflow.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use swf_runtime::WorkflowRunner;
    /// use serde_json::json;
    ///
    /// let runner = WorkflowRunner::new(workflow)
    ///     .expect("failed to create runner")
    ///     .with_variable("config", json!({"base_url": "https://api.example.com"}))
    ///     .with_variable("env", json!({"region": "us-east-1"}));
    /// ```
    pub fn with_variable(mut self, name: impl Into<String>, value: Value) -> Self {
        // Store as a sub-workflow-independent variable — we'll inject into context in run()
        // For simplicity, we use a dedicated field on WorkflowRunner
        self.custom_vars.insert(name.into(), value);
        self
    }

    /// Runs the workflow with the given input and returns the output
    pub async fn run(&self, input: Value) -> WorkflowResult<Value> {
        let span = tracing::info_span!(
            "workflow",
            name = %self.workflow.document.name,
            version = %self.workflow.document.version,
        );
        let _enter = span.enter();

        let mut context = WorkflowContext::new(&self.workflow)?;

        // Set secret manager if configured
        if let Some(ref mgr) = self.secret_manager {
            context.set_secret_manager(mgr.clone());
        }

        // Set listener if configured
        if let Some(ref listener) = self.listener {
            context.set_listener(listener.clone());
        }

        // Set sub-workflow registry
        if !self.sub_workflows.is_empty() {
            context.set_sub_workflows(self.sub_workflows.clone());
        }

        // Set event bus if configured
        if let Some(ref bus) = self.event_bus {
            context.set_event_bus(bus.clone());
        }

        // Set handler registry
        context.set_handler_registry(self.handler_registry.clone());

        // Set expression engines
        context.set_expression_engines(self.expression_engines.clone());

        // Inject custom variables into the JQ context
        if !self.custom_vars.is_empty() {
            context.add_local_expr_vars(self.custom_vars.clone());
        }

        // Set registered function definitions
        if !self.functions.is_empty() {
            context.set_functions(self.functions.clone());
        }

        // Share suspend/resume state with context
        context.set_suspend_state(self.suspend_state.clone());

        let instance_id = context.instance_id().to_string();
        tracing::info!(instance_id = %instance_id, "workflow started");

        // Handle schedule:after — delay before starting
        if let Some(ref schedule) = self.workflow.schedule {
            if let Some(ref after_duration) = schedule.after {
                let duration = crate::utils::duration_to_std(after_duration);
                if !duration.is_zero() {
                    context.set_status(StatusPhase::Waiting);
                    tokio::time::sleep(duration).await;
                }
            }
        }

        // Process input
        let processed_input = self.process_input(&input, &context)?;

        context.set_input(processed_input.clone());
        context.set_raw_input(&input);
        context.set_status(StatusPhase::Running);

        context.emit_event(WorkflowEvent::WorkflowStarted {
            instance_id: instance_id.clone(),
            input: processed_input.clone(),
        });

        // Run the top-level do tasks (with optional workflow timeout)
        let do_runner = DoTaskRunner::new_from_workflow(&self.workflow)?;

        // Resolve workflow timeout
        let workflow_timeout = self.resolve_workflow_timeout(&processed_input, &context);

        let mut support = TaskSupport::new(&self.workflow, &mut context);

        let run_result = if let Some(timeout_duration) = workflow_timeout {
            match tokio::time::timeout(
                timeout_duration,
                do_runner.run(processed_input, &mut support),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => {
                    // Cancel the context so any running wait points respond immediately
                    support.context.cancel();
                    support.context.set_status(StatusPhase::Faulted);
                    support.context.emit_event(WorkflowEvent::WorkflowFailed {
                        instance_id: instance_id.clone(),
                        error: "workflow timed out".to_string(),
                    });
                    return Err(WorkflowError::timeout(
                        format!("workflow timed out after {:?}", timeout_duration),
                        &self.workflow.document.name,
                    ));
                }
            }
        } else {
            do_runner.run(processed_input, &mut support).await
        };

        let output = match run_result {
            Ok(output) => output,
            Err(e) => {
                support.context.set_status(StatusPhase::Faulted);
                support.context.emit_event(WorkflowEvent::WorkflowFailed {
                    instance_id: instance_id.clone(),
                    error: format!("{}", e),
                });
                tracing::error!(instance_id = %instance_id, error = %e, "workflow failed");
                // Only set instance on Runtime errors, preserve error type for others
                if e.kind() == ErrorKind::Runtime {
                    let reference = support.get_task_reference().unwrap_or("/");
                    return Err(e.with_instance(reference));
                }
                return Err(e);
            }
        };

        support.context.clear_task_context();

        // Process output using TaskSupport (reuses shared output processing logic)
        let processed_output = support.process_task_output(
            self.workflow.output.as_ref(),
            &output,
            &self.workflow.document.name,
        )?;

        support.context.set_output(processed_output.clone());
        support.context.set_status(StatusPhase::Completed);

        support
            .context
            .emit_event(WorkflowEvent::WorkflowCompleted {
                instance_id: instance_id.clone(),
                output: processed_output.clone(),
            });

        tracing::info!(instance_id = %instance_id, "workflow completed");

        Ok(processed_output)
    }

    /// Returns a reference to the workflow definition
    pub fn workflow(&self) -> &WorkflowDefinition {
        &self.workflow
    }

    /// Returns a WorkflowHandle that can suspend/resume the running workflow
    ///
    /// Must be called before `run()`. The handle shares suspend/resume state
    /// with the workflow context via Arc.
    pub fn handle(&self) -> WorkflowHandle {
        WorkflowHandle {
            suspend_state: self.suspend_state.clone(),
        }
    }

    /// Runs the workflow on a recurring schedule based on the workflow's
    /// `schedule.every` or `schedule.cron` definition.
    ///
    /// For `every`: runs the workflow at fixed intervals.
    /// For `cron`: runs the workflow according to the cron expression schedule.
    ///
    /// Returns a `ScheduledWorkflow` that can be cancelled to stop the schedule.
    /// If no schedule is defined, runs once and returns a completed handle.
    pub fn schedule(self, input: Value) -> ScheduledWorkflow {
        if let Some(ref schedule) = self.workflow.schedule {
            if let Some(ref every_duration) = schedule.every {
                let interval = crate::utils::duration_to_std(every_duration);
                let (cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);
                let join_handle = tokio::spawn(async move {
                    let mut interval_timer = tokio::time::interval(interval);
                    loop {
                        tokio::select! {
                            _ = interval_timer.tick() => {
                                let _ = self.run(input.clone()).await;
                            }
                            _ = cancel_rx.changed() => {
                                break;
                            }
                        }
                    }
                });
                return ScheduledWorkflow {
                    join_handle,
                    cancel_tx,
                };
            }
            if let Some(ref cron_expr) = schedule.cron {
                let (cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);
                let cron_expr = cron_expr.clone();
                let join_handle = tokio::spawn(async move {
                    // Parse the cron expression (standard 5-field: min hour dom month dow)
                    let schedule = match cron::Schedule::from_str(&cron_expr) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("invalid cron expression '{}': {}", cron_expr, e);
                            return;
                        }
                    };
                    loop {
                        let next = schedule.upcoming(chrono::Utc).next();
                        let next = match next {
                            Some(t) => t,
                            None => break,
                        };
                        let delay = next - chrono::Utc::now();
                        let delay_std = delay.to_std().unwrap_or(Duration::ZERO);
                        if delay_std.is_zero() {
                            let _ = self.run(input.clone()).await;
                            continue;
                        }
                        tokio::select! {
                            _ = tokio::time::sleep(delay_std) => {
                                let _ = self.run(input.clone()).await;
                            }
                            _ = cancel_rx.changed() => {
                                break;
                            }
                        }
                    }
                });
                return ScheduledWorkflow {
                    join_handle,
                    cancel_tx,
                };
            }
            // after/on: run once (event-based scheduling not yet supported)
        }

        // No schedule or non-recurring: run once
        let (cancel_tx, _) = tokio::sync::watch::channel(false);
        let join_handle = tokio::spawn(async move {
            let _ = self.run(input).await;
        });
        ScheduledWorkflow {
            join_handle,
            cancel_tx,
        }
    }

    /// Resolves the workflow-level timeout duration, if configured
    fn resolve_workflow_timeout(
        &self,
        input: &Value,
        context: &WorkflowContext,
    ) -> Option<Duration> {
        let timeout_def = self.workflow.timeout.as_ref()?;
        let vars = context.get_vars();
        crate::utils::parse_duration_with_context(
            timeout_def,
            input,
            &vars,
            &self.workflow.document.name,
            Some(&self.workflow),
        )
        .ok()
    }

    /// Processes workflow input: schema validation and expression transformation
    fn process_input(&self, input: &Value, context: &WorkflowContext) -> WorkflowResult<Value> {
        let input_def = match &self.workflow.input {
            Some(def) => def,
            None => return Ok(input.clone()),
        };

        // Validate input schema
        if let Some(ref schema) = input_def.schema {
            validate_schema(input, schema, "/")?;
        }

        // Transform input via from expression
        let vars = context.get_vars();
        match input_def.from {
            Some(ref from_val) => evaluate_value_expr(from_val, input, &vars, "/"),
            None => Ok(input.clone()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::needless_borrow, clippy::unnecessary_to_owned, clippy::ptr_arg)]
mod runner_tests;
