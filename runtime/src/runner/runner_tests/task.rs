use super::*;

    #[tokio::test]
    async fn test_runner_task_timeout() {
        let result = run_workflow_from_yaml(&testdata("task_timeout.yaml"), json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "timeout");
    }

    // === Task Timeout with Try-Catch ===

    #[tokio::test]
    async fn test_runner_task_timeout_try_catch() {
        let output = run_workflow_from_yaml(&testdata("task_timeout_try_catch.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["timedOut"], json!(true));
    }

    // === Shell: await false (fire-and-forget) ===

    #[tokio::test]
    async fn test_runner_task_input_from() {
        let output = run_workflow_from_yaml(
            &testdata("task_input_from.yaml"),
            json!({"data": {"value": 42}}),
        )
        .await
        .unwrap();
        assert_eq!(output["result"], json!(42));
    }

    // === Expression: nested object construction ===

    #[tokio::test]
    async fn test_runner_task_timeout_reference() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: task-timeout-reference
  version: '0.1.0'
use:
  timeouts:
    shortTimeout:
      after: PT0.01S
do:
  - slowTask:
      wait: PT5S
      timeout: shortTimeout
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "timeout");
    }

    // === Task timeout: reference with try-catch ===

    #[tokio::test]
    async fn test_runner_task_timeout_reference_try_catch() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: task-timeout-ref-try-catch
  version: '0.1.0'
use:
  timeouts:
    shortTimeout:
      after: PT0.01S
do:
  - safeBlock:
      try:
        - slowTask:
            wait: PT5S
            timeout: shortTimeout
      catch:
        errors:
          with:
            type: timeout
        do:
          - handleTimeout:
              set:
                timedOut: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["timedOut"], json!(true));
    }

    // === Task with both input.from and output.as ===

    #[tokio::test]
    async fn test_runner_task_input_from_and_output_as() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: task-input-from-output-as
  version: '0.1.0'
do:
  - transformTask:
      set:
        result: "${ .value * 2 }"
      input:
        from: "${ {value: .rawNumber} }"
      output:
        as: .result
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"rawNumber": 21})).await.unwrap();
        assert_eq!(output, json!(42));
    }

    // === Do within For loop (composite nesting) ===

    #[tokio::test]
    async fn test_runner_task_input_schema_valid() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: task-input-schema-valid
  version: '0.1.0'
do:
  - validateInput:
      input:
        schema:
          format: json
          document:
            type: object
            properties:
              count:
                type: number
            required:
              - count
      set:
        doubled: "${ .count * 2 }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"count": 5})).await.unwrap();
        assert_eq!(output["doubled"], json!(10));
    }

    #[tokio::test]
    async fn test_runner_task_input_schema_invalid() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: task-input-schema-invalid
  version: '0.1.0'
do:
  - validateInput:
      input:
        schema:
          format: json
          document:
            type: object
            properties:
              count:
                type: number
            required:
              - count
      set:
        doubled: "${ .count * 2 }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // Missing required field 'count'
        let result = runner.run(json!({"name": "test"})).await;
        assert!(result.is_err());
    }

    // === Task output schema validation ===

    #[tokio::test]
    async fn test_runner_task_output_schema_valid() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: task-output-schema-valid
  version: '0.1.0'
do:
  - setOutput:
      set:
        result: "success"
      output:
        schema:
          format: json
          document:
            type: object
            properties:
              result:
                type: string
            required:
              - result
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["result"], json!("success"));
    }

    #[tokio::test]
    async fn test_runner_task_output_schema_invalid() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: task-output-schema-invalid
  version: '0.1.0'
do:
  - setOutput:
      set:
        count: 42
      output:
        schema:
          format: json
          document:
            type: object
            properties:
              count:
                type: string
            required:
              - count
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // count is number but schema expects string
        let result = runner.run(json!({})).await;
        assert!(result.is_err());
    }

    // === Task export schema validation ===

    #[tokio::test]
    async fn test_runner_task_export_schema_valid() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: task-export-schema-valid
  version: '0.1.0'
do:
  - exportData:
      set:
        key: "${ .inputKey }"
      export:
        as: '${ {exportedKey: .key} }'
        schema:
          format: json
          document:
            type: object
            properties:
              exportedKey:
                type: string
            required:
              - exportedKey
  - useExported:
      set:
        result: "${ $context.exportedKey }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"inputKey": "hello"})).await.unwrap();
        assert_eq!(output["result"], json!("hello"));
    }

    #[tokio::test]
    async fn test_runner_task_export_schema_invalid() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: task-export-schema-invalid
  version: '0.1.0'
do:
  - exportData:
      set:
        key: 123
      export:
        as: '${ {exportedKey: .key} }'
        schema:
          format: json
          document:
            type: object
            properties:
              exportedKey:
                type: string
            required:
              - exportedKey
  - useExported:
      set:
        result: "${ $context.exportedKey }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // exportedKey is 123 (number) but schema requires string
        let result = runner.run(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_runner_task_output_schema_dynamic_valid() {
        let output = run_workflow_from_yaml(
            &testdata("task_output_schema_with_dynamic_value.yaml"),
            json!({"taskInputKey": "validValue"}),
        )
        .await
        .unwrap();
        assert_eq!(output["finalOutputKey"], json!("validValue"));
    }

    #[tokio::test]
    async fn test_runner_task_output_schema_dynamic_invalid() {
        // taskInputKey is a number but output schema requires string
        let result = run_workflow_from_yaml(
            &testdata("task_output_schema_with_dynamic_value.yaml"),
            json!({"taskInputKey": 123}),
        )
        .await;
        assert!(result.is_err());
    }

    // === Expression: conditional in set value ===
