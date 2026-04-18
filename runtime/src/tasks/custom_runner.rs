use crate::error::{WorkflowError, WorkflowResult};
use crate::task_runner::{TaskRunner, TaskSupport};
use serde_json::Value;
use serverless_workflow_core::models::task::CustomTaskDefinition;

/// Runner for custom/extension task types.
///
/// Delegates to a registered `CustomTaskHandler` via the handler registry.
/// If no handler is found, returns an error.
pub struct CustomTaskRunner {
    name: String,
    task: CustomTaskDefinition,
}

impl CustomTaskRunner {
    pub fn new(name: &str, task: &CustomTaskDefinition) -> WorkflowResult<Self> {
        Ok(Self {
            name: name.to_string(),
            task: task.clone(),
        })
    }
}

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
            None => Err(WorkflowError::runtime(
                format!("custom task '{}' requires a CustomTaskHandler (register one via WorkflowRunner::with_custom_task_handler())", task_type),
                &self.name,
                "",
            )),
        }
    }

    fn task_name(&self) -> &str {
        &self.name
    }
}
