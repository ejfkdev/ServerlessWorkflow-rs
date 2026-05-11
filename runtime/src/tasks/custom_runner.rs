use crate::error::{WorkflowError, WorkflowResult};
use crate::handler::HandlerContext;
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::tasks::define_simple_task_runner;
use crate::tasks::task_name_impl;
use serde_json::Value;
use swf_core::models::task::CustomTaskDefinition;

define_simple_task_runner!(
    /// Runner for custom/extension task types.
    ///
    /// Delegates to a registered `CustomTaskHandler` via the handler registry.
    /// If no handler is found, returns an error.
    CustomTaskRunner, CustomTaskDefinition
);

#[async_trait::async_trait]
impl TaskRunner for CustomTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        let task_type = self.task.type_.as_deref().unwrap_or("unknown");
        let config =
            crate::error::serialize_to_value(&self.task, "custom task config", &self.name)?;
        let ctx = HandlerContext::from_vars(&support.get_vars());

        // Try unified handler registry first, then fall back to legacy custom handler
        if let Some(handler) = support.get_handler_registry().get_handler(task_type) {
            return handler.handle(&self.name, &config, &input, &ctx).await;
        }
        if let Some(handler) = support
            .get_handler_registry()
            .get_custom_task_handler(task_type)
        {
            return handler
                .handle(&self.name, task_type, &config, &input, &ctx)
                .await;
        }

        Err(WorkflowError::runtime_simple(
            format!("custom task '{}' requires a CustomTaskHandler (register one via WorkflowRunner::with_custom_task_handler())", task_type),
            &self.name,
        ))
    }

    task_name_impl!();
}
