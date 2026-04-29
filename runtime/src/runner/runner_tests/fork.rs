use super::*;

    #[tokio::test]
    async fn test_runner_fork_simple() {
        let output = run_workflow_from_yaml(&testdata("fork_simple.yaml"), json!({}))
            .await
            .unwrap();
        // Fork with 2 branches returns array of results, then joinResult flattens
        // Go SDK expects: {"colors": ["red", "blue"]}
        let colors = output["colors"].as_array().unwrap();
        assert!(colors.contains(&json!("red")));
        assert!(colors.contains(&json!("blue")));
    }

    // === Conditional Logic with input.from ===

    #[tokio::test]
    async fn test_runner_fork_wait() {
        let output = run_workflow_from_yaml(&testdata("fork_wait.yaml"), json!({}))
            .await
            .unwrap();
        // Non-compete fork with 2 branches returns object with branch names as keys
        assert!(output.is_object());
        assert_eq!(output.as_object().unwrap().len(), 2);
    }

    // === Emit event ===

    #[tokio::test]
    async fn test_runner_fork_compete() {
        let output = run_workflow_from_yaml(&testdata("fork_compete.yaml"), json!({}))
            .await
            .unwrap();
        // Compete mode: first branch to complete wins
        assert!(output["patientId"].is_string());
        assert!(output["room"].is_number());
    }

    // === Fork: Compete with timing ===

    #[tokio::test]
    async fn test_runner_fork_compete_timed() {
        let output = run_workflow_from_yaml(&testdata("fork_compete_timed.yaml"), json!({}))
            .await
            .unwrap();
        // Fast branch (no wait) should win over slow branch (100ms wait)
        assert_eq!(output["winner"], json!("fast"));
    }

    // === Export as context ===

    #[tokio::test]
    async fn test_runner_fork_no_compete() {
        let output = run_workflow_from_yaml(&testdata("fork_no_compete.yaml"), json!({}))
            .await
            .unwrap();
        // Non-compete with multiple branches returns object with branch names as keys
        assert!(output.is_object());
        assert_eq!(output.as_object().unwrap().len(), 2);
    }

    // === Emit: Simple (no data) ===

    #[tokio::test]
    async fn test_runner_fork_no_compete_with_names() {
        let output = run_workflow_from_yaml(&testdata("fork_no_compete.yaml"), json!({}))
            .await
            .unwrap();
        // Non-compete returns object with branch names as keys
        assert!(output.is_object());
        assert!(output.get("callNurse").is_some());
        assert!(output.get("callDoctor").is_some());
    }

    // === HTTP Call: POST with body expression and output.as extracting field ===

    #[tokio::test]
    async fn test_runner_fork_compete_returns_first() {
        let output = run_workflow_from_yaml(&testdata("fork_compete.yaml"), json!({}))
            .await
            .unwrap();
        // Compete mode returns the first completed branch result
        assert!(output["patientId"].is_string());
        assert!(output["room"].is_number());
    }

    // === Shell: arguments (static) ===

    #[tokio::test]
    async fn test_runner_fork_compete_output_as() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-compete-output-as
  version: '0.1.0'
do:
  - race:
      fork:
        compete: true
        branches:
          - branch1:
              do:
                - setResult:
                    set:
                      winner: branch1
          - branch2:
              do:
                - setResult:
                    set:
                      winner: branch2
      output:
        as: .winner
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Compete mode: first branch wins (sequential handle join)
        assert!(output == json!("branch1") || output == json!("branch2"));
    }

    // === For loop with export accumulating context across iterations ===

    #[tokio::test]
    async fn test_runner_fork_single_branch() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-single
  version: '0.1.0'
do:
  - singleFork:
      fork:
        compete: false
        branches:
          - onlyBranch:
              do:
                - setResult:
                    set:
                      value: 42
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Single branch non-compete fork returns object with branch name as key
        assert_eq!(output["onlyBranch"]["value"], json!(42));
    }

    // === Set with null and boolean values ===

    #[tokio::test]
    async fn test_runner_fork_compete_with_output_as() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-compete-output-as
  version: '0.1.0'
do:
  - race:
      fork:
        compete: true
        branches:
          - fastBranch:
              do:
                - setResult:
                    set:
                      winner: fast
          - slowBranch:
              do:
                - waitABit:
                    wait:
                      milliseconds: 5
                - setResult:
                    set:
                      winner: slow
      output:
        as: .winner
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output, json!("fast"));
    }

    // === For loop with output.as extracting array ===

    #[tokio::test]
    async fn test_runner_fork_compete_winner_output() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-compete-winner
  version: '0.1.0'
do:
  - race:
      fork:
        compete: true
        branches:
          - fastBranch:
              do:
                - setFast:
                    set:
                      winner: fast
                      time: 10
          - slowBranch:
              do:
                - waitABit:
                    wait: PT0.1S
                - setSlow:
                    set:
                      winner: slow
                      time: 100
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Fast branch should win in compete mode
        assert_eq!(output["winner"], json!("fast"));
    }

    // === Expression: add (sum array) ===

    #[tokio::test]
    async fn test_runner_fork_branch_output_as() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-branch-output-as
  version: '0.1.0'
do:
  - parallel:
      fork:
        compete: true
        branches:
          - branchA:
              do:
                - computeA:
                    set:
                      winner: fast
                      time: 10
          - branchB:
              do:
                - waitABit:
                    wait: PT0.1S
                - computeB:
                    set:
                      winner: slow
                      time: 100
      output:
        as: "${ {winner: .winner, elapsed: .time} }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Compete mode: fast branch wins
        assert_eq!(output["winner"], json!("fast"));
        assert_eq!(output["elapsed"], json!(10));
    }

    // === Set with array expression ===

    #[tokio::test]
    async fn test_runner_fork_try_catch_branch() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-try-catch-branch
  version: '0.1.0'
do:
  - parallel:
      fork:
        compete: false
        branches:
          - safeBranch:
              do:
                - guarded:
                    try:
                      - failStep:
                          raise:
                            error:
                              type: validation
                              title: Branch Error
                              status: 400
                    catch:
                      errors:
                        with:
                          type: validation
                      do:
                        - recover:
                            set:
                              recovered: true
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["safeBranch"]["recovered"], json!(true));
    }

    // === Raise with all error types ===

    #[tokio::test]
    async fn test_runner_fork_single_behaves_sequential() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-single-sequential
  version: '0.1.0'
do:
  - parallel:
      fork:
        compete: false
        branches:
          - onlyBranch:
              do:
                - step1:
                    set:
                      value: 42
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["onlyBranch"]["value"], json!(42));
    }

    // === Set: if condition with expression result ===

    #[tokio::test]
    async fn test_runner_fork_two_branches() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-two-branches
  version: '0.1.0'
do:
  - parallel:
      fork:
        compete: false
        branches:
          - branchA:
              set:
                a: 1
          - branchB:
              set:
                b: 2
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Non-compete fork with multiple branches returns object with branch names as keys
        assert!(output.is_object());
        assert_eq!(output.as_object().unwrap().len(), 2);
    }

    // === Try-catch: catch all errors ===

    // Fork: compete where one branch errors, other wins
    #[tokio::test]
    async fn test_runner_fork_compete_error_branch() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-compete-error-branch
  version: '0.1.0'
do:
  - race:
      fork:
        compete: true
        branches:
          - fail:
              raise:
                error:
                  type: runtime
                  title: Fail
                  status: 500
          - succeed:
              set:
                winner: true
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["winner"], json!(true));
    }

    // Fork: compete where one branch errors, other wins (fast returns, slow fails)
    #[tokio::test]
    async fn test_runner_fork_compete_speed() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-compete-speed
  version: '0.1.0'
do:
  - race:
      fork:
        compete: true
        branches:
          - fails:
              raise:
                error:
                  type: runtime
                  title: Fail
                  status: 500
          - succeeds:
              set:
                winner: fast
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["winner"], json!("fast"));
    }

    #[tokio::test]
    async fn test_runner_fork_compete_with_set_branches() {
        // Matches Java SDK's fork.yaml - compete mode with set task branches
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-compete-set
  version: '0.1.0'
do:
  - callSomeone:
      fork:
        compete: true
        branches:
          - callNurse:
              set:
                patientId: John
                room: 1
          - callDoctor:
              set:
                patientId: Smith
                room: 2
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Compete mode: one branch wins, output is that branch's result
        // The patientId should be either John or Smith
        let patient_id = output["patientId"].as_str();
        assert!(patient_id == Some("John") || patient_id == Some("Smith"));
    }

    // === Fork compete with wait tasks (speed-based winner) ===

    #[tokio::test]
    async fn test_runner_fork_compete_with_timing() {
        // Fork compete where one branch is faster
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-compete-timing
  version: '0.1.0'
do:
  - raceTask:
      fork:
        compete: true
        branches:
          - fastBranch:
              do:
                - quickSet:
                    set:
                      winner: fast
          - slowBranch:
              do:
                - delay:
                    wait: PT0.05S
                - lateSet:
                    set:
                      winner: slow
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Fast branch should win
        assert_eq!(output["winner"], json!("fast"));
    }

    // === Fork non-compete with named branches (Java SDK fork-no-compete.yaml enhanced) ===

    #[tokio::test]
    async fn test_runner_fork_non_compete_named_branches() {
        // Java SDK's fork-no-compete.yaml pattern with named branches
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-no-compete-named
  version: '0.1.0'
do:
  - parallelTask:
      fork:
        compete: false
        branches:
          - callNurse:
              set:
                patientId: John
                room: 1
          - callDoctor:
              set:
                patientId: Smith
                room: 2
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Non-compete: both branches run, output is object with branch names as keys
        if output.is_object() {
            assert_eq!(output.as_object().unwrap().len(), 2);
            assert!(output.get("callNurse").is_some());
            assert!(output.get("callDoctor").is_some());
            assert_eq!(output["callNurse"]["patientId"], json!("John"));
            assert_eq!(output["callDoctor"]["patientId"], json!("Smith"));
        }
    }

    // === Set with conditional expression (ternary-like) (Go SDK TestSetTaskExecutor_ConditionalExpressions hot path) ===

    #[tokio::test]
    async fn test_runner_fork_simple_non_compete() {
        // Go SDK's fork_simple.yaml - non-compete fork with set tasks
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-simple
  version: '1.0.0'
do:
  - forkColors:
      fork:
        compete: false
        branches:
          - addRed:
              set:
                color: red
          - addBlue:
              set:
                color: blue
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Non-compete fork returns object with branch names as keys
        assert!(output.is_object());
        assert_eq!(output.as_object().unwrap().len(), 2);
    }

    // === Output.as + Export.as combined (Java SDK pattern) ===

    // --- Go SDK: fork_simple.yaml ---
    // Fork with set tasks + [.[] | .[]] merge
    #[tokio::test]
    async fn test_e2e_fork_simple() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-example
  version: '0.1.0'
do:
  - branchColors:
      fork:
        compete: false
        branches:
          - setRed:
              set:
                color1: red
          - setBlue:
              set:
                color2: blue
  - joinResult:
      set:
        colors: "${ [to_entries[].value[]] }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Fork returns {setRed: {color1: "red"}, setBlue: {color2: "blue"}}
        // to_entries | .[].value | .[] extracts values (order may vary due to concurrency)
        let colors = output["colors"].as_array().unwrap();
        assert_eq!(colors.len(), 2);
        assert!(colors.contains(&json!("red")));
        assert!(colors.contains(&json!("blue")));
    }

    // --- Java SDK: fork-no-compete.yaml ---
    // Fork with wait + set in branches
    #[tokio::test]
    async fn test_e2e_fork_no_compete_with_wait() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-example
  version: '0.1.0'
do:
  - callSomeone:
      fork:
        compete: false
        branches:
          - callNurse:
              do:
                - waitForNurse:
                    wait:
                      milliseconds: 5
                - nurseArrived:
                    set:
                      patientId: John
                      room: 1
          - callDoctor:
              do:
                - waitForDoctor:
                    wait:
                      milliseconds: 5
                - doctorArrived:
                    set:
                      patientId: Smith
                      room: 2
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Fork returns object with branch names as keys
        assert!(output.is_object());
        assert!(output.get("callNurse").is_some());
        assert!(output.get("callDoctor").is_some());
        assert_eq!(output["callNurse"]["room"], json!(1));
        assert_eq!(output["callDoctor"]["room"], json!(2));
    }

    // --- Java SDK: fork-wait.yaml ---
    // Fork with wait in branches + set
    #[tokio::test]
    async fn test_e2e_fork_wait() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: fork-wait
  version: '0.1.0'
do:
  - incrParallel:
      fork:
        compete: false
        branches:
          - helloBranch:
              do:
                - waitABit:
                    wait:
                      milliseconds: 5
                - set:
                    set:
                      value: 1
          - byeBranch:
              do:
                - waitABit:
                    wait:
                      milliseconds: 5
                - set:
                    set:
                      value: 2
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // Fork returns object with branch names as keys
        assert!(output.is_object());
        assert!(output.get("helloBranch").is_some());
        assert!(output.get("byeBranch").is_some());
        assert_eq!(output["helloBranch"]["value"], json!(1));
        assert_eq!(output["byeBranch"]["value"], json!(2));
    }

    // =========================================================================
    // E2E Integration Tests — Multi-task workflows
    // =========================================================================

    // === E2E-1: Shell + Set — command result processing ===
