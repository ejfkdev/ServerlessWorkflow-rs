use super::*;

// ============== ListenTaskDefinitionBuilder ==============
/// Builder for constructing a listen task that waits for events.
#[derive(Default)]
pub struct ListenTaskDefinitionBuilder {
    task: ListenTaskDefinition,
}

impl ListenTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the event filter to listen for using a builder callback.
    pub fn to<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut EventFilterDefinitionBuilder),
    {
        let mut builder = EventFilterDefinitionBuilder::new();
        setup(&mut builder);
        let filter = builder.build();
        let strategy = EventConsumptionStrategyDefinition {
            one: Some(filter),
            ..Default::default()
        };
        self.task.listen.to = strategy;
        self
    }
}

impl_task_definition_builder_base!(ListenTaskDefinitionBuilder, task, |v| {
    TaskDefinition::Listen(Box::new(v))
});

// ============== EventFilterDefinitionBuilder ==============
/// Builder for constructing an `EventFilterDefinition` used in listen tasks.
pub struct EventFilterDefinitionBuilder {
    filter: EventFilterDefinition,
}

impl EventFilterDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            filter: EventFilterDefinition::default(),
        }
    }

    /// Adds a single attribute to the event filter.
    pub fn with(&mut self, name: &str, value: Value) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(HashMap::new)
            .insert(name.to_string(), value);
        self
    }

    /// Builds the `EventFilterDefinition`.
    pub fn build(self) -> EventFilterDefinition {
        self.filter
    }
}

impl Default for EventFilterDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
