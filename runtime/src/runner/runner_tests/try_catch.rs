use super::*;

#[tokio::test]
async fn test_runner_try_catch_match_status() {
    let output = run_workflow_from_yaml(&testdata("try_catch_match_status.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["recovered"], json!(true));
}

// === Try-Catch: match by details ===

#[tokio::test]
async fn test_runner_try_catch_match_details() {
    let output = run_workflow_from_yaml(&testdata("try_catch_match_details.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["recovered"], json!(true));
}

// === Try-Catch: error variable (catch.as) ===

#[tokio::test]
async fn test_runner_try_catch_error_variable() {
    let output = run_workflow_from_yaml(&testdata("try_catch_error_variable.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["errorMessage"], json!("Javierito was here!"));
}

// === Try-Catch: match by when condition ===

#[tokio::test]
async fn test_runner_try_catch_match_when() {
    let output = run_workflow_from_yaml(&testdata("try_catch_match_when.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["recovered"], json!(true));
}

// === Try-Catch: not match by status ===

#[tokio::test]
async fn test_runner_try_catch_not_match_status() {
    let result =
        run_workflow_from_yaml(&testdata("try_catch_not_match_status.yaml"), json!({})).await;
    assert!(result.is_err());
}

// === Try-Catch: not match by details ===

#[tokio::test]
async fn test_runner_try_catch_not_match_details() {
    let result =
        run_workflow_from_yaml(&testdata("try_catch_not_match_details.yaml"), json!({})).await;
    assert!(result.is_err());
}

// === Try-Catch: not match by when condition ===

#[tokio::test]
async fn test_runner_try_catch_not_match_when() {
    let result =
        run_workflow_from_yaml(&testdata("try_catch_not_match_when.yaml"), json!({})).await;
    assert!(result.is_err());
}

// === For Loop: collect with input.from ===

#[tokio::test]
async fn test_runner_try_catch_retry_inline() {
    let result = run_workflow_from_yaml(&testdata("try_catch_retry_inline.yaml"), json!({})).await;
    // Retry exhausts all attempts, error propagates
    assert!(result.is_err());
}

// === $task expression variable ===

#[tokio::test]
async fn test_runner_try_catch_retry_success() {
    let output = run_workflow_from_yaml(&testdata("try_catch_retry_success.yaml"), json!({}))
        .await
        .unwrap();
    // Try task succeeds on first attempt, retry not triggered
    assert_eq!(output["result"], json!("success"));
}

// === Try-Catch: Retry reusable reference ===

#[tokio::test]
async fn test_runner_try_catch_retry_reusable() {
    let result =
        run_workflow_from_yaml(&testdata("try_catch_retry_reusable.yaml"), json!({})).await;
    // Retry exhausts all attempts, error propagates
    assert!(result.is_err());
}

// === Secret Expression ($secret variable) ===

#[tokio::test]
async fn test_runner_try_catch_compensate() {
    let output = run_workflow_from_yaml(&testdata("try_catch_compensate.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["compensated"], json!(true));
    assert_eq!(output["originalTask"], json!("failTask"));
}

// === Try-Catch: exceptWhen skips catch ===

#[tokio::test]
async fn test_runner_try_catch_except_when_skips() {
    // exceptWhen evaluates against error context
    // .details == "Expected error" != "Skip this error" -> should NOT skip -> catch applies
    let output = run_workflow_from_yaml(&testdata("try_catch_except_when.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["handled"], json!(true));
}

// === Nested Do Tasks ===

#[tokio::test]
async fn test_runner_try_catch_retry_inline_exponential() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use warp::Reply;

    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_clone = attempt_count.clone();

    // Endpoint that fails twice then succeeds
    let endpoint = warp::path::end().and(warp::get()).map(move || {
        let count = attempt_clone.fetch_add(1, Ordering::SeqCst);
        if count < 2 {
            warp::reply::with_status(
                warp::reply::json(&serde_json::json!({"error": "not found"})),
                warp::http::StatusCode::NOT_FOUND,
            )
            .into_response()
        } else {
            warp::reply::json(&serde_json::json!({"status": "ok"})).into_response()
        }
    });

    let port = start_mock_server(endpoint);

    let yaml = format!(
        r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-retry-inline
  version: '0.1.0'
do:
  - tryGet:
      try:
        - getPet:
            call: http
            with:
              method: get
              endpoint: http://localhost:{port}
      catch:
        errors:
          with:
            type: https://serverlessworkflow.io/spec/1.0.0/errors/communication
        retry:
          delay: PT0.01S
          backoff:
            exponential: {{}}
          limit:
            attempt:
              count: 5
"#
    );
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
    let runner = WorkflowRunner::new(workflow).unwrap();

    let output = runner.run(json!({})).await.unwrap();
    assert_eq!(output["status"], json!("ok"));
    // Should have 3 attempts: 2 failures + 1 success
    let attempts = attempt_count.load(Ordering::SeqCst);
    assert!(
        attempts >= 3,
        "Expected at least 3 attempts, got {}",
        attempts
    );
}

// === Try-Catch-Retry: reusable constant backoff ===

#[tokio::test]
async fn test_runner_try_catch_retry_reusable_constant() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use warp::Reply;

    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_clone = attempt_count.clone();

    let endpoint = warp::path::end().and(warp::get()).map(move || {
        let count = attempt_clone.fetch_add(1, Ordering::SeqCst);
        if count < 1 {
            warp::reply::with_status(
                warp::reply::json(&serde_json::json!({"error": "not found"})),
                warp::http::StatusCode::NOT_FOUND,
            )
            .into_response()
        } else {
            warp::reply::json(&serde_json::json!({"result": "success"})).into_response()
        }
    });

    let port = start_mock_server(endpoint);

    let yaml = format!(
        r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-retry-reusable
  version: '0.1.0'
use:
  retries:
    default:
      delay:
        milliseconds: 10
      backoff:
        constant: {{}}
      limit:
        attempt:
          count: 5
do:
  - tryGet:
      try:
        - getPet:
            call: http
            with:
              method: get
              endpoint: http://localhost:{port}
      catch:
        errors:
          with:
            type: https://serverlessworkflow.io/spec/1.0.0/errors/communication
        retry: default
"#
    );
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
    let runner = WorkflowRunner::new(workflow).unwrap();

    let output = runner.run(json!({})).await.unwrap();
    assert_eq!(output["result"], json!("success"));
    let attempts = attempt_count.load(Ordering::SeqCst);
    assert!(
        attempts >= 2,
        "Expected at least 2 attempts, got {}",
        attempts
    );
}

// === Sub-workflow: output.as + export.as pattern (Java SDK) ===

#[tokio::test]
async fn test_runner_try_catch_communication_error() {
    // Server that returns 404 for any path
    let not_found = warp::any()
        .map(|| warp::reply::with_status("Not Found", warp::http::StatusCode::NOT_FOUND));

    let output =
        run_workflow_with_mock_server("try_catch_communication_error.yaml", not_found, json!({}))
            .await;
    assert_eq!(output["recovered"], json!(true));
}

// === For Loop: custom at variable ===

#[tokio::test]
async fn test_runner_try_catch_by_status() {
    let not_found = warp::path("api")
        .and(warp::path("items"))
        .map(|| warp::reply::with_status("Not Found", warp::http::StatusCode::NOT_FOUND));

    let port = start_mock_server(not_found);

    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-by-status
  version: '0.1.0'
do:
  - tryCall:
      try:
        - callFail:
            call: http
            with:
              method: get
              endpoint:
                uri: http://localhost:PORT/api/items
      catch:
        errors:
          with:
            type: communication
            status: 404
        do:
          - handleError:
              set:
                handled: true
"#
    .replace("PORT", &port.to_string());
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["handled"], json!(true));
}

// === HTTP Call: 404 returns input unchanged (non-error mode) ===

#[tokio::test]
async fn test_runner_try_catch_error_var_type() {
    let output = run_workflow_from_yaml(&testdata("try_catch_error_variable.yaml"), json!({}))
        .await
        .unwrap();
    // catch.as stores the error as a variable accessible in catch.do
    assert_eq!(output["errorMessage"], json!("Javierito was here!"));
}

// === Workflow input transformation with from ===

#[tokio::test]
async fn test_runner_try_catch_multiple_errors() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-multiple-errors
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: authentication
                status: 401
                title: Auth Failed
      catch:
        errors:
          with:
            type: authentication
        do:
          - handleAuth:
              set:
                caught: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["caught"], json!(true));
}

// === Try-Catch: error variable with details ===

#[tokio::test]
async fn test_runner_try_catch_error_with_details() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-error-details
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: validation
                title: Validation Error
                status: 400
      catch:
        errors:
          with:
            type: validation
        as: err
        do:
          - logError:
              set:
                errorTitle: '${ $err.title }'
                errorStatus: '${ $err.status }'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["errorTitle"], json!("Validation Error"));
    assert_eq!(output["errorStatus"], json!(400));
}

// === Switch: then goto with multiple jumps ===

#[tokio::test]
async fn test_runner_try_catch_filter_instance() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-filter-instance
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: compliance
                status: 400
                title: Compliance Error
                instance: raiseError
      catch:
        errors:
          with:
            instance: raiseError
        do:
          - handleInstance:
              set:
                caught: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["caught"], json!(true));
}

// === Try-Catch: error filter by instance — not matching ===

#[tokio::test]
async fn test_runner_try_catch_filter_instance_not_match() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-filter-instance-not-match
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: compliance
                status: 400
                title: Compliance Error
                instance: raiseError
      catch:
        errors:
          with:
            instance: differentInstance
        do:
          - handleInstance:
              set:
                caught: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let result = runner.run(json!({})).await;
    assert!(result.is_err());
}

// === Expression: has() function ===

#[tokio::test]
async fn test_runner_retry_success_on_retry() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    // Mock server: returns 404 for first 2 requests, then 200
    let handler = warp::path("pets").map(move || {
        let count = count_clone.fetch_add(1, Ordering::SeqCst);
        if count < 2 {
            warp::reply::with_status(
                warp::reply::json(&serde_json::json!({"error": "not found"})),
                warp::http::StatusCode::NOT_FOUND,
            )
        } else {
            warp::reply::with_status(
                warp::reply::json(&serde_json::json!({"name": "Buddy"})),
                warp::http::StatusCode::OK,
            )
        }
    });

    let port = start_mock_server(handler);

    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: retry-success
  version: '0.1.0'
do:
  - tryGetPet:
      try:
        - getPet:
            call: http
            with:
              method: get
              endpoint:
                uri: http://localhost:PORT/pets
      catch:
        errors:
          with:
            type: communication
        retry:
          delay:
            milliseconds: 10
          backoff:
            constant: {}
          limit:
            attempt:
              count: 5
"#
    .replace("PORT", &port.to_string());
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // After 2 retries, the 3rd request should succeed
    assert_eq!(output["name"], json!("Buddy"));
}

// === Shell: arguments with key-value format ===

#[tokio::test]
async fn test_runner_try_catch_except_when_match() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-except-when-match
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: validation
                status: 400
                title: Bad Request
      catch:
        errors:
          with:
            type: validation
        exceptWhen: ${ .status == 400 }
        do:
          - handleError:
              set:
                caught: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // exceptWhen matches status == 400, so catch should be SKIPPED → error propagates
    let result = runner.run(json!({})).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().error_type_short(), "validation");
}

// === Raise error with dynamic title from expression ===

#[tokio::test]
async fn test_runner_try_catch_retry_reference() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-retry-ref
  version: '0.1.0'
use:
  retries:
    myRetry:
      delay:
        milliseconds: 10
      limit:
        attempt:
          count: 2
do:
  - tryCall:
      try:
        - failTask:
            raise:
              error:
                type: runtime
                title: Fail
                status: 500
      catch:
        errors:
          with:
            type: runtime
        retry: myRetry
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // Retry exhausts all attempts
    let result = runner.run(json!({})).await;
    assert!(result.is_err());
}

// === Nested try-catch (inner catches, outer continues) ===

#[tokio::test]
async fn test_runner_try_catch_catch_do_modifies_input() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-modify
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: runtime
                title: Oops
                status: 500
      catch:
        errors:
          with:
            type: runtime
        do:
          - recover:
              set:
                originalName: "${ .name }"
                recovered: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "Alice", "age": 30}))
        .await
        .unwrap();
    assert_eq!(output["originalName"], json!("Alice"));
    assert_eq!(output["recovered"], json!(true));
}

// === Raise error with detail expression ===

#[tokio::test]
async fn test_runner_try_catch_type_only() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-type-only
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: validation
                title: Validation Error
                status: 400
      catch:
        errors:
          with:
            type: validation
        do:
          - handleError:
              set:
                caught: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["caught"], json!(true));
}

// === Nested export: export in nested do block ===

#[tokio::test]
async fn test_runner_try_catch_as_with_status() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-as-status
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: authentication
                title: Auth Error
                status: 401
      catch:
        errors:
          with:
            type: authentication
        as: err
        do:
          - handleError:
              set:
                errorStatus: '${ $err.status }'
                errorType: '${ $err.type }'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["errorStatus"], json!(401));
    assert_eq!(output["errorType"], json!("authentication"));
}

// === Set: multiple expressions in single task ===

#[tokio::test]
async fn test_runner_try_catch_error_details_var() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-error-details-var
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: validation
                title: Bad Request
                status: 400
                detail: Missing email field
      catch:
        errors:
          with:
            type: validation
        as: err
        do:
          - logError:
              set:
                errorTitle: '${ $err.title }'
                errorDetail: '${ $err.details }'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["errorTitle"], json!("Bad Request"));
    assert_eq!(output["errorDetail"], json!("Missing email field"));
}

// === For loop: while with custom at/each variables ===

#[tokio::test]
async fn test_runner_try_catch_when_filter_accepts() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-when-accepts
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: validation
                title: Bad
                status: 400
      catch:
        errors:
          with:
            type: validation
        when: ${ .status >= 400 }
        do:
          - handle:
              set:
                handled: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["handled"], json!(true));
}

#[tokio::test]
async fn test_runner_try_catch_when_filter_rejects() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-when-rejects
  version: '0.1.0'
do:
  - safeCall:
      try:
        - failTask:
            raise:
              error:
                type: validation
                title: Bad
                status: 400
      catch:
        errors:
          with:
            type: validation
        when: ${ .status >= 500 }
        do:
          - handle:
              set:
                handled: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // when condition doesn't match (400 < 500), so catch is skipped and error propagates
    let result = runner.run(json!({})).await;
    assert!(result.is_err());
}

// === Expression: alternative operator chaining ===

#[tokio::test]
async fn test_runner_try_catch_as_variable_in_catch_do() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-as-variable-in-catch-do
  version: '0.1.0'
do:
  - riskyTask:
      try:
        - failStep:
            raise:
              error:
                type: validation
                title: Bad Input
                status: 400
      catch:
        errors:
          with:
            type: validation
        as: err
        do:
          - handleError:
              set:
                errorType: "${ $err.type }"
                errorStatus: "${ $err.status }"
                recovered: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // $err.type contains the full URI or short name depending on error definition
    let err_type = output["errorType"].as_str().unwrap();
    assert!(
        err_type.contains("validation"),
        "Expected validation in error type, got: {}",
        err_type
    );
    assert_eq!(output["errorStatus"], json!(400));
    assert_eq!(output["recovered"], json!(true));
}

// === Expression: object construction with multiple fields ===

#[tokio::test]
async fn test_runner_try_catch_retry_exponential_backoff() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-retry-exponential
  version: '0.1.0'
do:
  - retryTask:
      try:
        - failStep:
            raise:
              error:
                type: runtime
                title: Temp Failure
                status: 500
      catch:
        errors:
          with:
            type: runtime
        retry:
          delay:
            milliseconds: 10
          backoff:
            exponential:
              factor: 2.0
          limit:
            attempt:
              count: 3
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let start = std::time::Instant::now();
    let result = runner.run(json!({})).await;
    let elapsed = start.elapsed();
    // Should fail after 3 attempts with exponential backoff
    assert!(result.is_err());
    // With exponential backoff: 0ms (first) + 10ms (second) + 40ms (third) ≈ 50ms+
    // Allow generous tolerance since we just want to verify it retries
    assert!(
        elapsed.as_millis() >= 10,
        "Should take at least 10ms with retry delay, got {}ms",
        elapsed.as_millis()
    );
}

// === Try-catch with retry and linear backoff ===

#[tokio::test]
async fn test_runner_try_catch_retry_linear_backoff() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-retry-linear
  version: '0.1.0'
do:
  - retryTask:
      try:
        - failStep:
            raise:
              error:
                type: runtime
                title: Temp Failure
                status: 500
      catch:
        errors:
          with:
            type: runtime
        retry:
          delay:
            milliseconds: 10
          backoff:
            linear:
              increment:
                milliseconds: 10
          limit:
            attempt:
              count: 3
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let start = std::time::Instant::now();
    let result = runner.run(json!({})).await;
    let elapsed = start.elapsed();
    assert!(result.is_err());
    // With linear backoff: 0ms (first) + 10ms (second) + 20ms (third) ≈ 30ms+
    assert!(
        elapsed.as_millis() >= 10,
        "Should take at least 10ms with retry delay, got {}ms",
        elapsed.as_millis()
    );
}

// === Try-catch with retry that eventually succeeds ===

#[tokio::test]
async fn test_runner_try_catch_retry_succeeds_eventually() {
    // Test that retry reference from use.retries works
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-retry-succeeds
  version: '0.1.0'
use:
  retries:
    myRetry:
      delay:
        milliseconds: 10
      limit:
        attempt:
          count: 3
do:
  - retryTask:
      try:
        - failStep:
            raise:
              error:
                type: runtime
                title: Temp Failure
                status: 500
      catch:
        errors:
          with:
            type: runtime
        retry: myRetry
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // Should fail after 3 attempts using referenced retry policy
    let result = runner.run(json!({})).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type_short(), "runtime");
}

// === Fork compete: verify winning branch output ===

#[tokio::test]
async fn test_runner_try_catch_catch_when() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-no-match-propagates
  version: '0.1.0'
do:
  - guardedTask:
      try:
        - failStep:
            raise:
              error:
                type: validation
                title: Validation Error
                status: 400
      catch:
        errors:
          with:
            type: communication
        do:
          - handleErr:
              set:
                caught: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // validation error doesn't match communication filter → propagates
    let result = runner.run(json!({})).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().error_type_short(), "validation");
}

// === Try-catch with catch.when rejecting ===

#[tokio::test]
async fn test_runner_try_catch_catch_when_rejects() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-when-rejects
  version: '0.1.0'
do:
  - guardedTask:
      try:
        - failStep:
            raise:
              error:
                type: validation
                title: Validation Failed
                status: 400
      catch:
        errors:
          with:
            type: validation
        when: ${ .shouldCatch == true }
        do:
          - handleErr:
              set:
                caught: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // shouldCatch=false → catch.when rejects → error propagates
    let result = runner.run(json!({"shouldCatch": false})).await;
    assert!(result.is_err());
}

// === Fork compete with output.as ===

#[tokio::test]
async fn test_runner_try_catch_as_status() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-as-status
  version: '0.1.0'
do:
  - guarded:
      try:
        - failStep:
            raise:
              error:
                type: validation
                title: Bad Request
                status: 400
      catch:
        errors:
          with:
            type: validation
        as: err
        do:
          - handleErr:
              set:
                caught: true
                errorStatus: "${ $err.status }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["caught"], json!(true));
    assert_eq!(output["errorStatus"], json!(400));
}

// === Export with nested object construction ===

#[tokio::test]
async fn test_runner_try_catch_modify_continue() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-modify-continue
  version: '0.1.0'
do:
  - step1:
      set:
        value: 10
  - guarded:
      try:
        - failStep:
            raise:
              error:
                type: validation
                title: Error
                status: 400
      catch:
        errors:
          with:
            type: validation
        do:
          - recover:
              set:
                value: 0
                recovered: true
  - step3:
      set:
        final: "${ .value }"
        wasRecovered: "${ .recovered }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["final"], json!(0));
    assert_eq!(output["wasRecovered"], json!(true));
}

// === Expression: deep object update ===

#[tokio::test]
async fn test_runner_try_catch_swallow_error() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-swallow
  version: '0.1.0'
do:
  - step1:
      set:
        value: before
  - guarded:
      try:
        - failStep:
            raise:
              error:
                type: validation
                title: Swallowed Error
                status: 400
      catch:
        errors:
          with:
            type: validation
  - step3:
      set:
        value: after
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // Error caught without catch.do — output preserved from before the error
    assert_eq!(output["value"], json!("after"));
}

// === Expression: select with nested conditions ===

#[tokio::test]
async fn test_runner_try_catch_all() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-all
  version: '0.1.0'
do:
  - risky:
      try:
        - failStep:
            raise:
              error:
                type: runtime
                title: Oops
                status: 500
      catch:
        as: err
        do:
          - handleErr:
              set:
                caught: true
                errType: "${ $err.type }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["caught"], json!(true));
    assert!(output["errType"].as_str().unwrap().contains("runtime"));
}

// === Raise: all error types via reference ===

#[tokio::test]
async fn test_runner_try_catch_retry_exhausted() {
    // After retry limit is exhausted, the error propagates out
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-retry-exhausted
  version: '0.1.0'
do:
  - risky:
      try:
        - failStep:
            raise:
              error:
                type: validation
                title: Fail
                status: 400
      catch:
        errors:
          with:
            type: validation
        retry:
          delay:
            milliseconds: 1
          backoff:
            constant: {}
          limit:
            attempt:
              count: 2
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // After retry limit, error propagates
    let result = runner.run(json!({})).await;
    assert!(result.is_err());
}

// === Expression: length on various types ===

// Try-catch: catch.do modifies output and continues
#[tokio::test]
async fn test_runner_try_catch_catch_do_export() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-catch-do-recover
  version: '0.1.0'
do:
  - safe:
      try:
        - fail:
            raise:
              error:
                type: runtime
                title: Fail
                status: 500
      catch:
        errors:
          type: runtime
        do:
          - recover:
              set:
                recovered: true
  - check:
      set:
        done: true
        recovered: "${ .recovered }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["recovered"], json!(true));
    assert_eq!(output["done"], json!(true));
}

// Try-catch: nested try-catch, inner catches, outer continues
#[tokio::test]
async fn test_runner_try_catch_nested_inner_catches() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-nested-inner
  version: '0.1.0'
do:
  - outer:
      try:
        - inner:
            try:
              - fail:
                  raise:
                    error:
                      type: validation
                      title: Inner Error
                      status: 400
            catch:
              errors:
                type: validation
              as: err
              do:
                - recover:
                    set:
                      innerRecovered: true
                      innerErr: "${ $err.type }"
      catch:
        errors:
          type: runtime
        as: err
        do:
          - recoverOuter:
              set:
                outerRecovered: true
  - verify:
      set:
        innerOk: "${ .innerRecovered }"
        innerErrType: "${ .innerErr }"
        noOuterRecovery: "${ .outerRecovered == null }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["innerOk"], json!(true));
    assert_eq!(output["innerErrType"], json!("validation"));
    assert_eq!(output["noOuterRecovery"], json!(true));
}

// Try-catch: catch with when expression
#[tokio::test]
async fn test_runner_try_catch_when_expr_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-when-expr-v2
  version: '0.1.0'
do:
  - safe:
      try:
        - fail:
            raise:
              error:
                type: runtime
                title: Test Error
                status: 500
                detail: "Expected error message"
      catch:
        errors:
          with:
            type: runtime
        when: ${ .status >= 400 }
        as: err
        do:
          - recover:
              set:
                caught: true
  - verify:
      set:
        wasCaught: "${ .caught }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["wasCaught"], json!(true));
}

#[tokio::test]
async fn test_runner_try_catch_error_variable_java_pattern() {
    // Matches Java SDK's try-catch-error-variable.yaml
    // catch with as: caughtError, then use $caughtError.details in set
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-error-variable
  version: '0.1.0'
do:
  - attemptTask:
      try:
        - failingTask:
            raise:
              error:
                type: https://example.com/errors/transient
                detail: Javierito was here!
                status: 503
      catch:
        as: caughtError
        do:
          - handleError:
              set:
                errorMessage: '${$caughtError.details}'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["errorMessage"], json!("Javierito was here!"));
}

#[tokio::test]
async fn test_runner_try_catch_match_when_java_pattern() {
    // Matches Java SDK's try-catch-match-when.yaml
    // catch with when: ${ .status == 503 } expression
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-match-when
  version: '0.1.0'
do:
  - attemptTask:
      try:
        - failingTask:
            raise:
              error:
                type: https://example.com/errors/transient
                status: 503
      catch:
        when: '${ .status == 503 }'
        do:
          - handleError:
              set:
                recovered: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["recovered"], json!(true));
}

#[tokio::test]
async fn test_runner_try_catch_not_match_when_java_pattern() {
    // Matches Java SDK's try-catch-not-match-when.yaml
    // catch with when: ${ .status == 400 } but error has status 503 - should not match
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-not-match-when
  version: '0.1.0'
do:
  - attemptTask:
      try:
        - failingTask:
            raise:
              error:
                type: https://example.com/errors/transient
                status: 503
      catch:
        when: '${ .status == 400 }'
        do:
          - handleError:
              set:
                recovered: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let result = runner.run(json!({})).await;
    // When catch when doesn't match, the error propagates
    assert!(result.is_err());
}

#[tokio::test]
async fn test_runner_try_catch_match_details_java_pattern() {
    // Matches Java SDK's try-catch-match-details.yaml
    // catch with errors.with.details matching error detail
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-match-details
  version: '0.1.0'
do:
  - attemptTask:
      try:
        - failingTask:
            raise:
              error:
                type: https://example.com/errors/transient
                status: 503
                detail: Enforcement Failure - invalid email
      catch:
        errors:
          with:
            type: https://example.com/errors/transient
            status: 503
            details: Enforcement Failure - invalid email
        do:
          - handleError:
              set:
                recovered: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["recovered"], json!(true));
}

#[tokio::test]
async fn test_runner_try_catch_not_match_details_java_pattern() {
    // Matches Java SDK's try-catch-not-match-details.yaml
    // catch with errors.with.details that doesn't match the actual error detail
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-not-match-details
  version: '0.1.0'
do:
  - attemptTask:
      try:
        - failingTask:
            raise:
              error:
                type: https://example.com/errors/security
                status: 403
                detail: Enforcement Failure - invalid email
      catch:
        errors:
          with:
            type: https://example.com/errors/security
            status: 403
            details: User not found in tenant catalog
        do:
          - handleError:
              set:
                recovered: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let result = runner.run(json!({})).await;
    // Details don't match, error propagates
    assert!(result.is_err());
}

// === Shell return: code on failing command (Java SDK echo-exitcode.yaml pattern) ===

#[tokio::test]
async fn test_runner_try_catch_retry_use_reference() {
    // Go SDK pattern - try with retry referencing use.retries definition
    // When retries are exhausted, the error propagates
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: retry-reference
  version: '0.1.0'
use:
  retries:
    myRetry:
      delay:
        milliseconds: 10
      limit:
        attempt:
          count: 2
do:
  - attemptTask:
      try:
        - failTask:
            raise:
              error:
                type: communication
                status: 500
      catch:
        errors:
          with:
            type: communication
        retry: myRetry
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    // After 2 retries, the error still propagates (always raises)
    let result = runner.run(json!({})).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type_short(), "communication");
}

// === Export with multiple context variables (accumulated via merge) ===

#[tokio::test]
async fn test_runner_try_catch_when_rejects_propagates() {
    // catch.when expression evaluates to false → error propagates
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: catch-when-rejects
  version: '0.1.0'
do:
  - attemptTask:
      try:
        - failTask:
            raise:
              error:
                type: validation
                status: 400
      catch:
        when: ${ .status == 404 }
        do:
          - handleRecovery:
              set:
                recovered: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let result = runner.run(json!({})).await;
    // catch.when requires .status == 404, but error has status 400 → propagates
    assert!(result.is_err());
}

// === Call function reference with use.functions (Go/Java SDK pattern) ===

#[tokio::test]
async fn test_runner_try_catch_error_variable_full() {
    // Java SDK's try-catch-error-variable.yaml - catch as: caughtError + reference $caughtError
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-error-var
  version: '0.1.0'
do:
  - tryTask:
      try:
        - raiseError:
            raise:
              error:
                type: https://serverlessworkflow.io/spec/1.0.0/errors/runtime
                status: 500
                detail: test error occurred
      catch:
        as: caughtError
        errors:
          with:
            type: https://serverlessworkflow.io/spec/1.0.0/errors/runtime
        do:
          - handleError:
              set:
                errorMessage: '${ $caughtError.details }'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["errorMessage"], json!("test error occurred"));
}

// === Schedule cron pattern ===

/// Test try-catch-match-status with type AND status combo — Java SDK's try-catch-match-status.yaml
/// Catch error by both type and status matching
#[tokio::test]
async fn test_runner_try_catch_match_status_and_type() {
    let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-match-status-type
  version: '0.1.0'
do:
  - attemptTask:
      try:
        - failingTask:
            raise:
              error:
                type: https://example.com/errors/transient
                status: 503
      catch:
        errors:
          with:
            type: https://example.com/errors/transient
            status: 503
        do:
          - handleError:
              set:
                recovered: true
"#;
    let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
    let runner = WorkflowRunner::new(workflow).unwrap();

    let output = runner.run(json!({})).await.unwrap();
    assert_eq!(output["recovered"], json!(true));
}

// === E2E: Shell + Set — command result processing ===
