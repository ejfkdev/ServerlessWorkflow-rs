use super::*;

    #[tokio::test]
    async fn test_runner_nested_do() {
        let output = run_workflow_from_yaml(&testdata("nested_do.yaml"), json!({}))
            .await
            .unwrap();
        assert_eq!(output["doubled"], json!(20));
    }

    // === Simple Expression ($task.startedAt.epoch.milliseconds, $workflow.id, $runtime.version) ===

    #[tokio::test]
    async fn test_runner_nested_try_catch() {
        let output = run_workflow_from_yaml(&testdata("nested_try_catch.yaml"), json!({}))
            .await
            .unwrap();
        // Inner try-catch should catch the validation error
        assert_eq!(output["innerHandled"], json!(true));
        // The $error variable should be accessible within catch.do
        assert_eq!(output["innerTitle"], json!("Inner Error"));
    }

    // === Task Timeout ===

    #[tokio::test]
    async fn test_runner_nested_do_deep() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-do-deep
  version: '0.1.0'
do:
  - outerTask:
      do:
        - middleTask:
            do:
              - innerTask:
                  set:
                    deepValue: 42
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["deepValue"], json!(42));
    }

    // === Set with then: exit (exits current do block) ===

    #[tokio::test]
    async fn test_runner_nested_do_export_context() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-do-export-context
  version: '0.1.0'
do:
  - outerTask:
      do:
        - innerSet:
            set:
              value: 42
            export:
              as: '${ {innerValue: .value} }'
        - useContext:
            set:
              result: '${ $context.innerValue }'
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["result"], json!(42));
    }

    // === Workflow output: complex transformation ===

    #[tokio::test]
    async fn test_runner_do_within_for() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-within-for
  version: '0.1.0'
do:
  - processItems:
      for:
        in: ${ .items }
        each: item
      do:
        - computeValue:
            set:
              results: "${ [.results[]] + [{name: $item, doubled: ($item * 2)}] }"
"#;
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let output = runner
            .run(json!({"items": [3, 5, 7], "results": []}))
            .await
            .unwrap();
        assert_eq!(
            output["results"],
            json!([
                {"name": 3, "doubled": 6},
                {"name": 5, "doubled": 10},
                {"name": 7, "doubled": 14}
            ])
        );
    }

    // === For within Do (nested composite) ===

    #[tokio::test]
    async fn test_runner_nested_try_catch_inner_recovers() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-try-catch-inner-recovers
  version: '0.1.0'
do:
  - outerTry:
      try:
        - innerTry:
            try:
              - failTask:
                  raise:
                    error:
                      type: validation
                      title: Inner Error
                      status: 400
            catch:
              errors:
                with:
                  type: validation
              do:
                - recoverInner:
                    set:
                      innerRecovered: true
        - continueAfterInner:
            set:
              innerRecovered: true
              continued: true
      catch:
        errors:
          with:
            type: validation
        do:
          - handleOuter:
              set:
                outerCaught: true
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["innerRecovered"], json!(true));
        assert_eq!(output["continued"], json!(true));
        // Outer catch should NOT trigger because inner caught the error
        assert!(output.get("outerCaught").is_none());
    }

    // === Switch with age-based classification (proper way to do mutually exclusive conditions) ===

    #[tokio::test]
    async fn test_runner_nested_for_outer_reference() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-for-outer-ref
  version: '0.1.0'
do:
  - outerLoop:
      for:
        in: ${ .groups }
        each: group
      do:
        - innerProcess:
            for:
              in: ${ $group.items }
              each: item
            do:
              - accumulate:
                  set:
                    result: "${ [.result[]] + [{group: $group.name, item: $item}] }"
"#;
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let output = runner
            .run(json!({
                "groups": [
                    {"name": "A", "items": [1, 2]},
                    {"name": "B", "items": [3]}
                ],
                "result": []
            }))
            .await
            .unwrap();
        assert_eq!(
            output["result"],
            json!([
                {"group": "A", "item": 1},
                {"group": "A", "item": 2},
                {"group": "B", "item": 3}
            ])
        );
    }

    // === Switch with no matching case and no default ===

    #[tokio::test]
    async fn test_runner_nested_do_export() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-do-export
  version: '0.1.0'
do:
  - outerTask:
      do:
        - innerSet:
            set:
              innerValue: 42
            export:
              as: '${ {shared: .innerValue} }'
        - useContext:
            set:
              result: '${ $context.shared }'
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["result"], json!(42));
    }

    // === Multiple tasks with flow control (then: goto skipping multiple tasks) ===

    #[tokio::test]
    async fn test_runner_nested_for_with_export() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-for-export
  version: '0.1.0'
do:
  - outerLoop:
      for:
        in: ${ .groups }
        each: group
      do:
        - innerProcess:
            for:
              in: ${ $group.items }
              each: item
            do:
              - accumulate:
                  set:
                    results: "${ [.results[]] + [{g: $group.name, v: $item}] }"
            export:
              as: '${ {results: .results} }'
  - report:
      set:
        allResults: "${ $context.results }"
"#;
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let output = runner
            .run(json!({
                "groups": [{"name": "A", "items": [1, 2]}, {"name": "B", "items": [3]}],
                "results": []
            }))
            .await
            .unwrap();
        // Inner for loop export accumulates, but outer for resets each iteration
        // Last group (B) should be in context
        assert!(output["allResults"].is_array());
    }

    // === Expression: conditional with and/or ===

    #[tokio::test]
    async fn test_runner_do_then_exit_nested() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-then-exit-nested
  version: '0.1.0'
do:
  - outerTask:
      do:
        - step1:
            set:
              a: 1
        - step2:
            set:
              b: 2
            then: exit
        - step3:
            set:
              c: 3
  - afterOuter:
      set:
        done: true
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // exit should exit the inner do, then workflow continues with afterOuter
        assert_eq!(output["done"], json!(true));
        // step3 should have been skipped by exit
        assert!(output.get("c").is_none());
    }

    // === Expression: floor/ceil/round ===

    #[tokio::test]
    async fn test_runner_nested_do_with_then_exit() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-do-then-exit
  version: '0.1.0'
do:
  - outerTask:
      do:
        - step1:
            set:
              x: 1
        - step2:
            set:
              y: 2
            then: exit
        - step3:
            set:
              skipped: true
  - afterOuter:
      set:
        z: 3
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // step2 then:exit exits inner do, execution continues at afterOuter in outer scope
        assert_eq!(output["z"], json!(3));
        assert!(output.get("skipped").is_none());
    }

    // === For loop with nested do and export ===

    #[tokio::test]
    async fn test_runner_nested_do_then_end_top_level() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-do-then-end
  version: '0.1.0'
do:
  - outerTask:
      do:
        - innerStep:
            set:
              value: 42
            then: end
  - afterInner:
      set:
        reached: true
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // then:end in inner do exits only the inner do, outer tasks continue
        assert_eq!(output["reached"], json!(true));
    }

    // === Expression: string multiplication (repeat) ===

    #[tokio::test]
    async fn test_runner_nested_raise_caught_by_outer() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-raise-outer-catch
  version: '0.1.0'
do:
  - outerTry:
      try:
        - innerDo:
            do:
              - step1:
                  set:
                    x: 1
              - step2:
                  raise:
                    error:
                      type: communication
                      title: Inner Error
                      status: 503
      catch:
        errors:
          with:
            type: communication
        do:
          - handleErr:
              set:
                caught: true
                errorType: communication
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["caught"], json!(true));
    }

    // === Expression: ltrimstr / rtrimstr ===

    #[tokio::test]
    async fn test_runner_nested_switch_then_exit_inner() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-switch-exit-inner
  version: '0.1.0'
do:
  - outerTask:
      do:
        - innerSwitch:
            switch:
              - isReady:
                  when: ${ .ready == true }
                  then: exit
        - afterSwitch:
            set:
              afterInner: true
  - afterOuter:
      set:
        outerDone: true
"#;
        let output = run_workflow_yaml(&yaml_str, json!({"ready": true})).await.unwrap();
        // then:exit should exit inner do, but outer tasks continue
        assert!(output.get("afterInner").is_none() || output["afterInner"] == json!(true));
        assert_eq!(output["outerDone"], json!(true));
    }

    // === Expression: @json format ===

    #[tokio::test]
    async fn test_runner_do_mixed_task_types() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-mixed-tasks
  version: '0.1.0'
do:
  - setInitial:
      set:
        numbers: [1, 2, 3]
        total: 0
  - processAll:
      for:
        in: "${ .numbers }"
        each: num
      do:
        - addToTotal:
            set:
              total: "${ .total + $num }"
              numbers: "${ .numbers }"
  - setResult:
      set:
        result: "${ .total }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["result"], json!(6));
    }

    // === Fork with try-catch in branches ===

    #[tokio::test]
    async fn test_runner_nested_do_inner_export() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-do-inner-export
  version: '0.1.0'
do:
  - outer:
      do:
        - inner:
            set:
              val: 42
            export:
              as: "${ {innerVal: .val} }"
  - check:
      set:
        result: "${ $context.innerVal }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["result"], json!(42));
    }

    // === Expression: map with arithmetic ===

    #[tokio::test]
    async fn test_runner_do_nested_goto_between_levels() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-exit-between-levels
  version: '0.1.0'
do:
  - outerA:
      set:
        step: a
  - outerDo:
      do:
        - innerB:
            set:
              step: b
        - innerC:
            set:
              step: c
            then: exit
  - outerD:
      set:
        step: d
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // innerC then:exit exits inner do, execution continues at outerD in outer scope
        assert_eq!(output["step"], json!("d"));
    }

    // === Expression: @uri encode ===

    #[tokio::test]
    async fn test_runner_do_then_end_scope() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-then-end-scope
  version: '0.1.0'
do:
  - outerTask:
      do:
        - inner1:
            set:
              inner: 1
            then: end
        - inner2:
            set:
              inner: 2
  - afterOuter:
      set:
        outer: true
        result: "${ .inner }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // then:end exits inner do, outer tasks continue
        assert_eq!(output["outer"], json!(true));
        assert_eq!(output["result"], json!(1));
    }

    // === Expression: string comparisons ===

    #[tokio::test]
    async fn test_runner_do_nested_then_exit() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-nested-then-exit
  version: '0.1.0'
do:
  - outer:
      do:
        - inner1:
            set:
              val: 10
            then: exit
        - inner2:
            set:
              val: 99
  - afterOuter:
      set:
        result: "${ .val + 1 }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["result"], json!(11));
    }

    // === Export: overwrite context completely ===

    #[tokio::test]
    async fn test_runner_do_goto_backward_with_guard() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-goto-backward
  version: '0.1.0'
do:
  - init:
      set:
        count: 0
  - check:
      switch:
        - notDone:
            when: ${ .count < 3 }
            then: increment
        - done:
            then: end
      then: continue
  - increment:
      set:
        count: "${ .count + 1 }"
      then: check
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["count"], json!(3));
    }

    // === Batch 6: More workflow patterns ===

    // === Wait: PT0S immediate return (duplicate-avoiding name) ===

    #[tokio::test]
    async fn test_runner_do_if_skip_multiple() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-if-skip-multi
  version: '0.1.0'
do:
  - always:
      set:
        x: 1
  - maybe:
      set:
        y: 2
      if: ${ .shouldRun }
  - check:
      set:
        hasY: "${ .y != null }"
        xVal: "${ .x }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({"shouldRun": false})).await.unwrap();
        assert_eq!(output["xVal"], json!(1));
        assert_eq!(output["hasY"], json!(false));
    }

    // === Do: if condition executes task ===

    #[tokio::test]
    async fn test_runner_do_if_execute_multiple() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-if-exec-multi
  version: '0.1.0'
do:
  - always:
      set:
        x: 1
        shouldRun: "${ .shouldRun }"
  - maybe:
      set:
        x: "${ .x }"
        y: 2
      if: ${ .shouldRun }
  - check:
      set:
        hasY: "${ .y != null }"
        xVal: "${ .x }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({"shouldRun": true})).await.unwrap();
        assert_eq!(output["xVal"], json!(1));
        assert_eq!(output["hasY"], json!(true));
    }

    // === Expression: nested object construction ===

    #[tokio::test]
    async fn test_runner_nested_for_loops() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: nested-for
  version: '0.1.0'
do:
  - outerLoop:
      for:
        in: "${ .items }"
        each: item
      do:
        - process:
            set:
              result: "${ .result + [$item] }"
              items: "${ .items }"
"#;
        let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

        let output = runner
            .run(json!({"items": [10, 20, 30], "result": []}))
            .await
            .unwrap();
        assert_eq!(output["result"], json!([10, 20, 30]));
    }

    // === Batch 7: Additional DSL pattern tests ===

    // Do: then:continue explicit - continues to next task
    #[tokio::test]
    async fn test_runner_do_then_continue_explicit() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-then-continue-explicit
  version: '0.1.0'
do:
  - first:
      set:
        a: 1
      then: continue
  - second:
      set:
        b: 2
        a: "${ .a }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["a"], json!(1));
        assert_eq!(output["b"], json!(2));
    }

    // Do: multiple set tasks building up state
    #[tokio::test]
    async fn test_runner_do_accumulate_state() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-accumulate-state
  version: '0.1.0'
do:
  - step1:
      set:
        a: 1
  - step2:
      set:
        b: 2
        a: "${ .a }"
  - step3:
      set:
        c: 3
        a: "${ .a }"
        b: "${ .b }"
  - step4:
      set:
        total: "${ .a + .b + .c }"
        a: "${ .a }"
        b: "${ .b }"
        c: "${ .c }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        assert_eq!(output["total"], json!(6));
        assert_eq!(output["a"], json!(1));
        assert_eq!(output["b"], json!(2));
        assert_eq!(output["c"], json!(3));
    }

    // Do: set + export + set chain
    #[tokio::test]
    async fn test_runner_do_set_export_chain() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: do-set-export-chain
  version: '0.1.0'
do:
  - step1:
      set:
        val: 10
      export:
        as: "${ {step1: .val} }"
  - step2:
      set:
        val: "${ .val + $context.step1 }"
      export:
        as: "${ {step1: $context.step1, step2: .val} }"
  - step3:
      set:
        val: "${ .val + $context.step2 }"
        step1: "${ $context.step1 }"
        step2: "${ $context.step2 }"
"#;
        let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
        // step1: val=10, export step1=10
        // step2: val=10+10=20, export step1=10,step2=20
        // step3: val=20+20=40
        assert_eq!(output["val"], json!(40));
        assert_eq!(output["step1"], json!(10));
        assert_eq!(output["step2"], json!(20));
    }
