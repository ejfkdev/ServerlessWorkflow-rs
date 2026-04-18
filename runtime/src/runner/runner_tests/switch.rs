use super::*;

    #[tokio::test]
    async fn test_runner_switch_match_red() {
        let output = run_workflow_from_yaml(
            &testdata("switch_match.yaml"),
            json!({"color": "red", "colors": []}),
        )
        .await
        .unwrap();
        assert_eq!(output["colors"], json!(["red"]));
    }

    #[tokio::test]
    async fn test_runner_switch_match_green() {
        let output = run_workflow_from_yaml(
            &testdata("switch_match.yaml"),
            json!({"color": "green", "colors": []}),
        )
        .await
        .unwrap();
        assert_eq!(output["colors"], json!(["green"]));
    }

    #[tokio::test]
    async fn test_runner_switch_match_blue() {
        let output = run_workflow_from_yaml(
            &testdata("switch_match.yaml"),
            json!({"color": "blue", "colors": []}),
        )
        .await
        .unwrap();
        assert_eq!(output["colors"], json!(["blue"]));
    }

    // === Switch With Default ===

    #[tokio::test]
    async fn test_runner_switch_default() {
        let output = run_workflow_from_yaml(
            &testdata("switch_with_default.yaml"),
            json!({"color": "yellow", "colors": []}),
        )
        .await
        .unwrap();
        assert_eq!(output["colors"], json!(["default"]));
    }

    // === For Loop: Colors ===

    #[tokio::test]
    async fn test_runner_switch_then_loop() {
        let output =
            run_workflow_from_yaml(&testdata("switch_then_loop.yaml"), json!({"count": 1}))
                .await
                .unwrap();
        assert_eq!(output["count"], json!(6));
    }

    // === Switch Then String ===

    #[tokio::test]
    async fn test_runner_switch_then_string_electronic() {
        let output = run_workflow_from_yaml(
            &testdata("switch_then_string.yaml"),
            json!({"orderType": "electronic"}),
        )
        .await
        .unwrap();
        assert_eq!(output["validate"], json!(true));
        assert_eq!(output["status"], json!("fulfilled"));
    }

    #[tokio::test]
    async fn test_runner_switch_then_string_physical() {
        let output = run_workflow_from_yaml(
            &testdata("switch_then_string.yaml"),
            json!({"orderType": "physical"}),
        )
        .await
        .unwrap();
        assert_eq!(output["inventory"], json!("clear"));
        assert_eq!(output["items"], json!(1));
    }

    #[tokio::test]
    async fn test_runner_switch_then_string_default() {
        let output = run_workflow_from_yaml(
            &testdata("switch_then_string.yaml"),
            json!({"orderType": "unknown"}),
        )
        .await
        .unwrap();
        assert_eq!(output["log"], json!("warn"));
        assert_eq!(output["message"], json!("something's wrong"));
    }

    // === Direct WorkflowRunner test (no YAML) ===

    #[tokio::test]
    async fn test_runner_switch_then_exit() {
        let output =
            run_workflow_from_yaml(&testdata("switch_then_exit.yaml"), json!({"value": "exit"}))
                .await
                .unwrap();
        // exit should stop the current composite task (do block)
        // shouldNotRun should NOT execute
        assert!(output.get("ran").is_none());
    }

    #[tokio::test]
    async fn test_runner_switch_then_end() {
        let output = run_workflow_from_yaml(
            &testdata("switch_then_exit.yaml"),
            json!({"value": "other"}),
        )
        .await
        .unwrap();
        // default then: end should stop the workflow
        assert!(output.get("ran").is_none());
    }

    // === Switch then continue ===

    #[tokio::test]
    async fn test_runner_switch_then_continue() {
        let output = run_workflow_from_yaml(
            &testdata("switch_then_continue.yaml"),
            json!({"action": "skip"}),
        )
        .await
        .unwrap();
        // then: continue should proceed to the next task
        assert_eq!(output["ran"], json!(true));
    }

    #[tokio::test]
    async fn test_runner_switch_then_continue_default() {
        let output = run_workflow_from_yaml(
            &testdata("switch_then_continue.yaml"),
            json!({"action": "other"}),
        )
        .await
        .unwrap();
        // default case then: end should stop the workflow
        assert!(output.get("ran").is_none());
    }

    // === HTTP Call: output response with output.as ===

    #[tokio::test]
    async fn test_runner_switch_then_goto() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-then-goto
  version: '0.1.0'
do:
  - checkColor:
      switch:
        - redCase:
            when: ${ .color == "red" }
            then: setRed
        - defaultCase:
            then: end
  - skippedTask:
      set:
        skipped: true
      then: end
  - setRed:
      set:
        color: "RED"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"color": "red"})).await.unwrap();
        assert_eq!(output["color"], json!("RED"));
        assert!(output.get("skipped").is_none());
    }

    // === Try-Catch with catch.as error variable ===

    #[tokio::test]
    async fn test_runner_switch_multi_match_a() {
        let output =
            run_workflow_from_yaml(&testdata("switch_multi_match.yaml"), json!({"score": 95}))
                .await
                .unwrap();
        assert_eq!(output["grade"], json!("A"));
        assert_eq!(output["passed"], json!(true));
    }

    #[tokio::test]
    async fn test_runner_switch_multi_match_b() {
        let output =
            run_workflow_from_yaml(&testdata("switch_multi_match.yaml"), json!({"score": 85}))
                .await
                .unwrap();
        assert_eq!(output["grade"], json!("B"));
        assert_eq!(output["passed"], json!(true));
    }

    #[tokio::test]
    async fn test_runner_switch_multi_match_c() {
        let output =
            run_workflow_from_yaml(&testdata("switch_multi_match.yaml"), json!({"score": 75}))
                .await
                .unwrap();
        assert_eq!(output["grade"], json!("C"));
        assert_eq!(output["passed"], json!(true));
    }

    #[tokio::test]
    async fn test_runner_switch_multi_match_f() {
        let output =
            run_workflow_from_yaml(&testdata("switch_multi_match.yaml"), json!({"score": 50}))
                .await
                .unwrap();
        assert_eq!(output["grade"], json!("F"));
        assert_eq!(output["passed"], json!(false));
    }

    // === Set: nested expressions (object with computed fields) ===

    #[tokio::test]
    async fn test_runner_switch_then_goto_multiple() {
        // Test that switch can goto a task which then continues to another task
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-then-goto-multiple
  version: '0.1.0'
do:
  - step1:
      switch:
        - goStep3:
            when: ${ .skip == true }
            then: step3
        - goStep2:
            then: step2
  - step2:
      set:
        result: step2
      then: step4
  - step3:
      set:
        result: step3
  - step4:
      set:
        final: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // When skip=true: step1 -> step3 -> step4
        let output = runner.run(json!({"skip": true})).await.unwrap();
        assert_eq!(output["final"], json!(true));
    }

    #[tokio::test]
    async fn test_runner_switch_then_goto_normal() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-then-goto-normal
  version: '0.1.0'
do:
  - step1:
      switch:
        - goStep3:
            when: ${ .skip == true }
            then: step3
        - goStep2:
            then: step2
  - step2:
      set:
        result: step2
      then: step4
  - step3:
      set:
        result: step3
  - step4:
      set:
        final: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // When skip=false: step1 -> step2 -> step4
        let output = runner.run(json!({"skip": false})).await.unwrap();
        assert_eq!(output["final"], json!(true));
    }

    // === Nested do: with export context passing ===

    #[tokio::test]
    async fn test_runner_switch_goto_switch() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-goto-switch
  version: '0.1.0'
do:
  - checkLevel:
      switch:
        - adminCase:
            when: ${ .role == "admin" }
            then: adminSwitch
        - defaultCase:
            then: end
  - userSwitch:
      switch:
        - regularCase:
            when: ${ .level > 5 }
            then: end
        - defaultCase:
            then: end
  - adminSwitch:
      switch:
        - superAdmin:
            when: ${ .level > 10 }
            then: grantAll
        - defaultCase:
            then: grantBasic
  - grantAll:
      set:
        access: all
      then: end
  - grantBasic:
      set:
        access: basic
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // Admin with level > 10 → grantAll
        let output = runner
            .run(json!({"role": "admin", "level": 15}))
            .await
            .unwrap();
        assert_eq!(output["access"], json!("all"));
    }

    #[tokio::test]
    async fn test_runner_switch_goto_switch_basic() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-goto-switch-basic
  version: '0.1.0'
do:
  - checkLevel:
      switch:
        - adminCase:
            when: ${ .role == "admin" }
            then: adminSwitch
        - defaultCase:
            then: end
  - userSwitch:
      switch:
        - regularCase:
            when: ${ .level > 5 }
            then: end
        - defaultCase:
            then: end
  - adminSwitch:
      switch:
        - superAdmin:
            when: ${ .level > 10 }
            then: grantAll
        - defaultCase:
            then: grantBasic
  - grantAll:
      set:
        access: all
      then: end
  - grantBasic:
      set:
        access: basic
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // Admin with level <= 10 → grantBasic
        let output = runner
            .run(json!({"role": "admin", "level": 5}))
            .await
            .unwrap();
        assert_eq!(output["access"], json!("basic"));
    }

    #[tokio::test]
    async fn test_runner_switch_goto_switch_not_admin() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-goto-switch-not-admin
  version: '0.1.0'
do:
  - checkLevel:
      switch:
        - adminCase:
            when: ${ .role == "admin" }
            then: adminSwitch
        - defaultCase:
            then: end
  - userSwitch:
      switch:
        - regularCase:
            when: ${ .level > 5 }
            then: end
        - defaultCase:
            then: end
  - adminSwitch:
      switch:
        - superAdmin:
            when: ${ .level > 10 }
            then: grantAll
        - defaultCase:
            then: grantBasic
  - grantAll:
      set:
        access: all
      then: end
  - grantBasic:
      set:
        access: basic
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // Not admin → end immediately (no access set)
        let output = runner
            .run(json!({"role": "user", "level": 5}))
            .await
            .unwrap();
        assert!(output.get("access").is_none());
    }

    // === Try-catch with when as expression ===

    #[tokio::test]
    async fn test_runner_switch_age_classification() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-age-classification
  version: '0.1.0'
do:
  - classify:
      switch:
        - senior:
            when: ${ .age >= 65 }
            then: setSenior
        - adult:
            when: ${ .age >= 18 }
            then: setAdult
        - minor:
            when: ${ .age < 18 }
            then: setMinor
  - setSenior:
      set:
        category: senior
      then: end
  - setAdult:
      set:
        category: adult
      then: end
  - setMinor:
      set:
        category: minor
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // Senior
        let output = runner.run(json!({"age": 70})).await.unwrap();
        assert_eq!(output["category"], json!("senior"));

        // Adult
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"age": 30})).await.unwrap();
        assert_eq!(output["category"], json!("adult"));

        // Minor
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"age": 10})).await.unwrap();
        assert_eq!(output["category"], json!("minor"));
    }

    // === Set with merge behavior (preserving existing fields) ===

    #[tokio::test]
    async fn test_runner_switch_no_match_no_default() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-no-match
  version: '0.1.0'
do:
  - checkColor:
      switch:
        - redCase:
            when: ${ .color == "red" }
            then: end
  - afterSwitch:
      set:
        continued: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"color": "blue"})).await.unwrap();
        // No match and no default: switch passes through, next task runs
        assert_eq!(output["continued"], json!(true));
    }

    // === For loop with empty collection ===

    #[tokio::test]
    async fn test_runner_switch_string_comparison() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-string-cmp
  version: '0.1.0'
do:
  - checkRole:
      switch:
        - adminCase:
            when: ${ .role == "admin" }
            then: setAdmin
        - userCase:
            when: ${ .role == "user" }
            then: setUser
        - defaultCase:
            then: setGuest
  - setAdmin:
      set:
        level: admin
      then: end
  - setUser:
      set:
        level: user
      then: end
  - setGuest:
      set:
        level: guest
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"role": "admin"})).await.unwrap();
        assert_eq!(output["level"], json!("admin"));

        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"role": "guest"})).await.unwrap();
        assert_eq!(output["level"], json!("guest"));
    }

    // === For loop: collection with single element ===

    #[tokio::test]
    async fn test_runner_switch_then_goto_backwards() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-goto-backwards
  version: '0.1.0'
do:
  - init:
      set:
        counter: 0
  - checkCounter:
      switch:
        - notDone:
            when: ${ .counter < 3 }
            then: increment
        - doneCase:
            then: end
  - increment:
      set:
        counter: "${ .counter + 1 }"
      then: checkCounter
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["counter"], json!(3));
    }

    // === For loop: with at (index) variable and custom name ===

    #[tokio::test]
    async fn test_runner_switch_goto_forward_skip() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-goto-forward
  version: '0.1.0'
do:
  - check:
      switch:
        - skipCase:
            when: ${ .skip == true }
            then: finalStep
        - proceedCase:
            then: step2
  - step2:
      set:
        ran: true
  - step3:
      set:
        alsoRan: true
  - finalStep:
      set:
        done: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // When skip=true: skip step2 and step3
        let output = runner.run(json!({"skip": true})).await.unwrap();
        assert_eq!(output["done"], json!(true));
        assert!(output.get("ran").is_none());
        assert!(output.get("alsoRan").is_none());
    }

    // === Nested for with inner export ===

    #[tokio::test]
    async fn test_runner_switch_then_end_short_circuit() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-then-end-short-circuit
  version: '0.1.0'
do:
  - classify:
      switch:
        - isAdult:
            when: ${ .age >= 18 }
            then: setAdult
        - isMinor:
            when: ${ .age < 18 }
            then: setMinor
  - setAdult:
      set:
        category: adult
      then: end
  - setMinor:
      set:
        category: minor
  - unreachable:
      set:
        shouldNotReach: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"age": 25})).await.unwrap();
        assert_eq!(output["category"], json!("adult"));
        assert!(output.get("shouldNotReach").is_none());
    }

    // === For loop with continue-like behavior (then: continue) ===

    #[tokio::test]
    async fn test_runner_switch_nested_goto_outer() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-nested-goto-outer
  version: '0.1.0'
do:
  - outerSwitch:
      switch:
        - isHigh:
            when: ${ .level == "high" }
            then: handleHigh
        - isLow:
            when: ${ .level == "low" }
            then: handleLow
  - handleHigh:
      set:
        result: high_priority
      then: end
  - handleLow:
      set:
        result: low_priority
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"level": "high"})).await.unwrap();
        assert_eq!(output["result"], json!("high_priority"));

        let workflow2: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner2 = WorkflowRunner::new(workflow2).unwrap();
        let output2 = runner2.run(json!({"level": "low"})).await.unwrap();
        assert_eq!(output2["result"], json!("low_priority"));
    }

    // === For with early exit via raise in try-catch ===

    #[tokio::test]
    async fn test_runner_switch_with_export() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-export
  version: '0.1.0'
do:
  - classify:
      do:
        - checkScore:
            switch:
              - isHigh:
                  when: ${ .score >= 80 }
                  then: continue
        - setLevel:
            set:
              level: high
      export:
        as: "${ {level: .level} }"
  - setResult:
      set:
        result: "${ $context.level }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"score": 90})).await.unwrap();
        assert_eq!(output["result"], json!("high"));
    }

    #[tokio::test]
    async fn test_runner_switch_task_export() {
        // Tests that export.as on a Switch task itself (not on a Do task) works correctly
        let output =
            run_workflow_from_yaml(&testdata("switch_export.yaml"), json!({"color": "red"}))
                .await
                .unwrap();
        // When color==red, switch matches, export.as writes matched=true to $context
        // setResult uses $context.matched
        assert_eq!(output["result"], json!(true));
    }

    #[tokio::test]
    async fn test_runner_switch_task_export_default() {
        // When switch doesn't match any specific case, default case applies
        let output =
            run_workflow_from_yaml(&testdata("switch_export.yaml"), json!({"color": "blue"}))
                .await
                .unwrap();
        // color is blue, no case matches, default then:continue runs
        // export.as still writes matched=true to context (export is unconditional)
        assert_eq!(output["result"], json!(true));
    }

    // === For loop with input.from ===

    #[tokio::test]
    async fn test_runner_switch_all_false_with_default() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-all-false-default
  version: '0.1.0'
do:
  - classify:
      switch:
        - isA:
            when: ${ .type == "a" }
            then: continue
        - isB:
            when: ${ .type == "b" }
            then: continue
        - defaultCase:
            then: continue
      set:
        matched: true
  - setResult:
      set:
        result: "${ .matched }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // No when matches → default case → continue
        let _output = runner.run(json!({"type": "unknown"})).await.unwrap();
        // Switch with default should match
    }

    // === Expression: path expression (paths with filter) ===

    #[tokio::test]
    async fn test_runner_switch_complex_when() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-complex-when
  version: '0.1.0'
do:
  - checkAge:
      switch:
        - isMinor:
            when: ${ .age < 18 }
            then: continue
        - isAdult:
            when: ${ .age >= 18 }
            then: continue
  - setResult:
      set:
        classification: "${ if .age < 18 then .minorLabel else .adultLabel end }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner
            .run(json!({"age": 25, "minorLabel": "child", "adultLabel": "grown-up"}))
            .await
            .unwrap();
        assert_eq!(output["classification"], json!("grown-up"));
    }

    // === Expression: string interpolation ===

    #[tokio::test]
    async fn test_runner_switch_then_continue_next() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-then-continue-next
  version: '0.1.0'
do:
  - check:
      switch:
        - isOk:
            when: ${ .ok == true }
            then: continue
  - afterCheck:
      set:
        afterSwitch: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"ok": true})).await.unwrap();
        assert_eq!(output["afterSwitch"], json!(true));
    }

    // === Expression: tostring on numbers ===

    #[tokio::test]
    async fn test_runner_switch_no_match_passthrough_continue() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-no-match-continue
  version: '0.1.0'
do:
  - check:
      switch:
        - isX:
            when: ${ .type == "x" }
            then: continue
  - afterCheck:
      set:
        passed: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // type != "x", no match, no default → pass through
        let output = runner.run(json!({"type": "y"})).await.unwrap();
        assert_eq!(output["passed"], json!(true));
    }

    // === Expression: complex object construction with multiple fields ===

    #[tokio::test]
    async fn test_runner_switch_then_string_branches() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-then-string-branches
  version: '0.1.0'
do:
  - classify:
      switch:
        - isType1:
            when: ${ .type == "A" }
            then: handleA
        - isType2:
            when: ${ .type == "B" }
            then: handleB
        - defaultCase:
            then: handleDefault
      then: continue
  - handleA:
      set:
        result: handledA
      then: end
  - handleB:
      set:
        result: handledB
      then: end
  - handleDefault:
      set:
        result: handledDefault
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"type": "B"})).await.unwrap();
        assert_eq!(output["result"], json!("handledB"));
    }

    // === Batch 5: Unique new DSL pattern tests ===

    // === Expression: ascii_upcase/downcase ===

    #[tokio::test]
    async fn test_runner_switch_default_continue() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-default-continue
  version: '0.1.0'
do:
  - classify:
      switch:
        - isA:
            when: ${ .type == "A" }
            then: continue
        - isDefault:
            then: continue
      then: continue
  - afterSwitch:
      set:
        done: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"type": "X"})).await.unwrap();
        assert_eq!(output["done"], json!(true));
    }

    // === Expression: object merge with * ===

    #[tokio::test]
    async fn test_runner_switch_first_match_wins() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-first-match
  version: '0.1.0'
do:
  - classify:
      switch:
        - isHigh:
            when: ${ .score > 50 }
            then: end
        - isVeryHigh:
            when: ${ .score > 90 }
            then: end
      then: continue
  - afterSwitch:
      set:
        reached: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // Score 95 matches isHigh first (score > 50), then:end
        let output = runner.run(json!({"score": 95})).await.unwrap();
        assert!(output.get("reached").is_none());
    }

    // === Expression: array concatenation ===

    // Switch: then:end in the middle stops workflow
    #[tokio::test]
    async fn test_runner_switch_then_end_mid() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-then-end-mid
  version: '0.1.0'
do:
  - check:
      switch:
        - stopNow:
            when: ${ .stop == true }
            then: end
        - continueNorm:
            when: ${ .stop == false }
            then: continue
  - shouldNotRun:
      set:
        ran: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"stop": true})).await.unwrap();
        assert!(output.get("ran").is_none());
    }

    // Switch: multiple conditions with complex expressions
    #[tokio::test]
    async fn test_runner_switch_complex_conditions() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-complex-conditions
  version: '0.1.0'
do:
  - classify:
      switch:
        - high:
            when: ${ .score >= 80 }
            then: end
        - medium:
            when: ${ .score >= 50 }
            then: continue
        - low:
            when: ${ .score < 50 }
            then: continue
  - bonus:
      set:
        bonus: true
        score: "${ .score }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        // Score 90 → high → then:end → no bonus
        let output = runner.run(json!({"score": 90})).await.unwrap();
        assert!(output.get("bonus").is_none());
    }

    // Switch: string then with default
    #[tokio::test]
    async fn test_runner_switch_then_string_default_v2() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-then-string-default-v2
  version: '0.1.0'
do:
  - classify:
      switch:
        - isA:
            when: ${ .type == "a" }
            then: handleA
        - isDefault:
            when: ${ .type != "a" }
            then: handleDefault
  - handleA:
      set:
        handler: a
        type: "${ .type }"
      then: end
  - handleDefault:
      set:
        handler: default
        type: "${ .type }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"type": "b"})).await.unwrap();
        assert_eq!(output["handler"], json!("default"));
    }

    #[tokio::test]
    async fn test_runner_switch_numeric_comparison() {
        // Switch case based on numeric comparison
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-numeric
  version: '0.1.0'
do:
  - checkAge:
      switch:
        - minorCase:
            when: ${ .age < 18 }
            then: setMinor
        - adultCase:
            when: ${ .age >= 18 }
            then: setAdult
  - setMinor:
      set:
        category: minor
      then: end
  - setAdult:
      set:
        category: adult
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();

        let runner = WorkflowRunner::new(workflow.clone()).unwrap();
        let output = runner.run(json!({"age": 12})).await.unwrap();
        assert_eq!(output["category"], json!("minor"));

        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"age": 25})).await.unwrap();
        assert_eq!(output["category"], json!("adult"));
    }

    // === For loop with object collection ===

    #[tokio::test]
    async fn test_runner_switch_goto_multiple_targets() {
        // Switch that jumps to different named tasks
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-goto-multi
  version: '0.1.0'
do:
  - dispatch:
      switch:
        - caseA:
            when: ${ .type == "A" }
            then: handleA
        - caseB:
            when: ${ .type == "B" }
            then: handleB
        - defaultCase:
            then: handleDefault
  - handleA:
      set:
        handled: A
      then: end
  - handleB:
      set:
        handled: B
      then: end
  - handleDefault:
      set:
        handled: unknown
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();

        let runner = WorkflowRunner::new(workflow.clone()).unwrap();
        let output = runner.run(json!({"type": "A"})).await.unwrap();
        assert_eq!(output["handled"], json!("A"));

        let runner = WorkflowRunner::new(workflow.clone()).unwrap();
        let output = runner.run(json!({"type": "B"})).await.unwrap();
        assert_eq!(output["handled"], json!("B"));

        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"type": "C"})).await.unwrap();
        assert_eq!(output["handled"], json!("unknown"));
    }

    // === For loop with at (index variable) and output.as ===

    #[tokio::test]
    async fn test_runner_switch_output_as_exports_transformed() {
        // Verify that switch task's output.as + export.as work together
        // The fix ensures process_task_output result is used for process_task_export
        // (previously `let _ =` discarded the transformed output)
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-output-export
  version: '0.1.0'
do:
  - classify:
      switch:
        - highCase:
            when: ${ .score >= 80 }
            then: continue
      output:
        as: .score
      export:
        as: '.'
  - check:
      set:
        exported: '${ $context }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"score": 90})).await.unwrap();
        // output.as: .score → 90, export.as: '.' → $context = 90
        // Without the fix, $context would be {"score": 90} (the untransformed output)
        assert_eq!(output["exported"], json!(90));
    }

    #[tokio::test]
    async fn test_runner_switch_output_as_propagates_to_next_task() {
        // Verify that switch task's output.as transformation propagates to the next task
        // (previously switch output was not updating the loop's `output` variable)
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-output-propagate
  version: '0.1.0'
do:
  - classify:
      switch:
        - highCase:
            when: ${ .score >= 80 }
            then: continue
      output:
        as: .score
  - afterSwitch:
      set:
        received: '${ . }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"score": 90})).await.unwrap();
        // output.as: .score → 90 is propagated to next task
        // Without the fix, next task would receive {"score": 90} (the untransformed input)
        assert_eq!(output["received"], json!(90));
    }

    // --- Go SDK: switch_match.yaml ---
    // Switch with then:goto + then:end on target tasks
    #[tokio::test]
    async fn test_e2e_switch_match_red() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: default
  name: switch-match
  version: '1.0.0'
do:
  - switchColor:
      switch:
        - red:
            when: '.color == "red"'
            then: setRed
        - green:
            when: '.color == "green"'
            then: setGreen
        - blue:
            when: '.color == "blue"'
            then: setBlue
  - setRed:
      set:
        colors: '${ .colors + [ "red" ] }'
      then: end
  - setGreen:
      set:
        colors: '${ .colors + [ "green" ] }'
      then: end
  - setBlue:
      set:
        colors: '${ .colors + [ "blue" ] }'
      then: end
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"color": "red"})).await.unwrap();
        assert_eq!(output["colors"], json!(["red"]));
    }

    #[tokio::test]
    async fn test_e2e_switch_match_green() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: default
  name: switch-match
  version: '1.0.0'
do:
  - switchColor:
      switch:
        - red:
            when: '.color == "red"'
            then: setRed
        - green:
            when: '.color == "green"'
            then: setGreen
        - blue:
            when: '.color == "blue"'
            then: setBlue
  - setRed:
      set:
        colors: '${ .colors + [ "red" ] }'
      then: end
  - setGreen:
      set:
        colors: '${ .colors + [ "green" ] }'
      then: end
  - setBlue:
      set:
        colors: '${ .colors + [ "blue" ] }'
      then: end
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"color": "green"})).await.unwrap();
        assert_eq!(output["colors"], json!(["green"]));
    }

    // --- Go SDK: switch_with_default.yaml ---
    // Switch with fallback case (no `when`)
    #[tokio::test]
    async fn test_e2e_switch_with_default() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: default
  name: switch-with-default
  version: '1.0.0'
do:
  - switchColor:
      switch:
        - red:
            when: '.color == "red"'
            then: setRed
        - green:
            when: '.color == "green"'
            then: setGreen
        - fallback:
            then: setDefault
  - setRed:
      set:
        colors: '${ .colors + [ "red" ] }'
      then: end
  - setGreen:
      set:
        colors: '${ .colors + [ "green" ] }'
      then: end
  - setDefault:
      set:
        colors: '${ .colors + [ "default" ] }'
      then: end
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        // "yellow" doesn't match red or green, so fallback is used
        let output = runner.run(json!({"color": "yellow"})).await.unwrap();
        assert_eq!(output["colors"], json!(["default"]));
    }

    // --- Java SDK: switch-then-string.yaml ---
    // Switch with then:goto + then:exit pattern
    #[tokio::test]
    async fn test_e2e_switch_then_string_electronic() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch
  version: '0.1.0'
do:
  - processOrder:
      switch:
        - case1:
            when: .orderType == "electronic"
            then: processElectronicOrder
        - case2:
            when: .orderType == "physical"
            then: processPhysicalOrder
        - default:
            then: handleUnknownOrderType
  - processElectronicOrder:
      set:
        validate: true
        status: fulfilled
      then: exit
  - processPhysicalOrder:
      set:
        inventory: clear
        items: 1
        address: Elmer St
      then: exit
  - handleUnknownOrderType:
      set:
        log: warn
        message: "something's wrong"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner
            .run(json!({"orderType": "electronic"}))
            .await
            .unwrap();
        assert_eq!(output["validate"], json!(true));
        assert_eq!(output["status"], json!("fulfilled"));
    }

    #[tokio::test]
    async fn test_e2e_switch_then_string_physical() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch
  version: '0.1.0'
do:
  - processOrder:
      switch:
        - case1:
            when: .orderType == "electronic"
            then: processElectronicOrder
        - case2:
            when: .orderType == "physical"
            then: processPhysicalOrder
        - default:
            then: handleUnknownOrderType
  - processElectronicOrder:
      set:
        validate: true
        status: fulfilled
      then: exit
  - processPhysicalOrder:
      set:
        inventory: clear
        items: 1
        address: Elmer St
      then: exit
  - handleUnknownOrderType:
      set:
        log: warn
        message: "something's wrong"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"orderType": "physical"})).await.unwrap();
        assert_eq!(output["inventory"], json!("clear"));
        assert_eq!(output["items"], json!(1));
        assert_eq!(output["address"], json!("Elmer St"));
    }

    #[tokio::test]
    async fn test_e2e_switch_then_string_unknown() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch
  version: '0.1.0'
do:
  - processOrder:
      switch:
        - case1:
            when: .orderType == "electronic"
            then: processElectronicOrder
        - case2:
            when: .orderType == "physical"
            then: processPhysicalOrder
        - default:
            then: handleUnknownOrderType
  - processElectronicOrder:
      set:
        validate: true
        status: fulfilled
      then: exit
  - processPhysicalOrder:
      set:
        inventory: clear
        items: 1
        address: Elmer St
      then: exit
  - handleUnknownOrderType:
      set:
        log: warn
        message: "something's wrong"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"orderType": "digital"})).await.unwrap();
        assert_eq!(output["log"], json!("warn"));
        assert_eq!(output["message"], json!("something's wrong"));
    }

    // --- Java SDK: switch-then-loop.yaml ---
    // Switch creating a loop: then:inc for count<6
    #[tokio::test]
    async fn test_e2e_switch_then_loop() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: switch-loop
  version: '0.1.0'
do:
  - inc:
      set:
        count: "${ .count + 1 }"
      then: looping
  - looping:
      switch:
        - loopCount:
            when: .count < 6
            then: inc
        - default:
            then: exit
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"count": 0})).await.unwrap();
        // Loop: count goes 0→1→2→3→4→5→6, then exit when count=6 (not <6)
        assert_eq!(output["count"], json!(6));
    }
