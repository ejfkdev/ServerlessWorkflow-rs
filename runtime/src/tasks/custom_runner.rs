use crate::error::{WorkflowError, WorkflowResult};
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

        let handler = support
            .get_handler_registry()
            .get_custom_task_handler(task_type);
        match handler {
            Some(handler) => {
                let config = crate::error::serialize_to_value(&self.task, "custom task config", &self.name)?;
                handler.handle(&self.name, task_type, &config, &input).await
            }
            None => Err(WorkflowError::runtime_simple(
                format!("custom task '{}' requires a CustomTaskHandler (register one via WorkflowRunner::with_custom_task_handler())", task_type),
                &self.name,
            )),
        }
    }

    task_name_impl!();
}
