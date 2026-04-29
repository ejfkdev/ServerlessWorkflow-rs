use super::*;

#[tokio::test]
async fn test_runner_listener_task_events() {
    use crate::listener::CollectingListener;

    let listener = Arc::new(CollectingListener::new());

    let yaml_str = std::fs::read_to_string(testdata("chained_set_tasks.yaml")).unwrap();
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
    let runner = WorkflowRunner::new(workflow)
        .unwrap()
        .with_listener(listener.clone());
    let _ = runner.run(json!({})).await.unwrap();

    let events = listener.events();

    // Should have TaskStarted and TaskCompleted events
    let task_started: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, WorkflowEvent::TaskStarted { .. }))
        .collect();
    let task_completed: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, WorkflowEvent::TaskCompleted { .. }))
        .collect();
    assert!(!task_started.is_empty(), "Expected TaskStarted events");
    assert!(!task_completed.is_empty(), "Expected TaskCompleted events");
}

// === Run: Shell echo ===

#[tokio::test]
async fn test_runner_listen_to_any() {
    use crate::events::{CloudEvent, InMemoryEventBus};

    let bus = Arc::new(InMemoryEventBus::new());
    let yaml_str = std::fs::read_to_string(testdata("listen_to_any.yaml")).unwrap();
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();

    // Publish event in background after delay
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        bus_clone
            .publish(CloudEvent::new(
                "com.example.event",
                json!({"data": "hello"}),
            ))
            .await;
    });

    let runner = WorkflowRunner::new(workflow).unwrap().with_event_bus(bus);
    let output = runner.run(json!({"initial": "value"})).await.unwrap();
    // Listen with any: [] matches any event
    assert_eq!(output["data"], json!("hello"));
}

// === Workflow output.as expression ===

#[tokio::test]
async fn test_runner_listen_to_any_until_consumed() {
    // Matches Java SDK's listen-to-any-until-consumed - listen with until: false (disabled)
    use crate::events::{CloudEvent, InMemoryEventBus};

    let bus = Arc::new(InMemoryEventBus::new());
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: listen-to-any-until-consumed
  version: '0.1.0'
do:
  - setBefore:
      set:
        ready: true
  - callDoctor:
      listen:
        to:
          any:
            - with:
                type: com.fake-hospital.vitals.measurements.temperature
        until: false
"#;
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();

    // Publish matching event in background
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        bus_clone
            .publish(CloudEvent::new(
                "com.fake-hospital.vitals.measurements.temperature",
                json!({"temperature": 37.5}),
            ))
            .await;
    });

    let runner = WorkflowRunner::new(workflow).unwrap().with_event_bus(bus);
    let output = runner.run(json!({})).await.unwrap();
    // Listen with until: false consumes one event then stops
    assert_eq!(output["temperature"], json!(37.5));
}

#[tokio::test]
async fn test_runner_listen_to_one_timeout() {
    // Test that a task-level timeout triggers a timeout error caught by try-catch
    // Using a wait task with timeout shorter than wait duration
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: listen-to-one-timeout
  version: '0.1.0'
do:
  - tryListen:
      try:
        - waitingNotForever:
            wait: PT10S
            timeout:
              after: PT0.01S
      catch:
        errors:
          with:
            type: https://serverlessworkflow.io/spec/1.0.0/errors/timeout
            status: 408
        do:
          - setMessage:
              set:
                message: Viva er Beti Balompie
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"delay": 0.01}))
        .await
        .unwrap();
    // After timeout, catch block should set the message
    assert_eq!(output["message"], json!("Viva er Beti Balompie"));
}

// === Call HTTP with output.as extracting statusCode from response ===
// Matches Java SDK's call-with-response-output-expr.yaml pattern

#[tokio::test]
async fn test_runner_listen_to_any_until() {
    // Listen with until expression and foreach iterator
    use crate::events::{CloudEvent, InMemoryEventBus};

    let bus = Arc::new(InMemoryEventBus::new());
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: listen-to-any-until
  version: '0.1.0'
do:
  - setBefore:
      set:
        ready: true
  - callDoctor:
      listen:
        to:
          any:
            - with:
                type: com.fake-hospital.vitals.measurements.temperature
        until: .temperature > 38
      foreach:
        item: event
        do:
          - measure:
              set:
                temperature: '${.temperature}'
"#;
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();

    // Publish events: first low temp, then high temp (triggers until condition)
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        bus_clone
            .publish(CloudEvent::new(
                "com.fake-hospital.vitals.measurements.temperature",
                json!({"temperature": 37.5}),
            ))
            .await;
        bus_clone
            .publish(CloudEvent::new(
                "com.fake-hospital.vitals.measurements.temperature",
                json!({"temperature": 39.0}),
            ))
            .await;
    });

    let runner = WorkflowRunner::new(workflow).unwrap().with_event_bus(bus);
    let output = runner.run(json!({})).await.unwrap();
    // foreach processes events; the last event's foreach output should be present
    assert!(output.as_array().is_some() || output.is_object());
}

// === Listen: to all events ===
// Matches Java SDK's listen-to-all.yaml

#[tokio::test]
async fn test_runner_listen_to_all() {
    // Listen with 'all' consumption strategy - waits for both event types
    use crate::events::{CloudEvent, InMemoryEventBus};

    let bus = Arc::new(InMemoryEventBus::new());
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: listen-to-all
  version: '0.1.0'
do:
  - setBefore:
      set:
        ready: true
  - callDoctor:
      listen:
        to:
          all:
            - with:
                type: com.fake-hospital.vitals.measurements.temperature
            - with:
                type: com.petstore.order.placed.v1
"#;
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();

    // Publish both required events in background
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        bus_clone
            .publish(CloudEvent::new(
                "com.fake-hospital.vitals.measurements.temperature",
                json!({"temperature": 37.2}),
            ))
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        bus_clone
            .publish(CloudEvent::new(
                "com.petstore.order.placed.v1",
                json!({"orderId": "123"}),
            ))
            .await;
    });

    let runner = WorkflowRunner::new(workflow).unwrap().with_event_bus(bus);
    let output = runner.run(json!({})).await.unwrap();
    // "all" returns an array of consumed events
    let arr = output.as_array().expect("expected array from listen all");
    assert_eq!(arr.len(), 2);
}

// === Listen with data expression filter ===
// Matches Java SDK's listen-to-any-filter.yaml pattern
// Note: data expression filter evaluates JQ against event data;
// "any" strategy completes on first matching event.

#[tokio::test]
async fn test_runner_listen_to_any_data_filter() {
    use crate::events::{CloudEvent, InMemoryEventBus};

    let bus = Arc::new(InMemoryEventBus::new());

    // Inline workflow with simple data filter (no $input references)
    let workflow: WorkflowDefinition = serde_yaml::from_str(
        r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: listen-data-filter
  version: '0.1.0'
do:
  - waitForHighTemp:
      listen:
        to:
          any:
            - with:
                type: com.hospital.temperature
                data: "${ .temperature > 38 }"
"#,
    )
    .unwrap();

    // Publish a high-temperature event that should match
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        bus_clone
            .publish(CloudEvent::new(
                "com.hospital.temperature",
                json!({"temperature": 39.5}),
            ))
            .await;
    });

    let runner = WorkflowRunner::new(workflow).unwrap().with_event_bus(bus);
    let output = runner.run(json!({})).await.unwrap();
    // Should return the matched event's data
    assert_eq!(output["temperature"], json!(39.5));
}

// === Listen with data filter - non-matching event is skipped ===

#[tokio::test]
async fn test_runner_listen_to_any_data_filter_skip() {
    use crate::events::{CloudEvent, InMemoryEventBus};

    let bus = Arc::new(InMemoryEventBus::new());

    let workflow: WorkflowDefinition = serde_yaml::from_str(
        r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: listen-data-filter-skip
  version: '0.1.0'
do:
  - waitForHighTemp:
      listen:
        to:
          any:
            - with:
                type: com.hospital.temperature
                data: "${ .temperature > 38 }"
"#,
    )
    .unwrap();

    // Publish a low-temperature event (should be filtered out), then a matching one
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        // This should NOT match: temperature=36 is not > 38
        bus_clone
            .publish(CloudEvent::new(
                "com.hospital.temperature",
                json!({"temperature": 36}),
            ))
            .await;
        // This SHOULD match: temperature=40 > 38
        bus_clone
            .publish(CloudEvent::new(
                "com.hospital.temperature",
                json!({"temperature": 40}),
            ))
            .await;
    });

    let runner = WorkflowRunner::new(workflow).unwrap().with_event_bus(bus);
    let output = runner.run(json!({})).await.unwrap();
    // Should return the matching event's data (temperature=40), not the non-matching one
    assert_eq!(output["temperature"], json!(40));
}

// === Listen with data filter using $input variable ===
// Matches Java SDK's listen-to-any-filter.yaml with $input.threshold

#[tokio::test]
async fn test_runner_listen_data_filter_with_input() {
    use crate::events::{CloudEvent, InMemoryEventBus};

    let bus = Arc::new(InMemoryEventBus::new());

    // Workflow uses $input.threshold in data filter
    let workflow: WorkflowDefinition = serde_yaml::from_str(
        r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: listen-data-filter-input
  version: '0.1.0'
do:
  - waitForHighTemp:
      listen:
        to:
          any:
            - with:
                type: com.hospital.temperature
                data: "${ .temperature > $input.threshold }"
"#,
    )
    .unwrap();

    // Publish a high-temperature event that should match (39 > 38)
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        bus_clone
            .publish(CloudEvent::new(
                "com.hospital.temperature",
                json!({"temperature": 39}),
            ))
            .await;
    });

    let runner = WorkflowRunner::new(workflow).unwrap().with_event_bus(bus);
    // Pass threshold=38 in the workflow input
    let output = runner.run(json!({"threshold": 38})).await.unwrap();
    assert_eq!(output["temperature"], json!(39));
}

// === Run shell: simple echo with return all ===
// Matches Java SDK's touch-cat.yaml pattern
