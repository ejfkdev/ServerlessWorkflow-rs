use super::*;

    #[tokio::test]
    async fn test_runner_wait_duration_iso8601() {
        let output = run_workflow_from_yaml(&testdata("wait_duration_iso8601.yaml"), json!({}))
            .await
            .unwrap();
        // Matches Go SDK's wait_duration_iso8601.yaml expected output
        assert_eq!(output["phase"], json!("completed"));
        assert_eq!(output["previousPhase"], json!("started"));
        assert_eq!(output["waitExpression"], json!("PT0.01S"));
    }

    // === Raise Error with Input Expression ===

    #[tokio::test]
    async fn test_runner_wait_set() {
        let output = run_workflow_from_yaml(&testdata("wait_set.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["name"], json!("Javierito"));
    }

    // === Try-Catch: match by status ===

    #[tokio::test]
    async fn test_runner_wait_various_durations() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: wait-various
  version: '0.1.0'
do:
  - wait1:
      wait: PT0S
  - setResult:
      set:
        done: true
"#;
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let start = std::time::Instant::now();
        let output = runner.run(json!({})).await.unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "Should be near-instant, got {}ms",
            elapsed.as_millis()
        );
        assert_eq!(output["done"], json!(true));
    }

    // === Try-catch with catch.do returning modified input ===

    #[tokio::test]
    async fn test_runner_wait_zero() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: wait-zero
  version: '0.1.0'
do:
  - noWait:
      wait: PT0S
  - setResult:
      set:
        done: true
"#;
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let start = std::time::Instant::now();
        let output = runner.run(json!({})).await.unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "Should be near-instant, got {}ms",
            elapsed.as_millis()
        );
        assert_eq!(output["done"], json!(true));
    }

    // === Expression: object key access with special characters ===

    #[tokio::test]
    async fn test_runner_wait_iso8601_variants() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: wait-iso-variants
  version: '0.1.0'
do:
  - wait1ms:
      wait: PT0.001S
  - setResult:
      set:
        waited: true
"#;
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let start = std::time::Instant::now();
        let output = runner.run(json!({})).await.unwrap();
        let elapsed = start.elapsed();
        assert_eq!(output["waited"], json!(true));
        assert!(elapsed.as_millis() < 500);
    }

    // === Expression: @json roundtrip ===

    #[tokio::test]
    async fn test_runner_wait_zero_immediate() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: wait-zero
  version: '0.1.0'
do:
  - noWait:
      wait: PT0S
  - setResult:
      set:
        done: true
"#;
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let start = std::time::Instant::now();
        let output = runner.run(json!({})).await.unwrap();
        let elapsed = start.elapsed();
        assert_eq!(output["done"], json!(true));
        assert!(elapsed.as_millis() < 100);
    }

    // === Raise: error with expression in detail ===

    #[tokio::test]
    async fn test_runner_wait_zero_immediate_v2() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: wait-zero-v2
  version: '0.1.0'
do:
  - noWait:
      wait: PT0S
  - done:
      set:
        finished: true
"#;
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let start = std::time::Instant::now();
        let output = runner.run(json!({})).await.unwrap();
        let elapsed = start.elapsed();
        assert_eq!(output["finished"], json!(true));
        assert!(elapsed.as_millis() < 100);
    }

    // === For: while with custom each/at names v2 ===

    // Wait: wait + set combination
    #[tokio::test]
    async fn test_runner_wait_then_set() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: wait-then-set
  version: '0.1.0'
do:
  - pause:
      wait: PT0.001S
  - afterWait:
      set:
        waited: true
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["waited"], json!(true));
    }

    #[tokio::test]
    async fn test_runner_wait_preserves_prior_values() {
        // Matches Go SDK's wait_duration_iso8601.yaml - set, wait, then set referencing previous values
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: wait-preserve
  version: '0.1.0'
do:
  - prepareWait:
      set:
        phase: started
        waitExpr: PT0.01S
  - waitBrief:
      wait: PT0.01S
  - completeWait:
      set:
        phase: completed
        previousPhase: started
        keptWaitExpr: PT0.01S
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["phase"], json!("completed"));
        assert_eq!(output["previousPhase"], json!("started"));
        assert_eq!(output["keptWaitExpr"], json!("PT0.01S"));
    }

    // === Switch with then: goto to multiple targets ===

    #[tokio::test]
    async fn test_runner_wait_then_preserves_prior_values() {
        // Go SDK's wait_duration_iso8601.yaml pattern
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: wait-then-preserve
  version: '0.1.0'
do:
  - prepareWaitExample:
      set:
        phase: started
        waitExpression: PT0.01S
  - waitBriefly:
      wait: PT0.01S
  - completeWaitExample:
      set:
        phase: completed
        previousPhase: "${ .phase }"
        waitExpression: "${ .waitExpression }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["phase"], json!("completed"));
        assert_eq!(output["previousPhase"], json!("started"));
        assert_eq!(output["waitExpression"], json!("PT0.01S"));
    }
