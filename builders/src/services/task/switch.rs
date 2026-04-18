use super::*;

// ============== SwitchTaskDefinitionBuilder ==============
/// Builder for constructing a switch task that selects a branch based on conditions.
#[derive(Default)]
pub struct SwitchTaskDefinitionBuilder {
    task: SwitchTaskDefinition,
}

impl SwitchTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a named switch case using a builder callback.
    pub fn case_<F>(&mut self, name: &str, setup: F) -> &mut Self
    where
        F: FnOnce(&mut SwitchCaseDefinitionBuilder),
    {
        let mut builder = SwitchCaseDefinitionBuilder::new();
        setup(&mut builder);
        let case_def = builder.build();
        self.task.switch.add(name.to_string(), case_def);
        self
    }
}

impl_task_definition_builder_base!(SwitchTaskDefinitionBuilder, task, |v| {
    TaskDefinition::Switch(v)
});

/// Builder for constructing a switch case definition.
pub struct SwitchCaseDefinitionBuilder {
    case_def: SwitchCaseDefinition,
}

impl SwitchCaseDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            case_def: SwitchCaseDefinition::default(),
        }
    }

    /// Sets the condition expression for this case.
    pub fn when(&mut self, when: &str) -> &mut Self {
        self.case_def.when = Some(when.to_string());
        self
    }

    /// Sets the flow directive to execute when this case matches.
    pub fn then(&mut self, then: &str) -> &mut Self {
        self.case_def.then = Some(then.to_string());
        self
    }

    /// Builds the `SwitchCaseDefinition`.
    pub fn build(self) -> SwitchCaseDefinition {
        self.case_def
    }
}

impl Default for SwitchCaseDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
