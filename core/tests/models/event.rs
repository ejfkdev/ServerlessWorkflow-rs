use super::*;

#[test]
fn test_event_filter_definition_serialization() {
    // Test EventFilterDefinition serialization
    use std::collections::HashMap;
    use swf_core::models::event::EventFilterDefinition;

    let mut with = HashMap::new();
    with.insert("type".to_string(), serde_json::json!("my.event.type"));
    with.insert(
        "source".to_string(),
        serde_json::json!("http://example.com"),
    );

    let event_filter = EventFilterDefinition {
        with: Some(with),
        correlate: None,
    };

    let json_str = serde_json::to_string(&event_filter).expect("Failed to serialize event filter");
    assert!(json_str.contains("type"));
}

#[test]
fn test_event_definition_serialization() {
    // Test EventDefinition serialization
    use std::collections::HashMap;
    use swf_core::models::event::EventDefinition;

    let mut with = HashMap::new();
    with.insert("id".to_string(), serde_json::json!("my-event-id"));
    with.insert("type".to_string(), serde_json::json!("my.event.type"));

    let event = EventDefinition::new(with);
    let json_str = serde_json::to_string(&event).expect("Failed to serialize event");
    let deserialized: EventDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.with.contains_key("id"));
}

#[test]
fn test_event_definition_with_all_fields() {
    // Test EventDefinition with all fields (matching Go SDK EventProperties)
    use swf_core::models::event::EventDefinition;

    let event_json = json!({
        "id": "my-event-id",
        "source": "http://example.com/source",
        "type": "com.example.myevent",
        "time": "${ .eventTime }",
        "subject": "My Subject",
        "datacontenttype": "application/json",
        "dataschema": "http://example.com/schema",
        "with": {
            "key": "value"
        }
    });

    let result: Result<EventDefinition, _> = serde_json::from_value(event_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize event definition: {:?}",
        result.err()
    );
    let event = result.unwrap();

    assert_eq!(event.id, Some("my-event-id".to_string()));
    assert_eq!(event.source, Some("http://example.com/source".to_string()));
    assert_eq!(event.type_, Some("com.example.myevent".to_string()));
    assert_eq!(event.time, Some("${ .eventTime }".to_string()));
    assert_eq!(event.subject, Some("My Subject".to_string()));
    assert_eq!(
        event.data_content_type,
        Some("application/json".to_string())
    );
    assert_eq!(
        event.data_schema,
        Some("http://example.com/schema".to_string())
    );
    assert!(event.with.contains_key("key"));
}

#[test]
fn test_event_definition_roundtrip() {
    // Test EventDefinition roundtrip serialization with all fields
    use swf_core::models::event::EventDefinition;

    let event = EventDefinition {
        id: Some("test-id".to_string()),
        source: Some("http://example.com".to_string()),
        type_: Some("test.type".to_string()),
        time: Some("${ .time }".to_string()),
        subject: Some("Test Subject".to_string()),
        data_content_type: Some("application/json".to_string()),
        data_schema: Some("http://example.com/schema".to_string()),
        data: None,
        with: std::collections::HashMap::new(),
    };

    let json_str = serde_json::to_string(&event).expect("Failed to serialize");
    let deserialized: EventDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");

    assert_eq!(event.id, deserialized.id);
    assert_eq!(event.source, deserialized.source);
    assert_eq!(event.type_, deserialized.type_);
}

#[test]
fn test_event_consumption_until_false() {
    // Test EventConsumptionStrategyDefinition with until: false (disabled)
    use swf_core::models::event::{
        EventConsumptionStrategyDefinition, OneOfEventConsumptionStrategyDefinitionOrExpression,
    };

    let json_true = json!({
        "any": [{"with": {"type": "event.type1"}}],
        "until": false
    });

    let result: Result<EventConsumptionStrategyDefinition, _> = serde_json::from_value(json_true);
    assert!(result.is_ok(), "Failed to deserialize: {:?}", result.err());

    let strategy = result.unwrap();
    assert!(strategy.any.is_some());
    assert!(strategy.until.is_some());

    match *strategy.until.unwrap() {
        OneOfEventConsumptionStrategyDefinitionOrExpression::Bool(false) => {}
        _ => panic!("Expected Bool(false) variant"),
    }
}

#[test]
fn test_event_consumption_until_expression() {
    // Test EventConsumptionStrategyDefinition with until: expression
    use swf_core::models::event::{
        EventConsumptionStrategyDefinition, OneOfEventConsumptionStrategyDefinitionOrExpression,
    };

    let json_str = json!({
        "any": [{"with": {"type": "event.type1"}}],
        "until": "${ .done }"
    });

    let result: Result<EventConsumptionStrategyDefinition, _> = serde_json::from_value(json_str);
    assert!(result.is_ok(), "Failed to deserialize: {:?}", result.err());

    let strategy = result.unwrap();
    match *strategy.until.unwrap() {
        OneOfEventConsumptionStrategyDefinitionOrExpression::Expression(expr) => {
            assert_eq!(expr, "${ .done }");
        }
        _ => panic!("Expected Expression variant"),
    }
}

#[test]
fn test_event_consumption_strategy_with_any() {
    use std::collections::HashMap;
    use swf_core::models::event::{EventConsumptionStrategyDefinition, EventFilterDefinition};
    let mut with = HashMap::new();
    with.insert("type".to_string(), serde_json::json!("example.event.type"));
    let filter = EventFilterDefinition {
        with: Some(with),
        ..Default::default()
    };
    let strategy = EventConsumptionStrategyDefinition {
        any: Some(vec![filter]),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&strategy).expect("Failed to serialize");
    assert!(json_str.contains("any"));
    assert!(json_str.contains("example.event.type"));
}

#[test]
fn test_event_consumption_strategy_with_all() {
    use std::collections::HashMap;
    use swf_core::models::event::{EventConsumptionStrategyDefinition, EventFilterDefinition};
    let mut with = HashMap::new();
    with.insert(
        "source".to_string(),
        serde_json::json!("http://example.com"),
    );
    let filter = EventFilterDefinition {
        with: Some(with),
        ..Default::default()
    };
    let strategy = EventConsumptionStrategyDefinition {
        all: Some(vec![filter]),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&strategy).expect("Failed to serialize");
    assert!(json_str.contains("all"));
}

#[test]
fn test_event_definition() {
    use std::collections::HashMap;
    use swf_core::models::event::EventDefinition;
    let mut with = HashMap::new();
    with.insert("id".to_string(), serde_json::json!("event-id"));
    with.insert("type".to_string(), serde_json::json!("example.event.type"));
    let event = EventDefinition::new(with);
    let json_str = serde_json::to_string(&event).expect("Failed to serialize");
    assert!(json_str.contains("event-id"));
    assert!(json_str.contains("example.event.type"));
}

#[test]
fn test_event_consumption_strategy_with_until_expression() {
    // Test EventConsumptionStrategyDefinition with until expression
    use std::collections::HashMap;
    use swf_core::models::event::{EventConsumptionStrategyDefinition, EventFilterDefinition};

    let mut with = HashMap::new();
    with.insert("type".to_string(), serde_json::json!("example.event.type"));

    let strategy = EventConsumptionStrategyDefinition {
        any: Some(vec![EventFilterDefinition {
            with: Some(with),
            ..Default::default()
        }]),
        until: Some(Box::new(swf_core::models::event::OneOfEventConsumptionStrategyDefinitionOrExpression::Expression("workflow.data.done == true".to_string()))),
        ..Default::default()
    };

    let json_str = serde_json::to_string(&strategy).expect("Failed to serialize");
    assert!(json_str.contains("until"));
    assert!(json_str.contains("workflow.data.done == true"));
}

#[test]
fn test_correlation_key_definition() {
    // Test CorrelationKeyDefinition serialization
    use swf_core::models::event::CorrelationKeyDefinition;

    let correlation =
        CorrelationKeyDefinition::new("event.source", Some("http://example.com".to_string()));
    let json_str =
        serde_json::to_string(&correlation).expect("Failed to serialize correlation key");
    assert!(json_str.contains("event.source"));
    let deserialized: CorrelationKeyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(correlation.from, deserialized.from);
}

#[test]
fn test_subscription_iterator_definition() {
    // Test SubscriptionIteratorDefinition serialization
    use swf_core::models::task::SubscriptionIteratorDefinition;

    let iterator = SubscriptionIteratorDefinition {
        item: Some("item".to_string()),
        at: Some("index".to_string()),
        do_: Some(Map::new()),
        ..Default::default()
    };
    let json_str =
        serde_json::to_string(&iterator).expect("Failed to serialize subscription iterator");
    assert!(json_str.contains("item"));
    assert!(json_str.contains("index"));
}

// ==================== Additional Tests from SDK Patterns ====================

#[test]
fn test_event_consumption_all_strategy() {
    // Test event consumption with 'all' strategy
    let strategy_json = json!({
        "all": [
            {
                "with": {
                    "type": "com.example.event1"
                }
            },
            {
                "with": {
                    "type": "com.example.event2"
                }
            }
        ]
    });

    let result: Result<swf_core::models::event::EventConsumptionStrategyDefinition, _> =
        serde_json::from_value(strategy_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize all strategy: {:?}",
        result.err()
    );
}

#[test]
fn test_subscription_iterator() {
    // Test subscription iterator definition
    let iterator_json = json!({
        "IEF": "someExpression"
    });

    let result: Result<swf_core::models::task::SubscriptionIteratorDefinition, _> =
        serde_json::from_value(iterator_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize subscription iterator: {:?}",
        result.err()
    );
}

#[test]
fn test_event_definition_with_multiple_attributes() {
    // Test event definition with multiple attributes
    use std::collections::HashMap;
    let mut with = HashMap::new();
    with.insert("type".to_string(), serde_json::json!("com.example.event"));
    with.insert(
        "source".to_string(),
        serde_json::json!("https://example.com"),
    );
    with.insert("id".to_string(), serde_json::json!("event-123"));

    let event_def = swf_core::models::event::EventDefinition::new(with);
    let json_str = serde_json::to_string(&event_def).expect("Failed to serialize");
    assert!(json_str.contains("com.example.event"));
}

#[test]
fn test_event_read_mode_constants() {
    assert_eq!(EventReadMode::DATA, "data");
    assert_eq!(EventReadMode::ENVELOPE, "envelope");
    assert_eq!(EventReadMode::RAW, "raw");
}
