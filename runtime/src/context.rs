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
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

/// Generates a setter, deref-getter, and clone-getter for an `Option<Arc<T>>` field.
macro_rules! arc_accessors {
    ($field:ident, $setter:ident, $getter:ident, $clone:ident, $ty:ty) => {
        pub fn $setter(&mut self, value: Arc<$ty>) {
            self.$field = Some(value);
        }
        pub fn $getter(&self) -> Option<&$ty> {
            self.$field.as_deref()
        }
        pub fn $clone(&self) -> Option<Arc<$ty>> {
            self.$field.clone()
        }
    };
}

/// Generates a setter, ref-getter, and clone-getter for an `Option<T>` field where T: Clone.
macro_rules! option_accessors {
    ($field:ident, $setter:ident, $getter:ident, $clone:ident, $ty:ty) => {
        pub fn $setter(&mut self, value: $ty) {
            self.$field = Some(value);
        }
        pub fn $getter(&self) -> Option<&$ty> {
            self.$field.as_ref()
        }
        pub fn $clone(&self) -> Option<$ty> {
            self.$field.clone()
        }
    };
}

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
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
pub struct WorkflowContext {
    /// The workflow input ($input)
    input: Option<Value>,
    /// The workflow output ($output)
    output: Option<Value>,
    /// The instance context ($context) - set by export.as
    instance_ctx: Option<Value>,
    /// The workflow descriptor ($workflow)
    workflow_descriptor: Arc<Value>,
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
    /// Per-task iteration counter (incremented each time a task executes)
    iterations: HashMap<String, u32>,
    /// Cached vars map for JQ expression evaluation (rebuilt when dirty)
    vars_cache: Mutex<Option<HashMap<String, Value>>>,
    /// Whether vars_cache is stale and needs rebuilding
    vars_dirty: AtomicBool,
}

impl Clone for WorkflowContext {
    fn clone(&self) -> Self {
        Self {
            input: self.input.clone(),
            output: self.output.clone(),
            instance_ctx: self.instance_ctx.clone(),
            workflow_descriptor: Arc::clone(&self.workflow_descriptor),
            task_descriptor: self.task_descriptor.clone(),
            local_expr_vars: self.local_expr_vars.clone(),
            authorization: self.authorization.clone(),
            secret_manager: self.secret_manager.clone(),
            listener: self.listener.clone(),
            event_bus: self.event_bus.clone(),
            sub_workflows: self.sub_workflows.clone(),
            cancellation_token: self.cancellation_token.clone(),
            suspend_state: self.suspend_state.clone(),
            handler_registry: self.handler_registry.clone(),
            functions: self.functions.clone(),
            status_log: self.status_log.clone(),
            task_status: self.task_status.clone(),
            iterations: self.iterations.clone(),
            vars_cache: Mutex::new(self.vars_cache.lock().unwrap().clone()),
            vars_dirty: AtomicBool::new(self.vars_dirty.load(Ordering::Acquire)),
        }
    }
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
            .field("iterations", &self.iterations)
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

        let workflow_descriptor = Arc::new(serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "definition": workflow_json,
        }));

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
            iterations: HashMap::new(),
            vars_cache: Mutex::new(None),
            vars_dirty: AtomicBool::new(true),
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

    // ---- Input / Output / Instance Context ----

    pub fn set_input(&mut self, value: Value) {
        self.input = Some(value);
        self.invalidate_vars_cache();
    }
    pub fn get_input(&self) -> Option<&Value> {
        self.input.as_ref()
    }
    pub fn set_output(&mut self, value: Value) {
        self.output = Some(value);
        self.invalidate_vars_cache();
    }
    pub fn get_output(&self) -> Option<&Value> {
        self.output.as_ref()
    }
    pub fn set_instance_ctx(&mut self, value: Value) {
        self.instance_ctx = Some(value);
        self.invalidate_vars_cache();
    }
    pub fn get_instance_ctx(&self) -> Option<&Value> {
        self.instance_ctx.as_ref()
    }

    // ---- Raw Input (in workflow descriptor) ----

    /// Sets the raw input in the workflow descriptor
    pub fn set_raw_input(&mut self, input: &Value) {
        let mut desc = (*self.workflow_descriptor).clone();
        if let Some(obj) = desc.as_object_mut() {
            obj.insert("input".to_string(), input.clone());
        }
        self.workflow_descriptor = Arc::new(desc);
        self.invalidate_vars_cache();
    }

    // ---- Task Descriptor ----

    /// Inserts a key-value pair into the task descriptor object.
    fn task_descriptor_insert(&mut self, key: &str, value: Value) {
        if let Some(obj) = self.task_descriptor.as_object_mut() {
            obj.insert(key.to_string(), value);
        }
        self.invalidate_vars_cache();
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
        self.workflow_descriptor
            .as_object()
            .and_then(|obj| obj.get("definition"))
    }

    /// Gets the workflow instance ID
    /// Sets the task definition in the task descriptor
    pub fn set_task_def(&mut self, task: &Value) {
        self.task_descriptor_insert("definition", task.clone());
    }

    /// Increments and returns the iteration counter for the given task position.
    /// Each time a task executes, this counter is incremented, starting at 1.
    pub fn inc_iteration(&mut self, position: &str) -> u32 {
        let count = self.iterations.entry(position.to_string()).or_insert(0);
        *count += 1;
        let value = *count;
        self.task_descriptor_insert("iteration", serde_json::json!(value));
        value
    }

    /// Sets the retry attempt count in the task descriptor
    pub fn set_retry_attempt(&mut self, attempt: u32) {
        self.task_descriptor_insert("retryAttempt", serde_json::json!(attempt));
    }

    /// Clears the current task context
    pub fn clear_task_context(&mut self) {
        self.task_descriptor = Value::Object(Default::default());
    }

    // ---- Secret Manager ----

    arc_accessors!(
        secret_manager,
        set_secret_manager,
        get_secret_manager,
        clone_secret_manager,
        dyn SecretManager
    );

    // ---- Execution Listener ----

    arc_accessors!(
        listener,
        set_listener,
        get_listener,
        clone_listener,
        dyn WorkflowExecutionListener
    );

    // ---- Event Emission ----

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

    option_accessors!(
        event_bus,
        set_event_bus,
        get_event_bus,
        clone_event_bus,
        SharedEventBus
    );

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
        self.invalidate_vars_cache();
    }

    /// Clears the authorization descriptor (called after task completes)
    pub fn clear_authorization(&mut self) {
        self.authorization = None;
        self.invalidate_vars_cache();
    }

    // ---- Local Expression Variables ----

    /// Sets local expression variables (replaces all)
    pub fn set_local_expr_vars(&mut self, vars: HashMap<String, Value>) {
        self.local_expr_vars = vars;
        self.invalidate_vars_cache();
    }

    /// Adds local expression variables (merges, does not overwrite existing keys)
    pub fn add_local_expr_vars(&mut self, vars: HashMap<String, Value>) {
        for (k, v) in vars {
            self.local_expr_vars.entry(k).or_insert(v);
        }
        self.invalidate_vars_cache();
    }

    /// Removes specified local expression variables
    pub fn remove_local_expr_vars(&mut self, keys: &[&str]) {
        for key in keys {
            self.local_expr_vars.remove(*key);
        }
        self.invalidate_vars_cache();
    }

    // ---- Variable Aggregation ----

    /// Marks the vars cache as dirty (needs rebuild on next access)
    fn invalidate_vars_cache(&self) {
        self.vars_dirty.store(true, Ordering::Release);
    }

    /// Returns all variables for JQ expression evaluation, using a cache
    /// to avoid rebuilding the map on every call.
    pub fn get_vars(&self) -> HashMap<String, Value> {
        if self.vars_dirty.load(Ordering::Acquire) {
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
            vars.insert(
                vars::WORKFLOW.to_string(),
                (*self.workflow_descriptor).clone(),
            );
            vars.insert(
                vars::RUNTIME.to_string(),
                runtime_info::runtime_info_value().clone(),
            );

            if let Some(ref mgr) = self.secret_manager {
                vars.insert(vars::SECRET.to_string(), mgr.get_all_secrets());
            }

            if let Some(ref auth) = self.authorization {
                vars.insert(vars::AUTHORIZATION.to_string(), auth.clone());
            }

            for (k, v) in &self.local_expr_vars {
                vars.insert(k.clone(), v.clone());
            }

            *self.vars_cache.lock().unwrap() = Some(vars);
            self.vars_dirty.store(false, Ordering::Release);
        }
        self.vars_cache.lock().unwrap().as_ref().unwrap().clone()
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
