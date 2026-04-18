use super::*;

    #[tokio::test]
    async fn test_runner_run_shell_echo() {
        let output = run_workflow_from_yaml(&testdata("run_shell_echo.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"]
            .as_str()
            .unwrap()
            .contains("Hello, anonymous"));
    }

    // === Run: Shell with JQ expression ===

    #[tokio::test]
    async fn test_runner_run_shell_jq() {
        let output = run_workflow_from_yaml(
            &testdata("run_shell_jq.yaml"),
            json!({"user": {"name": "John Doe"}}),
        )
        .await
        .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"]
            .as_str()
            .unwrap()
            .contains("Hello, John Doe"));
    }

    // === Run: Shell with environment variables ===

    #[tokio::test]
    async fn test_runner_run_shell_env() {
        let output = run_workflow_from_yaml(&testdata("run_shell_env.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"]
            .as_str()
            .unwrap()
            .contains("Hello John Doe from env!"));
    }

    // === Run: Shell exit code ===

    #[tokio::test]
    async fn test_runner_run_shell_exitcode() {
        let output = run_workflow_from_yaml(&testdata("run_shell_exitcode.yaml"), json!({}))
            .await
            .unwrap();
        // ls /nonexistent_directory should fail with non-zero exit code
        assert!(output.as_i64().unwrap() != 0);
    }

    // === Run: Shell stderr ===

    #[tokio::test]
    async fn test_runner_run_shell_stderr() {
        let output = run_workflow_from_yaml(&testdata("run_shell_stderr.yaml"), json!({}))
            .await
            .unwrap();
        // stderr should contain error message from ls
        assert!(output.as_str().unwrap().contains("ls:"));
    }

    // === Run: Shell return none ===

    #[tokio::test]
    async fn test_runner_run_shell_none() {
        let output = run_workflow_from_yaml(&testdata("run_shell_none.yaml"), json!({}))
            .await
            .unwrap();
        // return: none should return null
        assert!(output.is_null());
    }

    // === HTTP Call: GET ===

    #[tokio::test]
    async fn test_runner_run_shell_missing_command() {
        let result =
            run_workflow_from_yaml(&testdata("run_shell_missing_command.yaml"), json!({})).await;
        assert!(result.is_err());
    }

    // === Shell: pipeline (touch + cat) ===

    #[tokio::test]
    async fn test_runner_run_shell_pipeline() {
        let output = run_workflow_from_yaml(&testdata("run_shell_pipeline.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("hello world"));
    }

    // === Run shell: arguments with key-value pairs (map) ===
    // Matches Java SDK's echo-with-args-key-value.yaml pattern

    #[tokio::test]
    async fn test_runner_run_shell_args_kv_map() {
        let output = run_workflow_from_yaml(&testdata("run_shell_args_kv.yaml"), json!({}))
            .await
            .unwrap();
        // return: all gives {code, stdout, stderr}
        assert_eq!(output["code"], json!(0));
        let stdout = output["stdout"].as_str().unwrap();
        assert!(stdout.contains("--user"));
        assert!(stdout.contains("john"));
        assert!(stdout.contains("--password"));
        assert!(stdout.contains("doe"));
    }

    // === Run shell: arguments with only keys (positional) ===
    // Matches Java SDK's echo-with-args-only-key.yaml pattern

    #[tokio::test]
    async fn test_runner_run_shell_args_positional() {
        let output = run_workflow_from_yaml(
            &testdata("run_shell_args_only_key.yaml"),
            json!({
                "firstName": "Jane",
                "lastName": "Doe"
            }),
        )
        .await
        .unwrap();
        assert_eq!(output["code"], json!(0));
        let stdout = output["stdout"].as_str().unwrap();
        assert!(stdout.contains("Hello"));
        assert!(stdout.contains("Jane"));
        assert!(stdout.contains("Doe"));
        assert!(stdout.contains("from"));
        assert!(stdout.contains("args!"));
    }

    // === Run shell: command with environment variables ===
    // Matches Java SDK's touch-cat.yaml pattern

    #[tokio::test]
    async fn test_runner_run_shell_env_vars() {
        let output = run_workflow_from_yaml(
            &testdata("run_shell_touch_cat.yaml"),
            json!({
                "lastName": "Doe"
            }),
        )
        .await
        .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("hello world"));
    }

    // === HTTP Call: try-catch with communication error ===

    #[tokio::test]
    async fn test_runner_run_shell_no_await() {
        let output = run_workflow_from_yaml(&testdata("run_shell_no_await.yaml"), json!({}))
            .await
            .unwrap();
        // await: false should return null immediately
        assert!(output.is_null());
    }

    // === HTTP Call: Basic Auth with $secret ===

    #[tokio::test]
    async fn test_runner_run_shell_args() {
        let output = run_workflow_from_yaml(&testdata("run_shell_args.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("Hello"));
        assert!(output["stdout"].as_str().unwrap().contains("World"));
    }

    // === Shell: arguments with JQ expressions ===

    #[tokio::test]
    async fn test_runner_run_shell_args_expr() {
        let output = run_workflow_from_yaml(
            &testdata("run_shell_args_expr.yaml"),
            json!({"greeting": "Hello", "name": "World"}),
        )
        .await
        .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("Hello"));
        assert!(output["stdout"].as_str().unwrap().contains("World"));
    }

    // === Shell: environment with JQ expressions ===

    #[tokio::test]
    async fn test_runner_run_shell_env_expr() {
        let output = run_workflow_from_yaml(
            &testdata("run_shell_env_expr.yaml"),
            json!({"firstName": "Jane", "lastName": "Smith"}),
        )
        .await
        .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("Jane"));
        assert!(output["stdout"].as_str().unwrap().contains("Smith"));
    }

    // === Expression: default value (// operator) ===

    #[tokio::test]
    async fn test_runner_run_shell_args_key_value() {
        let output = run_workflow_from_yaml(&testdata("run_shell_args.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("Hello"));
        assert!(output["stdout"].as_str().unwrap().contains("World"));
    }

    // === Shell: arguments with key-value and JQ expression values ===

    #[tokio::test]
    async fn test_runner_run_shell_args_key_value_expr() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: run-shell-args-kv-expr
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: echo
          arguments:
            '--user': '${ .username }'
            '--password': '${ .password }'
        return: all
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner
            .run(json!({"username": "john", "password": "doe"}))
            .await
            .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("--user"));
        assert!(output["stdout"].as_str().unwrap().contains("john"));
        assert!(output["stdout"].as_str().unwrap().contains("--password"));
        assert!(output["stdout"].as_str().unwrap().contains("doe"));
    }

    // === Shell: arguments with JQ expression keys (key-only format) ===

    #[tokio::test]
    async fn test_runner_run_shell_args_expr_keys() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: run-shell-args-expr-keys
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: echo
          arguments:
            '${ .greeting }':
            '${ .name }':
            'from':
            'args!':
        return: all
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner
            .run(json!({"greeting": "Hello", "name": "World"}))
            .await
            .unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("Hello"));
        assert!(output["stdout"].as_str().unwrap().contains("World"));
        assert!(output["stdout"].as_str().unwrap().contains("from"));
        assert!(output["stdout"].as_str().unwrap().contains("args!"));
    }

    // === Expression: keys and values ===

    #[tokio::test]
    async fn test_runner_run_shell_return_none() {
        // Matches Java SDK's echo-none.yaml - return: none should return null
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: run-shell-none
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: 'echo "Serverless Workflow"'
        return: none
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert!(output.is_null());
    }

    #[tokio::test]
    async fn test_runner_run_shell_return_stderr() {
        // Matches Java SDK's echo-stderr.yaml - return: stderr on failing command
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: run-shell-stderr
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: 'ls /nonexistent_directory_for_test'
        return: stderr
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        // stderr output should be a string
        assert!(output.is_string());
        assert!(!output.as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_runner_run_shell_mixed_env() {
        // Matches Java SDK's echo-with-env.yaml - mix of literal and expression env vars
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: run-shell-env-mix
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: 'echo "Hello $FIRST_NAME $LAST_NAME from env!"'
          environment:
            FIRST_NAME: John
            LAST_NAME: '${.lastName}'
        return: all
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"lastName": "Doe"})).await.unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"]
            .as_str()
            .unwrap()
            .contains("Hello John Doe from env!"));
    }

    #[tokio::test]
    async fn test_runner_shell_args_only_key() {
        // Matches Java SDK's echo-with-args-only-key.yaml
        // Arguments with null values are passed as positional args
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-args-only-key
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: echo
          arguments:
            Hello:
            World:
        return: all
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("Hello"));
        assert!(output["stdout"].as_str().unwrap().contains("World"));
    }

    #[tokio::test]
    async fn test_runner_shell_args_key_value_expr() {
        // Matches Java SDK's echo-with-args-key-value-jq.yaml
        // Arguments with expression keys and values
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-args-kv-expr
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: echo
          arguments:
            '--user': '${.user}'
            '${.passwordKey}': 'doe'
        return: all
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner
            .run(json!({"user": "john", "passwordKey": "--password"}))
            .await
            .unwrap();
        assert_eq!(output["code"], json!(0));
        let stdout = output["stdout"].as_str().unwrap();
        assert!(stdout.contains("--user=john") || stdout.contains("john"));
        assert!(stdout.contains("--password=doe") || stdout.contains("doe"));
    }

    #[tokio::test]
    async fn test_runner_run_shell_echo_all() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: run-shell-echo-all
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: echo "hello world"
        return: all
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("hello world"));
    }

    // === Run shell: with map arguments key-value ===
    // Matches Java SDK's echo-with-args-key-value.yaml

    #[tokio::test]
    async fn test_runner_run_shell_args_map_key_value() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: run-shell-args-map-key-value
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          arguments:
            '--user': john
            '--password': doe
          command: echo
        return: all
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let result = runner.run(json!({})).await;
        // Check that it ran successfully (return: all gives {code, stdout, stderr})
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        let output = result.unwrap();
        assert_eq!(output["code"], json!(0));
        let stdout = output["stdout"].as_str().unwrap();
        assert!(
            stdout.contains("--user")
                || stdout.contains("john")
                || stdout.contains("--password")
                || stdout.contains("doe")
        );
    }

    // === Set: with default values (alternative operator) ===
    // Matches Go SDK's TestSetTaskExecutor_DefaultValues

    #[tokio::test]
    async fn test_runner_shell_return_code_failing_command() {
        // Matches Java SDK's echo-exitcode.yaml - return: code on a failing command
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-exitcode
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: 'ls /nonexistent_directory_xyz'
        return: code
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        // return: code should give exit code as number (non-zero for failing command)
        assert!(output.is_number());
        assert_ne!(output, json!(0));
    }

    // === Shell with JQ expression command (Java SDK echo-jq.yaml pattern) ===

    #[tokio::test]
    async fn test_runner_shell_not_awaiting() {
        // Matches Java SDK's echo-not-awaiting.yaml - async shell execution
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-not-awaiting
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: 'echo hello'
        await: false
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        // async execution: output should be null or empty since we don't wait
        let output = runner.run(json!({})).await.unwrap();
        assert!(
            output.is_null()
                || output == json!({})
                || output.as_str().is_some_and(|s| s.is_empty())
        );
    }

    // === Fork compete with set branches (Java SDK fork.yaml pattern) ===

    #[tokio::test]
    async fn test_e2e_shell_set_command_result() {
        // Execute `echo 42`, then set task builds a result object from the output
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-set-command
  version: '0.1.0'
do:
  - runCommand:
      run:
        shell:
          command: 'echo 42'
        return: all
  - buildResult:
      set:
        answer: '${ .stdout | tonumber }'
        success: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["answer"], json!(42));
        assert_eq!(output["success"], json!(true));
    }

    // === E2E: Shell + Switch — system detection ===

    #[tokio::test]
    async fn test_e2e_shell_switch_system_detect() {
        // Run `uname`, switch on the result to set an OS indicator
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-switch-system
  version: '0.1.0'
do:
  - detectOS:
      run:
        shell:
          command: 'uname'
        return: all
  - classify:
      switch:
        - isMac:
            when: '${ .stdout | test("Darwin") }}'
            then: setMac
        - isLinux:
            when: '${ .stdout | test("Linux") }}'
            then: setLinux
        - other:
            then: setUnknown
  - setMac:
      set:
        os: macos
        detected: true
      then: end
  - setLinux:
      set:
        os: linux
        detected: true
      then: end
  - setUnknown:
      set:
        os: unknown
        detected: true
      then: end
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["detected"], json!(true));
        // On macOS (CI runner), should be macos
        #[cfg(target_os = "macos")]
        assert_eq!(output["os"], json!("macos"));
    }

    // === E2E: HTTP + Try/Catch — API call with error recovery ===

    #[tokio::test]
    async fn test_e2e_shell_for_switch_pipeline() {
        // Generate data with shell, process with for loop, classify using if-then-else expressions
        // Note: switch then:goto inside a for loop's do block causes goto targets to execute
        // and then continue executing subsequent tasks, which doesn't work well for classification.
        // Using if-then-else JQ expressions in set tasks is the correct pattern.
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-for-ifelse-pipeline
  version: '0.1.0'
do:
  - generateData:
      set:
        scores: [85, 42, 95, 60, 73]
  - classifyScores:
      for:
        each: score
        in: '${ .scores }'
      do:
        - classify:
            set:
              results: '${ (.results // []) + [{score: $score, grade: (if $score >= 90 then "A" elif $score >= 70 then "B" elif $score >= 60 then "C" else "F" end)}] }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();

        let results = output["results"]
            .as_array()
            .expect("expected results array");
        assert_eq!(results.len(), 5);

        // 85→B, 42→F, 95→A, 60→C, 73→B
        let grade_85 = results
            .iter()
            .find(|r| r["score"] == json!(85))
            .expect("find 85");
        assert_eq!(grade_85["grade"], json!("B"));

        let grade_42 = results
            .iter()
            .find(|r| r["score"] == json!(42))
            .expect("find 42");
        assert_eq!(grade_42["grade"], json!("F"));

        let grade_95 = results
            .iter()
            .find(|r| r["score"] == json!(95))
            .expect("find 95");
        assert_eq!(grade_95["grade"], json!("A"));

        let grade_60 = results
            .iter()
            .find(|r| r["score"] == json!(60))
            .expect("find 60");
        assert_eq!(grade_60["grade"], json!("C"));

        let grade_73 = results
            .iter()
            .find(|r| r["score"] == json!(73))
            .expect("find 73");
        assert_eq!(grade_73["grade"], json!("B"));
    }

    // === E2E: HTTP + Export + Switch — order processing ===

    #[tokio::test]
    async fn test_e2e_shell_wait_set_timed_workflow() {
        // Shell produces data → short wait → set processes the result
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-wait-set
  version: '0.1.0'
do:
  - captureTime:
      run:
        shell:
          command: 'echo "hello-world"'
        return: all
  - briefPause:
      wait: PT0.01S
  - finalizeResult:
      set:
        message: '${ .stdout | rtrimstr("\n") }}'
        status: completed
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["message"], json!("hello-world"));
        assert_eq!(output["status"], json!("completed"));
    }

    // === E2E: For + Raise + Try — loop with error handling ===

    #[tokio::test]
    async fn test_e2e_shell_set_year() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: e2e-shell-set
  version: '0.1.0'
do:
  - getYear:
      run:
        shell:
          command: "date +%Y"
        return: stdout
  - buildResult:
      set:
        year: ${ . | tonumber }
        verified: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        let year = output["year"].as_i64().unwrap();
        assert!(year >= 2020, "year should be >= 2020, got: {}", year);
        assert_eq!(output["verified"], json!(true));
    }

    // === E2E-2: Shell + Switch — system detection ===

    #[tokio::test]
    async fn test_e2e_shell_switch_system() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: e2e-shell-switch
  version: '0.1.0'
do:
  - detectOS:
      run:
        shell:
          command: uname
        return: stdout
  - classify:
      switch:
        - isDarwin:
            when: ${ . == "Darwin" }
            then: setMac
        - isLinux:
            when: ${ . == "Linux" }
            then: setLinux
        - fallback:
            then: setOther
  - setMac:
      set:
        os: macos
        detected: true
  - setLinux:
      set:
        os: linux
        detected: true
  - setOther:
      set:
        os: other
        detected: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["detected"], json!(true));
        let os = output["os"].as_str().unwrap();
        assert!(
            os == "macos" || os == "linux" || os == "other",
            "unexpected os: {}",
            os
        );
    }

    // === E2E-3: HTTP + Try/Catch — API call with error recovery ===

    #[tokio::test]
    async fn test_e2e_shell_set_for_switch_pipeline() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: e2e-pipeline
  version: '0.1.0'
do:
  - generateData:
      run:
        shell:
          command: "echo 10"
        return: stdout
  - setupData:
      set:
        numbers: [1, 2, 3]
        threshold: ${ . | tonumber }
  - processLast:
      for:
        each: num
        in: ${ .numbers }
      do:
        - transform:
            set:
              value: ${ $num }
              doubled: ${ $num * 2 }
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        // For loop returns the output of the last iteration
        assert!(output.is_object(), "for loop should return an object");
        assert_eq!(output["value"], json!(3), "last item should be 3");
        assert_eq!(output["doubled"], json!(6), "doubled should be 6");
    }

    // === E2E-6: HTTP + Export Context + Conditional Logic ===

    #[tokio::test]
    async fn test_runner_run_container_with_custom_handler() {
        use crate::handler::RunHandler;

        struct MockContainerHandler;

        #[async_trait::async_trait]
        impl RunHandler for MockContainerHandler {
            fn run_type(&self) -> &str {
                "container"
            }

            async fn handle(
                &self,
                _task_name: &str,
                _run_config: &Value,
                input: &Value,
            ) -> WorkflowResult<Value> {
                Ok(json!({"container_output": input["image"].as_str().unwrap_or("default")}))
            }
        }

        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: container-test
  version: '0.1.0'
do:
  - runContainer:
      run:
        container:
          image: alpine:latest
          command: echo hello
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_run_handler(Box::new(MockContainerHandler));

        let output = runner.run(json!({"image": "alpine:latest"})).await.unwrap();
        assert_eq!(output["container_output"], json!("alpine:latest"));
    }

    #[tokio::test]
    async fn test_runner_run_script_with_custom_handler() {
        use crate::handler::RunHandler;

        struct MockScriptHandler;

        #[async_trait::async_trait]
        impl RunHandler for MockScriptHandler {
            fn run_type(&self) -> &str {
                "script"
            }

            async fn handle(
                &self,
                _task_name: &str,
                _run_config: &Value,
                _input: &Value,
            ) -> WorkflowResult<Value> {
                Ok(json!({"script_output": "executed"}))
            }
        }

        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: script-test
  version: '0.1.0'
do:
  - runScript:
      run:
        script:
          language: javascript
          code: 'return 42;'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_run_handler(Box::new(MockScriptHandler));

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["script_output"], json!("executed"));
    }

    #[tokio::test]
    async fn test_runner_run_container_without_handler_returns_error() {
        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: container-no-handler
  version: '0.1.0'
do:
  - runContainer:
      run:
        container:
          image: alpine:latest
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("container"),
            "error should mention 'container': {}",
            err
        );
        assert!(
            err.contains("RunHandler"),
            "error should mention 'RunHandler': {}",
            err
        );
    }

    /// Test shell with empty command — Java SDK's missing-shell-command.yaml
    /// Empty shell command runs but returns empty output (no error on Unix)
    #[tokio::test]
    async fn test_runner_shell_missing_command() {
        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: missing-shell-command
  version: '1.0.0'
do:
  - missingShellCommand:
      run:
        shell:
          command: ''
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        // Empty command on Unix succeeds with empty output
        assert!(
            output.is_string()
                || output.is_null()
                || (output.is_object() && output.as_object().unwrap().is_empty()),
            "empty shell command should return empty output, got: {:?}",
            output
        );
    }

    #[tokio::test]
    async fn test_e2e_shell_set_pipeline() {
        // Shell generates data → set processes it into structured output
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-set-pipeline
  version: '0.1.0'
do:
  - getDate:
      run:
        shell:
          command: date +%Y
        return: all
  - buildResult:
      set:
        year: '${ .stdout | tonumber }'
        greeting: 'Hello from ${ .stdout | gsub("\n";"") }!'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();

        // Verify year is a valid number (current year)
        let year = output["year"].as_u64().expect("year should be a number");
        assert!((2024..=2030).contains(&year));
        assert!(output["greeting"].as_str().unwrap().contains("Hello from"));
    }

    // === E2E: Shell + Switch — system detection ===

    #[tokio::test]
    async fn test_e2e_shell_switch_detection() {
        // Shell gets system name → switch branches based on result
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-switch-detection
  version: '0.1.0'
do:
  - getOS:
      run:
        shell:
          command: uname
        return: all
  - classifyOS:
      switch:
        - mac:
            when: '${ .stdout | test("Darwin") }'
            then: setMac
        - linux:
            when: '${ .stdout | test("Linux") }'
            then: setLinux
        - default:
            then: setUnknown
  - setMac:
      set:
        os: macos
        family: unix
      then: end
  - setLinux:
      set:
        os: linux
        family: unix
      then: end
  - setUnknown:
      set:
        os: unknown
        family: unknown
      then: end
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();

        // On macOS, should detect "macos"
        #[cfg(target_os = "macos")]
        {
            assert_eq!(output["os"], json!("macos"));
            assert_eq!(output["family"], json!("unix"));
        }
        #[cfg(target_os = "linux")]
        {
            assert_eq!(output["os"], json!("linux"));
            assert_eq!(output["family"], json!("unix"));
        }
    }

    // === CallFunction Catalog Mechanism ===
