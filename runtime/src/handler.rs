use crate::error::WorkflowResult;
use serde_json::Value;
use std::sync::Arc;

/// Handler for call task types that require custom implementations.
///
/// Implement this trait to provide support for call types like gRPC, OpenAPI,
/// AsyncAPI, and A2A. Register handlers with `WorkflowRunner::with_call_handler()`.
///
/// # Example
///
/// ```no_run
/// use async_trait::async_trait;
/// use serde_json::Value;
/// use serverless_workflow_runtime::{CallHandler, WorkflowResult};
///
/// struct GrpcCallHandler;
///
/// #[async_trait]
/// impl CallHandler for GrpcCallHandler {
///     fn call_type(&self) -> &str { "grpc" }
///
///     async fn handle(
///         &self,
///         task_name: &str,
///         call_config: &Value,
///         input: &Value,
///     ) -> WorkflowResult<Value> {
///         // Implement gRPC call logic here
///         Ok(serde_json::json!({ "result": "grpc response" }))
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait CallHandler: Send + Sync {
    /// Returns the call type this handler supports (e.g., "grpc", "openapi", "asyncapi", "a2a")
    fn call_type(&self) -> &str;

    /// Executes the call with the given configuration and input.
    async fn handle(
        &self,
        task_name: &str,
        call_config: &Value,
        input: &Value,
    ) -> WorkflowResult<Value>;
}

/// Handler for run task types that require custom implementations.
///
/// Implement this trait to provide support for run types like container and script.
/// Register handlers with `WorkflowRunner::with_run_handler()`.
///
/// # Example
///
/// ```no_run
/// use async_trait::async_trait;
/// use serde_json::Value;
/// use serverless_workflow_runtime::{RunHandler, WorkflowResult};
///
/// struct ContainerRunHandler;
///
/// #[async_trait]
/// impl RunHandler for ContainerRunHandler {
///     fn run_type(&self) -> &str { "container" }
///
///     async fn handle(
///         &self,
///         task_name: &str,
///         run_config: &Value,
///         input: &Value,
///     ) -> WorkflowResult<Value> {
///         // Implement container run logic here
///         Ok(serde_json::json!({ "exitCode": 0 }))
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait RunHandler: Send + Sync {
    /// Returns the run type this handler supports (e.g., "container", "script")
    fn run_type(&self) -> &str;

    /// Executes the run with the given configuration and input.
    async fn handle(
        &self,
        task_name: &str,
        run_config: &Value,
        input: &Value,
    ) -> WorkflowResult<Value>;
}

/// Handler for custom/extension task types.
///
/// Implement this trait to provide support for custom task types that are
/// not part of the built-in Serverless Workflow specification.
/// Register handlers with `WorkflowRunner::with_custom_task_handler()`.
///
/// # Example
///
/// ```no_run
/// use async_trait::async_trait;
/// use serde_json::Value;
/// use serverless_workflow_runtime::{CustomTaskHandler, WorkflowResult};
///
/// struct UppercaseHandler;
///
/// #[async_trait]
/// impl CustomTaskHandler for UppercaseHandler {
///     fn task_type(&self) -> &str { "uppercase" }
///
///     async fn handle(
///         &self,
///         task_name: &str,
///         task_type: &str,
///         task_config: &Value,
///         input: &Value,
///     ) -> WorkflowResult<Value> {
///         let text = input["text"].as_str().unwrap_or("");
///         Ok(serde_json::json!({ "result": text.to_uppercase() }))
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait CustomTaskHandler: Send + Sync {
    /// Returns the custom task type this handler supports (e.g., "myCustomTask")
    fn task_type(&self) -> &str;

    /// Executes the custom task with the given configuration and input.
    async fn handle(
        &self,
        task_name: &str,
        task_type: &str,
        task_config: &Value,
        input: &Value,
    ) -> WorkflowResult<Value>;
}

/// Registry of call, run, and custom task handlers.
/// Uses Arc for cheap cloning — handlers are shared across workflow context propagation.
#[derive(Default, Clone)]
pub struct HandlerRegistry {
    call_handlers:
        std::sync::Arc<std::collections::HashMap<String, std::sync::Arc<dyn CallHandler>>>,
    run_handlers: std::sync::Arc<std::collections::HashMap<String, std::sync::Arc<dyn RunHandler>>>,
    custom_task_handlers:
        std::sync::Arc<std::collections::HashMap<String, std::sync::Arc<dyn CustomTaskHandler>>>,
}

impl HandlerRegistry {
    /// Creates a new empty handler registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a call handler
    pub fn register_call_handler(&mut self, handler: Box<dyn CallHandler>) {
        let key = handler.call_type().to_string();
        Arc::make_mut(&mut self.call_handlers).insert(key, std::sync::Arc::from(handler));
    }

    /// Registers a run handler
    pub fn register_run_handler(&mut self, handler: Box<dyn RunHandler>) {
        let key = handler.run_type().to_string();
        Arc::make_mut(&mut self.run_handlers).insert(key, std::sync::Arc::from(handler));
    }

    /// Registers a custom task handler
    pub fn register_custom_task_handler(&mut self, handler: Box<dyn CustomTaskHandler>) {
        let key = handler.task_type().to_string();
        Arc::make_mut(&mut self.custom_task_handlers).insert(key, std::sync::Arc::from(handler));
    }

    /// Looks up a call handler by type
    pub fn get_call_handler(&self, call_type: &str) -> Option<std::sync::Arc<dyn CallHandler>> {
        self.call_handlers.get(call_type).cloned()
    }

    /// Looks up a run handler by type
    pub fn get_run_handler(&self, run_type: &str) -> Option<std::sync::Arc<dyn RunHandler>> {
        self.run_handlers.get(run_type).cloned()
    }

    /// Looks up a custom task handler by task type
    pub fn get_custom_task_handler(
        &self,
        task_type: &str,
    ) -> Option<std::sync::Arc<dyn CustomTaskHandler>> {
        self.custom_task_handlers.get(task_type).cloned()
    }
}
