use super::*;

    #[tokio::test]
    async fn test_runner_concatenating_strings() {
        let output = run_workflow_from_yaml(&testdata("concatenating_strings.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["fullName"], json!("John Doe"));
    }

    // === Conditional Logic ===

    #[tokio::test]
    async fn test_runner_conditional_logic() {
        let output = run_workflow_from_yaml(&testdata("conditional_logic.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["weather"], json!("hot"));
    }

    // === Set Tasks with Then Directive (control flow) ===

    #[tokio::test]
    async fn test_runner_conditional_logic_input_from() {
        let output = run_workflow_from_yaml(
            &testdata("conditional_logic_input_from.yaml"),
            json!({"localWeather": {"temperature": 30}}),
        )
        .await
        .unwrap();
        assert_eq!(output["weather"], json!("hot"));
    }

    #[tokio::test]
    async fn test_runner_conditional_logic_input_from_cold() {
        let output = run_workflow_from_yaml(
            &testdata("conditional_logic_input_from.yaml"),
            json!({"localWeather": {"temperature": 15}}),
        )
        .await
        .unwrap();
        assert_eq!(output["weather"], json!("cold"));
    }

    // === Sequential Set Colors ===

    #[tokio::test]
    async fn test_runner_sequential_set_colors() {
        let output = run_workflow_from_yaml(
            &testdata("sequential_set_colors.yaml"),
            json!({"colors": []}),
        )
        .await
        .unwrap();
        // Last task has output.as that transforms to resultColors
        assert_eq!(output["resultColors"], json!(["red", "green", "blue"]));
    }

    // === Sequential Set Colors with workflow output.as ===

    #[tokio::test]
    async fn test_runner_sequential_set_colors_output_as() {
        let output = run_workflow_from_yaml(
            &testdata("sequential_set_colors_output_as.yaml"),
            json!({"colors": []}),
        )
        .await
        .unwrap();
        assert_eq!(output["result"], json!(["red", "green", "blue"]));
    }

    // === Set Tasks with Termination (then: end) ===

    #[tokio::test]
    async fn test_runner_conditional_set_enabled() {
        let output =
            run_workflow_from_yaml(&testdata("conditional_set.yaml"), json!({"enabled": true}))
                .await
                .unwrap();
        assert_eq!(output["name"], json!("javierito"));
    }

    #[tokio::test]
    async fn test_runner_conditional_set_disabled() {
        let output =
            run_workflow_from_yaml(&testdata("conditional_set.yaml"), json!({"enabled": false}))
                .await
                .unwrap();
        // Task should be skipped, name not set — input passes through unchanged
        assert!(output.get("name").is_none());
        // Java SDK behavior: when condition is false, input is passed through
        assert_eq!(output["enabled"], json!(false));
    }

    // === Wait then Set ===

    #[tokio::test]
    async fn test_lifecycle_cloud_events_published() {
        use crate::events::{EventBus, InMemoryEventBus};
        use crate::listener::WorkflowEvent;

        let bus = Arc::new(InMemoryEventBus::new());

        // Subscribe to lifecycle events before running the workflow
        let mut sub = bus.subscribe_all().await;

        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: lifecycle-test
  version: '0.1.0'
do:
  - setName:
      set:
        name: test
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_event_bus(bus.clone());

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["name"], "test");

        // Collect lifecycle CloudEvents (with timeout since they're published async)
        let mut lifecycle_types = Vec::new();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(100);
        loop {
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            match tokio::time::timeout(std::time::Duration::from_millis(2), bus.recv(&mut sub))
                .await
            {
                Ok(Some(event)) if event.event_type.starts_with("io.serverlessworkflow.") => {
                    lifecycle_types.push(event.event_type.clone());
                }
                Ok(Some(_)) => continue, // non-lifecycle event
                Ok(None) => break,       // bus closed
                Err(_) => break,         // timeout - no more events
            }
        }

        // Should have: workflow.started, task.started, task.completed, workflow.completed
        assert!(
            lifecycle_types.contains(&WorkflowEvent::WORKFLOW_STARTED_TYPE.to_string()),
            "Expected workflow.started event, got: {:?}",
            lifecycle_types
        );
        assert!(
            lifecycle_types.contains(&WorkflowEvent::WORKFLOW_COMPLETED_TYPE.to_string()),
            "Expected workflow.completed event, got: {:?}",
            lifecycle_types
        );
        assert!(
            lifecycle_types.contains(&WorkflowEvent::TASK_STARTED_TYPE.to_string()),
            "Expected task.started event, got: {:?}",
            lifecycle_types
        );
        assert!(
            lifecycle_types.contains(&WorkflowEvent::TASK_COMPLETED_TYPE.to_string()),
            "Expected task.completed event, got: {:?}",
            lifecycle_types
        );
    }

    // === Runtime expression ($runtime, $workflow variables) ===

    #[tokio::test]
    async fn test_runner_execution_listener() {
        use crate::listener::CollectingListener;

        let listener = Arc::new(CollectingListener::new());

        let yaml_str = std::fs::read_to_string(testdata("chained_set_tasks.yaml")).unwrap();
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_listener(listener.clone());
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["tripled"], json!(60));

        // Check events were collected
        let events = listener.events();
        assert!(
            events.len() >= 2,
            "Expected at least 2 events, got {}",
            events.len()
        );

        // First event should be WorkflowStarted
        assert!(matches!(&events[0], WorkflowEvent::WorkflowStarted { .. }));

        // Last event should be WorkflowCompleted
        assert!(matches!(
            events.last(),
            Some(WorkflowEvent::WorkflowCompleted { .. })
        ));
    }

    #[tokio::test]
    async fn test_runner_then_goto_skip_multiple() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: then-goto-skip
  version: '0.1.0'
do:
  - startTask:
      set:
        start: true
      then: finalTask
  - skippedTask1:
      set:
        skipped1: true
  - skippedTask2:
      set:
        skipped2: true
  - finalTask:
      set:
        end: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        // set replaces entire output: finalTask's set only has "end"
        assert_eq!(output["end"], json!(true));
        assert!(output.get("skipped1").is_none());
        assert!(output.get("skipped2").is_none());
    }

    // === Expression: object construction with computed keys ===

    #[tokio::test]
    async fn test_runner_complex_workflow_combo() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: complex-combo
  version: '0.1.0'
do:
  - initialize:
      set:
        numbers: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        evens: []
        odds: []
  - classify:
      for:
        in: "${ .numbers }"
        each: n
      do:
        - addNum:
            set:
              evens: "${ .evens + (if $n % 2 == 0 then [$n] else [] end) }"
              odds: "${ .odds + (if $n % 2 != 0 then [$n] else [] end) }"
              numbers: "${ .numbers }"
  - result:
      set:
        evenCount: "${ .evens | length }"
        oddCount: "${ .odds | length }"
        evenSum: "${ .evens | add }"
        oddSum: "${ .odds | add }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["evenCount"], json!(5));
        assert_eq!(output["oddCount"], json!(5));
        assert_eq!(output["evenSum"], json!(30));
        assert_eq!(output["oddSum"], json!(25));
    }

    // === Wait with various ISO8601 durations ===

    #[tokio::test]
    async fn test_runner_conditional_set_bare_if_enabled() {
        // Matches Java SDK's conditional-set.yaml
        // Uses bare if condition (no ${} wrapper): if: .enabled
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: conditional-set
  version: '0.1.0'
do:
  - conditionalExpression:
      if: .enabled
      set:
        name: javierito
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"enabled": true})).await.unwrap();
        assert_eq!(output["name"], json!("javierito"));
    }

    #[tokio::test]
    async fn test_runner_conditional_set_bare_if_disabled() {
        // Matches Java SDK's conditional-set.yaml with enabled=false
        // Task is skipped, input passes through unchanged
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: conditional-set-disabled
  version: '0.1.0'
do:
  - conditionalExpression:
      if: .enabled
      set:
        name: javierito
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"enabled": false})).await.unwrap();
        // Task was skipped, enabled should still be false in output
        assert_eq!(output["enabled"], json!(false));
        // name should NOT be set since the task was skipped
        assert!(output.get("name").is_none());
    }

    #[tokio::test]
    async fn test_runner_conditional_logic_input_from_go_pattern() {
        // Go SDK's conditional_logic_input_from.yaml - input.from to extract nested data
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: conditional-input-from
  version: '1.0.0'
input:
  from: '${ .localWeather }'
do:
  - task2:
      set:
        weather: '${ if .temperature > 25 then "hot" else "cold" end }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner
            .run(json!({"localWeather": {"temperature": 34}}))
            .await
            .unwrap();
        assert_eq!(output["weather"], json!("hot"));
    }

    // === Conditional logic with input.from cold ===

    #[tokio::test]
    async fn test_runner_conditional_logic_input_from_cold_go_pattern() {
        // Same workflow but with cold temperature
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: conditional-input-from
  version: '1.0.0'
input:
  from: '${ .localWeather }'
do:
  - task2:
      set:
        weather: '${ if .temperature > 25 then "hot" else "cold" end }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner
            .run(json!({"localWeather": {"temperature": 10}}))
            .await
            .unwrap();
        assert_eq!(output["weather"], json!("cold"));
    }

    // === For colors with index ===

    #[tokio::test]
    async fn test_runner_conditional_logic_with_workflow_input_from() {
        // Go SDK's conditional_logic_input_from.yaml pattern
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: conditional-input-from
  version: '0.1.0'
input:
  from: "${ .localWeather }"
do:
  - task2:
      set:
        weather: "${ if .temperature > 25 then 'hot' else 'cold' end }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner
            .run(json!({"localWeather": {"temperature": 34}}))
            .await
            .unwrap();
        assert_eq!(output["weather"], json!("hot"));
    }

    // =========================================================================
    // Real-world E2E integration tests
    // =========================================================================

    // === E2E: Shell + Set — command output processing ===
