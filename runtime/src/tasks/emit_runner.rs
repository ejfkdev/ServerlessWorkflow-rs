use crate::error::WorkflowResult;
use crate::events::CloudEvent;
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::tasks::define_simple_task_runner;
use crate::tasks::task_name_impl;
use serde_json::Value;
use swf_core::models::task::EmitTaskDefinition;

define_simple_task_runner!(
    /// Runner for Emit tasks - emits events
    ///
    /// By default, emitted events are logged. To integrate with an external event bus,
    /// implement the `EventHandler` trait and register it via `set_event_handler()`.
    EmitTaskRunner, EmitTaskDefinition
);

#[async_trait::async_trait]
impl TaskRunner for EmitTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        // Evaluate event data expressions
        let mut event_data =
            crate::error::serialize_to_value(&self.task.emit.event, "emit event data", &self.name)?;

        support.eval_traverse(&mut event_data, &input)?;

        // The emit task returns the input unchanged (events are side effects).
        // The evaluated event data is available in the task's raw output for downstream use.
        support.set_task_raw_output(&event_data);

        // Publish to EventBus if configured
        if let Some(event_bus) = support.clone_event_bus() {
            let event_type = event_data
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let source = event_data
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("/serverless-workflow")
                .to_string();
            let data = event_data.get("data").cloned().unwrap_or(Value::Null);

            let mut cloud_event = CloudEvent::new(&event_type, data);
            cloud_event = cloud_event.with_source(&source);

            // Add CloudEvent mandatory fields
            let id = event_data
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "evt-{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    )
                });
            cloud_event = cloud_event.with_attribute("id", Value::String(id));

            let time = event_data
                .get("time")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                });
            cloud_event = cloud_event.with_attribute("time", Value::String(time));

            // Add optional CloudEvent attributes
            if let Some(subject) = event_data.get("subject").and_then(|v| v.as_str()) {
                cloud_event =
                    cloud_event.with_attribute("subject", Value::String(subject.to_string()));
            }
            if let Some(dct) = event_data.get("datacontenttype").and_then(|v| v.as_str()) {
                cloud_event =
                    cloud_event.with_attribute("datacontenttype", Value::String(dct.to_string()));
            }
            if let Some(ds) = event_data.get("dataschema").and_then(|v| v.as_str()) {
                cloud_event =
                    cloud_event.with_attribute("dataschema", Value::String(ds.to_string()));
            }

            event_bus.publish(cloud_event).await;
        }

        Ok(input)
    }

    task_name_impl!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_support;
    use serde_json::json;
    use swf_core::models::event::EventDefinition;
    use swf_core::models::task::{EmitTaskDefinition, EventEmissionDefinition};
    use swf_core::models::workflow::WorkflowDefinition;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_emit_task_returns_input() {
        let event_def = EventDefinition::default();
        let task = EmitTaskDefinition::new(EventEmissionDefinition::new(event_def));
        let runner = EmitTaskRunner::new("emitTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let input = json!({"existing": "data"});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_emit_with_source_and_type() {
        // Matches Java SDK's emit-out.yaml - emit with source and type
        let mut with = HashMap::new();
        with.insert("source".to_string(), json!("https://hospital.com"));
        with.insert(
            "type".to_string(),
            json!("com.fake-hospital.patient.checked-out"),
        );

        let event_def = EventDefinition::new(with);
        let task = EmitTaskDefinition::new(EventEmissionDefinition::new(event_def));
        let runner = EmitTaskRunner::new("emitOut", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let input = json!({"patient": "John"});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_emit_with_data() {
        // Matches Java SDK's emit.yaml - emit with source, type, and data
        let mut with = HashMap::new();
        with.insert("source".to_string(), json!("https://petstore.com"));
        with.insert("type".to_string(), json!("com.petstore.order.placed.v1"));
        with.insert(
            "data".to_string(),
            json!({
                "client": {"firstName": "Cruella", "lastName": "de Vil"},
                "items": [{"breed": "dalmatian", "quantity": 101}]
            }),
        );

        let event_def = EventDefinition::new(with);
        let task = EmitTaskDefinition::new(EventEmissionDefinition::new(event_def));
        let runner = EmitTaskRunner::new("emitEvent", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let input = json!({"orderId": "123"});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        // Emit returns input unchanged
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_emit_with_expression_data() {
        // Emit with runtime expression in event data
        let mut with = HashMap::new();
        with.insert("source".to_string(), json!("https://example.com"));
        with.insert("type".to_string(), json!("com.example.order.created"));
        with.insert("data".to_string(), json!("${ .orderDetails }"));

        let event_def = EventDefinition::new(with);
        let task = EmitTaskDefinition::new(EventEmissionDefinition::new(event_def));
        let runner = EmitTaskRunner::new("emitExpr", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let input = json!({"orderDetails": {"id": "456", "total": 99.99}});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_emit_with_full_event_properties() {
        // Emit with all CloudEvents properties
        let event_def = EventDefinition {
            id: Some("event-123".to_string()),
            source: Some("https://example.com/source".to_string()),
            type_: Some("com.example.event.occurred".to_string()),
            time: Some("2025-01-15T10:30:00Z".to_string()),
            subject: Some("my-subject".to_string()),
            data_content_type: Some("application/json".to_string()),
            data_schema: None,
            data: None,
            with: HashMap::new(),
        };
        let task = EmitTaskDefinition::new(EventEmissionDefinition::new(event_def));
        let runner = EmitTaskRunner::new("emitFull", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let input = json!({"data": "here"});
        let output = runner.run(input.clone(), &mut support).await.unwrap();
        assert_eq!(output, input);
    }
}
