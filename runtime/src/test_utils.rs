#[cfg(test)]
pub(crate) mod test_helpers {
    use serde_json::Value;
    use serverless_workflow_core::models::task::{
        SetTaskDefinition, SetValue, TaskDefinition, TaskDefinitionFields,
    };
    use std::collections::HashMap;

    /// Creates a Set task that sets a single key to a value
    pub fn make_set_task(key: &str, value: impl Into<Value>) -> TaskDefinition {
        let mut map = HashMap::new();
        map.insert(key.to_string(), value.into());
        TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(map),
            common: TaskDefinitionFields::new(),
        })
    }
}
