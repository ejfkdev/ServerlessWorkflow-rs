#![allow(clippy::result_large_err)]
#![doc = include_str!("../README.md")]

pub mod context;
pub mod error;
pub mod events;
pub mod handler;
pub mod listener;
pub mod runner;
pub mod secret;
pub mod status;

pub(crate) mod expression;
pub(crate) mod json_pointer;
pub(crate) mod json_schema;
pub(crate) mod task_runner;
pub(crate) mod tasks;
pub(crate) mod utils;

#[cfg(test)]
pub(crate) mod test_utils;

pub use context::WorkflowContext;
pub use error::{ErrorFields, WorkflowError, WorkflowResult};
pub use events::{CloudEvent, EventBus, EventSubscription, InMemoryEventBus, SharedEventBus};
pub use handler::{CallHandler, CustomTaskHandler, HandlerRegistry, RunHandler};
pub use listener::{
    CollectingListener, MultiListener, NoOpListener, WorkflowEvent, WorkflowExecutionListener,
};
pub use runner::{ScheduledWorkflow, WorkflowHandle, WorkflowRunner};
pub use secret::{EnvSecretManager, MapSecretManager, SecretManager};
pub use status::{StatusPhase, StatusPhaseLog};
