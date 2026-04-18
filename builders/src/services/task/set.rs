use super::*;

// ============== SetTaskDefinitionBuilder ==============
/// Builder for constructing a set task that assigns variables in the workflow state.
#[derive(Default)]
pub struct SetTaskDefinitionBuilder {
    task: SetTaskDefinition,
}

impl SetTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces all set variables with the provided map.
    pub fn variables(&mut self, vars: HashMap<String, Value>) -> &mut Self {
        self.task.set = SetValue::Map(vars);
        self
    }

    /// Adds or updates a single variable in the set task.
    pub fn put(&mut self, key: &str, value: Value) -> &mut Self {
        match &mut self.task.set {
            SetValue::Map(map) => {
                map.insert(key.to_string(), value);
            }
            _ => {
                let mut map = HashMap::new();
                map.insert(key.to_string(), value);
                self.task.set = SetValue::Map(map);
            }
        }
        self
    }
}

impl_task_definition_builder_base!(SetTaskDefinitionBuilder, task, TaskDefinition::Set);
