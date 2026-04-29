use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A simplified CloudEvent-like structure for event publishing and consumption
#[derive(Debug, Clone)]
pub struct CloudEvent {
    /// Event type (e.g., "com.example.event.occurred")
    pub event_type: String,
    /// Event source
    pub source: Option<String>,
    /// Event data payload
    pub data: Value,
    /// Additional attributes
    pub attributes: HashMap<String, Value>,
}

impl CloudEvent {
    pub fn new(event_type: &str, data: Value) -> Self {
        Self {
            event_type: event_type.to_string(),
            source: None,
            data,
            attributes: HashMap::new(),
        }
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }

    pub fn with_attribute(mut self, key: &str, value: Value) -> Self {
        self.attributes.insert(key.to_string(), value);
        self
    }

    /// Converts this CloudEvent to a JSON Value for expression evaluation
    pub fn to_json_value(&self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("type".to_string(), Value::String(self.event_type.clone()));
        if let Some(ref source) = self.source {
            obj.insert("source".to_string(), Value::String(source.clone()));
        }
        obj.insert("data".to_string(), self.data.clone());
        for (k, v) in &self.attributes {
            obj.insert(k.clone(), v.clone());
        }
        Value::Object(obj)
    }
}

/// A subscription handle that holds a receiver for events
pub struct EventSubscription {
    /// Unique subscription ID
    pub id: usize,
    /// The event type filter (None means subscribe to all events)
    pub event_type: Option<String>,
    /// The broadcast receiver — stored so events published after subscribe are captured
    receiver: tokio::sync::broadcast::Receiver<CloudEvent>,
}

/// Trait for event bus implementations
///
/// Provides publish/subscribe functionality for workflow events.
/// Used by `emit` tasks to publish and `listen` tasks to consume events.
#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    /// Publish an event to the bus
    async fn publish(&self, event: CloudEvent);

    /// Subscribe to events matching a specific type.
    /// The subscription starts receiving events from the point of subscription.
    async fn subscribe(&self, event_type: &str) -> EventSubscription;

    /// Subscribe to all events
    async fn subscribe_all(&self) -> EventSubscription;

    /// Unsubscribe a previously registered subscription
    async fn unsubscribe(&self, subscription: EventSubscription);

    /// Receive the next event matching a subscription (blocking wait)
    async fn recv(&self, subscription: &mut EventSubscription) -> Option<CloudEvent>;
}

/// In-memory event bus implementation using broadcast channels
pub struct InMemoryEventBus {
    sender: tokio::sync::broadcast::Sender<CloudEvent>,
    next_id: AtomicUsize,
}

const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

impl InMemoryEventBus {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHANNEL_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(capacity);
        Self {
            sender,
            next_id: AtomicUsize::new(0),
        }
    }

    fn allocate_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl EventBus for InMemoryEventBus {
    async fn publish(&self, event: CloudEvent) {
        let _ = self.sender.send(event);
    }

    async fn subscribe(&self, event_type: &str) -> EventSubscription {
        let id = self.allocate_id();
        EventSubscription {
            id,
            event_type: Some(event_type.to_string()),
            receiver: self.sender.subscribe(),
        }
    }

    async fn subscribe_all(&self) -> EventSubscription {
        let id = self.allocate_id();
        EventSubscription {
            id,
            event_type: None,
            receiver: self.sender.subscribe(),
        }
    }

    async fn unsubscribe(&self, _subscription: EventSubscription) {
        // Receiver is dropped when subscription is dropped
    }

    async fn recv(&self, subscription: &mut EventSubscription) -> Option<CloudEvent> {
        loop {
            match subscription.receiver.recv().await {
                Ok(event) => {
                    if let Some(ref filter_type) = subscription.event_type {
                        if event.event_type != *filter_type {
                            continue;
                        }
                    }
                    return Some(event);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    tokio::task::yield_now().await;
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    }
}

/// A shared, thread-safe event bus
pub type SharedEventBus = Arc<dyn EventBus>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_publish_subscribe() {
        let bus = InMemoryEventBus::new();

        let mut sub = bus.subscribe("com.example.test").await;
        bus.publish(CloudEvent::new("com.example.test", json!({"msg": "hello"})))
            .await;

        let event = bus.recv(&mut sub).await.unwrap();
        assert_eq!(event.event_type, "com.example.test");
        assert_eq!(event.data["msg"], "hello");
    }

    #[tokio::test]
    async fn test_subscribe_filters_by_type() {
        let bus = Arc::new(InMemoryEventBus::new());

        let mut sub = bus.subscribe("com.example.target").await;

        let bus_clone = bus.clone();
        tokio::spawn(async move {
            bus_clone
                .publish(CloudEvent::new("com.example.other", json!({})))
                .await;
            bus_clone
                .publish(CloudEvent::new(
                    "com.example.target",
                    json!({"found": true}),
                ))
                .await;
        });

        let event = bus.recv(&mut sub).await.unwrap();
        assert_eq!(event.event_type, "com.example.target");
        assert_eq!(event.data["found"], true);
    }

    #[tokio::test]
    async fn test_subscribe_all() {
        let bus = InMemoryEventBus::new();

        let mut sub = bus.subscribe_all().await;
        bus.publish(CloudEvent::new("type.a", json!({"a": 1})))
            .await;

        let event = bus.recv(&mut sub).await.unwrap();
        assert_eq!(event.event_type, "type.a");
    }

    #[tokio::test]
    async fn test_cloud_event_builder() {
        let event = CloudEvent::new("test.event", json!({"key": "value"}))
            .with_source("https://example.com")
            .with_attribute("correlationId", json!("abc-123"));

        assert_eq!(event.event_type, "test.event");
        assert_eq!(event.source, Some("https://example.com".to_string()));
        assert_eq!(event.attributes["correlationId"], json!("abc-123"));
    }

    #[tokio::test]
    async fn test_multiple_events() {
        let bus = InMemoryEventBus::new();

        let mut sub = bus.subscribe("test").await;
        bus.publish(CloudEvent::new("test", json!(1))).await;
        bus.publish(CloudEvent::new("test", json!(2))).await;
        bus.publish(CloudEvent::new("test", json!(3))).await;

        let e1 = bus.recv(&mut sub).await.unwrap();
        assert_eq!(e1.data, json!(1));
        let e2 = bus.recv(&mut sub).await.unwrap();
        assert_eq!(e2.data, json!(2));
        let e3 = bus.recv(&mut sub).await.unwrap();
        assert_eq!(e3.data, json!(3));
    }
}
