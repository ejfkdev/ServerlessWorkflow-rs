/// Represents the execution status phase of a workflow or task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusPhase {
    /// The workflow/task has been initiated and is pending execution
    Pending,
    /// The workflow/task is currently in progress
    Running,
    /// The workflow/task execution is temporarily paused
    Waiting,
    /// The workflow/task execution has been manually paused
    Suspended,
    /// The workflow/task execution has been terminated before completion
    Cancelled,
    /// The workflow/task execution has encountered an error
    Faulted,
    /// The workflow/task ran to completion
    Completed,
}

impl StatusPhase {
    /// Returns the string representation of the status phase
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusPhase::Pending => "pending",
            StatusPhase::Running => "running",
            StatusPhase::Waiting => "waiting",
            StatusPhase::Suspended => "suspended",
            StatusPhase::Cancelled => "cancelled",
            StatusPhase::Faulted => "faulted",
            StatusPhase::Completed => "completed",
        }
    }
}

impl std::fmt::Display for StatusPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A log entry recording a status phase transition with timestamp
#[derive(Debug, Clone, PartialEq)]
pub struct StatusPhaseLog {
    /// Unix timestamp in milliseconds
    pub timestamp: i64,
    /// The status phase
    pub status: StatusPhase,
}

impl StatusPhaseLog {
    /// Creates a new status phase log entry with the current timestamp
    pub fn new(status: StatusPhase) -> Self {
        Self {
            timestamp: chrono::Utc::now().timestamp_millis(),
            status,
        }
    }
}
