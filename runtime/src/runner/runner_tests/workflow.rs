use super::*;

    #[tokio::test]
    async fn test_runner_workflow_input_schema_valid() {
        let output = run_workflow_from_yaml(
            &testdata("workflow_input_schema.yaml"),
            json!({"key": "testValue"}),
        )
        .await
        .unwrap();
        assert_eq!(output["outputKey"], json!("testValue"));
    }

    #[tokio::test]
    async fn test_runner_workflow_input_schema_invalid() {
        let result = run_workflow_from_yaml(
            &testdata("workflow_input_schema.yaml"),
            json!({"wrongKey": "testValue"}),
        )
        .await;
        assert!(result.is_err());
    }

    // === Switch Then Loop ===

    #[tokio::test]
    async fn test_runner_workflow_output_schema_valid() {
        let output = run_workflow_from_yaml(&testdata("workflow_output_schema.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["result"], json!("success"));
    }

    // === Workflow Output Schema: invalid ===

    #[tokio::test]
    async fn test_runner_workflow_output_schema_invalid() {
        let result =
            run_workflow_from_yaml(&testdata("workflow_output_schema_invalid.yaml"), json!({}))
                .await;
        assert!(
            result.is_err(),
            "Expected error due to output schema validation failure"
        );
    }

    // === HTTP Call: with custom headers ===

    #[tokio::test]
    async fn test_runner_workflow_output_as() {
        let output = run_workflow_from_yaml(&testdata("workflow_output_as.yaml"), json!({}))
            .await
            .unwrap();
        // output.as: .result should extract just the result value
        assert_eq!(output, json!("hello"));
    }

    // === Nested try-catch ===

    #[tokio::test]
    async fn test_runner_sub_workflow_output_export_and_set() {
        // Child workflow: set task with output.as + export.as
        let child_yaml = r#"
document:
  dsl: '1.0.0'
  namespace: default
  name: set-into-context
  version: '1.0.0'
do:
  - updateUser:
      set:
        updated:
          userId: '${ .userId + "_tested" }'
          username: '${ .username + "_tested" }'
      output:
        as: .updated
      export:
        as: '.'
"#;
        let child: WorkflowDefinition = serde_yaml::from_str(child_yaml).unwrap();

        // Parent workflow: calls child with input, then reads exported context
        let parent_yaml = r#"
document:
  dsl: '1.0.0'
  namespace: default
  name: parent
  version: '1.0.0'
do:
  - sayHello:
      run:
        workflow:
          namespace: default
          name: set-into-context
          version: '1.0.0'
          input:
            userId: '123'
            username: 'alice'
"#;
        let parent: WorkflowDefinition = serde_yaml::from_str(parent_yaml).unwrap();

        let runner = WorkflowRunner::new(parent)
            .unwrap()
            .with_sub_workflow(child);

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["userId"], json!("123_tested"));
        assert_eq!(output["username"], json!("alice_tested"));
    }

    /// Java SDK's read-context-and-set-sub-workflow pattern
    /// Child reads $workflow.definition.document.name/version in its expression
    #[tokio::test]
    async fn test_runner_sub_workflow_read_context_and_set() {
        let child_yaml = r#"
document:
  dsl: '1.0.0'
  namespace: default
  name: set-into-context
  version: '1.0.0'
do:
  - updateUser:
      set:
        updated:
          userId: '${ .userId + "_tested" }'
          username: '${ .username + "_tested" }'
          password: '${ .password + "_tested" }'
        detail: '${ "The workflow " + $workflow.definition.document.name + ":" + $workflow.definition.document.version + " updated user in context" }'
      export:
        as: '.'
"#;
        let child: WorkflowDefinition = serde_yaml::from_str(child_yaml).unwrap();

        let parent_yaml = r#"
document:
  dsl: '1.0.0'
  namespace: default
  name: parent
  version: '1.0.0'
do:
  - sayHello:
      run:
        workflow:
          namespace: default
          name: set-into-context
          version: '1.0.0'
          input:
            userId: '123'
            username: 'alice'
            password: 'secret'
"#;
        let parent: WorkflowDefinition = serde_yaml::from_str(parent_yaml).unwrap();

        let runner = WorkflowRunner::new(parent)
            .unwrap()
            .with_sub_workflow(child);

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["updated"]["userId"], json!("123_tested"));
        assert_eq!(output["updated"]["username"], json!("alice_tested"));
        assert_eq!(output["updated"]["password"], json!("secret_tested"));
        // Verify $workflow context is correct inside the sub-workflow
        let detail = output["detail"].as_str();
        assert!(
            detail.is_some(),
            "detail field missing, output: {:?}",
            output
        );
        assert!(detail.unwrap().contains("set-into-context"));
        assert!(detail.unwrap().contains("1.0.0"));
    }

    // === HTTP Call: OIDC client_credentials ===

    #[tokio::test]
    async fn test_runner_workflow_input_transform() {
        let output = run_workflow_from_yaml(
            &testdata("conditional_logic_input_from.yaml"),
            json!({"localWeather": {"temperature": 30}}),
        )
        .await
        .unwrap();
        // input.from should transform the input
        assert_eq!(output["weather"], json!("hot"));
    }

    // === Fork: compete mode returns first completed branch ===

    #[tokio::test]
    async fn test_runner_workflow_output_complex_transform() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-output-complex
  version: '0.1.0'
do:
  - buildResult:
      set:
        items:
          - name: Alice
            score: 95
          - name: Bob
            score: 85
output:
  as: "${ {topScorer: .items[0].name, count: (.items | length)} }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["topScorer"], json!("Alice"));
        assert_eq!(output["count"], json!(2));
    }

    // === Workflow input: complex transformation ===

    #[tokio::test]
    async fn test_runner_workflow_input_complex_transform() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-input-complex
  version: '0.1.0'
input:
  from: "${ {name: .rawName, age: .rawAge} }"
do:
  - useInput:
      set:
        result: "${ .name + \" is \" + (.age | tostring) }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner
            .run(json!({"rawName": "Alice", "rawAge": 30}))
            .await
            .unwrap();
        assert_eq!(output["result"], json!("Alice is 30"));
    }

    // === Workflow timeout: workflow exceeds timeout ===

    #[tokio::test]
    async fn test_runner_workflow_timeout_exceeded() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-timeout
  version: '0.1.0'
timeout:
  after: PT0.01S
do:
  - slowTask:
      wait: PT5S
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "timeout");
    }

    // === Workflow timeout: completes within timeout ===

    #[tokio::test]
    async fn test_runner_workflow_timeout_not_exceeded() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-timeout-ok
  version: '0.1.0'
timeout:
  after: PT0.01S
do:
  - quickTask:
      set:
        result: done
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["result"], json!("done"));
    }

    // === Workflow timeout: reference to reusable timeout ===

    #[tokio::test]
    async fn test_runner_workflow_timeout_reference() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-timeout-ref
  version: '0.1.0'
use:
  timeouts:
    shortTimeout:
      after: PT0.01S
timeout: shortTimeout
do:
  - slowTask:
      wait: PT5S
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "timeout");
    }

    // === Workflow timeout with try-catch ===

    #[tokio::test]
    async fn test_runner_workflow_timeout_try_catch() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-timeout-try-catch
  version: '0.1.0'
timeout:
  after: PT0.01S
do:
  - safeBlock:
      try:
        - slowTask:
            wait: PT5S
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

        // Workflow-level timeout is applied outside try-catch, so it should still timeout
        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().error_type_short(), "timeout");
    }

    // === Workflow timeout: dynamic expression ===

    #[tokio::test]
    async fn test_runner_workflow_input_from_output_as() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-input-from-output-as
  version: '0.1.0'
input:
  from: "${ {name: .rawName, age: .rawAge} }"
output:
  as: "${ {greeting: (\"Hello \" + .name), yearsOld: .age} }"
do:
  - useInput:
      set:
        name: "${ .name }"
        age: "${ .age }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner
            .run(json!({"rawName": "Alice", "rawAge": 30}))
            .await
            .unwrap();
        assert_eq!(output["greeting"], json!("Hello Alice"));
        assert_eq!(output["yearsOld"], json!(30));
    }

    // === Try-catch: retry with reference to reusable retry policy ===

    #[tokio::test]
    async fn test_runner_workflow_input_output_schema_combined() {
        // Use testdata file for valid input, verify it works with both input+output schema
        let output = run_workflow_from_yaml(
            &testdata("workflow_input_schema.yaml"),
            json!({"key": "test"}),
        )
        .await
        .unwrap();
        assert_eq!(output["outputKey"], json!("test"));
    }

    #[tokio::test]
    async fn test_runner_workflow_input_output_schema_invalid_input() {
        // Verify existing testdata file rejects invalid input
        let result = run_workflow_from_yaml(
            &testdata("workflow_input_schema.yaml"),
            json!({"wrongKey": "testValue"}),
        )
        .await;
        assert!(
            result.is_err(),
            "Should fail with missing required field 'key'"
        );
    }

    // === Nested do with then:exit continues at outer scope ===

    // Workflow: input.from + output.as combined
    #[tokio::test]
    async fn test_runner_workflow_input_output_combo() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-input-output-combo
  version: '0.1.0'
input:
  from: "${ {x: .a, y: .b} }"
output:
  as: "${ {result: .sum} }"
do:
  - compute:
      set:
        sum: "${ .x + .y }"
        x: "${ .x }"
        y: "${ .y }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"a": 3, "b": 7})).await.unwrap();
        assert_eq!(output["result"], json!(10));
    }

    #[tokio::test]
    async fn test_runner_workflow_output_as_sequential_colors() {
        // Go SDK's sequential_set_colors_output_as.yaml pattern
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: sequential-colors-output-as
  version: '0.1.0'
do:
  - setRed:
      set:
        colors: ${ .colors + ["red"] }
  - setGreen:
      set:
        colors: ${ .colors + ["green"] }
  - setBlue:
      set:
        colors: ${ .colors + ["blue"] }
output:
  as: "${ { result: .colors } }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"colors": []})).await.unwrap();
        // workflow-level output.as transforms the final result
        assert_eq!(output["result"], json!(["red", "green", "blue"]));
    }

    #[tokio::test]
    async fn test_runner_sub_workflow_basic() {
        // Parent workflow invokes child workflow via run: workflow
        let parent_yaml = std::fs::read_to_string(testdata("sub_workflow_parent.yaml")).unwrap();
        let child_yaml = std::fs::read_to_string(testdata("sub_workflow_child.yaml")).unwrap();

        let parent: WorkflowDefinition = serde_yaml::from_str(&parent_yaml).unwrap();
        let child: WorkflowDefinition = serde_yaml::from_str(&child_yaml).unwrap();

        let runner = WorkflowRunner::new(parent)
            .unwrap()
            .with_sub_workflow(child);

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["counter"], json!(1));
        assert_eq!(output["greeting"], json!("helloWorld"));
    }

    #[tokio::test]
    async fn test_runner_sub_workflow_not_found() {
        // Parent references a sub-workflow that is not registered
        let parent_yaml = std::fs::read_to_string(testdata("sub_workflow_parent.yaml")).unwrap();
        let parent: WorkflowDefinition = serde_yaml::from_str(&parent_yaml).unwrap();

        let runner = WorkflowRunner::new(parent).unwrap();
        // No child workflow registered — should error
        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found in registry"),
            "Expected 'not found in registry', got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_runner_sub_workflow_with_export() {
        // Sub-workflow uses output.as and export.as — output should be transformed
        let parent_yaml =
            std::fs::read_to_string(testdata("sub_workflow_export_parent.yaml")).unwrap();
        let child_yaml =
            std::fs::read_to_string(testdata("sub_workflow_export_child.yaml")).unwrap();

        let parent: WorkflowDefinition = serde_yaml::from_str(&parent_yaml).unwrap();
        let child: WorkflowDefinition = serde_yaml::from_str(&child_yaml).unwrap();

        let runner = WorkflowRunner::new(parent)
            .unwrap()
            .with_sub_workflow(child);

        let input = json!({
            "userId": "userId_1",
            "username": "test",
            "password": "test"
        });
        let output = runner.run(input).await.unwrap();
        assert_eq!(output["userId"], json!("userId_1_tested"));
        assert_eq!(output["username"], json!("test_tested"));
        assert_eq!(output["password"], json!("test_tested"));
    }

    #[tokio::test]
    async fn test_runner_sub_workflow_inline() {
        // Test sub-workflow defined inline (no testdata files)
        let child_yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: child
  version: '1.0.0'
do:
  - doubleIt:
      set:
        result: '${ .value * 2 }'
"#;
        let parent_yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: parent-inline
  version: '1.0.0'
do:
  - callChild:
      run:
        workflow:
          namespace: test
          name: child
          version: '1.0.0'
"#;

        let parent: WorkflowDefinition = serde_yaml::from_str(parent_yaml).unwrap();
        let child: WorkflowDefinition = serde_yaml::from_str(child_yaml).unwrap();

        let runner = WorkflowRunner::new(parent)
            .unwrap()
            .with_sub_workflow(child);

        let output = runner.run(json!({"value": 21})).await.unwrap();
        assert_eq!(output["result"], json!(42));
    }

    #[tokio::test]
    async fn test_runner_sub_workflow_read_context_from_fixtures() {
        // Test $workflow.definition variable access in sub-workflow using YAML fixtures
        let parent_yaml =
            std::fs::read_to_string(testdata("sub_workflow_read_context_parent.yaml")).unwrap();
        let child_yaml =
            std::fs::read_to_string(testdata("sub_workflow_read_context_child.yaml")).unwrap();

        let parent: WorkflowDefinition = serde_yaml::from_str(&parent_yaml).unwrap();
        let child: WorkflowDefinition = serde_yaml::from_str(&child_yaml).unwrap();

        let runner = WorkflowRunner::new(parent)
            .unwrap()
            .with_sub_workflow(child);

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["updated"]["userId"], json!("123_tested"));
        assert_eq!(output["updated"]["username"], json!("alice_tested"));
        assert_eq!(output["updated"]["password"], json!("secret_tested"));
        let detail = output["detail"].as_str();
        assert!(detail.is_some(), "detail field missing, output: {:?}", output);
        assert!(detail.unwrap().contains("set-into-context"));
        assert!(detail.unwrap().contains("1.0.0"));
    }

    // ---- CallHandler and RunHandler Tests ----
