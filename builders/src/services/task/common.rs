use super::*;

// ============== InputDataModelDefinitionBuilder ==============
/// Builder for constructing an `InputDataModelDefinition` that configures task input data processing.
pub struct InputDataModelDefinitionBuilder {
    input: InputDataModelDefinition,
}

impl InputDataModelDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            input: InputDataModelDefinition::default(),
        }
    }

    /// Sets the `from` expression used to transform or filter task input data.
    pub fn from(&mut self, value: Value) -> &mut Self {
        self.input.from = Some(value);
        self
    }

    /// Builds the `InputDataModelDefinition`.
    pub fn build(self) -> InputDataModelDefinition {
        self.input
    }
}

impl Default for InputDataModelDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============== OutputDataModelDefinitionBuilder ==============
/// Builder for constructing an `OutputDataModelDefinition` that configures task output data processing.
pub struct OutputDataModelDefinitionBuilder {
    output: OutputDataModelDefinition,
}

impl OutputDataModelDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            output: OutputDataModelDefinition::default(),
        }
    }

    /// Sets the `as` expression used to transform task output data.
    pub fn r#as(&mut self, value: Value) -> &mut Self {
        self.output.as_ = Some(value);
        self
    }

    /// Builds the `OutputDataModelDefinition`.
    pub fn build(self) -> OutputDataModelDefinition {
        self.output
    }
}

impl Default for OutputDataModelDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============== EventDefinitionBuilder ==============
/// Builder for constructing an `EventDefinition` used in emit and listen tasks.
pub struct EventDefinitionBuilder {
    event: EventDefinition,
}

impl EventDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            event: EventDefinition::default(),
        }
    }

    /// Adds a single attribute to the event definition.
    pub fn with(&mut self, name: &str, value: Value) -> &mut Self {
        self.event.with.insert(name.to_string(), value);
        self
    }

    /// Replaces all event attributes with the provided map.
    pub fn with_attributes(&mut self, attrs: HashMap<String, Value>) -> &mut Self {
        self.event.with = attrs;
        self
    }

    /// Builds the `EventDefinition`.
    pub fn build(self) -> EventDefinition {
        self.event
    }
}

impl Default for EventDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
