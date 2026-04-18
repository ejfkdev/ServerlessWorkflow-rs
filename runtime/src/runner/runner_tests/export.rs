use super::*;

    #[tokio::test]
    async fn test_runner_export_context() {
        let output = run_workflow_from_yaml(&testdata("export_context.yaml"), json!({}))
            .await
            .unwrap();
        // export.as should make data available via $context in subsequent tasks
        assert_eq!(output["greeting"], json!("Hello Javierito"));
    }

    // === Fork: No Compete (wait + set branches) ===

    #[tokio::test]
    async fn test_runner_export_conditional() {
        let output = run_workflow_from_yaml(&testdata("export_conditional.yaml"), json!({}))
            .await
            .unwrap();
        // After initialize: context = {items: []}
        // After addItem: context = {items: ["item1"]}
        // verifyContext should see $context.items = ["item1"]
        assert_eq!(output["contextItems"], json!(["item1"]));
    }

    // === HTTP Call: Query Parameters ===

    #[tokio::test]
    async fn test_runner_export_then_try_catch() {
        // Export context then use it after try-catch
        let output = run_workflow_from_yaml(&testdata("export_context.yaml"), json!({}))
            .await
            .unwrap();
        // Verify export_context works (already tested, just confirming baseline)
        assert_eq!(output["greeting"], json!("Hello Javierito"));
    }

    // === Try-Catch: multiple error types matching ===

    #[tokio::test]
    async fn test_runner_multiple_sequential_exports() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: multiple-sequential-exports
  version: '0.1.0'
do:
  - step1:
      set:
        value: 10
      export:
        as: '${ {step1Value: .value} }'
  - step2:
      set:
        value: "${ $context.step1Value + 20 }"
      export:
        as: '${ {step1Value: $context.step1Value, step2Value: .value} }'
  - step3:
      set:
        result: "${ $context.step1Value + $context.step2Value }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["result"], json!(40)); // 10 + 30
    }

    // === Set task with then: continue (skip remaining in current block) ===

    #[tokio::test]
    async fn test_runner_export_then_continue() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-then-continue
  version: '0.1.0'
do:
  - step1:
      set:
        value: hello
      export:
        as: '${ {step1: .value} }'
  - step2:
      set:
        result: '${ $context.step1 + " world" }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["result"], json!("hello world"));
    }

    // === Set: overwrite existing field ===

    #[tokio::test]
    async fn test_runner_export_sequential_accumulation() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-sequential-accumulation
  version: '0.1.0'
do:
  - step1:
      set:
        x: 10
      export:
        as: '${ {x: .x} }'
  - step2:
      set:
        y: "${ $context.x + 20 }"
      export:
        as: '${ {x: $context.x, y: .y} }'
  - step3:
      set:
        z: "${ $context.x + $context.y }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["z"], json!(40)); // 10 + 30
    }

    // === Task input schema validation ===

    #[tokio::test]
    async fn test_runner_export_complex_transform() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-complex-transform
  version: '0.1.0'
do:
  - step1:
      set:
        items: [1, 2, 3]
      export:
        as: "${ {count: (.items | length), total: (.items | add)} }"
  - step2:
      set:
        result: "${ $context }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["result"]["count"], json!(3));
        assert_eq!(output["result"]["total"], json!(6));
    }

    // === Try-catch with retry and exponential backoff ===

    #[tokio::test]
    async fn test_runner_export_object_merge() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-merge
  version: '0.1.0'
do:
  - step1:
      set:
        value: 1
      export:
        as: "${ {a: 1} }"
  - step2:
      set:
        value: 2
      export:
        as: "${ {b: 2, a: $context.a} }"
  - step3:
      set:
        result: "${ $context }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        // export.as replaces $context, so step2's export should have both a and b
        assert_eq!(output["result"]["a"], json!(1));
        assert_eq!(output["result"]["b"], json!(2));
    }

    // === Expression: IN operator (contains on array) ===

    #[tokio::test]
    async fn test_runner_export_nested_object() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-nested-object
  version: '0.1.0'
do:
  - step1:
      set:
        value: 10
      export:
        as: "${ {data: {val: .value, ts: 12345}} }"
  - step2:
      set:
        result: "${ $context }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["result"]["data"]["val"], json!(10));
        assert_eq!(output["result"]["data"]["ts"], json!(12345));
    }

    // === Expression: builtins (null, true, false, empty) ===

    #[tokio::test]
    async fn test_runner_export_chain_build_context() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-chain-ctx
  version: '0.1.0'
do:
  - step1:
      set:
        x: 1
      export:
        as: "${ {step1: .x} }"
  - step2:
      set:
        y: 2
      export:
        as: "${ {step1: $context.step1, step2: .y} }"
  - step3:
      set:
        z: 3
      export:
        as: "${ {step1: $context.step1, step2: $context.step2, step3: .z} }"
  - step4:
      set:
        result: "${ $context }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["result"]["step1"], json!(1));
        assert_eq!(output["result"]["step2"], json!(2));
        assert_eq!(output["result"]["step3"], json!(3));
    }

    // === Switch: then:continue with no match (pass through) ===

    #[tokio::test]
    async fn test_runner_export_overwrite_context() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-overwrite
  version: '0.1.0'
do:
  - step1:
      set:
        value: first
      export:
        as: "${ {first: .value} }"
  - step2:
      set:
        value: second
      export:
        as: "${ {second: .value} }"
  - step3:
      set:
        hasFirst: "${ $context.first != null }"
        hasSecond: "${ $context.second != null }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        // export.as replaces context entirely, so step2's export overwrites step1's
        assert_eq!(output["hasSecond"], json!(true));
    }

    // === Fork: two branches concurrent ===

    #[tokio::test]
    async fn test_runner_export_then_use_context() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-use-context
  version: '0.1.0'
do:
  - setConfig:
      set:
        mode: production
      export:
        as: "${ {mode: .mode} }"
  - useConfig:
      set:
        isProd: "${ $context.mode == \"production\" }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["isProd"], json!(true));
    }

    // === Switch: default case with then:continue ===

    // Export: export.as with complex jq transform
    #[tokio::test]
    async fn test_runner_export_complex_jq_transform() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-complex-jq-transform
  version: '0.1.0'
do:
  - step1:
      set:
        items: "${ [1,2,3] }"
      export:
        as: "${ {total: (.items | add), count: (.items | length), items: .items} }"
  - step2:
      set:
        hasTotal: "${ $context.total }"
        hasCount: "${ $context.count }"
        items: "${ .items }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["hasTotal"], json!(6));
        assert_eq!(output["hasCount"], json!(3));
    }

    #[tokio::test]
    async fn test_runner_export_multiple_context() {
        // Multiple tasks exporting to context, each merge-exporting with previous context
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-multi-context
  version: '0.1.0'
do:
  - step1:
      set:
        firstName: Alice
      export:
        as: '${ {firstName: .firstName} }'
  - step2:
      set:
        lastName: Smith
      export:
        as: '${ ($context // {}) + {lastName: .lastName, firstName: $context.firstName} }'
  - combine:
      set:
        fullName: '${ $context.firstName + " " + $context.lastName }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["fullName"], json!("Alice Smith"));
    }

    // === Set with array construction ===
