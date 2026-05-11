use crate::error::{WorkflowError, WorkflowResult};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;

/// Read-only snapshot of workflow context variables available to task handlers.
///
/// Provides access to `$context`, `$secret`, `$workflow`, and other runtime variables
/// that were previously inaccessible from custom handlers.
///
/// # Example
///
/// ```no_run
/// use async_trait::async_trait;
/// use serde_json::Value;
/// use swf_runtime::{CustomTaskHandler, HandlerContext, WorkflowResult};
///
/// struct SmartHandler;
///
/// #[async_trait]
/// impl CustomTaskHandler for SmartHandler {
///     fn task_type(&self) -> &str { "smart" }
///
///     async fn handle(
///         &self,
///         task_name: &str,
///         task_type: &str,
///         task_config: &Value,
///         input: &Value,
///         context: &HandlerContext,
///     ) -> WorkflowResult<Value> {
///         // Access $context to read workflow state
///         let preferred = context.context().get("provider").and_then(|v| v.as_str());
///         // Access $secret for credentials
///         let api_key = context.secret().and_then(|s| s.get("API_KEY")).and_then(|v| v.as_str());
///         Ok(input.clone())
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct HandlerContext {
    context: Value,
    secret: Option<Value>,
    workflow: Value,
    authorization: Option<Value>,
}

impl HandlerContext {
    /// Creates a new HandlerContext from the current workflow context variables
    pub(crate) fn from_vars(vars: &std::collections::HashMap<String, Value>) -> Self {
        Self {
            context: vars
                .get(crate::context::vars::CONTEXT)
                .cloned()
                .unwrap_or(Value::Null),
            secret: vars.get(crate::context::vars::SECRET).cloned(),
            workflow: vars
                .get(crate::context::vars::WORKFLOW)
                .cloned()
                .unwrap_or(Value::Null),
            authorization: vars.get(crate::context::vars::AUTHORIZATION).cloned(),
        }
    }

    /// Returns the `$context` value (workflow instance state set by `export.as`)
    pub fn context(&self) -> &Value {
        &self.context
    }

    /// Returns the `$secret` value (all resolved secrets), if a secret manager is configured
    pub fn secret(&self) -> Option<&Value> {
        self.secret.as_ref()
    }

    /// Returns the `$workflow` descriptor (workflow metadata)
    pub fn workflow(&self) -> &Value {
        &self.workflow
    }

    /// Returns the `$authorization` value (set after HTTP authentication), if any
    pub fn authorization(&self) -> Option<&Value> {
        self.authorization.as_ref()
    }
}

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
/// use swf_runtime::{CallHandler, HandlerContext, WorkflowResult};
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
///         context: &HandlerContext,
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

    /// Executes the call with the given configuration, input, and workflow context.
    async fn handle(
        &self,
        task_name: &str,
        call_config: &Value,
        input: &Value,
        context: &HandlerContext,
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
/// use swf_runtime::{RunHandler, HandlerContext, WorkflowResult};
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
///         context: &HandlerContext,
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

    /// Executes the run with the given configuration, input, and workflow context.
    async fn handle(
        &self,
        task_name: &str,
        run_config: &Value,
        input: &Value,
        context: &HandlerContext,
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
/// use swf_runtime::{CustomTaskHandler, HandlerContext, WorkflowResult};
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
///         context: &HandlerContext,
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

    /// Executes the custom task with the given configuration, input, and workflow context.
    async fn handle(
        &self,
        task_name: &str,
        task_type: &str,
        task_config: &Value,
        input: &Value,
        context: &HandlerContext,
    ) -> WorkflowResult<Value>;
}

/// Type-safe custom task handler that receives a strongly-typed config instead of raw JSON.
///
/// Implement this trait to avoid manual JSON parsing in every handler. The framework
/// automatically deserializes `task_config` into [`Self::Config`] and converts
/// deserialization errors into `WorkflowError::validation`.
///
/// Use [`TypedCustomTaskHandler::into_boxed`] to convert a typed handler into a
/// `Box<dyn CustomTaskHandler>` that can be registered with `WorkflowRunner::with_custom_task_handler()`.
///
/// # Example
///
/// ```no_run
/// use async_trait::async_trait;
/// use serde::Deserialize;
/// use serde_json::{json, Value};
/// use swf_runtime::{TypedCustomTaskHandler, HandlerContext, WorkflowResult};
///
/// #[derive(Deserialize)]
/// struct ProviderConfig {
///     name: String,
///     operation: String,
///     #[serde(default)]
///     timeout: Option<String>,
/// }
///
/// struct ProviderHandler;
///
/// #[async_trait]
/// impl TypedCustomTaskHandler for ProviderHandler {
///     type Config = ProviderConfig;
///
///     fn task_type(&self) -> &str { "provider" }
///
///     async fn handle(
///         &self,
///         task_name: &str,
///         config: &ProviderConfig,
///         input: &Value,
///         context: &HandlerContext,
///     ) -> WorkflowResult<Value> {
///         // config.name and config.operation are directly available — no manual parsing
///         Ok(json!({ "provider": config.name, "op": config.operation }))
///     }
/// }
///
/// // Register with the runner:
/// // runner.with_custom_task_handler(ProviderHandler.into_boxed())
/// ```
#[async_trait::async_trait]
pub trait TypedCustomTaskHandler: Send + Sync + 'static {
    /// The strongly-typed configuration for this handler.
    type Config: DeserializeOwned + Send + Sync + 'static;

    /// Returns the custom task type this handler supports
    fn task_type(&self) -> &str;

    /// Executes the custom task with the given typed configuration, input, and workflow context.
    async fn handle(
        &self,
        task_name: &str,
        config: &Self::Config,
        input: &Value,
        context: &HandlerContext,
    ) -> WorkflowResult<Value>;

    /// Converts this typed handler into a `Box<dyn CustomTaskHandler>` for registration.
    ///
    /// The returned wrapper automatically deserializes the config JSON into `Self::Config`
    /// before calling [`handle`](TypedCustomTaskHandler::handle).
    fn into_boxed(self) -> Box<dyn CustomTaskHandler>
    where
        Self: Sized,
    {
        Box::new(TypedHandlerWrapper(self))
    }
}

/// Wrapper that adapts a [`TypedCustomTaskHandler`] into a [`CustomTaskHandler`].
struct TypedHandlerWrapper<H: TypedCustomTaskHandler>(H);

#[async_trait::async_trait]
impl<H: TypedCustomTaskHandler> CustomTaskHandler for TypedHandlerWrapper<H> {
    fn task_type(&self) -> &str {
        self.0.task_type()
    }

    async fn handle(
        &self,
        task_name: &str,
        _task_type: &str,
        task_config: &Value,
        input: &Value,
        context: &HandlerContext,
    ) -> WorkflowResult<Value> {
        let config: H::Config = serde_json::from_value(task_config.clone()).map_err(|e| {
            WorkflowError::validation(
                format!("invalid config for '{}' task: {}", self.0.task_type(), e),
                task_name,
            )
        })?;
        self.0.handle(task_name, &config, input, context).await
    }
}

/// Unified handler interface for all task types.
///
/// This trait provides a single interface that abstracts over [`CallHandler`],
/// [`RunHandler`], and [`CustomTaskHandler`]. All three can be adapted to `TaskHandler`
/// via blanket implementations, and registered in the same [`HandlerRegistry`].
///
/// Users can also implement `TaskHandler` directly for maximum flexibility,
/// bypassing the type-specific traits entirely.
#[async_trait::async_trait]
pub trait TaskHandler: Send + Sync {
    /// Returns the handler type key (e.g., "grpc", "provider", "container")
    fn handler_type(&self) -> &str;

    /// Executes the handler with the given configuration, input, and workflow context.
    async fn handle(
        &self,
        task_name: &str,
        config: &Value,
        input: &Value,
        context: &HandlerContext,
    ) -> WorkflowResult<Value>;
}

/// Wrapper adapting a [`CallHandler`] into a [`TaskHandler`].
struct CallHandlerAdapter(std::sync::Arc<dyn CallHandler>);

#[async_trait::async_trait]
impl TaskHandler for CallHandlerAdapter {
    fn handler_type(&self) -> &str {
        self.0.call_type()
    }

    async fn handle(
        &self,
        task_name: &str,
        config: &Value,
        input: &Value,
        context: &HandlerContext,
    ) -> WorkflowResult<Value> {
        self.0.handle(task_name, config, input, context).await
    }
}

/// Wrapper adapting a [`RunHandler`] into a [`TaskHandler`].
struct RunHandlerAdapter(std::sync::Arc<dyn RunHandler>);

#[async_trait::async_trait]
impl TaskHandler for RunHandlerAdapter {
    fn handler_type(&self) -> &str {
        self.0.run_type()
    }

    async fn handle(
        &self,
        task_name: &str,
        config: &Value,
        input: &Value,
        context: &HandlerContext,
    ) -> WorkflowResult<Value> {
        self.0.handle(task_name, config, input, context).await
    }
}

/// Wrapper adapting a [`CustomTaskHandler`] into a [`TaskHandler`].
struct CustomHandlerAdapter(std::sync::Arc<dyn CustomTaskHandler>);

#[async_trait::async_trait]
impl TaskHandler for CustomHandlerAdapter {
    fn handler_type(&self) -> &str {
        self.0.task_type()
    }

    async fn handle(
        &self,
        task_name: &str,
        config: &Value,
        input: &Value,
        context: &HandlerContext,
    ) -> WorkflowResult<Value> {
        self.0
            .handle(task_name, self.0.task_type(), config, input, context)
            .await
    }
}

/// Registry of call, run, and custom task handlers.
///
/// Internally stores all handlers in a unified map keyed by handler type.
/// Legacy `get_call_handler` / `get_run_handler` / `get_custom_task_handler` methods
/// remain for backward compatibility, but `get_handler` provides unified access.
#[derive(Default, Clone)]
pub struct HandlerRegistry {
    handlers: std::sync::Arc<std::collections::HashMap<String, std::sync::Arc<dyn TaskHandler>>>,
    // Legacy typed maps kept for backward-compatible getter methods
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

    /// Registers a unified [`TaskHandler`].
    pub fn register_handler(&mut self, handler: std::sync::Arc<dyn TaskHandler>) {
        let key = handler.handler_type().to_string();
        Arc::make_mut(&mut self.handlers).insert(key, handler);
    }

    /// Registers a call handler (backward compatible)
    pub fn register_call_handler(&mut self, handler: Box<dyn CallHandler>) {
        let key = handler.call_type().to_string();
        let arc: std::sync::Arc<dyn CallHandler> = std::sync::Arc::from(handler);
        Arc::make_mut(&mut self.handlers).insert(
            key.clone(),
            std::sync::Arc::new(CallHandlerAdapter(arc.clone())),
        );
        Arc::make_mut(&mut self.call_handlers).insert(key, arc);
    }

    /// Registers a run handler (backward compatible)
    pub fn register_run_handler(&mut self, handler: Box<dyn RunHandler>) {
        let key = handler.run_type().to_string();
        let arc: std::sync::Arc<dyn RunHandler> = std::sync::Arc::from(handler);
        Arc::make_mut(&mut self.handlers).insert(
            key.clone(),
            std::sync::Arc::new(RunHandlerAdapter(arc.clone())),
        );
        Arc::make_mut(&mut self.run_handlers).insert(key, arc);
    }

    /// Registers a custom task handler (backward compatible)
    pub fn register_custom_task_handler(&mut self, handler: Box<dyn CustomTaskHandler>) {
        let key = handler.task_type().to_string();
        let arc: std::sync::Arc<dyn CustomTaskHandler> = std::sync::Arc::from(handler);
        Arc::make_mut(&mut self.handlers).insert(
            key.clone(),
            std::sync::Arc::new(CustomHandlerAdapter(arc.clone())),
        );
        Arc::make_mut(&mut self.custom_task_handlers).insert(key, arc);
    }

    /// Looks up a handler by type from the unified registry
    pub fn get_handler(&self, handler_type: &str) -> Option<std::sync::Arc<dyn TaskHandler>> {
        self.handlers.get(handler_type).cloned()
    }

    /// Looks up a call handler by type (backward compatible)
    pub fn get_call_handler(&self, call_type: &str) -> Option<std::sync::Arc<dyn CallHandler>> {
        self.call_handlers.get(call_type).cloned()
    }

    /// Looks up a run handler by type (backward compatible)
    pub fn get_run_handler(&self, run_type: &str) -> Option<std::sync::Arc<dyn RunHandler>> {
        self.run_handlers.get(run_type).cloned()
    }

    /// Looks up a custom task handler by task type (backward compatible)
    pub fn get_custom_task_handler(
        &self,
        task_type: &str,
    ) -> Option<std::sync::Arc<dyn CustomTaskHandler>> {
        self.custom_task_handlers.get(task_type).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct TestConfig {
        name: String,
        #[serde(default)]
        count: u32,
    }

    struct TestTypedHandler;

    #[async_trait::async_trait]
    impl TypedCustomTaskHandler for TestTypedHandler {
        type Config = TestConfig;

        fn task_type(&self) -> &str {
            "test_typed"
        }

        async fn handle(
            &self,
            _task_name: &str,
            config: &TestConfig,
            _input: &Value,
            _context: &HandlerContext,
        ) -> WorkflowResult<Value> {
            Ok(serde_json::json!({
                "name": config.name,
                "count": config.count,
            }))
        }
    }

    #[tokio::test]
    async fn test_typed_handler_wrapper() {
        let handler = TestTypedHandler.into_boxed();
        let ctx = HandlerContext::from_vars(&std::collections::HashMap::new());
        let config = serde_json::json!({ "name": "hello", "count": 42 });

        let result = handler
            .handle("task1", "test_typed", &config, &serde_json::json!({}), &ctx)
            .await
            .unwrap();

        assert_eq!(result["name"], "hello");
        assert_eq!(result["count"], 42);
    }

    #[tokio::test]
    async fn test_typed_handler_invalid_config_returns_validation_error() {
        let handler = TestTypedHandler.into_boxed();
        let ctx = HandlerContext::from_vars(&std::collections::HashMap::new());
        let bad_config = serde_json::json!({ "count": 5 });

        let result = handler
            .handle(
                "task1",
                "test_typed",
                &bad_config,
                &serde_json::json!({}),
                &ctx,
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid config"));
        assert!(err.to_string().contains("test_typed"));
    }

    #[tokio::test]
    async fn test_typed_handler_register_in_registry() {
        let mut registry = HandlerRegistry::new();
        registry.register_custom_task_handler(TestTypedHandler.into_boxed());

        let handler = registry.get_custom_task_handler("test_typed");
        assert!(handler.is_some());

        let ctx = HandlerContext::from_vars(&std::collections::HashMap::new());
        let config = serde_json::json!({ "name": "world" });
        let result = handler
            .unwrap()
            .handle("task1", "test_typed", &config, &serde_json::json!({}), &ctx)
            .await
            .unwrap();
        assert_eq!(result["name"], "world");
        assert_eq!(result["count"], 0);
    }
}
