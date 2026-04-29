use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Represents the configuration of an event consumption strategy
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventConsumptionStrategyDefinition {
    /// Gets/sets a list containing all the events that must be consumed, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<EventFilterDefinition>>,

    /// Gets/sets a list containing any of the events to consume, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any: Option<Vec<EventFilterDefinition>>,

    /// Gets/sets the single event to consume
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one: Option<EventFilterDefinition>,

    /// Gets/sets the consumption strategy, if any, that defines the events that must be consumed to stop listening
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<Box<OneOfEventConsumptionStrategyDefinitionOrExpression>>,
}

/// Represents the configuration of an event filter
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventFilterDefinition {
    /// Gets/sets a name/value mapping of the attributes filtered events must define. Supports both regular expressions and runtime expressions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<HashMap<String, Value>>,

    /// Gets/sets a name/definition mapping of the correlation to attempt when filtering events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlate: Option<HashMap<String, CorrelationKeyDefinition>>,
}

/// Represents the definition of an event correlation key
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationKeyDefinition {
    /// Gets/sets a runtime expression used to extract the correlation key value from events
    pub from: String,

    /// Gets/sets a constant or a runtime expression, if any, used to determine whether or not the extracted correlation key value matches expectations and should be correlated. If not set, the first extracted value will be used as the correlation key's expectation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect: Option<String>,
}
impl CorrelationKeyDefinition {
    pub fn new(from: &str, expect: Option<String>) -> Self {
        Self {
            from: from.to_string(),
            expect,
        }
    }
}

/// Represents the definition of an event
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventDefinition {
    /// Gets/sets the unique event identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Gets/sets the event source (URI template or runtime expression)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Gets/sets the event type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    /// Gets/sets the event time (ISO 8601 date-time string or runtime expression)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,

    /// Gets/sets the event subject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Gets/sets the data content type
    #[serde(rename = "datacontenttype", skip_serializing_if = "Option::is_none")]
    pub data_content_type: Option<String>,

    /// Gets/sets the data schema (URI template or runtime expression)
    #[serde(rename = "dataschema", skip_serializing_if = "Option::is_none")]
    pub data_schema: Option<String>,

    /// Gets/sets the event data payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    /// Gets/sets a key/value mapping of the attributes of the configured event
    #[serde(default)]
    pub with: HashMap<String, Value>,
}
impl EventDefinition {
    pub fn new(with: HashMap<String, Value>) -> Self {
        Self {
            with,
            ..Default::default()
        }
    }
}

/// Represents a value that can be either a EventConsumptionStrategyDefinition, a runtime expression, or a boolean (for disabled)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOfEventConsumptionStrategyDefinitionOrExpression {
    /// Variant holding an EventConsumptionStrategyDefinition
    Strategy(EventConsumptionStrategyDefinition),
    /// Variant holding a runtime expression string
    Expression(String),
    /// Variant holding a boolean (true = continue, false = disabled)
    Bool(bool),
}
impl Default for OneOfEventConsumptionStrategyDefinitionOrExpression {
    fn default() -> Self {
        OneOfEventConsumptionStrategyDefinitionOrExpression::Expression(String::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_definition_deserialize() {
        let json = r#"{
            "id": "evt-123",
            "source": "https://example.com/events",
            "type": "com.example.event",
            "time": "2025-01-01T00:00:00Z",
            "subject": "user/123",
            "datacontenttype": "application/json",
            "dataschema": "https://example.com/schema"
        }"#;
        let event: EventDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, Some("evt-123".to_string()));
        assert_eq!(event.source, Some("https://example.com/events".to_string()));
        assert_eq!(event.type_, Some("com.example.event".to_string()));
        assert_eq!(event.subject, Some("user/123".to_string()));
        assert_eq!(
            event.data_content_type,
            Some("application/json".to_string())
        );
    }

    #[test]
    fn test_event_definition_roundtrip() {
        let json = r#"{
            "id": "evt-123",
            "source": "https://example.com/events",
            "type": "com.example.event"
        }"#;
        let event: EventDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: EventDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_event_filter_deserialize() {
        let json = r#"{
            "with": {"type": "com.example.event", "source": "https://example.com"},
            "correlate": {
                "userId": {"from": "${ .userId }", "expect": "user-123"}
            }
        }"#;
        let filter: EventFilterDefinition = serde_json::from_str(json).unwrap();
        assert!(filter.with.is_some());
        assert!(filter.correlate.is_some());
        let corr = filter.correlate.unwrap();
        assert_eq!(corr.get("userId").unwrap().from, "${ .userId }");
    }

    #[test]
    fn test_correlation_key_deserialize() {
        let json = r#"{"from": "${ .orderId }", "expect": "order-456"}"#;
        let key: CorrelationKeyDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(key.from, "${ .orderId }");
        assert_eq!(key.expect, Some("order-456".to_string()));
    }

    #[test]
    fn test_correlation_key_without_expect() {
        let json = r#"{"from": "${ .userId }"}"#;
        let key: CorrelationKeyDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(key.from, "${ .userId }");
        assert!(key.expect.is_none());
    }

    #[test]
    fn test_consumption_strategy_all() {
        let json = r#"{
            "all": [
                {"with": {"type": "event1"}},
                {"with": {"type": "event2"}}
            ]
        }"#;
        let strategy: EventConsumptionStrategyDefinition = serde_json::from_str(json).unwrap();
        assert!(strategy.all.is_some());
        assert_eq!(strategy.all.unwrap().len(), 2);
        assert!(strategy.any.is_none());
        assert!(strategy.one.is_none());
    }

    #[test]
    fn test_consumption_strategy_any() {
        let json = r#"{
            "any": [
                {"with": {"type": "event1"}},
                {"with": {"type": "event2"}}
            ]
        }"#;
        let strategy: EventConsumptionStrategyDefinition = serde_json::from_str(json).unwrap();
        assert!(strategy.any.is_some());
        assert!(strategy.all.is_none());
    }

    #[test]
    fn test_consumption_strategy_one() {
        let json = r#"{
            "one": {"with": {"type": "event1"}}
        }"#;
        let strategy: EventConsumptionStrategyDefinition = serde_json::from_str(json).unwrap();
        assert!(strategy.one.is_some());
        assert!(strategy.all.is_none());
        assert!(strategy.any.is_none());
    }

    #[test]
    fn test_consumption_strategy_with_until_strategy() {
        let json = r#"{
            "any": [{"with": {"type": "event1"}}],
            "until": {"any": [{"with": {"type": "stop-event"}}]}
        }"#;
        let strategy: EventConsumptionStrategyDefinition = serde_json::from_str(json).unwrap();
        assert!(strategy.until.is_some());
        match *strategy.until.unwrap() {
            OneOfEventConsumptionStrategyDefinitionOrExpression::Strategy(s) => {
                assert!(s.any.is_some());
            }
            _ => panic!("Expected Strategy variant"),
        }
    }

    #[test]
    fn test_consumption_strategy_with_until_expression() {
        let json = r#"{
            "any": [{"with": {"type": "event1"}}],
            "until": "${ .count >= 5 }"
        }"#;
        let strategy: EventConsumptionStrategyDefinition = serde_json::from_str(json).unwrap();
        assert!(strategy.until.is_some());
        match *strategy.until.unwrap() {
            OneOfEventConsumptionStrategyDefinitionOrExpression::Expression(expr) => {
                assert_eq!(expr, "${ .count >= 5 }");
            }
            _ => panic!("Expected Expression variant"),
        }
    }

    #[test]
    fn test_consumption_strategy_with_until_false() {
        let json = r#"{
            "any": [{"with": {"type": "event1"}}],
            "until": false
        }"#;
        let strategy: EventConsumptionStrategyDefinition = serde_json::from_str(json).unwrap();
        assert!(strategy.until.is_some());
        match *strategy.until.unwrap() {
            OneOfEventConsumptionStrategyDefinitionOrExpression::Bool(b) => {
                assert!(!b);
            }
            _ => panic!("Expected Bool variant"),
        }
    }

    #[test]
    fn test_event_consumption_roundtrip() {
        let json = r#"{
            "all": [{"with": {"type": "event1"}}],
            "until": false
        }"#;
        let strategy: EventConsumptionStrategyDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&strategy).unwrap();
        let deserialized: EventConsumptionStrategyDefinition =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(strategy, deserialized);
    }

    #[test]
    fn test_event_with_additional_properties() {
        let json = r#"{
            "id": "evt-123",
            "type": "com.example.event",
            "customField": "customValue"
        }"#;
        let event: EventDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, Some("evt-123".to_string()));
        assert_eq!(event.type_, Some("com.example.event".to_string()));
    }

    // Additional tests matching Go SDK's task_event_test.go patterns

    #[test]
    fn test_emit_event_properties_full() {
        // Matches Go SDK's TestEmitTask_UnmarshalJSON event properties
        let json = r#"{
            "id": "event-id",
            "source": "http://example.com/source",
            "type": "example.event.type",
            "time": "2023-01-01T00:00:00Z",
            "subject": "example.subject",
            "datacontenttype": "application/json",
            "dataschema": "http://example.com/schema",
            "extra": "value"
        }"#;
        let event: EventDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, Some("event-id".to_string()));
        assert_eq!(event.source, Some("http://example.com/source".to_string()));
        assert_eq!(event.type_, Some("example.event.type".to_string()));
        assert_eq!(event.time, Some("2023-01-01T00:00:00Z".to_string()));
        assert_eq!(event.subject, Some("example.subject".to_string()));
        assert_eq!(
            event.data_content_type,
            Some("application/json".to_string())
        );
        assert_eq!(
            event.data_schema,
            Some("http://example.com/schema".to_string())
        );
    }

    #[test]
    fn test_event_filter_roundtrip() {
        let json = r#"{
            "with": {"type": "com.example.event", "source": "http://example.com/source"},
            "correlate": {
                "orderId": {"from": "${ .orderId }", "expect": "order-123"}
            }
        }"#;
        let filter: EventFilterDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&filter).unwrap();
        let deserialized: EventFilterDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(filter, deserialized);
    }

    #[test]
    fn test_event_definition_roundtrip_full() {
        // Full roundtrip test with all fields
        let json = r#"{
            "id": "evt-456",
            "source": "https://example.com/events",
            "type": "com.example.event",
            "time": "2025-06-15T10:30:00Z",
            "subject": "user/456",
            "datacontenttype": "application/json",
            "dataschema": "https://example.com/schema"
        }"#;
        let event: EventDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: EventDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_consumption_until_disabled() {
        // Matches Go SDK's TestEventConsumptionUntil_MarshalJSON "Until Disabled"
        // until: false means disabled
        let json = r#"{
            "any": [{"with": {"type": "event1"}}],
            "until": false
        }"#;
        let strategy: EventConsumptionStrategyDefinition = serde_json::from_str(json).unwrap();
        match *strategy.until.unwrap() {
            OneOfEventConsumptionStrategyDefinitionOrExpression::Bool(b) => {
                assert!(!b);
            }
            _ => panic!("Expected Bool(false) variant"),
        }
    }

    #[test]
    fn test_consumption_until_expression_string() {
        // Matches Go SDK's TestEventConsumptionUntil_MarshalJSON "Until Condition Set"
        let json = r#"{
            "any": [{"with": {"type": "event1"}}],
            "until": "workflow.data.condition == true"
        }"#;
        let strategy: EventConsumptionStrategyDefinition = serde_json::from_str(json).unwrap();
        match *strategy.until.unwrap() {
            OneOfEventConsumptionStrategyDefinitionOrExpression::Expression(expr) => {
                assert_eq!(expr, "workflow.data.condition == true");
            }
            _ => panic!("Expected Expression variant"),
        }
    }

    #[test]
    fn test_consumption_until_nested_strategy() {
        // Matches Go SDK's TestEventConsumptionUntil_MarshalJSON "Until Nested Strategy"
        let json = r#"{
            "any": [{"with": {"type": "event1"}}],
            "until": {"one": {"with": {"type": "example.event.type"}}}
        }"#;
        let strategy: EventConsumptionStrategyDefinition = serde_json::from_str(json).unwrap();
        match *strategy.until.unwrap() {
            OneOfEventConsumptionStrategyDefinitionOrExpression::Strategy(s) => {
                assert!(s.one.is_some());
                assert!(s.all.is_none());
                assert!(s.any.is_none());
            }
            _ => panic!("Expected Strategy variant"),
        }
    }

    #[test]
    fn test_correlation_key_roundtrip() {
        let json = r#"{"from": "${ .orderId }", "expect": "order-789"}"#;
        let key: CorrelationKeyDefinition = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&key).unwrap();
        let deserialized: CorrelationKeyDefinition = serde_json::from_str(&serialized).unwrap();
        assert_eq!(key, deserialized);
    }

    #[test]
    fn test_event_filter_minimal() {
        // Only "with" field, no correlate
        let json = r#"{"with": {"type": "com.example.event"}}"#;
        let filter: EventFilterDefinition = serde_json::from_str(json).unwrap();
        assert!(filter.with.is_some());
        assert!(filter.correlate.is_none());
    }
}
