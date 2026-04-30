use crate::events::CloudEvent;
use serde_json::Value;
use std::sync::Arc;

/// Events emitted during workflow execution
#[derive(Debug, Clone)]
pub enum WorkflowEvent {
    /// Workflow started
    WorkflowStarted { instance_id: String, input: Value },
    /// Workflow completed successfully
    WorkflowCompleted { instance_id: String, output: Value },
    /// Workflow failed with an error
    WorkflowFailed { instance_id: String, error: String },
    /// Workflow suspended
    WorkflowSuspended { instance_id: String },
    /// Workflow resumed after suspension
    WorkflowResumed { instance_id: String },
    /// Workflow cancelled
    WorkflowCancelled { instance_id: String },
    /// Task started
    TaskStarted {
        instance_id: String,
        task_name: String,
    },
    /// Task completed successfully
    TaskCompleted {
        instance_id: String,
        task_name: String,
        output: Value,
    },
    /// Task failed
    TaskFailed {
        instance_id: String,
        task_name: String,
        error: String,
    },
    /// Task retried
    TaskRetried {
        instance_id: String,
        task_name: String,
        attempt: u32,
    },
    /// Task suspended
    TaskSuspended {
        instance_id: String,
        task_name: String,
    },
    /// Task resumed after suspension
    TaskResumed {
        instance_id: String,
        task_name: String,
    },
    /// Task cancelled
    TaskCancelled {
        instance_id: String,
        task_name: String,
    },
    /// Workflow status changed
    WorkflowStatusChanged { instance_id: String, status: String },
}

impl WorkflowEvent {
    /// CloudEvent type constants matching Java SDK's lifecycle event types
    pub const WORKFLOW_STARTED_TYPE: &'static str = "io.serverlessworkflow.workflow.started.v1";
    pub const WORKFLOW_COMPLETED_TYPE: &'static str = "io.serverlessworkflow.workflow.completed.v1";
    pub const WORKFLOW_FAILED_TYPE: &'static str = "io.serverlessworkflow.workflow.faulted.v1";
    pub const WORKFLOW_SUSPENDED_TYPE: &'static str = "io.serverlessworkflow.workflow.suspended.v1";
    pub const WORKFLOW_RESUMED_TYPE: &'static str = "io.serverlessworkflow.workflow.resumed.v1";
    pub const WORKFLOW_CANCELLED_TYPE: &'static str = "io.serverlessworkflow.workflow.cancelled.v1";
    pub const TASK_STARTED_TYPE: &'static str = "io.serverlessworkflow.task.started.v1";
    pub const TASK_COMPLETED_TYPE: &'static str = "io.serverlessworkflow.task.completed.v1";
    pub const TASK_FAILED_TYPE: &'static str = "io.serverlessworkflow.task.faulted.v1";
    pub const TASK_RETRIED_TYPE: &'static str = "io.serverlessworkflow.task.retried.v1";
    pub const TASK_SUSPENDED_TYPE: &'static str = "io.serverlessworkflow.task.suspended.v1";
    pub const TASK_RESUMED_TYPE: &'static str = "io.serverlessworkflow.task.resumed.v1";
    pub const TASK_CANCELLED_TYPE: &'static str = "io.serverlessworkflow.task.cancelled.v1";
    pub const WORKFLOW_STATUS_CHANGED_TYPE: &'static str =
        "io.serverlessworkflow.workflow.status-changed.v1";

    /// Converts this WorkflowEvent to a CloudEvent for publishing to the EventBus
    pub fn to_cloud_event(&self) -> CloudEvent {
        match self {
            WorkflowEvent::WorkflowStarted { instance_id, input } => CloudEvent::new(
                Self::WORKFLOW_STARTED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "startedAt": now_millis(),
                    "input": input,
                }),
            ),
            WorkflowEvent::WorkflowCompleted {
                instance_id,
                output,
            } => CloudEvent::new(
                Self::WORKFLOW_COMPLETED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "completedAt": now_millis(),
                    "output": output,
                }),
            ),
            WorkflowEvent::WorkflowFailed { instance_id, error } => CloudEvent::new(
                Self::WORKFLOW_FAILED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "failedAt": now_millis(),
                    "error": { "detail": error },
                }),
            ),
            WorkflowEvent::WorkflowSuspended { instance_id } => CloudEvent::new(
                Self::WORKFLOW_SUSPENDED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "suspendedAt": now_millis(),
                }),
            ),
            WorkflowEvent::WorkflowResumed { instance_id } => CloudEvent::new(
                Self::WORKFLOW_RESUMED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "resumedAt": now_millis(),
                }),
            ),
            WorkflowEvent::WorkflowCancelled { instance_id } => CloudEvent::new(
                Self::WORKFLOW_CANCELLED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "cancelledAt": now_millis(),
                }),
            ),
            WorkflowEvent::TaskStarted {
                instance_id,
                task_name,
            } => CloudEvent::new(
                Self::TASK_STARTED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "taskName": task_name,
                    "startedAt": now_millis(),
                }),
            ),
            WorkflowEvent::TaskCompleted {
                instance_id,
                task_name,
                output,
            } => CloudEvent::new(
                Self::TASK_COMPLETED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "taskName": task_name,
                    "completedAt": now_millis(),
                    "output": output,
                }),
            ),
            WorkflowEvent::TaskFailed {
                instance_id,
                task_name,
                error,
            } => CloudEvent::new(
                Self::TASK_FAILED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "taskName": task_name,
                    "failedAt": now_millis(),
                    "error": { "detail": error },
                }),
            ),
            WorkflowEvent::TaskRetried {
                instance_id,
                task_name,
                attempt,
            } => CloudEvent::new(
                Self::TASK_RETRIED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "taskName": task_name,
                    "attempt": attempt,
                    "retriedAt": now_millis(),
                }),
            ),
            WorkflowEvent::TaskSuspended {
                instance_id,
                task_name,
            } => CloudEvent::new(
                Self::TASK_SUSPENDED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "taskName": task_name,
                    "suspendedAt": now_millis(),
                }),
            ),
            WorkflowEvent::TaskResumed {
                instance_id,
                task_name,
            } => CloudEvent::new(
                Self::TASK_RESUMED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "taskName": task_name,
                    "resumedAt": now_millis(),
                }),
            ),
            WorkflowEvent::TaskCancelled {
                instance_id,
                task_name,
            } => CloudEvent::new(
                Self::TASK_CANCELLED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "taskName": task_name,
                    "cancelledAt": now_millis(),
                }),
            ),
            WorkflowEvent::WorkflowStatusChanged {
                instance_id,
                status,
            } => CloudEvent::new(
                Self::WORKFLOW_STATUS_CHANGED_TYPE,
                serde_json::json!({
                    "instanceId": instance_id,
                    "changedAt": now_millis(),
                    "status": status,
                }),
            ),
        }
    }
}

/// Returns current time as milliseconds since epoch
pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Trait for listening to workflow execution events
///
/// Implement this trait to observe workflow execution lifecycle events.
/// This is useful for logging, metrics, tracing, and debugging.
///
/// # Example
///
/// ```
/// use swf_runtime::listener::{WorkflowExecutionListener, WorkflowEvent};
///
/// struct LoggingListener;
///
/// impl WorkflowExecutionListener for LoggingListener {
///     fn on_event(&self, event: &WorkflowEvent) {
///         match event {
///             WorkflowEvent::WorkflowStarted { instance_id, .. } => {
///                 println!("Workflow {} started", instance_id);
///             }
///             WorkflowEvent::TaskCompleted { task_name, .. } => {
///                 println!("Task {} completed", task_name);
///             }
///             _ => {}
///         }
///     }
/// }
/// ```
pub trait WorkflowExecutionListener: Send + Sync {
    /// Called when a workflow execution event occurs
    fn on_event(&self, event: &WorkflowEvent);
}

/// A listener that collects events in a thread-safe Vec
#[derive(Debug, Default)]
pub struct CollectingListener {
    events: std::sync::Mutex<Vec<WorkflowEvent>>,
}

impl CollectingListener {
    /// Creates a new empty CollectingListener
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all collected events
    pub fn events(&self) -> Vec<WorkflowEvent> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Returns the number of collected events
    pub fn len(&self) -> usize {
        self.events.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Returns true if no events have been collected
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears all collected events
    pub fn clear(&self) {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
}

impl WorkflowExecutionListener for CollectingListener {
    fn on_event(&self, event: &WorkflowEvent) {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(event.clone());
    }
}

/// A no-op listener that does nothing
#[derive(Debug, Default)]
pub struct NoOpListener;

impl WorkflowExecutionListener for NoOpListener {
    fn on_event(&self, _event: &WorkflowEvent) {}
}

/// Multi-listener that delegates to multiple listeners
pub struct MultiListener {
    listeners: Vec<Arc<dyn WorkflowExecutionListener>>,
}

impl MultiListener {
    /// Creates a new MultiListener
    pub fn new(listeners: Vec<Arc<dyn WorkflowExecutionListener>>) -> Self {
        Self { listeners }
    }
}

impl WorkflowExecutionListener for MultiListener {
    fn on_event(&self, event: &WorkflowEvent) {
        for listener in &self.listeners {
            listener.on_event(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_collecting_listener() {
        let listener = CollectingListener::new();
        assert!(listener.is_empty());

        listener.on_event(&WorkflowEvent::WorkflowStarted {
            instance_id: "test-1".to_string(),
            input: json!({}),
        });
        listener.on_event(&WorkflowEvent::TaskStarted {
            instance_id: "test-1".to_string(),
            task_name: "task1".to_string(),
        });
        assert_eq!(listener.len(), 2);

        let events = listener.events();
        assert!(
            matches!(&events[0], WorkflowEvent::WorkflowStarted { instance_id, .. } if instance_id == "test-1")
        );
        assert!(
            matches!(&events[1], WorkflowEvent::TaskStarted { task_name, .. } if task_name == "task1")
        );
    }

    #[test]
    fn test_multi_listener() {
        let l1 = Arc::new(CollectingListener::new());
        let l2 = Arc::new(CollectingListener::new());

        let multi = MultiListener::new(vec![l1.clone(), l2.clone()]);
        multi.on_event(&WorkflowEvent::WorkflowStarted {
            instance_id: "test".to_string(),
            input: json!({}),
        });

        assert_eq!(l1.len(), 1);
        assert_eq!(l2.len(), 1);
    }
}
