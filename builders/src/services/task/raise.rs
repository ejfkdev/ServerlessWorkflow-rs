use super::*;

// ============== RaiseErrorDefinitionBuilder ==============
/// Builder for constructing an error definition used in raise tasks.
pub struct RaiseErrorDefinitionBuilder {
    error: ErrorDefinition,
}

impl RaiseErrorDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            error: ErrorDefinition::default(),
        }
    }

    /// Sets the error type URI template.
    pub fn with_type(&mut self, type_: &str) -> &mut Self {
        self.error.type_ = ErrorType::uri_template(type_);
        self
    }

    /// Sets the error status value.
    pub fn with_status(&mut self, status: Value) -> &mut Self {
        self.error.status = status;
        self
    }

    /// Sets the error title.
    pub fn with_title(&mut self, title: &str) -> &mut Self {
        self.error.title = Some(title.to_string());
        self
    }

    /// Sets the error detail message.
    pub fn with_detail(&mut self, detail: &str) -> &mut Self {
        self.error.detail = Some(detail.to_string());
        self
    }

    /// Sets the error instance URI.
    pub fn with_instance(&mut self, instance: &str) -> &mut Self {
        self.error.instance = Some(instance.to_string());
        self
    }

    /// Builds the error definition as an inline definition (not a reference).
    pub fn build(self) -> OneOfErrorDefinitionOrReference {
        OneOfErrorDefinitionOrReference::Error(self.error)
    }
}

impl Default for RaiseErrorDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============== RaiseTaskDefinitionBuilder ==============
/// Builder for constructing a raise task that produces an error.
#[derive(Default)]
pub struct RaiseTaskDefinitionBuilder {
    task: RaiseTaskDefinition,
    error_builder: Option<RaiseErrorDefinitionBuilder>,
}

impl RaiseTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an inline error definition using a builder.
    pub fn error(&mut self) -> &mut RaiseErrorDefinitionBuilder {
        self.error_builder = Some(RaiseErrorDefinitionBuilder::new());
        self.get_error_builder()
    }

    /// References a named error definition instead of defining one inline.
    pub fn error_reference(&mut self, ref_name: &str) -> &mut Self {
        self.task.raise.error = OneOfErrorDefinitionOrReference::Reference(ref_name.to_string());
        self.error_builder = None;
        self
    }

    fn get_error_builder(&mut self) -> &mut RaiseErrorDefinitionBuilder {
        if let Some(ref mut builder) = self.error_builder {
            builder
        } else {
            unreachable!("Error builder should be set")
        }
    }
}

impl TaskDefinitionBuilderBase for RaiseTaskDefinitionBuilder {
    fn if_(&mut self, condition: &str) -> &mut Self {
        self.task.common.if_ = Some(condition.to_string());
        self
    }

    fn with_timeout_reference(&mut self, reference: &str) -> &mut Self {
        self.task.common.timeout = Some(OneOfTimeoutDefinitionOrReference::Reference(
            reference.to_string(),
        ));
        self
    }

    fn with_timeout<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TimeoutDefinitionBuilder),
    {
        let mut builder = TimeoutDefinitionBuilder::new();
        setup(&mut builder);
        let timeout = builder.build();
        self.task.common.timeout = Some(OneOfTimeoutDefinitionOrReference::Timeout(timeout));
        self
    }

    fn with_input<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut InputDataModelDefinitionBuilder),
    {
        let mut builder = InputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.input = Some(builder.build());
        self
    }

    fn with_output<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut OutputDataModelDefinitionBuilder),
    {
        let mut builder = OutputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.output = Some(builder.build());
        self
    }

    fn with_export<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut OutputDataModelDefinitionBuilder),
    {
        let mut builder = OutputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.export = Some(builder.build());
        self
    }

    fn then(&mut self, directive: &str) -> &mut Self {
        self.task.common.then = Some(directive.to_string());
        self
    }

    fn build(mut self) -> TaskDefinition {
        if let Some(error_builder) = self.error_builder.take() {
            self.task.raise.error = error_builder.build();
        }
        TaskDefinition::Raise(self.task)
    }
}
