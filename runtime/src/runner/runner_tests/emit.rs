use super::*;

    #[tokio::test]
    async fn test_runner_emit_event() {
        let output = run_workflow_from_yaml(&testdata("emit_event.yaml"), json!({}))
            .await
            .unwrap();
        // Emit returns input unchanged
        assert_eq!(output, json!({}));
    }

    // === Lifecycle CloudEvent publishing ===

    #[tokio::test]
    async fn test_runner_emit_out() {
        let output = run_workflow_from_yaml(&testdata("emit_out.yaml"), json!({}))
            .await
            .unwrap();
        // Emit without data returns input unchanged
        assert_eq!(output, json!({}));
    }

    // === Emit: With input expression in data ===

    #[tokio::test]
    async fn test_runner_emit_doctor() {
        let output =
            run_workflow_from_yaml(&testdata("emit_doctor.yaml"), json!({"temperature": 38.5}))
                .await
                .unwrap();
        // Emit with expression in data
        assert_eq!(output, json!({"temperature": 38.5}));
    }

    // === Emit: With data containing expressions ===

    #[tokio::test]
    async fn test_runner_emit_data() {
        let output = run_workflow_from_yaml(
            &testdata("emit_data.yaml"),
            json!({"firstName": "John", "lastName": "Doe"}),
        )
        .await
        .unwrap();
        // Emit returns input unchanged (event is emitted as side effect)
        assert_eq!(output["firstName"], json!("John"));
        assert_eq!(output["lastName"], json!("Doe"));
    }

    // === Try-Catch: Retry inline policy ===

    #[tokio::test]
    async fn test_runner_emit_structured_event() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: emit-structured
  version: '0.1.0'
do:
  - emitOrder:
      emit:
        event:
          with:
            source: https://petstore.com
            type: com.petstore.order.placed.v1
            data:
              client:
                firstName: Cruella
                lastName: de Vil
              items:
                - breed: dalmatian
                  quantity: 101
  - setResult:
      set:
        emitted: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        // Emit returns input unchanged, then set adds emitted field
        assert_eq!(output["emitted"], json!(true));
    }

    // === Switch: then goto continues at specific task ===

    #[tokio::test]
    async fn test_runner_emit_with_source_and_type() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: emit-source-type
  version: '0.1.0'
do:
  - emitOrder:
      emit:
        event:
          with:
            source: '${ "https://" + .domain + "/orders" }'
            type: com.petstore.order.placed.v1
            data:
              orderId: "${ .orderId }"
  - setResult:
      set:
        emitted: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner
            .run(json!({"domain": "petstore.com", "orderId": "12345"}))
            .await
            .unwrap();
        assert_eq!(output["emitted"], json!(true));
    }

    // === Wait with ISO8601 duration in different formats ===

    #[tokio::test]
    async fn test_runner_emit_multiple_events() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: emit-multiple
  version: '0.1.0'
do:
  - emitFirst:
      emit:
        event:
          with:
            source: https://test.com
            type: com.test.first.v1
            data:
              id: 1
  - emitSecond:
      emit:
        event:
          with:
            source: https://test.com
            type: com.test.second.v1
            data:
              id: 2
  - setResult:
      set:
        done: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["done"], json!(true));
    }

    // === Do: then:end ends only current do level ===

    #[tokio::test]
    async fn test_runner_emit_with_data_expr_v2() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: emit-data-expr-v2
  version: '0.1.0'
do:
  - emitResult:
      emit:
        event:
          type: resultEvent
          data: "${ . }"
  - done:
      set:
        emitted: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"value": 42})).await.unwrap();
        assert_eq!(output["emitted"], json!(true));
    }

    // === Try-catch: retry exhausted propagates error ===

    #[tokio::test]
    async fn test_runner_emit_doctor_data() {
        // Matches Java SDK's emit-doctor.yaml - emit with data expression
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: emit-doctor
  version: '0.1.0'
do:
  - emitEvent:
      emit:
        event:
          with:
            source: https://hospital.com
            type: com.fake-hospital.vitals.measurements.temperature
            data:
              temperature: '${.temperature}'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"temperature": 38.5})).await.unwrap();
        // Emit task returns the input unchanged
        assert_eq!(output["temperature"], json!(38.5));
    }

    #[tokio::test]
    async fn test_runner_emit_with_data_and_type() {
        // Emit event with specific type and data
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: emit-data-type
  version: '0.1.0'
do:
  - emitEvent:
      emit:
        event:
          with:
            type: com.example.greeting
            data: '${ .message }'
  - afterEmit:
      set:
        done: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"message": "hello"})).await.unwrap();
        // Emit doesn't change output, next task runs
        assert_eq!(output["done"], json!(true));
    }

    // === Secret missing error test ===
