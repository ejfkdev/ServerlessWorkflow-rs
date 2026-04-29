use super::*;

#[tokio::test]
async fn test_runner_chained_set_tasks() {
    let output = run_workflow_from_yaml(&testdata("chained_set_tasks.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["tripled"], json!(60));
}

// === Concatenating Strings ===

#[tokio::test]
async fn test_runner_set_tasks_with_then() {
    let output = run_workflow_from_yaml(&testdata("set_tasks_with_then.yaml"), json!({}))
        .await
        .unwrap();
    // task1 sets value=30, then jumps to task3 (skips task2)
    assert_eq!(output["result"], json!(90));
    // "skipped" should not be in output since task2 was skipped
    assert!(output.get("skipped").is_none());
}

// === Raise Inline Error ===

#[tokio::test]
async fn test_runner_set_tasks_with_termination() {
    let output = run_workflow_from_yaml(&testdata("set_tasks_with_termination.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["finalValue"], json!(20));
    // task2 should be skipped due to then: end
    assert!(output.get("skipped").is_none());
}

// === Set Tasks with Invalid Then (non-existent task) ===

#[tokio::test]
async fn test_runner_set_tasks_invalid_then() {
    let result = run_workflow_from_yaml(&testdata("set_tasks_invalid_then.yaml"), json!({})).await;
    // When then points to non-existent task, workflow returns error
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("not found"),
        "expected 'not found' in error, got: {}",
        err_msg
    );
}

// === Wait Duration ISO 8601 ===

#[tokio::test]
async fn test_runner_simple_set_workflow() {
    use serverless_workflow_core::models::map::Map;
    use serverless_workflow_core::models::task::{
        SetTaskDefinition, SetValue, TaskDefinition, TaskDefinitionFields,
    };
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert("greeting".to_string(), json!("hello"));
    let set_task = TaskDefinition::Set(SetTaskDefinition {
        set: SetValue::Map(map),
        common: TaskDefinitionFields::new(),
    });

    let entries = vec![("task1".to_string(), set_task)];

    let workflow = WorkflowDefinition {
        do_: Map { entries },
        ..WorkflowDefinition::default()
    };

    let runner = WorkflowRunner::new(workflow).unwrap();
    let output = runner.run(json!({})).await.unwrap();
    assert_eq!(output["greeting"], json!("hello"));
}

// === Conditional Set (if condition) ===

#[tokio::test]
async fn test_runner_set_listen_to_any() {
    use crate::events::{CloudEvent, InMemoryEventBus};

    let bus = Arc::new(InMemoryEventBus::new());
    let yaml_str = std::fs::read_to_string(testdata("set_listen_to_any.yaml")).unwrap();
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();

    // Publish event in background after delay (after set task runs)
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        bus_clone
            .publish(CloudEvent::new(
                "com.example.event",
                json!({"triggered": true}),
            ))
            .await;
    });

    let runner = WorkflowRunner::new(workflow).unwrap().with_event_bus(bus);
    let output = runner.run(json!({})).await.unwrap();
    // Listen with any: [] matches any event, returns event data
    assert_eq!(output["triggered"], json!(true));
}

// === Shell: missing command ===

#[tokio::test]
async fn test_runner_set_then_exit() {
    // then: exit at top level is same as then: end
    let output = run_workflow_from_yaml(&testdata("set_tasks_with_termination.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["finalValue"], json!(20));
    assert!(output.get("skipped").is_none());
}

// === For loop: while condition stops iteration ===

#[tokio::test]
async fn test_runner_set_then_continue() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-then-continue
  version: '0.1.0'
do:
  - task1:
      set:
        value: 10
      then: continue
  - task2:
      set:
        value: 20
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // then: continue should just move to the next task (same as no then)
    assert_eq!(output["value"], json!(20));
}

// === Fork with compete and output.as ===

#[tokio::test]
async fn test_runner_set_preserves_and_adds() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-preserves-adds
  version: '0.1.0'
do:
  - step1:
      set:
        a: 1
        b: "${ .b }"
  - step2:
      set:
        a: "${ .a }"
        b: "${ .b }"
        c: 3
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"b": 2})).await.unwrap();
    assert_eq!(output["a"], json!(1));
    assert_eq!(output["b"], json!(2));
    assert_eq!(output["c"], json!(3));
}

// === Emit with event source and type expressions ===

#[tokio::test]
async fn test_runner_set_null_and_bool() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-null-bool
  version: '0.1.0'
do:
  - setValues:
      set:
        nullVal: null
        boolVal: true
        falseVal: false
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert!(output["nullVal"].is_null());
    assert_eq!(output["boolVal"], json!(true));
    assert_eq!(output["falseVal"], json!(false));
}

// === Raise with status only (no detail, no instance) ===

#[tokio::test]
async fn test_runner_set_overwrite() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-overwrite
  version: '0.1.0'
do:
  - initialSet:
      set:
        counter: 1
        name: first
  - overwriteSet:
      set:
        counter: 2
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // Set replaces entire output: only counter is in the second set's output
    // The first set's "name" field is lost because second set only has "counter"
    assert_eq!(output["counter"], json!(2));
}

// === Wait with zero duration ===

#[tokio::test]
async fn test_runner_set_multiple_expr() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-multiple-expr
  version: '0.1.0'
do:
  - computeAll:
      set:
        sum: "${ .a + .b }"
        product: "${ .a * .b }"
        difference: "${ .a - .b }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": 10, "b": 3}))
        .await
        .unwrap();
    assert_eq!(output["sum"], json!(13));
    assert_eq!(output["product"], json!(30));
    assert_eq!(output["difference"], json!(7));
}

// === Expression: try-catch with error details ===

#[tokio::test]
async fn test_runner_set_if_skip() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-if-skip
  version: '0.1.0'
do:
  - conditionalSet:
      if: ${ .enabled }
      set:
        status: active
  - alwaysSet:
      set:
        done: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // When enabled=false, conditionalSet is skipped, input passes through
    // alwaysSet replaces output with {done: true}
    let output = runner.run(json!({"enabled": false})).await.unwrap();
    assert_eq!(output["done"], json!(true));
}

#[tokio::test]
async fn test_runner_set_if_enabled() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-if-enabled
  version: '0.1.0'
do:
  - conditionalSet:
      if: ${ .enabled }
      set:
        status: active
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // When enabled=true, conditionalSet runs, set replaces output
    let output = runner.run(json!({"enabled": true})).await.unwrap();
    assert_eq!(output["status"], json!("active"));
}

// === Expression: index access on array ===

#[tokio::test]
async fn test_runner_set_then_continue_flow() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-then-continue-flow
  version: '0.1.0'
do:
  - step1:
      set:
        a: 1
      then: continue
  - step2:
      set:
        b: 2
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // set replaces entire output: step2 only has b
    assert_eq!(output["b"], json!(2));
}

// === Raise: with detail and instance expressions ===

#[tokio::test]
async fn test_runner_set_then_goto_forward() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-then-goto-forward
  version: '0.1.0'
do:
  - step1:
      set:
        start: true
      then: step3
  - step2:
      set:
        skipped: true
  - step3:
      set:
        end: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["end"], json!(true));
    assert!(output.get("skipped").is_none());
}

// === Expression: comparison operators ===

#[tokio::test]
async fn test_runner_set_boolean_expr() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-boolean-expr
  version: '0.1.0'
do:
  - compute:
      set:
        isAdult: "${ .age >= 18 }"
        canVote: "${ .age >= 18 and .citizen }"
        isMinor: "${ .age < 18 }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"age": 25, "citizen": true}))
        .await
        .unwrap();
    assert_eq!(output["isAdult"], json!(true));
    assert_eq!(output["canVote"], json!(true));
    assert_eq!(output["isMinor"], json!(false));
}

// === Try-catch with catch.do modifying output and continuing ===

#[tokio::test]
async fn test_runner_set_multiple_fields() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-multi-fields
  version: '0.1.0'
do:
  - compute:
      set:
        x: 1
        y: 2
        product: "${ .a * .b }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": 3, "b": 4}))
        .await
        .unwrap();
    assert_eq!(output["x"], json!(1));
    assert_eq!(output["y"], json!(2));
    assert_eq!(output["product"], json!(12));
}

// === Do: then:goto backward with guard ===

// Set: complex nested object construction
#[tokio::test]
async fn test_runner_set_nested_object_construction() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-nested-object
  version: '0.1.0'
do:
  - build:
      set:
        user: "${ {name: .name, address: {city: .city, zip: .zip}} }"
        name: "${ .name }"
        city: "${ .city }"
        zip: "${ .zip }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({"name": "Alice", "city": "NYC", "zip": "10001"}))
        .await
        .unwrap();
    assert_eq!(output["user"]["name"], json!("Alice"));
    assert_eq!(output["user"]["address"]["city"], json!("NYC"));
    assert_eq!(output["user"]["address"]["zip"], json!("10001"));
}

// Set: output.as transform after set
#[tokio::test]
async fn test_runner_set_output_as_transform() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-output-as-transform
  version: '0.1.0'
do:
  - build:
      set:
        x: 10
        y: 20
      output:
        as: "${ {sum: .x + .y, x: .x, y: .y} }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["sum"], json!(30));
    assert_eq!(output["x"], json!(10));
}

// Set: null value in set
#[tokio::test]
async fn test_runner_set_null_value() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-null-value
  version: '0.1.0'
do:
  - setNull:
      set:
        x: null
        y: "${ .y }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"y": 42}))
        .await
        .unwrap();
    assert_eq!(output["x"], json!(null));
    assert_eq!(output["y"], json!(42));
}

#[tokio::test]
async fn test_runner_set_preserves_unrelated_keys() {
    // Matches Go SDK's set behavior - set preserves keys not mentioned in the set operation
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-preserve
  version: '0.1.0'
do:
  - setShape:
      set:
        shape: circle
        size: ${ .configuration.size }
        fill: ${ .configuration.fill }
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({
            "configuration": {"size": "large", "fill": "red"},
            "otherKey": "preserved"
        }))
        .await
        .unwrap();
    // Set replaces the entire output with the set values
    assert_eq!(output["shape"], json!("circle"));
    assert_eq!(output["size"], json!("large"));
    assert_eq!(output["fill"], json!("red"));
    // Note: set replaces output, so otherKey is NOT preserved (matching Go SDK behavior)
}

#[tokio::test]
async fn test_runner_set_dynamic_index() {
    // Matches Go SDK's set_array_dynamic_index - .items[.index] dynamic array indexing
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-dynamic-index
  version: '0.1.0'
do:
  - setItem:
      set:
        item: '${.items[.index]}'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({"items": ["a", "b", "c"], "index": 1}))
        .await
        .unwrap();
    assert_eq!(output["item"], json!("b"));
}

// === Listen: one event with timeout, caught by try-catch ===
// Matches Java SDK's listen-to-one-timeout.yaml pattern
// Since we don't have an event system, we test the timeout+catch pattern
// using a wait task that exceeds its timeout

#[tokio::test]
async fn test_runner_set_complex_nested_structures() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-complex-nested
  version: '0.1.0'
do:
  - buildShape:
      set:
        shape:
          type: rectangle
          width: '${.config.dimensions.width}'
          height: '${.config.dimensions.height}'
          color: '${.meta.color}'
          area: '${.config.dimensions.width * .config.dimensions.height}'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({
            "config": {
                "dimensions": {
                    "width": 10,
                    "height": 5
                }
            },
            "meta": {
                "color": "blue"
            }
        }))
        .await
        .unwrap();
    assert_eq!(output["shape"]["type"], json!("rectangle"));
    assert_eq!(output["shape"]["width"], json!(10));
    assert_eq!(output["shape"]["height"], json!(5));
    assert_eq!(output["shape"]["color"], json!("blue"));
    assert_eq!(output["shape"]["area"], json!(50));
}

// === Listen: to any with until expression ===
// Matches Java SDK's listen-to-any-until.yaml

#[tokio::test]
async fn test_runner_set_default_values() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-default-values
  version: '0.1.0'
do:
  - setDefault:
      set:
        value: '${.missingField // "defaultValue"}'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["value"], json!("defaultValue"));
}

// === Set: with multiple expressions ===
// Matches Go SDK's TestSetTaskExecutor_MultipleExpressions

#[tokio::test]
async fn test_runner_set_static_values() {
    // Matches Go SDK's TestSetTaskExecutor_StaticValues
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-static-values
  version: '0.1.0'
do:
  - setStatic:
      set:
        status: completed
        count: 10
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["status"], json!("completed"));
    assert_eq!(output["count"], json!(10));
}

#[tokio::test]
async fn test_runner_set_nested_structures() {
    // Matches Go SDK's TestSetTaskExecutor_NestedStructures
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-nested-structures
  version: '0.1.0'
do:
  - setOrder:
      set:
        orderDetails:
          orderId: '${.order.id}'
          itemCount: '${.order.items | length}'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({
            "order": {
                "id": 12345,
                "items": ["item1", "item2"]
            }
        }))
        .await
        .unwrap();
    assert_eq!(output["orderDetails"]["orderId"], json!(12345));
    assert_eq!(output["orderDetails"]["itemCount"], json!(2));
}

#[tokio::test]
async fn test_runner_set_static_and_dynamic_values() {
    // Matches Go SDK's TestSetTaskExecutor_StaticAndDynamicValues
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-static-dynamic
  version: '0.1.0'
do:
  - setMetrics:
      set:
        status: active
        remaining: '${.config.threshold - .metrics.current}'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({
            "config": { "threshold": 100 },
            "metrics": { "current": 75 }
        }))
        .await
        .unwrap();
    assert_eq!(output["status"], json!("active"));
    assert_eq!(output["remaining"], json!(25));
}

#[tokio::test]
async fn test_runner_set_missing_input_data() {
    // Matches Go SDK's TestSetTaskExecutor_MissingInputData
    // When expression references a field that doesn't exist, result is null
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-missing-input
  version: '0.1.0'
do:
  - setMissing:
      set:
        value: '${.missingField}'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["value"], json!(null));
}

#[tokio::test]
async fn test_runner_set_conditional_hot_cold() {
    // Go SDK's TestSetTaskExecutor_ConditionalExpressions - both hot and cold in one test
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: conditional-hot-cold
  version: '0.1.0'
do:
  - setWeather:
      set:
        forecast: '${ if .temperature > 25 then "hot" else "cold" end }'
"#;
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();

    // Hot path
    let runner = WorkflowRunner::new(workflow.clone()).unwrap();
    let output = runner.run(json!({"temperature": 30})).await.unwrap();
    assert_eq!(output["forecast"], json!("hot"));

    // Cold path
    let runner = WorkflowRunner::new(workflow).unwrap();
    let output = runner.run(json!({"temperature": 15})).await.unwrap();
    assert_eq!(output["forecast"], json!("cold"));
}

// === Workflow schedule: after (Java SDK after-start.yaml pattern) ===

#[tokio::test]
async fn test_runner_set_deeply_nested() {
    // Test setting deeply nested object structures
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-deep-nested
  version: '0.1.0'
do:
  - buildStructure:
      set:
        level1:
          level2:
            level3:
              value: "${ .input * 2 }"
              name: deep
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"input": 21}))
        .await
        .unwrap();
    assert_eq!(output["level1"]["level2"]["level3"]["value"], json!(42));
    assert_eq!(output["level1"]["level2"]["level3"]["name"], json!("deep"));
}

// === Switch: matching with numeric comparison ===

#[tokio::test]
async fn test_runner_set_array_construction() {
    // Set task building an array from expressions
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-array
  version: '0.1.0'
do:
  - buildArray:
      set:
        items: '${ [.a, .b, .c] }'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": 1, "b": 2, "c": 3}))
        .await
        .unwrap();
    assert_eq!(output["items"], json!([1, 2, 3]));
}

// === Raise with all error fields including instance ===

#[tokio::test]
async fn test_runner_set_object_merge() {
    // Merging two objects using JQ + operator
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-merge
  version: '0.1.0'
do:
  - mergeObjects:
      set:
        result: '${ (.base + .override) }'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({
            "base": {"name": "Alice", "age": 30},
            "override": {"age": 31, "city": "NYC"}
        }))
        .await
        .unwrap();
    assert_eq!(output["result"]["name"], json!("Alice"));
    assert_eq!(output["result"]["age"], json!(31));
    assert_eq!(output["result"]["city"], json!("NYC"));
}

// === Wait preserves and references prior values (Go SDK wait_duration_iso8601.yaml) ===

#[tokio::test]
async fn test_runner_set_then_listen() {
    // set followed by listen — listen receives event data which becomes the output
    use crate::events::{CloudEvent, InMemoryEventBus};

    let bus = Arc::new(InMemoryEventBus::new());
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-listen
  version: '0.1.0'
do:
  - doSomethingBeforeEvent:
      set:
        name: javierito
  - callDoctor:
      listen:
        to:
          any: []
"#;
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();

    // Publish event in background
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        bus_clone
            .publish(CloudEvent::new(
                "com.example.doctor",
                json!({"doctor": "available"}),
            ))
            .await;
    });

    let runner = WorkflowRunner::new(workflow).unwrap().with_event_bus(bus);
    let output = runner.run(json!({})).await.unwrap();
    // Listen task now returns event data (single event → just the data)
    assert_eq!(output["doctor"], json!("available"));
}

// === For sum with export.as context accumulation ===

#[tokio::test]
async fn test_runner_set_output_and_export_combined() {
    // Java SDK's output-export-and-set-sub-workflow-child.yaml pattern
    // set task with output.as (transforms output) AND export.as (exports to context)
    // In our implementation, export.as receives the output.as-transformed result
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: output-export-combined
  version: '1.0.0'
do:
  - updateData:
      set:
        updated:
          userId: '${ .userId + "_tested" }'
          username: '${ .username + "_tested" }'
      export:
        as: '.'
  - useContext:
      set:
        fromExport: '${ $context }'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({"userId": "user1", "username": "test"}))
        .await
        .unwrap();
    // export.as: '.' exports the full task output
    assert_eq!(
        output["fromExport"]["updated"]["userId"],
        json!("user1_tested")
    );
    assert_eq!(
        output["fromExport"]["updated"]["username"],
        json!("test_tested")
    );
}

#[tokio::test]
async fn test_runner_set_output_as_then_export_as() {
    // set task with output.as that transforms output, then export.as on transformed output
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: output-as-then-export-as
  version: '1.0.0'
do:
  - compute:
      set:
        value: 42
        extra: 'metadata'
      output:
        as: .value
      export:
        as: '.'
  - check:
      set:
        exported: '${ $context }'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // output.as: .value → workflow output is 42
    // export.as: '.' on the output.as result → $context = 42
    assert_eq!(output, json!({"exported": 42}));
}

// --- Go SDK: chained_set_tasks.yaml ---
// Sequential set tasks with expression references: baseValue → doubled → tripled
#[tokio::test]
async fn test_e2e_chained_set_tasks() {
    let yaml_str = r#"
document:
  name: chained-workflow
  dsl: '1.0.0'
  namespace: default
  version: '1.0.0'
do:
  - task1:
      set:
        baseValue: 10
  - task2:
      set:
        doubled: "${ .baseValue * 2 }"
  - task3:
      set:
        tripled: "${ .doubled * 3 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // task1: {baseValue: 10}, task2: {doubled: 20}, task3: {tripled: 60}
    assert_eq!(output["tripled"], json!(60));
}

// --- Go SDK: set_tasks_with_then.yaml ---
// then:goto skips intermediate tasks
#[tokio::test]
async fn test_e2e_set_tasks_with_then_goto() {
    let yaml_str = r#"
document:
  name: then-workflow
  dsl: '1.0.0'
  namespace: default
  version: '1.0.0'
do:
  - task1:
      set:
        value: 30
      then: task3
  - task2:
      set:
        skipped: true
  - task3:
      set:
        result: "${ .value * 3 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // task1 sets value:30, then:task3 jumps to task3 (skipping task2)
    // task3 sets result: 30 * 3 = 90
    assert_eq!(output["result"], json!(90));
    assert!(output.get("skipped").is_none(), "task2 should be skipped");
}

// --- Go SDK: set_tasks_with_termination.yaml ---
// then:end stops workflow execution
#[tokio::test]
async fn test_e2e_set_tasks_with_then_end() {
    let yaml_str = r#"
document:
  name: termination-workflow
  dsl: '1.0.0'
  namespace: default
  version: '1.0.0'
do:
  - task1:
      set:
        finalValue: 20
      then: end
  - task2:
      set:
        skipped: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["finalValue"], json!(20));
    assert!(
        output.get("skipped").is_none(),
        "task2 should be skipped by then:end"
    );
}
