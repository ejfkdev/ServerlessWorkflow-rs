use crate::error::{WorkflowError, WorkflowResult};
use crate::expression::prepare_expression;
use crate::task_runner::{TaskRunner, TaskSupport};
use serde_json::Value;
use serverless_workflow_core::models::task::{SetTaskDefinition, SetValue};

/// Runner for Set tasks - evaluates and sets data
pub struct SetTaskRunner {
    name: String,
    set_value: SetValue,
}

impl SetTaskRunner {
    pub fn new(name: &str, task: &SetTaskDefinition) -> WorkflowResult<Self> {
        Ok(Self {
            name: name.to_string(),
            set_value: task.set.clone(),
        })
    }
}

#[async_trait::async_trait]
impl TaskRunner for SetTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        match &self.set_value {
            SetValue::Map(map) => {
                let mut result = serde_json::Map::new();
                for (k, v) in map {
                    let mut evaluated = v.clone();
                    support.eval_traverse(&mut evaluated, &input)?;
                    result.insert(k.clone(), evaluated);
                }
                Ok(Value::Object(result))
            }
            SetValue::Expression(expr) => {
                let sanitized = prepare_expression(expr);
                let result = support.eval_jq(&sanitized, &input, &self.name)?;
                match result {
                    Value::Object(map) => Ok(Value::Object(map)),
                    other => Err(WorkflowError::runtime(
                        format!("expected map output from set expression, got: {}", other),
                        &self.name,
                        "",
                    )),
                }
            }
        }
    }

    fn task_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::WorkflowContext;
    use crate::task_runner::TaskSupport;
    use serde_json::json;
    use serverless_workflow_core::models::task::TaskDefinitionFields;
    use serverless_workflow_core::models::workflow::WorkflowDefinition;
    use std::collections::HashMap;

    /// Helper to create a SetTaskRunner and run it with a minimal support context
    async fn run_set(name: &str, set: SetValue, input: Value) -> WorkflowResult<Value> {
        let task = SetTaskDefinition {
            set,
            common: TaskDefinitionFields::new(),
        };
        let runner = SetTaskRunner::new(name, &task)?;
        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow)?;
        let mut support = TaskSupport::new(&workflow, &mut context);
        runner.run(input, &mut support).await
    }

    #[tokio::test]
    async fn test_set_exec() {
        let input = json!({
            "configuration": {
                "size": {"width": 6, "height": 6},
                "fill": {"red": 69, "green": 69, "blue": 69}
            }
        });
        let mut set_map = HashMap::new();
        set_map.insert("shape".to_string(), json!("circle"));
        set_map.insert("size".to_string(), json!("${ .configuration.size }"));
        set_map.insert("fill".to_string(), json!("${ .configuration.fill }"));

        let output = run_set("task1", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["shape"], json!("circle"));
        assert_eq!(output["size"]["width"], json!(6));
        assert_eq!(output["fill"]["red"], json!(69));
    }

    #[tokio::test]
    async fn test_set_static_values() {
        let input = json!({});
        let mut set_map = HashMap::new();
        set_map.insert("status".to_string(), json!("completed"));
        set_map.insert("count".to_string(), json!(10));

        let output = run_set("task_static", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["status"], json!("completed"));
        assert_eq!(output["count"], json!(10));
    }

    #[tokio::test]
    async fn test_set_nested_structures() {
        let input = json!({
            "order": {"id": 12345, "items": ["item1", "item2"]}
        });
        let mut inner = HashMap::new();
        inner.insert("orderId".to_string(), json!("${ .order.id }"));
        inner.insert("itemCount".to_string(), json!("${ .order.items | length }"));
        let mut set_map = HashMap::new();
        set_map.insert(
            "orderDetails".to_string(),
            Value::Object(inner.into_iter().collect()),
        );

        let output = run_set("task_nested", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["orderDetails"]["orderId"], json!(12345));
        assert_eq!(output["orderDetails"]["itemCount"], json!(2));
    }

    #[tokio::test]
    async fn test_set_static_and_dynamic() {
        let input = json!({
            "config": {"threshold": 100},
            "metrics": {"current": 75}
        });
        let mut set_map = HashMap::new();
        set_map.insert("status".to_string(), json!("active"));
        set_map.insert(
            "remaining".to_string(),
            json!("${ .config.threshold - .metrics.current }"),
        );

        let output = run_set("task_static_dynamic", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["status"], json!("active"));
        assert_eq!(output["remaining"], json!(25));
    }

    #[tokio::test]
    async fn test_set_missing_input_data() {
        let input = json!({});
        let mut set_map = HashMap::new();
        set_map.insert("value".to_string(), json!("${ .missingField }"));

        let output = run_set("task_missing", SetValue::Map(set_map), input)
            .await
            .unwrap();
        // jq returns null for missing fields
        assert_eq!(output["value"], Value::Null);
    }

    #[tokio::test]
    async fn test_set_expressions_with_functions() {
        let input = json!({"values": [1, 2, 3, 4, 5]});
        let mut set_map = HashMap::new();
        set_map.insert("sum".to_string(), json!("${ .values | add }"));

        let output = run_set("task_functions", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["sum"], json!(15));
    }

    #[tokio::test]
    async fn test_set_conditional_expressions() {
        let input = json!({"temperature": 30});
        let mut set_map = HashMap::new();
        set_map.insert(
            "weather".to_string(),
            json!("${ if .temperature > 25 then \"hot\" else \"cold\" end }"),
        );

        let output = run_set("task_conditional", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["weather"], json!("hot"));
    }

    #[tokio::test]
    async fn test_set_default_values() {
        let input = json!({});
        let mut set_map = HashMap::new();
        // jq uses // as alternative operator
        set_map.insert(
            "value".to_string(),
            json!("${ .missingField // \"defaultValue\" }"),
        );

        let output = run_set("task_defaults", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["value"], json!("defaultValue"));
    }

    #[tokio::test]
    async fn test_set_complex_nested() {
        let input = json!({
            "config": {"dimensions": {"width": 10, "height": 5}},
            "meta": {"color": "blue"}
        });
        let mut shape = serde_json::Map::new();
        shape.insert("type".to_string(), json!("rectangle"));
        shape.insert("width".to_string(), json!("${ .config.dimensions.width }"));
        shape.insert(
            "height".to_string(),
            json!("${ .config.dimensions.height }"),
        );
        shape.insert("color".to_string(), json!("${ .meta.color }"));
        shape.insert(
            "area".to_string(),
            json!("${ .config.dimensions.width * .config.dimensions.height }"),
        );
        let mut set_map = HashMap::new();
        set_map.insert("shape".to_string(), Value::Object(shape));

        let output = run_set("task_complex", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["shape"]["type"], json!("rectangle"));
        assert_eq!(output["shape"]["width"], json!(10));
        assert_eq!(output["shape"]["height"], json!(5));
        assert_eq!(output["shape"]["color"], json!("blue"));
        assert_eq!(output["shape"]["area"], json!(50));
    }

    #[tokio::test]
    async fn test_set_multiple_expressions() {
        let input = json!({
            "user": {"name": "Alice", "email": "alice@example.com"}
        });
        let mut set_map = HashMap::new();
        set_map.insert("username".to_string(), json!("${ .user.name }"));
        set_map.insert("contact".to_string(), json!("${ .user.email }"));

        let output = run_set("task_multi", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["username"], json!("Alice"));
        assert_eq!(output["contact"], json!("alice@example.com"));
    }

    #[tokio::test]
    async fn test_set_expression_variant() {
        let input = json!({"a": 1, "b": 2});
        // SetValue::Expression returns the evaluated result directly
        let output = run_set(
            "task_expr",
            SetValue::Expression("{a: .a, b: .b}".to_string()),
            input,
        )
        .await
        .unwrap();
        assert_eq!(output["a"], json!(1));
        assert_eq!(output["b"], json!(2));
    }

    // --- Go SDK pattern tests ---

    #[tokio::test]
    async fn test_set_array_dynamic_index() {
        // Matches Go SDK's TestSetTaskExecutor_ArrayDynamicIndex
        // Dynamic array indexing: .items[.index]
        let input = json!({"items": ["apple", "banana", "cherry"], "index": 1});
        let mut set_map = HashMap::new();
        set_map.insert("selectedItem".to_string(), json!("${ .items[.index] }"));

        let output = run_set("task_array_indexing", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["selectedItem"], json!("banana"));
    }

    #[tokio::test]
    async fn test_set_nested_conditional_logic() {
        // Matches Go SDK's TestSetTaskExecutor_NestedConditionalLogic
        // Nested if-then-else expression
        let input = json!({"age": 20});
        let mut set_map = HashMap::new();
        set_map.insert("status".to_string(), json!("${ if .age < 18 then \"minor\" else if .age < 65 then \"adult\" else \"senior\" end end }"));

        let output = run_set("task_nested_condition", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["status"], json!("adult"));
    }

    #[tokio::test]
    async fn test_set_nested_conditional_logic_minor() {
        // Same nested conditional, but with minor age
        let input = json!({"age": 12});
        let mut set_map = HashMap::new();
        set_map.insert("status".to_string(), json!("${ if .age < 18 then \"minor\" else if .age < 65 then \"adult\" else \"senior\" end end }"));

        let output = run_set("task_nested_condition_minor", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["status"], json!("minor"));
    }

    #[tokio::test]
    async fn test_set_nested_conditional_logic_senior() {
        // Same nested conditional, but with senior age
        let input = json!({"age": 70});
        let mut set_map = HashMap::new();
        set_map.insert("status".to_string(), json!("${ if .age < 18 then \"minor\" else if .age < 65 then \"adult\" else \"senior\" end end }"));

        let output = run_set(
            "task_nested_condition_senior",
            SetValue::Map(set_map),
            input,
        )
        .await
        .unwrap();
        assert_eq!(output["status"], json!("senior"));
    }

    #[tokio::test]
    async fn test_set_runtime_string_interpolation() {
        // Matches Go SDK's TestSetTaskExecutor_RuntimeExpressions
        // String interpolation with \() syntax
        let input = json!({"user": {"firstName": "John", "lastName": "Doe"}});
        let mut set_map = HashMap::new();
        set_map.insert(
            "fullName".to_string(),
            json!("${ \"\\(.user.firstName) \\(.user.lastName)\" }"),
        );

        let output = run_set("task_runtime_expr", SetValue::Map(set_map), input)
            .await
            .unwrap();
        assert_eq!(output["fullName"], json!("John Doe"));
    }
}
