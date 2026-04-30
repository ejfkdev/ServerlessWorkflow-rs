use swf_core::models::duration::OneOfDurationOrIso8601Expression;
use swf_core::models::timeout::TimeoutDefinition;

/// Builder for TimeoutDefinition
pub struct TimeoutDefinitionBuilder {
    timeout: TimeoutDefinition,
}

impl TimeoutDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            timeout: TimeoutDefinition::default(),
        }
    }

    pub fn after(&mut self, duration: impl Into<OneOfDurationOrIso8601Expression>) -> &mut Self {
        self.timeout.after = duration.into();
        self
    }

    pub fn build(self) -> TimeoutDefinition {
        self.timeout
    }
}

impl Default for TimeoutDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
