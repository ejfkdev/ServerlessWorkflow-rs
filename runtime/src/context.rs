use crate::events::SharedEventBus;
use crate::handler::HandlerRegistry;
use crate::listener::{WorkflowEvent, WorkflowExecutionListener};
use crate::secret::SecretManager;
use crate::status::{StatusPhase, StatusPhaseLog};
use serde_json::Value;
use serverless_workflow_core::models::task::TaskDefinition;
use serverless_workflow_core::models::workflow::WorkflowDefinition;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

/// Shared suspend/resume state for workflow execution.
/// Cloned between WorkflowHandle and WorkflowContext to avoid duplicating logic.
#[derive(Clone)]
pub(crate) struct SuspendState {
    suspended: Arc<AtomicBool>,
    resume_notify: Arc<Notify>,
}

impl SuspendState {
    pub(crate) fn new() -> Self {
        Self {
            suspended: Arc::new(AtomicBool::new(false)),
            resume_notify: Arc::new(Notify::new()),
        }
    }

    /// Suspends the workflow. Returns true if suspended, false if already suspended.
    pub fn suspend(&self) -> bool {
        self.suspended
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Resumes a suspended workflow. Returns true if resumed, false if not suspended.
    pub fn resume(&self) -> bool {
        if self
            .suspended
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.resume_notify.notify_waiters();
            true
        } else {
            false
        }
    }

    /// Checks if the workflow is currently suspended.
    pub fn is_suspended(&self) -> bool {
        self.suspended.load(Ordering::SeqCst)
    }

    /// Returns the cancellation-aware resume notifier.
    pub(crate) fn resume_notify(&self) -> &Arc<Notify> {
        &self.resume_notify
    }
}
use tokio_util::sync::CancellationToken;

/// Variable name constants used in JQ expressions
pub mod vars {
    pub const CONTEXT: &str = "$context";
    pub const INPUT: &str = "$input";
    pub const OUTPUT: &str = "$output";
    pub const WORKFLOW: &str = "$workflow";
    pub const RUNTIME: &str = "$runtime";
    pub const TASK: &str = "$task";
    pub const SECRET: &str = "$secret";
    pub const AUTHORIZATION: &str = "$authorization";
}

/// Runtime name and version constants
pub mod runtime_info {
    pub const NAME: &str = "CNCF Serverless Workflow Specification Rust SDK";
    pub const VERSION: &str = "1.0.0-alpha6.3";

    /// Cached runtime info JSON value (constructed once)
    static RUNTIME_INFO: std::sync::LazyLock<serde_json::Value> = std::sync::LazyLock::new(|| {
        serde_json::json!({
            "name": NAME,
            "version": VERSION,
        })
    });

    pub fn runtime_info_value() -> &'static serde_json::Value {
        &RUNTIME_INFO
    }
}

/// Holds the runtime context for a workflow execution
#[derive(Clone)]
pub struct WorkflowContext {
    /// The workflow input ($input)
    input: Option<Value>,
    /// The workflow output ($output)
    output: Option<Value>,
    /// The instance context ($context) - set by export.as
    instance_ctx: Option<Value>,
    /// The workflow descriptor ($workflow)
    workflow_descriptor: Value,
    /// The current task descriptor ($task)
    task_descriptor: Value,
    /// Local expression variables (e.g., $item, $index in for loops)
    local_expr_vars: HashMap<String, Value>,
    /// The authorization descriptor ($authorization) — set after HTTP auth
    authorization: Option<Value>,
    /// The secret manager ($secret)
    secret_manager: Option<Arc<dyn SecretManager>>,
    /// The execution listener
    listener: Option<Arc<dyn WorkflowExecutionListener>>,
    /// The event bus for publish/subscribe (used by emit and listen tasks)
    event_bus: Option<SharedEventBus>,
    /// Sub-workflow registry keyed by "namespace/name/version"
    sub_workflows: HashMap<String, WorkflowDefinition>,
    /// Cancellation token for graceful shutdown (e.g., workflow timeout)
    cancellation_token: CancellationToken,
    /// Suspend flag: true when the workflow is suspended
    suspend_state: SuspendState,
    /// Handler registry for custom call/run handlers
    handler_registry: HandlerRegistry,
    /// Registered function definitions for call.function resolution (catalog mechanism)
    functions: HashMap<String, TaskDefinition>,
    /// Overall workflow status log
    status_log: Vec<StatusPhaseLog>,
    /// Per-task status log
    task_status: HashMap<String, Vec<StatusPhaseLog>>,
}

impl std::fmt::Debug for WorkflowContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowContext")
            .field("input", &self.input)
            .field("output", &self.output)
            .field("instance_ctx", &self.instance_ctx)
            .field("workflow_descriptor", &self.workflow_descriptor)
            .field("task_descriptor", &self.task_descriptor)
            .field("local_expr_vars", &self.local_expr_vars)
            .field(
                "secret_manager",
                &self.secret_manager.as_ref().map(|_| "..."),
            )
            .field("listener", &self.listener.as_ref().map(|_| "..."))
            .field("event_bus", &self.event_bus.as_ref().map(|_| "..."))
            .field("status_log", &self.status_log)
            .field("task_status", &self.task_status)
            .finish()
    }
}

impl WorkflowContext {
    /// Creates a new workflow context from a workflow definition
    pub fn new(
        workflow: &serverless_workflow_core::models::workflow::WorkflowDefinition,
    ) -> crate::error::WorkflowResult<Self> {
        let workflow_json = serde_json::to_value(workflow).map_err(|e| {
            crate::error::WorkflowError::runtime(
                format!("failed to serialize workflow definition: {}", e),
                "/",
                "/",
            )
        })?;

        let workflow_descriptor = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "definition": workflow_json,
        });

        let mut ctx = Self {
            input: None,
            output: None,
            instance_ctx: None,
            workflow_descriptor,
            task_descriptor: Value::Object(Default::default()),
            local_expr_vars: HashMap::new(),
            authorization: None,
            secret_manager: None,
            listener: None,
            event_bus: None,
            sub_workflows: HashMap::new(),
            cancellation_token: CancellationToken::new(),
            suspend_state: SuspendState::new(),
            handler_registry: HandlerRegistry::new(),
            functions: HashMap::new(),
            status_log: Vec::new(),
            task_status: HashMap::new(),
        };
        ctx.set_status(StatusPhase::Pending);
        Ok(ctx)
    }

    // ---- Status ----

    /// Sets the overall workflow status
    pub fn set_status(&mut self, status: StatusPhase) {
        self.status_log.push(StatusPhaseLog::new(status));
    }

    /// Gets the workflow instance ID
    pub fn instance_id(&self) -> &str {
        self.workflow_descriptor
            .as_object()
            .and_then(|obj| obj.get("id"))
            .and_then(|id| id.as_str())
            .unwrap_or("unknown")
    }

    /// Gets the current overall workflow status
    pub fn get_status(&self) -> StatusPhase {
        self.status_log
            .last()
            .map(|log| log.status)
            .unwrap_or(StatusPhase::Pending)
    }

    /// Sets the status for a specific task
    pub fn set_task_status(&mut self, task: &str, status: StatusPhase) {
        self.task_status
            .entry(task.to_string())
            .or_default()
            .push(StatusPhaseLog::new(status));
    }

    /// Gets the current status for a specific task
    pub fn get_task_status(&self, task: &str) -> Option<StatusPhase> {
        self.task_status
            .get(task)
            .and_then(|logs| logs.last())
            .map(|log| log.status)
    }

    // ---- Input / Output ----

    /// Sets the workflow input
    pub fn set_input(&mut self, input: Value) {
        self.input = Some(input);
    }

    /// Gets the workflow input
    pub fn get_input(&self) -> Option<&Value> {
        self.input.as_ref()
    }

    /// Sets the workflow output
    pub fn set_output(&mut self, output: Value) {
        self.output = Some(output);
    }

    /// Gets the workflow output
    pub fn get_output(&self) -> Option<&Value> {
        self.output.as_ref()
    }

    // ---- Instance Context ($context) ----

    /// Sets the instance context ($context variable)
    pub fn set_instance_ctx(&mut self, value: Value) {
        self.instance_ctx = Some(value);
    }

    /// Gets the instance context
    pub fn get_instance_ctx(&self) -> Option<&Value> {
        self.instance_ctx.as_ref()
    }

    // ---- Raw Input (in workflow descriptor) ----

    /// Sets the raw input in the workflow descriptor
    pub fn set_raw_input(&mut self, input: &Value) {
        if let Some(obj) = self.workflow_descriptor.as_object_mut() {
            obj.insert("input".to_string(), input.clone());
        }
    }

    // ---- Task Descriptor ----

    /// Inserts a key-value pair into the task descriptor object.
    /// Panics if task_descriptor is not an Object (it always is by construction).
    fn task_descriptor_insert(&mut self, key: &str, value: Value) {
        self.task_descriptor
            .as_object_mut()
            .expect("task_descriptor is always an Object")
            .insert(key.to_string(), value);
    }

    /// Sets the task name in the current task descriptor
    pub fn set_task_name(&mut self, name: &str) {
        self.task_descriptor_insert("name", Value::String(name.to_string()));
    }

    /// Sets the task raw input
    pub fn set_task_raw_input(&mut self, input: &Value) {
        self.task_descriptor_insert("input", input.clone());
    }

    /// Sets the task raw output
    pub fn set_task_raw_output(&mut self, output: &Value) {
        self.task_descriptor_insert("output", output.clone());
    }

    /// Sets the task startedAt timestamp with nested structure:
    /// { iso8601: "...", epoch: { seconds: 123, milliseconds: 123456 } }
    pub fn set_task_started_at(&mut self) {
        let now = chrono::Utc::now();
        let iso8601 = now.to_rfc3339();
        let epoch_seconds = now.timestamp();
        let epoch_millis = now.timestamp_millis();
        self.task_descriptor_insert(
            "startedAt",
            serde_json::json!({
                "iso8601": iso8601,
                "epoch": {
                    "seconds": epoch_seconds,
                    "milliseconds": epoch_millis,
                }
            }),
        );
    }

    /// Sets the task reference (JSON Pointer)
    pub fn set_task_reference(&mut self, reference: &str) {
        self.task_descriptor_insert("reference", Value::String(reference.to_string()));
    }

    /// Gets the task reference
    pub fn get_task_reference(&self) -> Option<&str> {
        self.task_descriptor
            .as_object()
            .and_then(|obj| obj.get("reference"))
            .and_then(|v| v.as_str())
    }

    /// Gets the serialized workflow JSON value (for json_pointer resolution)
    pub fn get_workflow_json(&self) -> Option<&Value> {
        self.workflow_descriptor.as_object()
            .and_then(|obj| obj.get("definition"))
    }

    /// Sets the task definition in the task descriptor
    pub fn set_task_def(&mut self, task: &Value) {
        self.task_descriptor_insert("definition", task.clone());
    }

    /// Clears the current task context
    pub fn clear_task_context(&mut self) {
        self.task_descriptor = Value::Object(Default::default());
    }

    // ---- Secret Manager ----

    /// Sets the secret manager for $secret expression variable
    pub fn set_secret_manager(&mut self, manager: Arc<dyn SecretManager>) {
        self.secret_manager = Some(manager);
    }

    /// Gets the secret manager
    pub fn get_secret_manager(&self) -> Option<&dyn SecretManager> {
        self.secret_manager.as_deref()
    }

    /// Gets a cloned Arc to the secret manager (for propagating to child runners)
    pub fn clone_secret_manager(&self) -> Option<Arc<dyn SecretManager>> {
        self.secret_manager.clone()
    }

    /// Gets a cloned Arc to the listener (for propagating to child runners)
    pub fn clone_listener(&self) -> Option<Arc<dyn WorkflowExecutionListener>> {
        self.listener.clone()
    }

    // ---- Execution Listener ----

    /// Sets the execution listener
    pub fn set_listener(&mut self, listener: Arc<dyn WorkflowExecutionListener>) {
        self.listener = Some(listener);
    }

    /// Emits an event to the listener if configured, and publishes as CloudEvent to EventBus
    pub fn emit_event(&self, event: WorkflowEvent) {
        // Notify the synchronous listener
        if let Some(ref listener) = self.listener {
            listener.on_event(&event);
        }

        // Publish lifecycle CloudEvent to EventBus if configured
        if let Some(ref event_bus) = self.event_bus {
            let cloud_event = event.to_cloud_event();
            let bus = event_bus.clone();
            tokio::spawn(async move {
                bus.publish(cloud_event).await;
            });
        }
    }

    // ---- Event Bus ----

    /// Sets the event bus for emit/listen tasks
    pub fn set_event_bus(&mut self, bus: SharedEventBus) {
        self.event_bus = Some(bus);
    }

    /// Gets the event bus
    pub fn get_event_bus(&self) -> Option<&SharedEventBus> {
        self.event_bus.as_ref()
    }

    /// Gets a cloned Arc to the event bus (for propagating to child runners)
    pub fn clone_event_bus(&self) -> Option<SharedEventBus> {
        self.event_bus.clone()
    }

    // ---- Sub-Workflow Registry ----

    /// Sets the sub-workflow registry
    pub fn set_sub_workflows(&mut self, sub_workflows: HashMap<String, WorkflowDefinition>) {
        self.sub_workflows = sub_workflows;
    }

    /// Looks up a sub-workflow by namespace/name/version key
    pub fn get_sub_workflow(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
    ) -> Option<&WorkflowDefinition> {
        let key = format!("{}/{}/{}", namespace, name, version);
        self.sub_workflows.get(&key)
    }

    /// Clones the entire sub-workflow registry (for propagating to child runners)
    pub fn clone_sub_workflows(&self) -> HashMap<String, WorkflowDefinition> {
        self.sub_workflows.clone()
    }

    // ---- Handler Registry ----

    /// Sets the handler registry (replaces all handlers)
    pub fn set_handler_registry(&mut self, registry: HandlerRegistry) {
        self.handler_registry = registry;
    }

    /// Gets a reference to the handler registry
    pub fn get_handler_registry(&self) -> &HandlerRegistry {
        &self.handler_registry
    }

    /// Clones the handler registry (for propagating to child runners)
    pub fn clone_handler_registry(&self) -> HandlerRegistry {
        self.handler_registry.clone()
    }

    // ---- Functions (Catalog) ----

    /// Sets the registered function definitions (for call.function resolution)
    pub fn set_functions(&mut self, functions: HashMap<String, TaskDefinition>) {
        self.functions = functions;
    }

    /// Looks up a registered function definition by name
    pub fn get_function(&self, name: &str) -> Option<&TaskDefinition> {
        self.functions.get(name)
    }

    // ---- Cancellation ----

    /// Gets a clone of the cancellation token (for use in tokio::select!)
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// Cancels the workflow (triggers cancellation for all wait points)
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /// Checks if cancellation has been requested
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    // ---- Suspend / Resume ----

    /// Suspends the workflow execution
    ///
    /// Returns `true` if the workflow was successfully suspended,
    /// `false` if it was already suspended.
    pub fn suspend(&self) -> bool {
        self.suspend_state.suspend()
    }

    /// Resumes a suspended workflow execution
    ///
    /// Returns `true` if the workflow was resumed from a suspended state,
    /// `false` if it was not suspended.
    pub fn resume(&self) -> bool {
        self.suspend_state.resume()
    }

    /// Checks if the workflow is currently suspended
    pub fn is_suspended(&self) -> bool {
        self.suspend_state.is_suspended()
    }

    /// Waits until the workflow is resumed (or cancelled)
    ///
    /// Should be called from task runners at cooperative yield points
    /// when the workflow is detected as suspended.
    pub async fn wait_for_resume(&self) {
        if self.is_suspended() {
            tokio::select! {
                _ = self.suspend_state.resume_notify().notified() => {}
                _ = self.cancellation_token.cancelled() => {}
            }
        }
    }

    // ---- Suspend State Sharing ----

    /// Sets the shared suspend/resume state from the WorkflowRunner
    ///
    /// This allows the WorkflowHandle to share the same AtomicBool and Notify
    /// as the context, enabling external suspend/resume control.
    pub(crate) fn set_suspend_state(&mut self, state: SuspendState) {
        self.suspend_state = state;
    }

    // ---- Authorization ----

    /// Sets the authorization descriptor for the current task
    /// Called after HTTP authentication succeeds (Basic, Bearer, Digest, OAuth2, OIDC)
    pub fn set_authorization(&mut self, scheme: &str, parameter: &str) {
        self.authorization = Some(serde_json::json!({
            "scheme": scheme,
            "parameter": parameter,
        }));
    }

    /// Clears the authorization descriptor (called after task completes)
    pub fn clear_authorization(&mut self) {
        self.authorization = None;
    }

    // ---- Local Expression Variables ----

    /// Sets local expression variables (replaces all)
    pub fn set_local_expr_vars(&mut self, vars: HashMap<String, Value>) {
        self.local_expr_vars = vars;
    }

    /// Adds local expression variables (merges, does not overwrite existing keys)
    pub fn add_local_expr_vars(&mut self, vars: HashMap<String, Value>) {
        for (k, v) in vars {
            self.local_expr_vars.entry(k).or_insert(v);
        }
    }

    /// Removes specified local expression variables
    pub fn remove_local_expr_vars(&mut self, keys: &[&str]) {
        for key in keys {
            self.local_expr_vars.remove(*key);
        }
    }

    // ---- Variable Aggregation ----

    /// Returns all variables for JQ expression evaluation
    pub fn get_vars(&self) -> HashMap<String, Value> {
        let mut vars = HashMap::new();

        vars.insert(
            vars::INPUT.to_string(),
            self.input.clone().unwrap_or(Value::Null),
        );
        vars.insert(
            vars::OUTPUT.to_string(),
            self.output.clone().unwrap_or(Value::Null),
        );
        vars.insert(
            vars::CONTEXT.to_string(),
            self.instance_ctx.clone().unwrap_or(Value::Null),
        );
        vars.insert(vars::TASK.to_string(), self.task_descriptor.clone());
        vars.insert(vars::WORKFLOW.to_string(), self.workflow_descriptor.clone());
        vars.insert(
            vars::RUNTIME.to_string(),
            runtime_info::runtime_info_value().clone(),
        );

        // Add $secret variable if secret manager is configured
        if let Some(ref mgr) = self.secret_manager {
            vars.insert(vars::SECRET.to_string(), mgr.get_all_secrets());
        }

        // Add $authorization variable if set (after HTTP authentication)
        if let Some(ref auth) = self.authorization {
            vars.insert(vars::AUTHORIZATION.to_string(), auth.clone());
        }

        for (k, v) in &self.local_expr_vars {
            vars.insert(k.clone(), v.clone());
        }

        vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serverless_workflow_core::models::workflow::WorkflowDefinition;

    fn new_context() -> WorkflowContext {
        let workflow = WorkflowDefinition::default();
        WorkflowContext::new(&workflow).unwrap()
    }

    #[test]
    fn test_context_new() {
        let ctx = new_context();
        assert!(ctx.get_input().is_none());
        assert!(ctx.get_output().is_none());
        assert_eq!(ctx.get_status(), StatusPhase::Pending);
    }

    #[test]
    fn test_context_set_input_output() {
        let mut ctx = new_context();
        ctx.set_input(json!({"key": "value"}));
        assert_eq!(ctx.get_input(), Some(&json!({"key": "value"})));

        ctx.set_output(json!(42));
        assert_eq!(ctx.get_output(), Some(&json!(42)));
    }

    #[test]
    fn test_context_status_transitions() {
        let mut ctx = new_context();
        assert_eq!(ctx.get_status(), StatusPhase::Pending);

        ctx.set_status(StatusPhase::Running);
        assert_eq!(ctx.get_status(), StatusPhase::Running);

        ctx.set_status(StatusPhase::Completed);
        assert_eq!(ctx.get_status(), StatusPhase::Completed);
    }

    #[test]
    fn test_context_instance_ctx() {
        let mut ctx = new_context();
        assert!(ctx.get_instance_ctx().is_none());

        ctx.set_instance_ctx(json!({"exported": "data"}));
        assert_eq!(ctx.get_instance_ctx(), Some(&json!({"exported": "data"})));
    }

    #[test]
    fn test_context_local_expr_vars() {
        let mut ctx = new_context();
        let mut vars = HashMap::new();
        vars.insert("$item".to_string(), json!("hello"));
        vars.insert("$index".to_string(), json!(0));
        ctx.add_local_expr_vars(vars);

        let all_vars = ctx.get_vars();
        assert_eq!(all_vars.get("$item"), Some(&json!("hello")));
        assert_eq!(all_vars.get("$index"), Some(&json!(0)));

        ctx.remove_local_expr_vars(&["$item", "$index"]);
        let all_vars = ctx.get_vars();
        assert!(!all_vars.contains_key("$item"));
        assert!(!all_vars.contains_key("$index"));
    }

    #[test]
    fn test_context_get_vars_includes_runtime() {
        let ctx = new_context();
        let vars = ctx.get_vars();
        assert!(vars.contains_key(vars::RUNTIME));
        assert!(vars.contains_key(vars::WORKFLOW));
        assert!(vars.contains_key(vars::TASK));
    }

    #[test]
    fn test_context_task_status() {
        let mut ctx = new_context();
        ctx.set_task_status("task1", StatusPhase::Running);
        ctx.set_task_status("task1", StatusPhase::Completed);
        ctx.set_task_status("task2", StatusPhase::Pending);

        let task1_status = ctx.get_task_status("task1");
        assert_eq!(task1_status, Some(StatusPhase::Completed));
    }

    #[test]
    fn test_context_authorization() {
        let mut ctx = new_context();

        // No authorization by default
        let vars = ctx.get_vars();
        assert!(!vars.contains_key("$authorization"));

        // Set authorization
        ctx.set_authorization("Bearer", "my-token-123");
        let vars = ctx.get_vars();
        let auth = vars
            .get("$authorization")
            .expect("$authorization should be set");
        assert_eq!(auth["scheme"], "Bearer");
        assert_eq!(auth["parameter"], "my-token-123");

        // Clear authorization
        ctx.clear_authorization();
        let vars = ctx.get_vars();
        assert!(!vars.contains_key("$authorization"));
    }
}
