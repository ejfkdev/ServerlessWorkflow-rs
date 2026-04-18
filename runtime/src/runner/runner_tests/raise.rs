use super::*;

    #[tokio::test]
    async fn test_runner_raise_inline() {
        let result = run_workflow_from_yaml(&testdata("raise_inline.yaml"), json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "validation");
        assert_eq!(err.status(), Some(&json!(400)));
        assert_eq!(err.title(), Some("Validation Error"));
        assert_eq!(err.detail(), Some("Invalid input provided to workflow"));
        // Instance should reflect the task reference path
        assert!(
            err.instance().is_some(),
            "raise error should have instance set"
        );
        assert!(
            err.instance().unwrap().contains("inlineError"),
            "instance should contain task name"
        );
    }

    // === Raise Reusable Error (reference) ===

    #[tokio::test]
    async fn test_runner_raise_reusable() {
        let result = run_workflow_from_yaml(&testdata("raise_reusable.yaml"), json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "authentication");
        assert_eq!(err.status(), Some(&json!(401)));
        assert_eq!(err.title(), Some("Authentication Error"));
        assert_eq!(err.detail(), Some("User is not authenticated"));
    }

    // === Raise Conditional Error ===

    #[tokio::test]
    async fn test_runner_raise_conditional() {
        let result = run_workflow_from_yaml(
            &testdata("raise_conditional.yaml"),
            json!({"user": {"age": 16}}),
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "authorization");
        assert_eq!(err.status(), Some(&json!(403)));
        assert_eq!(err.title(), Some("Authorization Error"));
        assert_eq!(err.detail(), Some("User is under the required age"));
    }

    // === Raise Conditional - no error (age >= 18) ===

    #[tokio::test]
    async fn test_runner_raise_conditional_no_error() {
        let output = run_workflow_from_yaml(
            &testdata("raise_conditional.yaml"),
            json!({"user": {"age": 25}}),
        )
        .await
        .unwrap();
        assert_eq!(output["message"], json!("User is allowed"));
    }

    // === Switch Match ===

    #[tokio::test]
    async fn test_runner_raise_error_with_input() {
        let result = run_workflow_from_yaml(
            &testdata("raise_error_with_input.yaml"),
            json!({"reason": "User token expired"}),
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "authentication");
        assert_eq!(err.status(), Some(&json!(401)));
        assert_eq!(err.title(), Some("Authentication Error"));
        assert_eq!(
            err.detail(),
            Some("User authentication failed: User token expired")
        );
    }

    // === Raise Undefined Reference ===

    #[tokio::test]
    async fn test_runner_raise_undefined_reference() {
        let result =
            run_workflow_from_yaml(&testdata("raise_undefined_reference.yaml"), json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Undefined error reference should produce a validation error
        assert_eq!(err.error_type_short(), "validation");
    }

    // === Workflow Input Schema ===

    #[tokio::test]
    async fn test_runner_raise_with_detail() {
        let result = run_workflow_from_yaml(&testdata("raise_with_detail.yaml"), json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "validation");
        assert_eq!(err.status(), Some(&json!(400)));
        assert_eq!(err.title(), Some("Validation Error"));
        assert_eq!(err.detail(), Some("Missing required field: email"));
    }

    #[tokio::test]
    async fn test_runner_raise_detail_with_workflow_context() {
        // Tests that raise detail expressions can reference $workflow variables
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-workflow-ctx
  version: '0.2.0'
do:
  - notImplemented:
      raise:
        error:
          type: https://serverlessworkflow.io/errors/not-implemented
          status: 500
          title: Not Implemented
          detail: '${ "The workflow " + $workflow.definition.document.name + " is not implemented" }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "not-implemented");
        assert_eq!(err.status(), Some(&json!(500)));
        assert_eq!(
            err.detail(),
            Some("The workflow raise-workflow-ctx is not implemented")
        );
    }

    // === Fork: Compete Mode ===

    #[tokio::test]
    async fn test_runner_raise_with_instance() {
        let result = run_workflow_from_yaml(&testdata("raise_with_instance.yaml"), json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "compliance");
        assert_eq!(err.status(), Some(&json!(400)));
        assert_eq!(err.title(), Some("Compliance Error"));
        assert_eq!(err.instance(), Some("raiseError"));
    }

    // === Emit with data expression interpolation ===

    #[tokio::test]
    async fn test_runner_raise_validation_type() {
        let result = run_workflow_from_yaml(
            &testdata("raise_conditional.yaml"),
            json!({"user": {"age": 16}}),
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "authorization");
    }

    // === Nested do: deep nesting ===

    #[tokio::test]
    async fn test_runner_raise_dynamic_title() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-dynamic-title
  version: '0.1.0'
do:
  - failTask:
      raise:
        error:
          type: validation
          title: '${ "Validation failed for field: " + .field }'
          status: 400
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({"field": "email"})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "validation");
        assert_eq!(err.title(), Some("Validation failed for field: email"));
        assert_eq!(err.status(), Some(&json!(400)));
    }

    // === Multiple sequential exports building up context ===

    #[tokio::test]
    async fn test_runner_raise_status_only() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-status-only
  version: '0.1.0'
do:
  - failTask:
      raise:
        error:
          type: authentication
          title: Unauthorized
          status: 401
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "authentication");
        assert_eq!(err.status(), Some(&json!(401)));
    }

    // === Try-catch: catch with only error type (no status, no instance) ===

    #[tokio::test]
    async fn test_runner_raise_detail_from_input() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-detail-input
  version: '0.1.0'
do:
  - validateAge:
      switch:
        - tooYoung:
            when: ${ .age < 18 }
            then: failYoung
        - validCase:
            then: end
  - failYoung:
      raise:
        error:
          type: validation
          title: Age Check Failed
          status: 400
          detail: '${ "User age " + (.age | tostring) + " is below minimum of 18" }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({"age": 15})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "validation");
        assert_eq!(err.detail(), Some("User age 15 is below minimum of 18"));
    }

    // === Set: if condition skipping task ===

    #[tokio::test]
    async fn test_runner_raise_with_all_fields() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-all-fields
  version: '0.1.0'
do:
  - failTask:
      raise:
        error:
          type: compliance
          title: '${ "Policy violation: " + .policy }'
          status: 403
          detail: '${ "User " + .user + " violated " + .policy }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner
            .run(json!({"user": "alice", "policy": "data-access"}))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "compliance");
        assert_eq!(err.status(), Some(&json!(403)));
        assert_eq!(err.title(), Some("Policy violation: data-access"));
        assert_eq!(err.detail(), Some("User alice violated data-access"));
    }

    // === Expression: add/merge objects ===

    #[tokio::test]
    async fn test_runner_sequential_raises_first_caught() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: sequential-raises
  version: '0.1.0'
do:
  - firstTry:
      try:
        - raiseFirst:
            raise:
              error:
                type: validation
                title: First Error
                status: 400
      catch:
        errors:
          with:
            type: validation
        do:
          - handleFirst:
              set:
                firstCaught: true
  - secondTry:
      raise:
        error:
          type: communication
          title: Second Error
          status: 500
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({})).await;
        // First error is caught, second is not
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "communication");
    }

    // === Export with object merge ($context + new data) ===

    #[tokio::test]
    async fn test_runner_raise_all_error_types() {
        // Test each error type category
        let error_types = vec![
            ("validation", 400),
            ("runtime", 500),
            ("communication", 503),
            ("security", 401),
            ("timeout", 408),
        ];

        for (err_type, status) in error_types {
            let yaml_str = format!(
                r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-{}
  version: '0.1.0'
do:
  - failStep:
      raise:
        error:
          type: {}
          title: Test Error
          status: {}
"#,
                err_type, err_type, status
            );
            let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
            let runner = WorkflowRunner::new(workflow).unwrap();

            let result = runner.run(json!({})).await;
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(
                err.error_type_short(),
                err_type,
                "Expected error type {} but got {}",
                err_type,
                err.error_type_short()
            );
        }
    }

    // === Expression: object construction from array ===

    #[tokio::test]
    async fn test_runner_raise_detail_from_context() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-detail-ctx
  version: '0.1.0'
do:
  - guarded:
      try:
        - failStep:
            raise:
              error:
                type: validation
                title: Validation Failed
                status: 400
                detail: "${ .field } is required"
      catch:
        errors:
          with:
            type: validation
        as: err
        do:
          - handleErr:
              set:
                errType: "${ $err.type }"
                errTitle: "${ $err.title }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({"field": "email"})).await.unwrap();
        assert!(output["errType"].as_str().unwrap().contains("validation"));
        assert_eq!(output["errTitle"], json!("Validation Failed"));
    }

    // === Fork: single branch behaves like sequential ===

    #[tokio::test]
    async fn test_runner_raise_all_types() {
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-all-types
  version: '0.1.0'
use:
  errors:
    validationError:
      type: validation
      title: Validation Error
      status: 400
    timeoutError:
      type: timeout
      title: Timeout Error
      status: 408
do:
  - step1:
      try:
        - failStep:
            raise:
              error: validationError
      catch:
        errors:
          with:
            type: validation
        as: err
        do:
          - handleErr:
              set:
                caughtType: "${ $err.type }"
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert!(output["caughtType"]
            .as_str()
            .unwrap()
            .contains("validation"));
    }

    // === Set: multiple fields in single set ===

    // Raise: error reference from use.errors (reuse existing testdata)
    #[tokio::test]
    async fn test_runner_raise_use_errors_ref() {
        let result = run_workflow_from_yaml(&testdata("raise_reusable.yaml"), json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "authentication");
    }

    #[tokio::test]
    async fn test_runner_raise_reusable_with_workflow_context() {
        // Matches Java SDK's raise-reusable.yaml
        // Error defined in use.errors with $workflow.definition.document.name and version in detail
        // Uses + concatenation instead of \() interpolation for YAML compatibility
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-not-implemented-reusable
  version: '0.1.0'
use:
  errors:
    notImplemented:
      type: https://serverlessworkflow.io/errors/not-implemented
      status: 500
      title: Not Implemented
      detail: '${ "The workflow " + $workflow.definition.document.name + ":" + $workflow.definition.document.version + " is a work in progress" }'
do:
  - notImplemented:
      raise:
        error: notImplemented
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.error_type().contains("not-implemented"));
        // Detail should contain workflow name and version from $workflow context
        assert!(err.to_string().contains("raise-not-implemented-reusable"));
        assert!(err.to_string().contains("0.1.0"));
    }

    #[tokio::test]
    async fn test_runner_raise_with_instance_field() {
        // Matches Java SDK's raise.yaml - raise with explicit instance field
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-custom-error
  version: '1.0.0'
do:
  - raiseError:
      raise:
        error:
          status: 400
          type: https://serverlessworkflow.io/errors/types/compliance
          title: Compliance Error
          instance: raiseError
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.error_type().contains("compliance"));
        assert_eq!(err.instance(), Some("raiseError"));
        assert_eq!(err.status(), Some(&json!(400)));
    }

    #[tokio::test]
    async fn test_runner_raise_inline_full_workflow_context() {
        // Matches Java SDK's raise-inline.yaml with full $workflow context including version
        // Uses + concatenation instead of \() interpolation for YAML compatibility
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-not-implemented
  version: '0.1.0'
do:
  - notImplemented:
      raise:
        error:
          type: https://serverlessworkflow.io/errors/not-implemented
          status: 500
          title: Not Implemented
          detail: '${ "The workflow " + $workflow.definition.document.name + ":" + $workflow.definition.document.version + " is a work in progress" }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.error_type().contains("not-implemented"));
        // Detail should contain both workflow name and version
        assert!(err.to_string().contains("raise-not-implemented"));
        assert!(err.to_string().contains("0.1.0"));
    }

    #[tokio::test]
    async fn test_runner_raise_full_error_all_fields() {
        // Raise error with all fields: type, title, status, detail, instance
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-full-error
  version: '0.1.0'
do:
  - failTask:
      raise:
        error:
          type: authentication
          title: Auth Failed
          status: 401
          detail: 'Invalid credentials provided'
          instance: '/auth/login'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "authentication");
        assert_eq!(err.status(), Some(&json!(401)));
        assert_eq!(err.title(), Some("Auth Failed"));
        assert_eq!(err.detail(), Some("Invalid credentials provided"));
        assert_eq!(err.instance(), Some("/auth/login"));
    }

    // === Set with object merge (using + operator) ===

    #[tokio::test]
    async fn test_runner_raise_error_with_input_go_pattern() {
        // Go SDK's raise_error_with_input.yaml - raise with dynamic detail from input
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-with-input
  version: '1.0.0'
do:
  - dynamicError:
      raise:
        error:
          type: https://serverlessworkflow.io/spec/1.0.0/errors/authentication
          status: 401
          title: Authentication Error
          detail: '${ "User authentication failed: " + .reason }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let result = runner.run(json!({"reason": "User token expired"})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "authentication");
        assert!(err
            .detail()
            .unwrap()
            .contains("User authentication failed: User token expired"));
    }

    // === Conditional logic with input.from ===

    #[tokio::test]
    async fn test_runner_raise_conditional_go_pattern() {
        // Go SDK's raise_conditional.yaml - raise with if condition
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-conditional
  version: '1.0.0'
do:
  - checkAge:
      if: '${ .user.age < 18 }'
      raise:
        error:
          type: https://serverlessworkflow.io/spec/1.0.0/errors/authorization
          status: 403
          title: Authorization Error
          detail: User is under the required age
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let result = runner.run(json!({"user": {"age": 16}})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "authorization");
        assert_eq!(err.detail().unwrap(), "User is under the required age");
    }

    // === Raise conditional skipped ===

    #[tokio::test]
    async fn test_runner_raise_conditional_skipped_go_pattern() {
        // Go SDK's raise_conditional.yaml - raise skipped when condition is false
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-conditional-skip
  version: '1.0.0'
do:
  - checkAge:
      if: '${ .user.age < 18 }'
      raise:
        error:
          type: https://serverlessworkflow.io/spec/1.0.0/errors/authorization
          status: 403
          detail: User is under the required age
  - proceed:
      set:
        allowed: true
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"user": {"age": 25}})).await.unwrap();
        assert_eq!(output["allowed"], json!(true));
    }

    // === Fork simple non-compete ===

    /// Test raise error with input-driven detail — Go SDK's raise_error_with_input.yaml
    /// Raise with detail that references workflow input data via expression
    #[tokio::test]
    async fn test_runner_raise_error_with_input_driven_detail() {
        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-with-input
  version: '1.0.0'
do:
  - dynamicError:
      raise:
        error:
          type: https://serverlessworkflow.io/spec/1.0.0/errors/authentication
          status: 401
          title: Authentication Error
          detail: '${ "User authentication failed: " + .reason }'
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({"reason": "invalid token"})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.error_type_short(), "authentication");
        assert_eq!(err.status(), Some(&json!(401)));
        assert_eq!(err.title(), Some("Authentication Error"));
        let details = err.detail().unwrap_or("");
        assert!(
            details.contains("User authentication failed"),
            "detail should contain message, got: {}",
            details
        );
        assert!(
            details.contains("invalid token"),
            "detail should contain input reason, got: {}",
            details
        );
    }
