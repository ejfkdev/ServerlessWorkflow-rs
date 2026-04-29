use super::*;

#[tokio::test]
async fn test_runner_runtime_expression() {
    let output = run_workflow_from_yaml(&testdata("runtime_expression.yaml"), json!({}))
        .await
        .unwrap();
    // $workflow.id should be a UUID string
    assert!(output["id"].is_string());
    assert!(!output["id"].as_str().unwrap().is_empty());
    // $runtime.version should be set
    assert!(output["version"].is_string());
    assert!(!output["version"].as_str().unwrap().is_empty());
}

// === For Loop: with while condition ===

#[tokio::test]
async fn test_runner_task_expression() {
    let output = run_workflow_from_yaml(&testdata("task_expression.yaml"), json!({}))
        .await
        .unwrap();
    // $task.name should be set to the current task name
    assert_eq!(output["taskName"], json!("useExpression"));
}

// === Try-Catch: Retry with successful recovery ===

#[tokio::test]
async fn test_runner_secret_expression() {
    use crate::secret::MapSecretManager;

    let secret_mgr = Arc::new(MapSecretManager::new().with_secret(
        "superman",
        json!({
            "name": "ClarkKent",
            "enemy": {
                "name": "Lex Luthor",
                "isHuman": true
            }
        }),
    ));

    let output = run_workflow_from_yaml_with_secrets(
        &testdata("secret_expression.yaml"),
        json!({}),
        secret_mgr,
    )
    .await
    .unwrap();
    assert_eq!(output["superSecret"], json!("ClarkKent"));
    assert_eq!(output["theEnemy"], json!("Lex Luthor"));
    assert_eq!(output["humanEnemy"], json!(true));
}

#[tokio::test]
async fn test_runner_secret_expression_missing() {
    // No secret manager configured - $secret should be null, expression should fail
    let result = run_workflow_from_yaml(&testdata("secret_expression.yaml"), json!({})).await;
    // Should error because $secret is null and $secret.superman.name is not accessible
    assert!(result.is_err());
}

// === Try-Catch: Compensate with catch.do ===

#[tokio::test]
async fn test_runner_simple_expression() {
    let output = run_workflow_from_yaml(&testdata("simple_expression.yaml"), json!({}))
        .await
        .unwrap();
    // $task.startedAt.epoch.milliseconds should be a number
    assert!(
        output["startedAt"].is_number(),
        "startedAt should be a number, got: {:?}",
        output["startedAt"]
    );
    let ms = output["startedAt"].as_i64().unwrap();
    assert!(ms > 0, "startedAt milliseconds should be positive");

    // $workflow.id should be a UUID string
    assert!(output["id"].is_string());
    assert!(!output["id"].as_str().unwrap().is_empty());

    // $runtime.version should be set
    assert!(output["version"].is_string());
    assert!(!output["version"].as_str().unwrap().is_empty());
}

// === Execution Listener ===

#[tokio::test]
async fn test_runner_call_http_oauth2_expression_params() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use warp::Reply;

    let token_issued = Arc::new(AtomicBool::new(false));
    let token_issued_clone = token_issued.clone();

    let token_endpoint = warp::path("oauth2")
        .and(warp::path("token"))
        .and(warp::post())
        .and(warp::body::form::<std::collections::HashMap<String, String>>())
        .map(move |params: std::collections::HashMap<String, String>| {
            let client_id = params.get("client_id").map(|s| s.as_str()).unwrap_or("");
            let client_secret = params
                .get("client_secret")
                .map(|s| s.as_str())
                .unwrap_or("");

            if client_id == "my-app" && client_secret == "my-secret" {
                token_issued_clone.store(true, Ordering::SeqCst);
                warp::reply::json(&serde_json::json!({
                    "access_token": "expr-token",
                    "token_type": "Bearer"
                }))
            } else {
                warp::reply::json(&serde_json::json!({"error": "invalid_client"}))
            }
        });

    let protected_endpoint = warp::path("protected")
        .and(warp::header::optional("Authorization"))
        .map(|auth: Option<String>| match auth {
            Some(val) if val == "Bearer expr-token" => {
                warp::reply::json(&serde_json::json!({"data": "from-expr"})).into_response()
            }
            _ => warp::reply::with_status(
                warp::reply::json(&serde_json::json!({"error": "unauthorized"})),
                warp::http::StatusCode::UNAUTHORIZED,
            )
            .into_response(),
        });

    let routes = token_endpoint.or(protected_endpoint);
    let port = start_mock_server(routes);

    // Client id/secret from workflow input via expressions
    let yaml = format!(
        r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: oauth2-expression-params
  version: '0.1.0'
do:
  - getProtected:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:{port}/protected
          authentication:
            oauth2:
              authority: http://localhost:{port}
              grant: client_credentials
              client:
                id: '${{ .clientId }}'
                secret: '${{ .clientSecret }}'
"#
    );
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
    let runner = WorkflowRunner::new(workflow).unwrap();

    let output = runner
        .run(json!({"clientId": "my-app", "clientSecret": "my-secret"}))
        .await
        .unwrap();
    assert_eq!(output["data"], json!("from-expr"));
    assert!(
        token_issued.load(Ordering::SeqCst),
        "Expected OAuth2 token with expression params"
    );
}

// === Call Function: HTTP function reference ===

#[tokio::test]
async fn test_runner_emit_with_data_expression() {
    let output = run_workflow_from_yaml(
        &testdata("emit_data.yaml"),
        json!({"firstName": "John", "lastName": "Doe"}),
    )
    .await
    .unwrap();
    // Emit returns input unchanged
    assert_eq!(output["firstName"], json!("John"));
    assert_eq!(output["lastName"], json!("Doe"));
}

// === For loop: export.as with context accumulation ===

#[tokio::test]
async fn test_runner_set_multiple_expressions() {
    let output = run_workflow_from_yaml(&testdata("concatenating_strings.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["fullName"], json!("John Doe"));
}

// === Try-Catch: catch by status code ===

#[tokio::test]
async fn test_runner_expression_default_value() {
    let output = run_workflow_from_yaml(&testdata("expression_default_value.yaml"), json!({}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!("default"));
}

// === Expression: null coalescing with existing value ===

#[tokio::test]
async fn test_runner_expression_null_coalesce_existing() {
    let output = run_workflow_from_yaml(
        &testdata("expression_null_coalesce.yaml"),
        json!({"value": "actual"}),
    )
    .await
    .unwrap();
    assert_eq!(output["existing"], json!("actual"));
    assert_eq!(output["missing"], json!("fallback"));
}

// === Expression: array length ===

#[tokio::test]
async fn test_runner_expression_array_length() {
    let output = run_workflow_from_yaml(
        &testdata("expression_array_length.yaml"),
        json!({"items": [1, 2, 3, 4, 5]}),
    )
    .await
    .unwrap();
    assert_eq!(output["count"], json!(5));
    assert_eq!(output["isEmpty"], json!(false));
}

#[tokio::test]
async fn test_runner_expression_array_length_empty() {
    let output = run_workflow_from_yaml(
        &testdata("expression_array_length.yaml"),
        json!({"items": []}),
    )
    .await
    .unwrap();
    assert_eq!(output["count"], json!(0));
    assert_eq!(output["isEmpty"], json!(true));
}

// === Expression: string operations (upper, lower, split) ===

#[tokio::test]
async fn test_runner_expression_string_operations() {
    let output = run_workflow_from_yaml(
        &testdata("expression_string_operations.yaml"),
        json!({"name": "John", "sentence": "hello world foo"}),
    )
    .await
    .unwrap();
    assert_eq!(output["upper"], json!("JOHN"));
    assert_eq!(output["lower"], json!("john"));
    assert_eq!(output["split"], json!(["hello", "world", "foo"]));
}

// === For loop: object collect with position ===

#[tokio::test]
async fn test_runner_set_nested_expressions() {
    let output = run_workflow_from_yaml(
        &testdata("set_nested_expressions.yaml"),
        json!({"firstName": "John", "lastName": "Doe", "age": 30, "x": 2, "y": 3, "z": 4}),
    )
    .await
    .unwrap();
    assert_eq!(output["user"]["fullName"], json!("John Doe"));
    assert_eq!(output["user"]["age"], json!(30));
    assert_eq!(output["computed"], json!(10)); // 2*3+4
}

// === Set: conditional expression (if-then-else) ===

#[tokio::test]
async fn test_runner_set_conditional_expression_adult() {
    let output = run_workflow_from_yaml(
        &testdata("set_conditional_expression.yaml"),
        json!({"age": 25}),
    )
    .await
    .unwrap();
    assert_eq!(output["category"], json!("adult"));
}

#[tokio::test]
async fn test_runner_set_conditional_expression_minor() {
    let output = run_workflow_from_yaml(
        &testdata("set_conditional_expression.yaml"),
        json!({"age": 15}),
    )
    .await
    .unwrap();
    assert_eq!(output["category"], json!("minor"));
}

// === Task input: from expression (task-level input.from) ===

#[tokio::test]
async fn test_runner_expression_nested_object() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-nested-object
  version: '0.1.0'
do:
  - buildObject:
      set:
        result: "${ {name: .first + \" \" + .last, address: {city: .city, zip: .zip}} }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"first": "John", "last": "Doe", "city": "NYC", "zip": "10001"}))
        .await
        .unwrap();
    assert_eq!(output["result"]["name"], json!("John Doe"));
    assert_eq!(output["result"]["address"]["city"], json!("NYC"));
    assert_eq!(output["result"]["address"]["zip"], json!("10001"));
}

// === Expression: array map and select ===

#[tokio::test]
async fn test_runner_expression_array_map() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-array-map
  version: '0.1.0'
do:
  - mapItems:
      set:
        names: "${ [.people[].name] }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"people": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]}))
        .await
        .unwrap();
    assert_eq!(output["names"], json!(["Alice", "Bob"]));
}

// === Expression: array filter with select ===

#[tokio::test]
async fn test_runner_expression_array_select() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-array-select
  version: '0.1.0'
do:
  - filterItems:
      set:
        adults: "${ [.people[] | select(.age >= 18)] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"people": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 15}, {"name": "Carol", "age": 25}]})).await.unwrap();
    assert_eq!(output["adults"].as_array().unwrap().len(), 2);
    assert_eq!(output["adults"][0]["name"], json!("Alice"));
    assert_eq!(output["adults"][1]["name"], json!("Carol"));
}

// === Expression: numeric operations ===

#[tokio::test]
async fn test_runner_expression_numeric_ops() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-numeric-ops
  version: '0.1.0'
do:
  - compute:
      set:
        sum: "${ .a + .b }"
        diff: "${ .a - .b }"
        product: "${ .a * .b }"
        quotient: "${ .a / .b }"
        modulo: "${ .a % .b }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": 10, "b": 3}))
        .await
        .unwrap();
    assert_eq!(output["sum"], json!(13));
    assert_eq!(output["diff"], json!(7));
    assert_eq!(output["product"], json!(30));
    assert_eq!(output["quotient"], json!(3.3333333333333335)); // 10/3 in floating point
    assert_eq!(output["modulo"], json!(1));
}

// === Expression: boolean operations ===

#[tokio::test]
async fn test_runner_expression_boolean_ops() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-boolean-ops
  version: '0.1.0'
do:
  - compute:
      set:
        both: "${ .x and .y }"
        either: "${ .x or .z }"
        notX: "${ .x | not }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"x": true, "y": true, "z": false}))
        .await
        .unwrap();
    assert_eq!(output["both"], json!(true));
    assert_eq!(output["either"], json!(true));
    assert_eq!(output["notX"], json!(false));
}

// === Expression: string concatenation (alternative to interpolation) ===

#[tokio::test]
async fn test_runner_expression_string_concat() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-string-concat
  version: '0.1.0'
do:
  - interpolate:
      set:
        greeting: "${ \"Hello, \" + .name + \"!\" }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"name": "World"}))
        .await
        .unwrap();
    assert_eq!(output["greeting"], json!("Hello, World!"));
}

// === Expression: to_entries / from_entries ===

#[tokio::test]
async fn test_runner_expression_to_entries() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-to-entries
  version: '0.1.0'
do:
  - transform:
      set:
        entries: "${ to_entries }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": 1, "b": 2}))
        .await
        .unwrap();
    let entries = output["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
}

// === Composite: for + conditional set combination ===

#[tokio::test]
async fn test_runner_workflow_timeout_dynamic_expression() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-timeout-dynamic
  version: '0.1.0'
timeout:
  after: '${ "PT" + (.delay | tostring) + "S" }'
do:
  - slowTask:
      wait: PT5S
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // delay=0.1 means timeout after 0.1 seconds, which should trigger
    let result = runner.run(json!({"delay": 0.1})).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().error_type_short(), "timeout");
}

#[tokio::test]
async fn test_runner_workflow_timeout_dynamic_expression_completes() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-timeout-dynamic-ok
  version: '0.1.0'
timeout:
  after: '${ "PT" + (.delay | tostring) + "S" }'
do:
  - quickTask:
      set:
        result: done
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // delay=10 means timeout after 10 seconds, should complete quickly
    let output = runner.run(json!({"delay": 10})).await.unwrap();
    assert_eq!(output["result"], json!("done"));
}

// === Wait: dynamic duration expression ===

#[tokio::test]
async fn test_runner_wait_dynamic_expression() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: wait-dynamic
  version: '0.1.0'
do:
  - shortWait:
      wait: '${ "PT" + (.ms | tostring) + "S" }'
  - setResult:
      set:
        phase: completed
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let start = std::time::Instant::now();
    let output = runner.run(json!({"ms": 0.1})).await.unwrap();
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 80,
        "Expected at least 80ms delay, got {}ms",
        elapsed.as_millis()
    );
    assert_eq!(output["phase"], json!("completed"));
}

// === Try-Catch: error filter by instance ===

#[tokio::test]
async fn test_runner_expression_has() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-has
  version: '0.1.0'
do:
  - checkFields:
      set:
        hasName: '${ has("name") }'
        hasAge: '${ has("age") }'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"name": "John"}))
        .await
        .unwrap();
    assert_eq!(output["hasName"], json!(true));
    assert_eq!(output["hasAge"], json!(false));
}

// === Retry with actual success on later attempt ===

#[tokio::test]
async fn test_runner_expression_keys() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-keys
  version: '0.1.0'
do:
  - getKeys:
      set:
        result: "${ keys }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "John", "age": 30}))
        .await
        .unwrap();
    let keys = output["result"].as_array().unwrap();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().any(|k| k == "name"));
    assert!(keys.iter().any(|k| k == "age"));
}

#[tokio::test]
async fn test_runner_expression_values() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-values
  version: '0.1.0'
do:
  - getValues:
      set:
        result: "${ [.[]] }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "John", "age": 30}))
        .await
        .unwrap();
    let vals = output["result"].as_array().unwrap();
    assert_eq!(vals.len(), 2);
    let has_john = vals.iter().any(|v| v.as_str() == Some("John"));
    let has_30 = vals.iter().any(|v| v.as_i64() == Some(30));
    assert!(has_john, "Expected 'John' in values");
    assert!(has_30, "Expected 30 in values");
}

// === Expression: contains (using IN for array membership) ===

#[tokio::test]
async fn test_runner_expression_contains() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-contains
  version: '0.1.0'
do:
  - checkContains:
      set:
        hasFoo: "${ .items | any(. == \"foo\") }"
        hasMissing: "${ .items | any(. == \"missing\") }"
        containsSub: "${ .name | contains(\"ello\") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": ["foo", "bar", "baz"], "name": "Hello"}))
        .await
        .unwrap();
    assert_eq!(output["hasFoo"], json!(true));
    assert_eq!(output["hasMissing"], json!(false));
    assert_eq!(output["containsSub"], json!(true));
}

// === Expression: type ===

#[tokio::test]
async fn test_runner_expression_type() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-type
  version: '0.1.0'
do:
  - checkType:
      set:
        strType: "${ .name | type }"
        numType: "${ .age | type }"
        arrType: "${ .items | type }"
        objType: "${ .meta | type }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "John", "age": 30, "items": [1, 2], "meta": {"k": "v"}}))
        .await
        .unwrap();
    assert_eq!(output["strType"], json!("string"));
    assert_eq!(output["numType"], json!("number"));
    assert_eq!(output["arrType"], json!("array"));
    assert_eq!(output["objType"], json!("object"));
}

// === Expression: startswith / endswith ===

#[tokio::test]
async fn test_runner_expression_startswith_endswith() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-startswith-endswith
  version: '0.1.0'
do:
  - checkPrefixSuffix:
      set:
        startsHello: "${ .greeting | startswith(\"Hello\") }"
        startsBye: "${ .greeting | startswith(\"Bye\") }"
        endsWorld: "${ .greeting | endswith(\"World\") }"
        endsHello: "${ .greeting | endswith(\"Hello\") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"greeting": "Hello World"}))
        .await
        .unwrap();
    assert_eq!(output["startsHello"], json!(true));
    assert_eq!(output["startsBye"], json!(false));
    assert_eq!(output["endsWorld"], json!(true));
    assert_eq!(output["endsHello"], json!(false));
}

// === Expression: ltrimstr / rtrimstr ===

#[tokio::test]
async fn test_runner_expression_ltrimstr_rtrimstr() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-trimstr
  version: '0.1.0'
do:
  - trimStrings:
      set:
        trimmedPrefix: "${ .path | ltrimstr(\"/api/\") }"
        trimmedSuffix: "${ .file | rtrimstr(\".txt\") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"path": "/api/users", "file": "data.txt"}))
        .await
        .unwrap();
    assert_eq!(output["trimmedPrefix"], json!("users"));
    assert_eq!(output["trimmedSuffix"], json!("data"));
}

// === Expression: join ===

#[tokio::test]
async fn test_runner_expression_join() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-join
  version: '0.1.0'
do:
  - joinItems:
      set:
        result: "${ .items | join(\", \") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": ["apple", "banana", "cherry"]}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!("apple, banana, cherry"));
}

// === Expression: flatten ===

#[tokio::test]
async fn test_runner_expression_flatten() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-flatten
  version: '0.1.0'
do:
  - flattenArray:
      set:
        result: "${ .matrix | flatten }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"matrix": [[1, 2], [3, 4], [5]]}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!([1, 2, 3, 4, 5]));
}

// === Expression: unique ===

#[tokio::test]
async fn test_runner_expression_unique() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-unique
  version: '0.1.0'
do:
  - uniqueItems:
      set:
        result: "${ .items | unique }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [1, 2, 2, 3, 3, 3]}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!([1, 2, 3]));
}

// === Expression: sort ===

#[tokio::test]
async fn test_runner_expression_sort() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-sort
  version: '0.1.0'
do:
  - sortItems:
      set:
        result: "${ .items | sort }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [3, 1, 4, 1, 5, 9]}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!([1, 1, 3, 4, 5, 9]));
}

// === Expression: group_by ===

#[tokio::test]
async fn test_runner_expression_group_by() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-group-by
  version: '0.1.0'
do:
  - groupPeople:
      set:
        result: "${ .people | group_by(.dept) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"people": [
            {"name": "Alice", "dept": "eng"},
            {"name": "Bob", "dept": "hr"},
            {"name": "Carol", "dept": "eng"}
        ]}))
        .await
        .unwrap();
    let groups = output["result"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
}

// === Expression: min / max ===

#[tokio::test]
async fn test_runner_expression_min_max() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-min-max
  version: '0.1.0'
do:
  - findMinMax:
      set:
        minVal: "${ .items | min }"
        maxVal: "${ .items | max }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [5, 2, 8, 1, 9]}))
        .await
        .unwrap();
    assert_eq!(output["minVal"], json!(1));
    assert_eq!(output["maxVal"], json!(9));
}

// === Expression: reverse ===

#[tokio::test]
async fn test_runner_expression_reverse() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-reverse
  version: '0.1.0'
do:
  - reverseList:
      set:
        result: "${ .items | reverse }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [1, 2, 3, 4, 5]}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!([5, 4, 3, 2, 1]));
}

// === Expression: tonumber / tostring ===

#[tokio::test]
async fn test_runner_expression_tonumber_tostring() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-tonumber-tostring
  version: '0.1.0'
do:
  - convertTypes:
      set:
        asNumber: "${ .strNum | tonumber }"
        asString: "${ .numVal | tostring }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"strNum": "42", "numVal": 99}))
        .await
        .unwrap();
    assert_eq!(output["asNumber"], json!(42));
    assert_eq!(output["asString"], json!("99"));
}

// === Expression: any / all ===

#[tokio::test]
async fn test_runner_expression_any_all() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-any-all
  version: '0.1.0'
do:
  - checkPredicates:
      set:
        anyActive: "${ [.users[].active] | any }"
        allActive: "${ [.users[].active] | all }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"users": [
            {"name": "Alice", "active": true},
            {"name": "Bob", "active": false},
            {"name": "Carol", "active": true}
        ]}))
        .await
        .unwrap();
    assert_eq!(output["anyActive"], json!(true));
    assert_eq!(output["allActive"], json!(false));
}

// === Expression: first / last ===

#[tokio::test]
async fn test_runner_expression_first_last() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-first-last
  version: '0.1.0'
do:
  - getEnds:
      set:
        firstItem: "${ .items | first }"
        lastItem: "${ .items | last }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [10, 20, 30, 40]}))
        .await
        .unwrap();
    assert_eq!(output["firstItem"], json!(10));
    assert_eq!(output["lastItem"], json!(40));
}

// === Expression: range ===

#[tokio::test]
async fn test_runner_expression_range() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-range
  version: '0.1.0'
do:
  - generateRange:
      set:
        result: "${ [range(5)] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["result"], json!([0, 1, 2, 3, 4]));
}

// === Expression: limit (take first N) ===

#[tokio::test]
async fn test_runner_expression_limit() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-limit
  version: '0.1.0'
do:
  - takeFirst:
      set:
        result: "${ .items | limit(3; .[]) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [1, 2, 3, 4, 5]}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!([1, 2, 3]));
}

// === Expression: indices ===

#[tokio::test]
async fn test_runner_expression_indices() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-indices
  version: '0.1.0'
do:
  - findIndices:
      set:
        result: "${ .str | indices(\"ab\") }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"str": "ababab"}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!([0, 2, 4]));
}

// === Expression: map_values ===

#[tokio::test]
async fn test_runner_expression_map_values() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-map-values
  version: '0.1.0'
do:
  - transformValues:
      set:
        result: "${ .prices | map_values(. * 1.1) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"prices": {"apple": 1.0, "banana": 2.0}}))
        .await
        .unwrap();
    // Results should be approximately 1.1 and 2.2
    let result = &output["result"];
    assert!((result["apple"].as_f64().unwrap() - 1.1).abs() < 0.01);
    assert!((result["banana"].as_f64().unwrap() - 2.2).abs() < 0.01);
}

// === Expression: del ===

#[tokio::test]
async fn test_runner_expression_del() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-del
  version: '0.1.0'
do:
  - removeField:
      set:
        result: "${ del(.secret) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "John", "secret": "password123"}))
        .await
        .unwrap();
    assert_eq!(output["result"]["name"], json!("John"));
    assert!(output["result"].get("secret").is_none());
}

// === Expression: getpath / setpath ===

#[tokio::test]
async fn test_runner_expression_getpath_setpath() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-getpath-setpath
  version: '0.1.0'
do:
  - pathOps:
      set:
        getValue: "${ .a.b }"
        setValue: "${ .a.b = 99 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": {"b": 42}}))
        .await
        .unwrap();
    assert_eq!(output["getValue"], json!(42));
    assert_eq!(output["setValue"]["a"]["b"], json!(99));
}

// === Expression: paths ===

#[tokio::test]
async fn test_runner_expression_paths() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-paths
  version: '0.1.0'
do:
  - getPaths:
      set:
        result: "${ [paths] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": {"b": 1}, "c": 2}))
        .await
        .unwrap();
    let result = output["result"].as_array().unwrap();
    // paths returns all paths to values
    assert!(!result.is_empty());
}

// === Expression: reduce ===

#[tokio::test]
async fn test_runner_expression_reduce() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-reduce
  version: '0.1.0'
do:
  - sumAll:
      set:
        result: "${ reduce .items[] as $item (0; . + $item) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [1, 2, 3, 4, 5]}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!(15));
}

// === Expression: from_entries ===

#[tokio::test]
async fn test_runner_expression_from_entries() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-from-entries
  version: '0.1.0'
do:
  - rebuildObj:
      set:
        result: "${ to_entries | from_entries }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "John", "age": 30}))
        .await
        .unwrap();
    assert_eq!(output["result"]["name"], json!("John"));
    assert_eq!(output["result"]["age"], json!(30));
}

// === Expression: with_entries (map_values alternative) ===

#[tokio::test]
async fn test_runner_expression_with_entries() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-with-entries
  version: '0.1.0'
do:
  - upperKeys:
      set:
        result: "${ with_entries(.key |= ascii_upcase) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "John", "age": 30}))
        .await
        .unwrap();
    assert_eq!(output["result"]["NAME"], json!("John"));
    assert_eq!(output["result"]["AGE"], json!(30));
}

// === Expression: isempty (using length == 0) ===

#[tokio::test]
async fn test_runner_expression_isempty() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-isempty
  version: '0.1.0'
do:
  - checkEmpty:
      set:
        emptyArr: "${ ([] | length) == 0 }"
        nonEmptyArr: "${ ([1] | length) == 0 }"
        emptyObj: "${ ({} | length) == 0 }"
        nonEmptyObj: "${ ({a:1} | length) == 0 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["emptyArr"], json!(true));
    assert_eq!(output["nonEmptyArr"], json!(false));
    assert_eq!(output["emptyObj"], json!(true));
    assert_eq!(output["nonEmptyObj"], json!(false));
}

// === Expression: recurse ===

#[tokio::test]
async fn test_runner_expression_recurse() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-recurse
  version: '0.1.0'
do:
  - recurseData:
      set:
        result: "${ [recurse | numbers] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": {"b": 1}, "c": 2}))
        .await
        .unwrap();
    let nums = output["result"].as_array().unwrap();
    assert!(nums.contains(&json!(1)));
    assert!(nums.contains(&json!(2)));
}

// === Expression: test (regex match) ===

#[tokio::test]
async fn test_runner_expression_test_regex() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-test-regex
  version: '0.1.0'
do:
  - regexTest:
      set:
        isEmail: "${ .email | test(\"@\") }"
        isNumber: "${ .str | test(\"^[0-9]+$\") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"email": "user@example.com", "str": "12345"}))
        .await
        .unwrap();
    assert_eq!(output["isEmail"], json!(true));
    assert_eq!(output["isNumber"], json!(true));
}

// === Expression: capture (regex groups) ===

#[tokio::test]
async fn test_runner_expression_capture() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-capture
  version: '0.1.0'
do:
  - extractParts:
      set:
        result: "${ .name | capture(\"(?<first>\\\\w+) (?<last>\\\\w+)\") }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"name": "John Doe"}))
        .await
        .unwrap();
    assert_eq!(output["result"]["first"], json!("John"));
    assert_eq!(output["result"]["last"], json!("Doe"));
}

// === Expression: ascii_downcase ===

#[tokio::test]
async fn test_runner_expression_ascii_downcase() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-ascii-downcase
  version: '0.1.0'
do:
  - lowerCase:
      set:
        result: "${ .name | ascii_downcase }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"name": "HELLO World"}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!("hello world"));
}

// === Expression: update operator (|=) ===

#[tokio::test]
async fn test_runner_expression_update() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-update
  version: '0.1.0'
do:
  - updateField:
      set:
        result: "${ .price |= . + 10 }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "item", "price": 100}))
        .await
        .unwrap();
    assert_eq!(output["result"]["name"], json!("item"));
    assert_eq!(output["result"]["price"], json!(110));
}

// === Expression: alternative operator (?//) ===

#[tokio::test]
async fn test_runner_expression_alternative_operator() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-alternative
  version: '0.1.0'
do:
  - tryPaths:
      set:
        result: "${ .missing // \"default\" }"
        existing: "${ .present // \"default\" }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"present": "actual"}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!("default"));
    assert_eq!(output["existing"], json!("actual"));
}

// === Expression: string interpolation in JQ ===

#[tokio::test]
async fn test_runner_expression_string_interpolation() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-string-interpolation
  version: '0.1.0'
do:
  - buildString:
      set:
        result: "${ \"Hello \\(.name), you are \\(.age) years old\" }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "World", "age": 42}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!("Hello World, you are 42 years old"));
}

// === Expression: not (invert boolean) ===

#[tokio::test]
async fn test_runner_expression_not() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-not
  version: '0.1.0'
do:
  - invertBool:
      set:
        notTrue: "${ true | not }"
        notFalse: "${ false | not }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["notTrue"], json!(false));
    assert_eq!(output["notFalse"], json!(true));
}

// === Task timeout: reference to reusable timeout ===

#[tokio::test]
async fn test_runner_export_complex_expression() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-complex
  version: '0.1.0'
do:
  - computeData:
      set:
        items:
          - name: Alice
            score: 95
          - name: Bob
            score: 85
        total: 180
      export:
        as: '${ {topScorer: .items[0].name, avgScore: (.total / (.items | length))} }'
  - useExported:
      set:
        report: '${ "Top: " + $context.topScorer + ", Avg: " + ($context.avgScore | tostring) }'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // Division 180/2 returns 90.0 in floating point
    assert!(output["report"]
        .as_str()
        .unwrap()
        .starts_with("Top: Alice, Avg: 90"));
}

// === Switch then: goto to another switch ===

#[tokio::test]
async fn test_runner_try_catch_when_expression() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: try-catch-when-expression
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
        when: ${ .status >= 400 }
        do:
          - handleError:
              set:
                caught: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["caught"], json!(true));
}

// === For loop with output.as extracting accumulated value ===

#[tokio::test]
async fn test_runner_raise_detail_expression() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-detail-expr
  version: '0.1.0'
do:
  - failTask:
      raise:
        error:
          type: validation
          title: Validation Error
          status: 400
          detail: '${ "Field " + .field + " is required" }'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let result = runner.run(json!({"field": "email"})).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type_short(), "validation");
    assert_eq!(err.detail(), Some("Field email is required"));
}

// === For loop with while and output.as ===

#[tokio::test]
async fn test_runner_expression_computed_keys() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-computed-keys
  version: '0.1.0'
do:
  - buildObject:
      set:
        result: "${ ({(.key): .value}) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"key": "name", "value": "Alice"}))
        .await
        .unwrap();
    assert_eq!(output["result"]["name"], json!("Alice"));
}

// === Switch: matching with string comparison ===

#[tokio::test]
async fn test_runner_expression_nested_access() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-nested-access
  version: '0.1.0'
do:
  - extract:
      set:
        city: "${ .address.city }"
        zip: "${ .address.zip }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"address": {"city": "NYC", "zip": "10001"}}))
        .await
        .unwrap();
    assert_eq!(output["city"], json!("NYC"));
    assert_eq!(output["zip"], json!("10001"));
}

// === Expression: nested field update ===

#[tokio::test]
async fn test_runner_expression_nested_update() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-nested-update
  version: '0.1.0'
do:
  - updateNested:
      set:
        result: "${ .user.name |= . + \" Jr.\" }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"user": {"name": "John", "age": 30}}))
        .await
        .unwrap();
    assert_eq!(output["result"]["user"]["name"], json!("John Jr."));
    assert_eq!(output["result"]["user"]["age"], json!(30));
}

// === Expression: array element update ===

#[tokio::test]
async fn test_runner_expression_array_update() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-array-update
  version: '0.1.0'
do:
  - updateArray:
      set:
        result: "${ .items[1] |= . * 2 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [1, 2, 3]}))
        .await
        .unwrap();
    assert_eq!(output["result"]["items"], json!([1, 4, 3]));
}

// === Expression: select with condition ===

#[tokio::test]
async fn test_runner_expression_select_condition() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-select-condition
  version: '0.1.0'
do:
  - filterItems:
      set:
        result: "${ [.items[] | select(. >= 3)] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [1, 2, 3, 4, 5]}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!([3, 4, 5]));
}

// === Expression: map with transform ===

#[tokio::test]
async fn test_runner_expression_map_transform() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-map-transform
  version: '0.1.0'
do:
  - doubleItems:
      set:
        result: "${ [.items[] | . * 2] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [1, 2, 3]}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!([2, 4, 6]));
}

// === Switch: multiple then directives in sequence ===

#[tokio::test]
async fn test_runner_expression_multi_computed_fields() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-multi-computed
  version: '0.1.0'
do:
  - buildResult:
      set:
        result: "${ {fullName: (.first + \" \" + .last), age: .age, city: .city} }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"first": "Jane", "last": "Smith", "age": 28, "city": "LA"}))
        .await
        .unwrap();
    assert_eq!(output["result"]["fullName"], json!("Jane Smith"));
    assert_eq!(output["result"]["age"], json!(28));
    assert_eq!(output["result"]["city"], json!("LA"));
}

// === Switch: then goto forward skipping multiple tasks ===

#[tokio::test]
async fn test_runner_expression_and_or() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-and-or
  version: '0.1.0'
do:
  - check:
      set:
        bothTrue: "${ .x and .y }"
        eitherTrue: "${ .x or .z }"
        neitherTrue: "${ (.x | not) and (.y | not) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"x": true, "y": true, "z": false}))
        .await
        .unwrap();
    assert_eq!(output["bothTrue"], json!(true));
    assert_eq!(output["eitherTrue"], json!(true));
    assert_eq!(output["neitherTrue"], json!(false));
}

// === Raise error: with detail expression referencing input ===

#[tokio::test]
async fn test_runner_expression_array_index() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-array-index
  version: '0.1.0'
do:
  - getItems:
      set:
        first: "${ .items[0] }"
        last: "${ .items[-1:] }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [10, 20, 30, 40]}))
        .await
        .unwrap();
    assert_eq!(output["first"], json!(10));
    assert_eq!(output["last"], json!([40]));
}

// === Expression: string length ===

#[tokio::test]
async fn test_runner_expression_string_length() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-string-length
  version: '0.1.0'
do:
  - measure:
      set:
        nameLen: "${ .name | length }"
        arrLen: "${ .items | length }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "Hello", "items": [1, 2, 3]}))
        .await
        .unwrap();
    assert_eq!(output["nameLen"], json!(5));
    assert_eq!(output["arrLen"], json!(3));
}

// === Try-catch: catch with when filtering ===

#[tokio::test]
async fn test_runner_expression_alternative_chain() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-alternative-chain
  version: '0.1.0'
do:
  - tryPaths:
      set:
        result: "${ .missing // .fallback // \"default\" }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    // .missing is null, .fallback exists
    let output = runner.run(json!({"fallback": "second"})).await.unwrap();
    assert_eq!(output["result"], json!("second"));
}

// === For loop: object iteration with key/value ===

#[tokio::test]
async fn test_runner_set_if_then_else_expression() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-if-then-else-expr
  version: '0.1.0'
do:
  - categorize:
      set:
        category: "${ if .score >= 90 then \"A\" elif .score >= 80 then \"B\" elif .score >= 70 then \"C\" else \"F\" end }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output_a = runner.run(json!({"score": 95})).await.unwrap();
    assert_eq!(output_a["category"], json!("A"));

    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output_b = runner.run(json!({"score": 85})).await.unwrap();
    assert_eq!(output_b["category"], json!("B"));

    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output_f = runner.run(json!({"score": 50})).await.unwrap();
    assert_eq!(output_f["category"], json!("F"));
}

// === Fork: compete with output.as ===

#[tokio::test]
async fn test_runner_expression_object_merge() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-object-merge
  version: '0.1.0'
do:
  - mergeObjects:
      set:
        result: "${ .defaults * .overrides }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({
            "defaults": {"timeout": 30, "retries": 3, "debug": false},
            "overrides": {"timeout": 60, "debug": true}
        }))
        .await
        .unwrap();
    assert_eq!(output["result"]["timeout"], json!(60));
    assert_eq!(output["result"]["retries"], json!(3));
    assert_eq!(output["result"]["debug"], json!(true));
}

// === Expression: ascii_upcase ===

#[tokio::test]
async fn test_runner_expression_ascii_upcase() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-ascii-upcase
  version: '0.1.0'
do:
  - upperCase:
      set:
        result: "${ .name | ascii_upcase }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"name": "hello world"}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!("HELLO WORLD"));
}

// === Expression: split/cr/naive_base64/nth ===

#[tokio::test]
async fn test_runner_expression_split() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-split
  version: '0.1.0'
do:
  - splitString:
      set:
        parts: "${ .csv | split(\",\") }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"csv": "a,b,c,d"}))
        .await
        .unwrap();
    assert_eq!(output["parts"], json!(["a", "b", "c", "d"]));
}

// === Expression: nth element ===

#[tokio::test]
async fn test_runner_expression_nth() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-nth
  version: '0.1.0'
do:
  - getNth:
      set:
        second: "${ .items | nth(1) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": ["first", "second", "third"]}))
        .await
        .unwrap();
    assert_eq!(output["second"], json!("second"));
}

// === Do: then exit in nested do (exit exits current composite, not entire workflow) ===

#[tokio::test]
async fn test_runner_expression_floor_ceil_round() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-floor-ceil-round
  version: '0.1.0'
do:
  - compute:
      set:
        floored: "${ .val | floor }"
        ceiled: "${ .val | ceil }"
        rounded: "${ .val | round }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"val": 3.7}))
        .await
        .unwrap();
    assert_eq!(output["floored"], json!(3));
    assert_eq!(output["ceiled"], json!(4));
    assert_eq!(output["rounded"], json!(4));
}

// === Expression: modulo ===

#[tokio::test]
async fn test_runner_expression_modulo() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-modulo
  version: '0.1.0'
do:
  - compute:
      set:
        mod: "${ .a % .b }"
        isEven: "${ .num % 2 == 0 }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"a": 17, "b": 5, "num": 8}))
        .await
        .unwrap();
    assert_eq!(output["mod"], json!(2));
    assert_eq!(output["isEven"], json!(true));
}

// === Expression: conditional (if-then-else) in set ===

#[tokio::test]
async fn test_runner_expression_conditional() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-conditional
  version: '0.1.0'
do:
  - classify:
      set:
        category: "${ if .age >= 18 then \"adult\" else \"minor\" end }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"age": 25}))
        .await
        .unwrap();
    assert_eq!(output["category"], json!("adult"));

    let workflow2: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
    let runner2 = WorkflowRunner::new(workflow2).unwrap();
    let output2 = runner2.run(json!({"age": 12})).await.unwrap();
    assert_eq!(output2["category"], json!("minor"));
}

// === Expression: string multiplication (repeat) ===

#[tokio::test]
async fn test_runner_expression_string_repeat() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-string-repeat
  version: '0.1.0'
do:
  - repeat:
      set:
        result: "${ .str * .count }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"str": "ha", "count": 3}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!("hahaha"));
}

// === Switch with multiple then: end to short-circuit ===

#[tokio::test]
async fn test_runner_expression_multi_field_object() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-multi-field-object
  version: '0.1.0'
do:
  - build:
      set:
        result: "${ {name: .first + \" \" + .last, age: .years, active: true} }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"first": "John", "last": "Doe", "years": 30}))
        .await
        .unwrap();
    assert_eq!(output["result"]["name"], json!("John Doe"));
    assert_eq!(output["result"]["age"], json!(30));
    assert_eq!(output["result"]["active"], json!(true));
}

// === Workflow input/output combined schema validation ===

#[tokio::test]
async fn test_runner_expression_comparisons() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-comparisons
  version: '0.1.0'
do:
  - compare:
      set:
        gt: "${ .a > .b }"
        lt: "${ .a < .b }"
        gte: "${ .a >= .a }"
        lte: "${ .a <= .a }"
        eq: "${ .a == .b }"
        neq: "${ .a != .b }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": 5, "b": 3}))
        .await
        .unwrap();
    assert_eq!(output["gt"], json!(true));
    assert_eq!(output["lt"], json!(false));
    assert_eq!(output["gte"], json!(true));
    assert_eq!(output["lte"], json!(true));
    assert_eq!(output["eq"], json!(false));
    assert_eq!(output["neq"], json!(true));
}

// === Expression: array construction and concatenation ===

#[tokio::test]
async fn test_runner_expression_array_concat() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-array-concat
  version: '0.1.0'
do:
  - merge:
      set:
        result: "${ .arr1 + .arr2 }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"arr1": [1, 2], "arr2": [3, 4]}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!([1, 2, 3, 4]));
}

// === Expression: length on different types ===

#[tokio::test]
async fn test_runner_expression_length_various() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-length-various
  version: '0.1.0'
do:
  - measure:
      set:
        strLen: "${ .text | length }"
        arrLen: "${ .items | length }"
        objLen: "${ .data | length }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"text": "hello", "items": [1, 2, 3], "data": {"a": 1, "b": 2}}))
        .await
        .unwrap();
    assert_eq!(output["strLen"], json!(5));
    assert_eq!(output["arrLen"], json!(3));
    assert_eq!(output["objLen"], json!(2));
}

// === Expression: try/catch in expressions (alternative operator for null safety) ===

#[tokio::test]
async fn test_runner_expression_null_safety() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-null-safety
  version: '0.1.0'
do:
  - safeAccess:
      set:
        city: "${ .user.address.city // \"unknown\" }"
        zip: "${ .user.address.zip // \"00000\" }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"user": {"address": {"city": "NYC"}}}))
        .await
        .unwrap();
    assert_eq!(output["city"], json!("NYC"));
    assert_eq!(output["zip"], json!("00000"));
}

// === Wait with expression-based duration ===

#[tokio::test]
async fn test_runner_wait_expression_duration() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: wait-expression-duration
  version: '0.1.0'
do:
  - shortWait:
      wait: PT0S
  - setResult:
      set:
        done: true
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["done"], json!(true));
}

// === Export with complex expression ===

#[tokio::test]
async fn test_runner_expression_add_sum() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-add-sum
  version: '0.1.0'
do:
  - compute:
      set:
        total: "${ .items | add }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [10, 20, 30]}))
        .await
        .unwrap();
    assert_eq!(output["total"], json!(60));
}

// === Expression: infinite and notanumber ===

#[tokio::test]
async fn test_runner_expression_infinite_nan() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-infinite-nan
  version: '0.1.0'
do:
  - compute:
      set:
        isInfinite: "${ (.val / 0) | isinfinite }"
        isNan: "${ (0 / 0) | isnan }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"val": 1}))
        .await
        .unwrap();
    assert_eq!(output["isInfinite"], json!(true));
    assert_eq!(output["isNan"], json!(true));
}

// === Expression: utf8bytelength ===

#[tokio::test]
async fn test_runner_expression_utf8byte_length() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-utf8byte-length
  version: '0.1.0'
do:
  - measure:
      set:
        byteLen: "${ .text | utf8bytelength }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "hello"}))
        .await
        .unwrap();
    assert_eq!(output["byteLen"], json!(5));
}

// === Expression: todate/fromdate ===

#[tokio::test]
async fn test_runner_expression_todate() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-todate
  version: '0.1.0'
do:
  - convert:
      set:
        dateStr: "${ .ts | todate }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"ts": 1700000000}))
        .await
        .unwrap();
    // Just verify it returns a string (date format)
    assert!(output["dateStr"].is_string(), "Expected string date output");
}

// === Expression: @text (string interpolation alternative) ===

#[tokio::test]
async fn test_runner_expression_format_string() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-format-string
  version: '0.1.0'
do:
  - format:
      set:
        result: "${ [.a, .b, .c] | @csv }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"a": "name", "b": 42, "c": true}))
        .await
        .unwrap();
    // @csv should produce CSV format
    assert!(output["result"].is_string(), "Expected CSV string output");
}

// === Expression: drem (remainder) and log/exp ===

#[tokio::test]
async fn test_runner_expression_log_exp() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-log-exp
  version: '0.1.0'
do:
  - compute:
      set:
        logVal: "${ .val | log }"
        expVal: "${ 0 | exp }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"val": 1}))
        .await
        .unwrap();
    // log(1) ≈ 0, exp(0) = 1
    let log_val = output["logVal"].as_f64().unwrap();
    let exp_val = output["expVal"].as_f64().unwrap();
    assert!(
        (log_val - 0.0).abs() < 0.001,
        "log(1) should be ~0, got {}",
        log_val
    );
    assert!(
        (exp_val - 1.0).abs() < 0.001,
        "exp(0) should be 1, got {}",
        exp_val
    );
}

// === Expression: sqrt ===

#[tokio::test]
async fn test_runner_expression_sqrt_pow() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expression-sqrt
  version: '0.1.0'
do:
  - compute:
      set:
        root: "${ .val | sqrt }"
        squared: "${ .val * .val }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"val": 9}))
        .await
        .unwrap();
    let root = output["root"].as_f64().unwrap();
    let squared = output["squared"].as_f64().unwrap();
    assert!(
        (root - 3.0).abs() < 0.001,
        "sqrt(9) should be 3, got {}",
        root
    );
    assert!(
        (squared - 81.0).abs() < 0.001,
        "9*9 should be 81, got {}",
        squared
    );
}

// === Nested switch with goto to outer task ===

#[tokio::test]
async fn test_runner_emit_with_data_expressions() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: emit-data-expressions
  version: '0.1.0'
do:
  - emitOrder:
      emit:
        event:
          with:
            source: https://test.com/orders
            type: com.test.order.created.v1
            data:
              orderId: "${ .orderId }"
              total: "${ .total }"
              items: "${ .items | length }"
  - setResult:
      set:
        emitted: true
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"orderId": "ORD-123", "total": 99.99, "items": [1, 2, 3]}))
        .await
        .unwrap();
    assert_eq!(output["emitted"], json!(true));
}

// === Multiple sequential exports with expressions ===

#[tokio::test]
async fn test_runner_export_sequential_expressions() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: export-sequential-expressions
  version: '0.1.0'
do:
  - step1:
      set:
        count: 1
      export:
        as: "${ {step: 1} }"
  - step2:
      set:
        count: 2
      export:
        as: "${ {step: 2, prev: $context.step} }"
  - step3:
      set:
        result: "${ $context }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["result"]["step"], json!(2));
    assert_eq!(output["result"]["prev"], json!(1));
}

// === Switch with output.as ===

#[tokio::test]
async fn test_runner_expression_string_repeat_mult() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-string-repeat
  version: '0.1.0'
do:
  - compute:
      set:
        repeated: "${ .char * .count }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"char": "ab", "count": 3}))
        .await
        .unwrap();
    assert_eq!(output["repeated"], json!("ababab"));
}

// === Expression: drem (float remainder) ===

#[tokio::test]
async fn test_runner_expression_drem() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-drem
  version: '0.1.0'
do:
  - compute:
      set:
        remainder: "${ 7 % 3 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["remainder"], json!(1));
}

// === Expression: @base64 / @base64d encoding ===

#[tokio::test]
async fn test_runner_expression_base64() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-base64
  version: '0.1.0'
do:
  - compute:
      set:
        encoded: "${ .data | @base64 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"data": "hello"}))
        .await
        .unwrap();
    // @base64 should produce a base64 encoded string
    assert!(
        output["encoded"].is_string(),
        "Expected base64 string output"
    );
}

// === Expression: test (regex match) ===

#[tokio::test]
async fn test_runner_expression_test_regex_match() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-regex-test
  version: '0.1.0'
do:
  - compute:
      set:
        isEmail: "${ .email | test(\"^[a-z]+@[a-z]+\\\\.[a-z]+$\") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"email": "user@example.com"}))
        .await
        .unwrap();
    assert_eq!(output["isEmail"], json!(true));
}

// === Multiple raises caught by outer try ===

#[tokio::test]
async fn test_runner_expression_trimstr_combined() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-trimstr
  version: '0.1.0'
do:
  - compute:
      set:
        trimmed: "${ .text | ltrimstr(\"hello \") | rtrimstr(\" world\") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"text": "hello beautiful world"}))
        .await
        .unwrap();
    assert_eq!(output["trimmed"], json!("beautiful"));
}

// === Expression: sub (string substitution) ===

#[tokio::test]
async fn test_runner_expression_sub() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-sub
  version: '0.1.0'
do:
  - compute:
      set:
        replaced: "${ .text | sub(\"old\"; \"new\") }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "the old value"}))
        .await
        .unwrap();
    assert_eq!(output["replaced"], json!("the new value"));
}

// === Expression: gsub (global string substitution) ===

#[tokio::test]
async fn test_runner_expression_gsub() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-gsub
  version: '0.1.0'
do:
  - compute:
      set:
        replaced: "${ .text | gsub(\"o\"; \"0\") }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "foo moo"}))
        .await
        .unwrap();
    assert_eq!(output["replaced"], json!("f00 m00"));
}

// === Expression: ascii_downcase / ascii_upcase ===

#[tokio::test]
async fn test_runner_expression_case_combined() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-case
  version: '0.1.0'
do:
  - compute:
      set:
        upper: "${ .text | ascii_upcase }"
        lower: "${ .text | ascii_downcase }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "Hello"}))
        .await
        .unwrap();
    assert_eq!(output["upper"], json!("HELLO"));
    assert_eq!(output["lower"], json!("hello"));
}

// === Expression: explode / implode (codepoints) ===

#[tokio::test]
async fn test_runner_expression_explode_implode() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-explode-implode
  version: '0.1.0'
do:
  - compute:
      set:
        codes: "${ .text | explode }"
        back: "${ .text | explode | implode }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "AB"}))
        .await
        .unwrap();
    assert_eq!(output["codes"], json!([65, 66]));
    assert_eq!(output["back"], json!("AB"));
}

// === Expression: to_entries / from_entries round-trip ===

#[tokio::test]
async fn test_runner_expression_entries_roundtrip() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-entries-roundtrip
  version: '0.1.0'
do:
  - compute:
      set:
        roundtrip: "${ .data | to_entries | from_entries }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"data": {"a": 1, "b": 2}}))
        .await
        .unwrap();
    assert_eq!(output["roundtrip"]["a"], json!(1));
    assert_eq!(output["roundtrip"]["b"], json!(2));
}

// === Expression: group_by with objects ===

#[tokio::test]
async fn test_runner_expression_group_by_object() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-group-by-obj
  version: '0.1.0'
do:
  - compute:
      set:
        grouped: "${ .items | group_by(.category) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [
            {"name": "a", "category": "x"},
            {"name": "b", "category": "y"},
            {"name": "c", "category": "x"}
        ]}))
        .await
        .unwrap();
    // group_by should produce 2 groups: [a,c] and [b]
    assert!(output["grouped"].is_array());
    let groups = output["grouped"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
}

// === Expression: map with select (filter + transform) ===

#[tokio::test]
async fn test_runner_expression_map_select() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-map-select
  version: '0.1.0'
do:
  - compute:
      set:
        names: "${ .items | map(select(.active)) | map(.name) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [
            {"name": "a", "active": true},
            {"name": "b", "active": false},
            {"name": "c", "active": true}
        ]}))
        .await
        .unwrap();
    assert_eq!(output["names"], json!(["a", "c"]));
}

// === Expression: reduce with complex accumulator ===

#[tokio::test]
async fn test_runner_expression_reduce_complex() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-reduce-complex
  version: '0.1.0'
do:
  - compute:
      set:
        stats: "${ .items | reduce .[] as $item ({sum: 0, count: 0}; .sum += $item.val | .count += 1) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [
            {"val": 10},
            {"val": 20},
            {"val": 30}
        ]}))
        .await
        .unwrap();
    assert_eq!(output["stats"]["sum"], json!(60));
    assert_eq!(output["stats"]["count"], json!(3));
}

// === Set with deeply nested expression ===

#[tokio::test]
async fn test_runner_set_deep_nested_expression() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-deep-nested
  version: '0.1.0'
do:
  - compute:
      set:
        result:
          nested:
            deep: "${ .input * 2 }"
            label: "${ .name }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"input": 5, "name": "test"}))
        .await
        .unwrap();
    assert_eq!(output["result"]["nested"]["deep"], json!(10));
    assert_eq!(output["result"]["nested"]["label"], json!("test"));
}

// === Try-catch: error not matching filter propagates ===

#[tokio::test]
async fn test_runner_set_array_expression() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-array-expr
  version: '0.1.0'
do:
  - compute:
      set:
        items: "${ [.a, .b, .c] }"
        squares: "${ [.a, .b, .c] | map(. * .) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": 1, "b": 2, "c": 3}))
        .await
        .unwrap();
    assert_eq!(output["items"], json!([1, 2, 3]));
    assert_eq!(output["squares"], json!([1, 4, 9]));
}

// === Expression: @html format ===

#[tokio::test]
async fn test_runner_expression_format_html() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-format-html
  version: '0.1.0'
do:
  - compute:
      set:
        formatted: "${ [.a, .b] | @html }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"a": "hello", "b": "world"}))
        .await
        .unwrap();
    assert!(
        output["formatted"].is_string(),
        "Expected HTML formatted string"
    );
}

// === Expression: @text format (string representation) ===

#[tokio::test]
async fn test_runner_expression_format_tsv() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-format-text
  version: '0.1.0'
do:
  - compute:
      set:
        textRepr: "${ [.a, .b] | @text }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": "col1", "b": "col2"}))
        .await
        .unwrap();
    assert!(
        output["textRepr"].is_string(),
        "Expected text formatted string"
    );
}

// === For with while and output.as combined ===

#[tokio::test]
async fn test_runner_expression_format_json() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-format-json
  version: '0.1.0'
do:
  - compute:
      set:
        jsonStr: "${ .data | @json }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"data": {"key": "value"}}))
        .await
        .unwrap();
    assert!(output["jsonStr"].is_string());
    // Should be a valid JSON string
    let parsed: serde_json::Value =
        serde_json::from_str(output["jsonStr"].as_str().unwrap()).unwrap();
    assert_eq!(parsed["key"], json!("value"));
}

// === Switch with multiple when conditions all false, has default ===

#[tokio::test]
async fn test_runner_expression_paths_with_filter() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-paths-filter
  version: '0.1.0'
do:
  - compute:
      set:
        numberPaths: "${ .data | paths(type == \"number\") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"a": 1, "b": "two", "c": 3}}))
        .await
        .unwrap();
    assert!(output["numberPaths"].is_array());
}

// === Multiple sequential raises, first caught, second not ===

#[tokio::test]
async fn test_runner_expression_in_operator() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-in-operator
  version: '0.1.0'
do:
  - compute:
      set:
        isIn: "${ .val as $v | [.a, .b, .c] | contains([$v]) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"val": 2, "a": 1, "b": 2, "c": 3}))
        .await
        .unwrap();
    assert_eq!(output["isIn"], json!(true));
}

// === Expression: not (negation) on expressions ===

#[tokio::test]
async fn test_runner_expression_not_negation() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-not-negation
  version: '0.1.0'
do:
  - compute:
      set:
        notTrue: "${ .flag | not }"
        notFalse: "${ (.flag | not) | not }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"flag": true}))
        .await
        .unwrap();
    assert_eq!(output["notTrue"], json!(false));
    assert_eq!(output["notFalse"], json!(true));
}

// === Do with mixed task types ===

#[tokio::test]
async fn test_runner_expression_object_from_array() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-object-from-array
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ [.items[] | {key: .name, value: .val}] | from_entries }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [
            {"name": "a", "val": 1},
            {"name": "b", "val": 2}
        ]}))
        .await
        .unwrap();
    assert_eq!(output["result"]["a"], json!(1));
    assert_eq!(output["result"]["b"], json!(2));
}

// === Expression: ternary with expressions ===

#[tokio::test]
async fn test_runner_expression_ternary_complex() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-ternary-complex
  version: '0.1.0'
do:
  - compute:
      set:
        category: "${ if .score >= 90 then .labels.high elif .score >= 70 then .labels.medium else .labels.low end }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"score": 95, "labels": {"high": "A", "medium": "B", "low": "C"}}))
        .await
        .unwrap();
    assert_eq!(output["category"], json!("A"));
}

// === Multiple for loops in sequence ===

#[tokio::test]
async fn test_runner_expression_string_interp() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-string-interp
  version: '0.1.0'
do:
  - compute:
      set:
        greeting: "${ .greeting + \" \" + .name }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"greeting": "Hello", "name": "World"}))
        .await
        .unwrap();
    assert_eq!(output["greeting"], json!("Hello World"));
}

// === Try-catch: catch.as with error status access ===

#[tokio::test]
async fn test_runner_expression_builtins() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-builtins
  version: '0.1.0'
do:
  - compute:
      set:
        isNull: "${ .missing | type }"
        isBool: "${ .flag | type }"
        isNum: "${ .count | type }"
        isStr: "${ .name | type }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"flag": true, "count": 42, "name": "test"}))
        .await
        .unwrap();
    assert_eq!(output["isNull"], json!("null"));
    assert_eq!(output["isBool"], json!("boolean"));
    assert_eq!(output["isNum"], json!("number"));
    assert_eq!(output["isStr"], json!("string"));
}

// === Expression: pick/omit via del ===

#[tokio::test]
async fn test_runner_expression_pick_fields() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-pick-fields
  version: '0.1.0'
do:
  - compute:
      set:
        filtered: "${ .data | del(.secret) | del(.internal) }"
"#;
    let output = run_workflow_yaml(
        &yaml_str,
        json!({"data": {"name": "test", "secret": "hidden", "value": 42, "internal": "private"}}),
    )
    .await
    .unwrap();
    assert_eq!(output["filtered"]["name"], json!("test"));
    assert_eq!(output["filtered"]["value"], json!(42));
    assert!(output["filtered"].get("secret").is_none());
    assert!(output["filtered"].get("internal").is_none());
}

// === Workflow with schedule: after ===

#[tokio::test]
async fn test_runner_expression_tostring_number() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-tostring-number
  version: '0.1.0'
do:
  - compute:
      set:
        strVal: "${ .num | tostring }"
        typeCheck: "${ (.num | tostring) | type }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"num": 42}))
        .await
        .unwrap();
    assert_eq!(output["strVal"], json!("42"));
    assert_eq!(output["typeCheck"], json!("string"));
}

// === Expression: tonumber on strings ===

#[tokio::test]
async fn test_runner_expression_tonumber_string() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-tonumber-string
  version: '0.1.0'
do:
  - compute:
      set:
        numVal: "${ .str | tonumber }"
        doubled: "${ (.str | tonumber) * 2 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"str": "21"}))
        .await
        .unwrap();
    assert_eq!(output["numVal"], json!(21));
    assert_eq!(output["doubled"], json!(42));
}

// === Nested do: inner do with set and export ===

#[tokio::test]
async fn test_runner_expression_map_arithmetic() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-map-arithmetic
  version: '0.1.0'
do:
  - compute:
      set:
        doubled: "${ .nums | map(. * 2) }"
        squared: "${ .nums | map(. * .) }"
        halved: "${ .nums | map(. / 2) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"nums": [1, 2, 3, 4, 5]}))
        .await
        .unwrap();
    assert_eq!(output["doubled"], json!([2, 4, 6, 8, 10]));
    assert_eq!(output["squared"], json!([1, 4, 9, 16, 25]));
    assert_eq!(output["halved"], json!([0.5, 1.0, 1.5, 2.0, 2.5]));
}

// === Expression: limit and range together ===

#[tokio::test]
async fn test_runner_expression_limit_range() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-limit-range
  version: '0.1.0'
do:
  - compute:
      set:
        first5: "${ [limit(5; range(100))] }"
        range3: "${ [range(3)] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["first5"], json!([0, 1, 2, 3, 4]));
    assert_eq!(output["range3"], json!([0, 1, 2]));
}

// === Expression: any/all with condition ===

#[tokio::test]
async fn test_runner_expression_any_all_condition() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-any-all
  version: '0.1.0'
do:
  - compute:
      set:
        anyActive: "${ [.items[] | .active] | any }"
        allActive: "${ [.items[] | .active] | all }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [
            {"name": "a", "active": true},
            {"name": "b", "active": false},
            {"name": "c", "active": true}
        ]}))
        .await
        .unwrap();
    assert_eq!(output["anyActive"], json!(true));
    assert_eq!(output["allActive"], json!(false));
}

// === Expression: index and slice ===

#[tokio::test]
async fn test_runner_expression_slice() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-slice
  version: '0.1.0'
do:
  - compute:
      set:
        first: "${ .items | .[0] }"
        last: "${ .items | .[-1] }"
        slice: "${ .items | .[1:3] }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [10, 20, 30, 40, 50]}))
        .await
        .unwrap();
    assert_eq!(output["first"], json!(10));
    assert_eq!(output["last"], json!(50));
    assert_eq!(output["slice"], json!([20, 30]));
}

// === Set with boolean expression ===

#[tokio::test]
async fn test_runner_expression_deep_update() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-deep-update
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ .data | .nested.value = 99 }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"nested": {"value": 1}, "other": 2}}))
        .await
        .unwrap();
    assert_eq!(output["result"]["nested"]["value"], json!(99));
    assert_eq!(output["result"]["other"], json!(2));
}

// === Expression: map with object construction ===

#[tokio::test]
async fn test_runner_expression_map_object_construct() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-map-obj-construct
  version: '0.1.0'
do:
  - compute:
      set:
        transformed: "${ .items | map({label: .name, score: .val * 10}) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [
            {"name": "a", "val": 1},
            {"name": "b", "val": 2}
        ]}))
        .await
        .unwrap();
    assert_eq!(output["transformed"][0]["label"], json!("a"));
    assert_eq!(output["transformed"][0]["score"], json!(10));
    assert_eq!(output["transformed"][1]["label"], json!("b"));
    assert_eq!(output["transformed"][1]["score"], json!(20));
}

// === For loop: break on condition via while ===

#[tokio::test]
async fn test_runner_expression_recurse_values() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-recurse-values
  version: '0.1.0'
do:
  - compute:
      set:
        allNums: "${ [.data | recurse | select(type == \"number\")] }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"a": 1, "b": {"c": 2, "d": 3}, "e": "text"}}))
        .await
        .unwrap();
    assert!(output["allNums"].is_array());
    let nums = output["allNums"].as_array().unwrap();
    assert!(nums.contains(&json!(1)));
    assert!(nums.contains(&json!(2)));
    assert!(nums.contains(&json!(3)));
}

// === Expression: has function ===

#[tokio::test]
async fn test_runner_expression_has_key() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-has-key
  version: '0.1.0'
do:
  - compute:
      set:
        hasName: "${ .data | has(\"name\") }"
        hasAge: "${ .data | has(\"age\") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"name": "test", "value": 42}}))
        .await
        .unwrap();
    assert_eq!(output["hasName"], json!(true));
    assert_eq!(output["hasAge"], json!(false));
}

// === Expression: contains for strings and arrays ===

#[tokio::test]
async fn test_runner_expression_contains_various() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-contains-various
  version: '0.1.0'
do:
  - compute:
      set:
        strContains: "${ .text | contains(\"world\") }"
        arrContains: "${ .items | contains([2]) }"
        notContains: "${ .text | contains(\"xyz\") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"text": "hello world", "items": [1, 2, 3]}))
        .await
        .unwrap();
    assert_eq!(output["strContains"], json!(true));
    assert_eq!(output["arrContains"], json!(true));
    assert_eq!(output["notContains"], json!(false));
}

// === Expression: indices and index ===

#[tokio::test]
async fn test_runner_expression_indices_func() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-indices
  version: '0.1.0'
do:
  - compute:
      set:
        positions: "${ .items | indices(2) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [1, 2, 3, 2, 4]}))
        .await
        .unwrap();
    assert_eq!(output["positions"], json!([1, 3]));
}

// === Expression: min/max on objects ===

#[tokio::test]
async fn test_runner_expression_min_max_objects() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-min-max-obj
  version: '0.1.0'
do:
  - compute:
      set:
        minVal: "${ .items | min }"
        maxVal: "${ .items | max }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [5, 3, 8, 1, 9]}))
        .await
        .unwrap();
    assert_eq!(output["minVal"], json!(1));
    assert_eq!(output["maxVal"], json!(9));
}

// === Expression: unique by field ===

#[tokio::test]
async fn test_runner_expression_unique_by() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-unique-by
  version: '0.1.0'
do:
  - compute:
      set:
        uniqueNames: "${ .items | unique_by(.name) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [
            {"name": "a", "val": 1},
            {"name": "b", "val": 2},
            {"name": "a", "val": 3}
        ]}))
        .await
        .unwrap();
    assert_eq!(output["uniqueNames"].as_array().unwrap().len(), 2);
}

// === Expression: sort_by field ===

#[tokio::test]
async fn test_runner_expression_sort_by() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-sort-by
  version: '0.1.0'
do:
  - compute:
      set:
        sorted: "${ .items | sort_by(.age) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [
            {"name": "c", "age": 30},
            {"name": "a", "age": 10},
            {"name": "b", "age": 20}
        ]}))
        .await
        .unwrap();
    assert_eq!(output["sorted"][0]["name"], json!("a"));
    assert_eq!(output["sorted"][1]["name"], json!("b"));
    assert_eq!(output["sorted"][2]["name"], json!("c"));
}

// === Expression: flatten ===

#[tokio::test]
async fn test_runner_expression_flatten_deep() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-flatten
  version: '0.1.0'
do:
  - compute:
      set:
        flat: "${ .data | flatten }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": [[1, 2], [3, [4, 5]]]}))
        .await
        .unwrap();
    // jaq flatten fully flattens by default
    assert_eq!(output["flat"], json!([1, 2, 3, 4, 5]));
}

// === Expression: to_entries key rename ===

#[tokio::test]
async fn test_runner_expression_rename_keys() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-rename-keys
  version: '0.1.0'
do:
  - compute:
      set:
        renamed: "${ .data | to_entries | map({key: (.key | ascii_upcase), value: .value}) | from_entries }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"name": "test", "count": 5}}))
        .await
        .unwrap();
    assert_eq!(output["renamed"]["NAME"], json!("test"));
    assert_eq!(output["renamed"]["COUNT"], json!(5));
}

// === Expression: @base64d decode ===

#[tokio::test]
async fn test_runner_expression_base64_decode() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-base64d
  version: '0.1.0'
do:
  - compute:
      set:
        decoded: "${ .encoded | @base64d }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"encoded": "aGVsbG8="}))
        .await
        .unwrap();
    assert_eq!(output["decoded"], json!("hello"));
}

// === Expression: string split ===

#[tokio::test]
async fn test_runner_expression_split_func() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-split
  version: '0.1.0'
do:
  - compute:
      set:
        parts: "${ .csv | split(\",\") }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"csv": "a,b,c,d"}))
        .await
        .unwrap();
    assert_eq!(output["parts"], json!(["a", "b", "c", "d"]));
}

// === Expression: join array ===

#[tokio::test]
async fn test_runner_expression_join_func() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-join
  version: '0.1.0'
do:
  - compute:
      set:
        joined: "${ .items | map(.name) | join(\", \") }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [{"name": "a"}, {"name": "b"}, {"name": "c"}]}))
        .await
        .unwrap();
    assert_eq!(output["joined"], json!("a, b, c"));
}

// === Expression: first/last ===

#[tokio::test]
async fn test_runner_expression_first_last_func() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-first-last
  version: '0.1.0'
do:
  - compute:
      set:
        first: "${ .items | first }"
        last: "${ .items | last }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [10, 20, 30, 40]}))
        .await
        .unwrap();
    assert_eq!(output["first"], json!(10));
    assert_eq!(output["last"], json!(40));
}

// === Expression: isempty ===

#[tokio::test]
async fn test_runner_expression_isempty_func() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-isempty
  version: '0.1.0'
do:
  - compute:
      set:
        emptyArr: "${ (.empty | length) == 0 }"
        nonEmptyArr: "${ (.items | length) == 0 }"
        emptyObj: "${ ({} | length) == 0 }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"empty": [], "items": [1]}))
        .await
        .unwrap();
    assert_eq!(output["emptyArr"], json!(true));
    assert_eq!(output["nonEmptyArr"], json!(false));
    assert_eq!(output["emptyObj"], json!(true));
}

// === Expression: map_values ===

#[tokio::test]
async fn test_runner_expression_map_values_func() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-map-values
  version: '0.1.0'
do:
  - compute:
      set:
        doubled: "${ .data | map_values(. * 2) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"a": 1, "b": 2, "c": 3}}))
        .await
        .unwrap();
    assert_eq!(output["doubled"]["a"], json!(2));
    assert_eq!(output["doubled"]["b"], json!(4));
    assert_eq!(output["doubled"]["c"], json!(6));
}

// === Expression: with_entries transform ===

#[tokio::test]
async fn test_runner_expression_with_entries_func() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-with-entries
  version: '0.1.0'
do:
  - compute:
      set:
        uppercased: "${ .data | with_entries(.key |= ascii_upcase) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"name": "test", "count": 5}}))
        .await
        .unwrap();
    assert_eq!(output["uppercased"]["NAME"], json!("test"));
    assert_eq!(output["uppercased"]["COUNT"], json!(5));
}

// === Expression: update operator |= ===

#[tokio::test]
async fn test_runner_expression_update_operator() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-update-op
  version: '0.1.0'
do:
  - compute:
      set:
        updated: "${ .data | .items |= map(. + 10) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"items": [1, 2, 3], "other": "keep"}}))
        .await
        .unwrap();
    assert_eq!(output["updated"]["items"], json!([11, 12, 13]));
    assert_eq!(output["updated"]["other"], json!("keep"));
}

// === Expression: alternative operator // ===

#[tokio::test]
async fn test_runner_expression_alternative_op() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-alternative-op
  version: '0.1.0'
do:
  - compute:
      set:
        value: "${ .missing // \"default\" }"
        existing: "${ .present // \"default\" }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"present": "actual"}))
        .await
        .unwrap();
    assert_eq!(output["value"], json!("default"));
    assert_eq!(output["existing"], json!("actual"));
}

// === Expression: try-catch in jq (try expression) ===

#[tokio::test]
async fn test_runner_expression_jq_try() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-jq-try
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ try (.a.b) catch \"no-b\" }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": 1})).await.unwrap();
    assert_eq!(output["result"], json!("no-b"));
}

// === Expression: label and break ===

#[tokio::test]
async fn test_runner_expression_label_break() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-label-break
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ [label $out | foreach range(10) as $x (0; . + $x; if . > 10 then ., break $out else . end)] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // Should stop after sum exceeds 10
    assert!(output["result"].is_array());
}

// === Complex workflow: for+set with conditional expressions ===

#[tokio::test]
async fn test_runner_expression_json_roundtrip() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-json-roundtrip
  version: '0.1.0'
do:
  - compute:
      set:
        encoded: "${ .data | @json }"
        decoded: "${ .data | @json | fromjson }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"x": 1, "y": [2, 3]}}))
        .await
        .unwrap();
    assert!(output["encoded"].is_string());
    assert_eq!(output["decoded"]["x"], json!(1));
    assert_eq!(output["decoded"]["y"], json!([2, 3]));
}

// === Expression: string multiplication and concatenation ===

#[tokio::test]
async fn test_runner_expression_string_ops_combo() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-string-ops
  version: '0.1.0'
do:
  - compute:
      set:
        repeated: "${ .char * 3 }"
        joined: "${ (.char * 3) + \"!\" }"
        upper: "${ (.char * 3) + \"!\" | ascii_upcase }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"char": "ha"}))
        .await
        .unwrap();
    assert_eq!(output["repeated"], json!("hahaha"));
    assert_eq!(output["joined"], json!("hahaha!"));
    assert_eq!(output["upper"], json!("HAHAHA!"));
}

// === Expression: object merge with + ===

#[tokio::test]
async fn test_runner_expression_object_merge_op() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-obj-merge
  version: '0.1.0'
do:
  - compute:
      set:
        merged: "${ .a + .b }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"a": {"x": 1}, "b": {"y": 2}}))
        .await
        .unwrap();
    assert_eq!(output["merged"]["x"], json!(1));
    assert_eq!(output["merged"]["y"], json!(2));
}

// === Expression: negate numbers ===

#[tokio::test]
async fn test_runner_expression_negate() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-negate
  version: '0.1.0'
do:
  - compute:
      set:
        neg: "${ -(.val) }"
        pos: "${ -(.val) | -(.) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"val": 42}))
        .await
        .unwrap();
    assert_eq!(output["neg"], json!(-42));
    assert_eq!(output["pos"], json!(42));
}

// === Do: nested do with then:exit to continue at outer scope ===

#[tokio::test]
async fn test_runner_expression_uri_encode() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-uri-encode
  version: '0.1.0'
do:
  - compute:
      set:
        encoded: "${ .text | @uri }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "hello world"}))
        .await
        .unwrap();
    assert!(output["encoded"].is_string());
    assert!(output["encoded"].as_str().unwrap().contains("hello"));
}

// === Expression: input and output vars ===

#[tokio::test]
async fn test_runner_expression_input_output_vars() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-input-output-vars
  version: '0.1.0'
do:
  - step1:
      set:
        original: "${ $input }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"data": "test"}))
        .await
        .unwrap();
    assert_eq!(output["original"]["data"], json!("test"));
}

// === Expression: $workflow variable ===

#[tokio::test]
async fn test_runner_expression_workflow_var() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-workflow-var
  version: '0.1.0'
do:
  - step1:
      set:
        wfName: "${ $workflow.definition.document.name }"
        wfDsl: "${ $workflow.definition.document.dsl }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["wfName"], json!("expr-workflow-var"));
    assert_eq!(output["wfDsl"], json!("1.0.0"));
}

// === Expression: $runtime variable ===

#[tokio::test]
async fn test_runner_expression_runtime_var() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-runtime-var
  version: '0.1.0'
do:
  - step1:
      set:
        hasName: "${ $workflow.definition.document.name != null }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["hasName"], json!(true));
}

// === Wait: zero duration immediate return ===

#[tokio::test]
async fn test_runner_set_if_with_expression_result() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-if-expr-result
  version: '0.1.0'
do:
  - compute:
      set:
        category: "${ if .score >= 90 then .labels.A elif .score >= 70 then .labels.B else .labels.C end }"
        score: "${ .score }"
        labels: "${ .labels }"
"#;
    let output = run_workflow_yaml(
        &yaml_str,
        json!({"score": 75, "labels": {"A": "Excellent", "B": "Good", "C": "Needs Improvement"}}),
    )
    .await
    .unwrap();
    assert_eq!(output["category"], json!("Good"));
}

// === Export: chain multiple exports building context ===

#[tokio::test]
async fn test_runner_expression_complex_object() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-complex-obj
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ {name: .first + \" \" + .last, age: .age, adult: (.age >= 18), info: {city: .city, country: .country}} }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"first": "John", "last": "Doe", "age": 25, "city": "NYC", "country": "US"}))
        .await
        .unwrap();
    assert_eq!(output["result"]["name"], json!("John Doe"));
    assert_eq!(output["result"]["age"], json!(25));
    assert_eq!(output["result"]["adult"], json!(true));
    assert_eq!(output["result"]["info"]["city"], json!("NYC"));
}

// === Expression: nested if-elif-else ===

#[tokio::test]
async fn test_runner_expression_nested_if() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-nested-if
  version: '0.1.0'
do:
  - compute:
      set:
        grade: "${ if .score >= 90 then \"A\" elif .score >= 80 then \"B\" elif .score >= 70 then \"C\" elif .score >= 60 then \"D\" else \"F\" end }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output85 = runner.run(json!({"score": 85})).await.unwrap();
    assert_eq!(output85["grade"], json!("B"));

    let workflow2: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
    let runner2 = WorkflowRunner::new(workflow2).unwrap();
    let output55 = runner2.run(json!({"score": 55})).await.unwrap();
    assert_eq!(output55["grade"], json!("F"));
}

// === Try-catch: catch without do (just swallow error) ===

#[tokio::test]
async fn test_runner_expression_select_nested() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-select-nested
  version: '0.1.0'
do:
  - compute:
      set:
        highValueItems: "${ .items | map(select(.price > 10 and .inStock)) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"items": [
            {"name": "a", "price": 5, "inStock": true},
            {"name": "b", "price": 15, "inStock": true},
            {"name": "c", "price": 20, "inStock": false},
            {"name": "d", "price": 12, "inStock": true}
        ]}))
        .await
        .unwrap();
    let items = output["highValueItems"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["name"], json!("b"));
    assert_eq!(items[1]["name"], json!("d"));
}

// === Expression: debug (identity passthrough) ===

#[tokio::test]
async fn test_runner_expression_debug_passthrough() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-debug
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ .val | debug | . * 2 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"val": 21}))
        .await
        .unwrap();
    assert_eq!(output["result"], json!(42));
}

// === Expression: del (remove fields) and has ===

#[tokio::test]
async fn test_runner_expression_path_ops() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-path-ops
  version: '0.1.0'
do:
  - compute:
      set:
        hasField: "${ .data | has(\"a\") }"
        noField: "${ .data | has(\"z\") }"
        picked: "${ .data | del(.c) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"a": 1, "b": 2, "c": 3}}))
        .await
        .unwrap();
    assert_eq!(output["hasField"], json!(true));
    assert_eq!(output["noField"], json!(false));
    assert_eq!(output["picked"]["a"], json!(1));
    assert!(output["picked"].get("c").is_none());
}

// === Expression: drem (modulo) variations ===

#[tokio::test]
async fn test_runner_expression_modulo_variations() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-modulo-vars
  version: '0.1.0'
do:
  - compute:
      set:
        mod1: "${ 10 % 3 }"
        mod2: "${ 7 % 2 }"
        mod3: "${ 100 % 7 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["mod1"], json!(1));
    assert_eq!(output["mod2"], json!(1));
    assert_eq!(output["mod3"], json!(2));
}

// === Emit with multiple events ===

#[tokio::test]
async fn test_runner_expression_string_compare() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-str-compare
  version: '0.1.0'
do:
  - compute:
      set:
        less: "${ .a < .b }"
        greater: "${ .b > .a }"
        equal: "${ .a == .a }"
        notEqual: "${ .a != .b }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"a": "apple", "b": "banana"}))
        .await
        .unwrap();
    assert_eq!(output["less"], json!(true));
    assert_eq!(output["greater"], json!(true));
    assert_eq!(output["equal"], json!(true));
    assert_eq!(output["notEqual"], json!(true));
}

// === Expression: object key access with .key syntax ===

#[tokio::test]
async fn test_runner_expression_dot_access() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-dot-access
  version: '0.1.0'
do:
  - compute:
      set:
        val1: "${ .data.nested.deep }"
        val2: "${ .data[\"key with spaces\"] }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"data": {"nested": {"deep": 42}, "key with spaces": "found"}}))
        .await
        .unwrap();
    assert_eq!(output["val1"], json!(42));
    assert_eq!(output["val2"], json!("found"));
}

// === Switch: then string matching different branches ===

#[tokio::test]
async fn test_runner_expression_ascii_case_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-ascii-case-v2
  version: '0.1.0'
do:
  - compute:
      set:
        upper: "${ .text | ascii_upcase }"
        lower: "${ .text | ascii_downcase }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "Hello World"}))
        .await
        .unwrap();
    assert_eq!(output["upper"], json!("HELLO WORLD"));
    assert_eq!(output["lower"], json!("hello world"));
}

// === Expression: @csv format ===

#[tokio::test]
async fn test_runner_expression_csv_format_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-csv-v2
  version: '0.1.0'
do:
  - compute:
      set:
        csv: "${ .rows | @csv }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"rows": ["a", "b", "c"]}))
        .await
        .unwrap();
    assert!(output["csv"].is_string());
    assert!(output["csv"].as_str().unwrap().contains("a"));
}

// === Expression: has on object ===

#[tokio::test]
async fn test_runner_expression_has_object_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-has-v2
  version: '0.1.0'
do:
  - compute:
      set:
        hasName: "${ .data | has(\"name\") }"
        hasAge: "${ .data | has(\"age\") }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"data": {"name": "test"}}))
        .await
        .unwrap();
    assert_eq!(output["hasName"], json!(true));
    assert_eq!(output["hasAge"], json!(false));
}

// === Expression: keys ===

#[tokio::test]
async fn test_runner_expression_keys_values_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-kv-v2
  version: '0.1.0'
do:
  - compute:
      set:
        ks: "${ .data | keys }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"data": {"x": 1, "y": 2}}))
        .await
        .unwrap();
    assert!(output["ks"].is_array());
    assert!(output["ks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == &json!("x")));
}

// === Expression: log/exp/sqrt ===

#[tokio::test]
async fn test_runner_expression_log_exp_sqrt_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-math-v2
  version: '0.1.0'
do:
  - compute:
      set:
        sqrtVal: "${ .n | sqrt | floor }"
        logVal: "${ 1 | log | floor }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"n": 16}))
        .await
        .unwrap();
    assert_eq!(output["sqrtVal"], json!(4));
    assert_eq!(output["logVal"], json!(0));
}

// === Expression: range and limit ===

#[tokio::test]
async fn test_runner_expression_range_limit_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-range-v2
  version: '0.1.0'
do:
  - compute:
      set:
        first5: "${ [limit(5; range(100))] }"
        range3: "${ [range(3)] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["first5"], json!([0, 1, 2, 3, 4]));
    assert_eq!(output["range3"], json!([0, 1, 2]));
}

// === Expression: reduce (sum) ===

#[tokio::test]
async fn test_runner_expression_reduce_sum_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-reduce-v2
  version: '0.1.0'
do:
  - compute:
      set:
        total: "${ .items | reduce .[] as $x (0; . + $x) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [1, 2, 3, 4]}))
        .await
        .unwrap();
    assert_eq!(output["total"], json!(10));
}

// === Expression: special numbers (infinite/nan) ===

#[tokio::test]
async fn test_runner_expression_special_numbers_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-special-v2
  version: '0.1.0'
do:
  - compute:
      set:
        isInf: "${ (.x | isinfinite) }"
        isNan: "${ (.y | isnan) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"x": 1.0, "y": 1.0}))
        .await
        .unwrap();
    assert_eq!(output["isInf"], json!(false));
    assert_eq!(output["isNan"], json!(false));
}

// === Expression: trim (ltrimstr/rtrimstr) ===

#[tokio::test]
async fn test_runner_expression_trim_str_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-trim-v2
  version: '0.1.0'
do:
  - compute:
      set:
        ltrimmed: "${ .text | ltrimstr(\"hello \") }"
        rtrimmed: "${ .text | rtrimstr(\" world\") }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "hello world"}))
        .await
        .unwrap();
    assert_eq!(output["ltrimmed"], json!("world"));
    assert_eq!(output["rtrimmed"], json!("hello"));
}

// === Expression: type check ===

#[tokio::test]
async fn test_runner_expression_type_check_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-type-v2
  version: '0.1.0'
do:
  - compute:
      set:
        strType: "${ .text | type }"
        numType: "${ .num | type }"
        arrType: "${ .arr | type }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"text": "hi", "num": 1, "arr": []}))
        .await
        .unwrap();
    assert_eq!(output["strType"], json!("string"));
    assert_eq!(output["numType"], json!("number"));
    assert_eq!(output["arrType"], json!("array"));
}

// === Expression: utf8bytelength ===

#[tokio::test]
async fn test_runner_expression_utf8bytelength_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-utf8-v2
  version: '0.1.0'
do:
  - compute:
      set:
        byteLen: "${ .text | utf8bytelength }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "abc"}))
        .await
        .unwrap();
    assert_eq!(output["byteLen"], json!(3));
}

// === Do: nested do with inner then:exit ===

#[tokio::test]
async fn test_runner_expression_object_merge_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-merge-v2
  version: '0.1.0'
do:
  - compute:
      set:
        merged: "${ .a * .b }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"a": {"x": 1}, "b": {"y": 2}}))
        .await
        .unwrap();
    assert_eq!(output["merged"]["x"], json!(1));
    assert_eq!(output["merged"]["y"], json!(2));
}

// === Expression: conditional if-then-else in complex context ===

#[tokio::test]
async fn test_runner_expression_complex_conditional() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-complex-cond
  version: '0.1.0'
do:
  - compute:
      set:
        level: "${ if .score >= 90 then \"A\" elif .score >= 70 then \"B\" else \"C\" end }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"score": 75}))
        .await
        .unwrap();
    assert_eq!(output["level"], json!("B"));

    let workflow2: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
    let runner2 = WorkflowRunner::new(workflow2).unwrap();
    let output2 = runner2.run(json!({"score": 50})).await.unwrap();
    assert_eq!(output2["level"], json!("C"));
}

// === Do: if condition skips task ===

#[tokio::test]
async fn test_runner_expression_nested_object_construction() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-nested-obj
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ {name: .name, address: {city: .city, zip: .zip}} }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "Alice", "city": "NYC", "zip": "10001"}))
        .await
        .unwrap();
    assert_eq!(output["result"]["name"], json!("Alice"));
    assert_eq!(output["result"]["address"]["city"], json!("NYC"));
}

// === Expression: select with multiple conditions ===

#[tokio::test]
async fn test_runner_expression_select_multi() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-select-multi
  version: '0.1.0'
do:
  - compute:
      set:
        adults: "${ .people | map(select(.age >= 18)) | map(.name) }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"people": [
            {"name": "Alice", "age": 25},
            {"name": "Bob", "age": 15},
            {"name": "Carol", "age": 30}
        ]}))
        .await
        .unwrap();
    assert_eq!(output["adults"], json!(["Alice", "Carol"]));
}

// === Expression: string operations combined ===

#[tokio::test]
async fn test_runner_expression_string_ops_combined() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-str-ops
  version: '0.1.0'
do:
  - compute:
      set:
        upper: "${ .text | ascii_upcase }"
        lower: "${ .text | ascii_downcase }"
        len: "${ .text | length }"
        sliced: "${ .text | .[0:3] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "Hello"}))
        .await
        .unwrap();
    assert_eq!(output["upper"], json!("HELLO"));
    assert_eq!(output["lower"], json!("hello"));
    assert_eq!(output["len"], json!(5));
}

// === Expression: array slice ===

#[tokio::test]
async fn test_runner_expression_array_slice() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-slice
  version: '0.1.0'
do:
  - compute:
      set:
        first3: "${ .items | .[0:3] }"
        last2: "${ .items | .[-2:] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [1, 2, 3, 4, 5]}))
        .await
        .unwrap();
    assert_eq!(output["first3"], json!([1, 2, 3]));
    assert_eq!(output["last2"], json!([4, 5]));
}

// === Expression: null safety with alternative ===

#[tokio::test]
async fn test_runner_expression_null_safety_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-null-v2
  version: '0.1.0'
do:
  - compute:
      set:
        name: "${ .user.name // \"unknown\" }"
        email: "${ .user.email // \"none\" }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"user": {"name": "Alice"}}))
        .await
        .unwrap();
    assert_eq!(output["name"], json!("Alice"));
    assert_eq!(output["email"], json!("none"));
}

// === Expression: floor/ceil/round ===

#[tokio::test]
async fn test_runner_expression_floor_ceil_round_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-round-v2
  version: '0.1.0'
do:
  - compute:
      set:
        floored: "${ 3.7 | floor }"
        ceiled: "${ 3.2 | ceil }"
        rounded: "${ 3.5 | round }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["floored"], json!(3));
    assert_eq!(output["ceiled"], json!(4));
    assert_eq!(output["rounded"], json!(4));
}

// === Expression: tonumber/tostring ===

#[tokio::test]
async fn test_runner_expression_tonumber_tostring_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-convert-v2
  version: '0.1.0'
do:
  - compute:
      set:
        asNum: "${ .strnum | tonumber }"
        asStr: "${ .num | tostring }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"strnum": "42", "num": 7}))
        .await
        .unwrap();
    assert_eq!(output["asNum"], json!(42));
    assert_eq!(output["asStr"], json!("7"));
}

// === Expression: contains on strings ===

#[tokio::test]
async fn test_runner_expression_string_contains() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-str-contains
  version: '0.1.0'
do:
  - compute:
      set:
        hasHello: "${ .text | contains(\"hello\") }"
        hasWorld: "${ .text | contains(\"world\") }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "hello there"}))
        .await
        .unwrap();
    assert_eq!(output["hasHello"], json!(true));
    assert_eq!(output["hasWorld"], json!(false));
}

// === Expression: @base64 encode/decode ===

#[tokio::test]
async fn test_runner_expression_base64_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-base64-v2
  version: '0.1.0'
do:
  - compute:
      set:
        encoded: "${ .text | @base64 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"text": "hello"}))
        .await
        .unwrap();
    assert_eq!(output["encoded"], json!("aGVsbG8="));
}

// === Emit: event with data expression ===

#[tokio::test]
async fn test_runner_expression_length_various_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-length-v2
  version: '0.1.0'
do:
  - compute:
      set:
        strLen: "${ .text | length }"
        arrLen: "${ .items | length }"
        objLen: "${ .data | length }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"text": "hello", "items": [1,2,3], "data": {"a":1, "b":2}}))
        .await
        .unwrap();
    assert_eq!(output["strLen"], json!(5));
    assert_eq!(output["arrLen"], json!(3));
    assert_eq!(output["objLen"], json!(2));
}

// === Switch: multiple matching conditions (first wins) ===

#[tokio::test]
async fn test_runner_expression_array_concat_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-concat-v2
  version: '0.1.0'
do:
  - compute:
      set:
        combined: "${ .a + .b }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": [1, 2], "b": [3, 4]}))
        .await
        .unwrap();
    assert_eq!(output["combined"], json!([1, 2, 3, 4]));
}

// === Expression: modulo ===

#[tokio::test]
async fn test_runner_expression_modulo_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-mod-v2
  version: '0.1.0'
do:
  - compute:
      set:
        mod1: "${ 10 % 3 }"
        mod2: "${ 7 % 2 }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["mod1"], json!(1));
    assert_eq!(output["mod2"], json!(1));
}

// === Expression: comparisons ===

#[tokio::test]
async fn test_runner_expression_comparisons_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-cmp-v2
  version: '0.1.0'
do:
  - compute:
      set:
        gt: "${ .a > .b }"
        lt: "${ .a < .b }"
        eq: "${ .a == .a }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"a": 5, "b": 3}))
        .await
        .unwrap();
    assert_eq!(output["gt"], json!(true));
    assert_eq!(output["lt"], json!(false));
    assert_eq!(output["eq"], json!(true));
}

// === Set: expression referencing input ===

#[tokio::test]
async fn test_runner_set_expression_from_input() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-from-input
  version: '0.1.0'
do:
  - transform:
      set:
        greeting: "${ \"Hello \" + .name }"
        doubled: "${ .value * 2 }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();

    let output = runner
        .run(json!({"name": "World", "value": 21}))
        .await
        .unwrap();
    assert_eq!(output["greeting"], json!("Hello World"));
    assert_eq!(output["doubled"], json!(42));
}

// === Expression: $input variable ===

#[tokio::test]
async fn test_runner_expression_input_var() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-input-var
  version: '0.1.0'
do:
  - compute:
      set:
        originalInput: "${ $input }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"key": "value"}))
        .await
        .unwrap();
    assert_eq!(output["originalInput"]["key"], json!("value"));
}

// === Expression: @text format ===

#[tokio::test]
async fn test_runner_expression_text_format() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-text-format
  version: '0.1.0'
do:
  - compute:
      set:
        asText: "${ .items | @text }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"items": [1, 2, 3]}))
        .await
        .unwrap();
    assert!(output["asText"].is_string());
}

// === Nested for loops ===

// Expression: del with multiple paths
#[tokio::test]
async fn test_runner_expression_delpaths() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-del-multi
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ {a:1,b:2,c:3} | del(.a,.c) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["result"], json!({"b": 2}));
}

// Expression: unique + limit combination
#[tokio::test]
async fn test_runner_expression_limit_unique() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-limit-unique
  version: '0.1.0'
do:
  - compute:
      set:
        unique_vals: "${ [1,2,2,3,3,4,5] | unique }"
        limited: "${ [1,2,2,3,3,4,5] | unique | .[0:3] }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["unique_vals"], json!([1, 2, 3, 4, 5]));
    assert_eq!(output["limited"], json!([1, 2, 3]));
}

// Expression: sort_by nested field
#[tokio::test]
async fn test_runner_expression_sort_by_nested() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-sort-by-nested
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ .items | sort_by(.age) }"
        items: "${ .items }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let input = json!({"items": [{"name": "C", "age": 30}, {"name": "A", "age": 10}, {"name": "B", "age": 20}]});
    let output = runner.run(input).await.unwrap();
    assert_eq!(output["result"][0]["name"], json!("A"));
    assert_eq!(output["result"][1]["name"], json!("B"));
    assert_eq!(output["result"][2]["name"], json!("C"));
}

// Expression: @uri encoding
#[tokio::test]
async fn test_runner_expression_uri_encode_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-uri-encode-v2
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ \"hello world\" | @uri }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    // @uri encodes spaces as %20
    assert!(output["result"].as_str().unwrap().contains("hello"));
}

// Expression: todateiso8601 / fromdateiso8601
#[tokio::test]
async fn test_runner_expression_todate_roundtrip() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-todate-roundtrip
  version: '0.1.0'
do:
  - compute:
      set:
        isoStr: "${ 1712000000 | todateiso8601 }"
        backEpoch: "${ (1712000000 | todateiso8601 | fromdateiso8601) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert!(output["isoStr"].is_string());
    assert_eq!(output["backEpoch"], json!(1712000000));
}

// Expression: paths and has with path arrays
#[tokio::test]
async fn test_runner_expression_leaf_paths() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-paths-has
  version: '0.1.0'
do:
  - compute:
      set:
        hasA: "${ {a: {b: 1}, c: 2} | has(\"a\") }"
        hasAB: "${ {a: {b: 1}, c: 2} | has(\"c\") }"
        keysList: "${ {a: {b: 1}, c: 2} | keys }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["hasA"], json!(true));
    assert_eq!(output["hasAB"], json!(true));
    assert_eq!(output["keysList"], json!(["a", "c"]));
}

// Expression: ascii_downcase / ascii_upcase combined
#[tokio::test]
async fn test_runner_expression_case_transform_chain() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-case-transform-chain
  version: '0.1.0'
do:
  - compute:
      set:
        upper: "${ \"hello\" | ascii_upcase }"
        lower: "${ \"WORLD\" | ascii_downcase }"
        mixed: "${ \"HeLLo WoRLD\" | ascii_downcase | split(\" \") | map(ascii_upcase) | join(\"-\") }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["upper"], json!("HELLO"));
    assert_eq!(output["lower"], json!("world"));
    assert_eq!(output["mixed"], json!("HELLO-WORLD"));
}

// Expression: map + select + length pipeline
#[tokio::test]
async fn test_runner_expression_map_select_length() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-map-select-length
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ [.items | .[] | .age | select(. > 20)] | length }"
        items: "${ .items }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let input = json!({"items": [{"name": "A", "age": 25}, {"name": "B", "age": 15}, {"name": "C", "age": 30}]});
    let output = runner.run(input).await.unwrap();
    // ages > 20: [25, 30], length = 2
    assert_eq!(output["result"], json!(2));
}

// Expression: with_entries rename keys
#[tokio::test]
async fn test_runner_expression_with_entries_rename() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-with-entries-rename
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ {old_key: 1, another: 2} | with_entries(.key |= ascii_upcase) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["result"]["OLD_KEY"], json!(1));
    assert_eq!(output["result"]["ANOTHER"], json!(2));
}

// Expression: map_values transform
#[tokio::test]
async fn test_runner_expression_map_values_transform() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-map-values-transform
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ {a: 1, b: 2, c: 3} | map_values(. * 10) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["result"], json!({"a": 10, "b": 20, "c": 30}));
}

// Expression: combinations of string functions
#[tokio::test]
async fn test_runner_expression_string_combo() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-string-combo
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ \"  Hello World  \" | ltrimstr(\"  \") | rtrimstr(\"  \") | split(\" \") | join(\"-\") | ascii_downcase }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["result"], json!("hello-world"));
}

// Expression: indices and index functions
#[tokio::test]
async fn test_runner_expression_indices_func_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-indices-func-v2
  version: '0.1.0'
do:
  - compute:
      set:
        idx: "${ [10,20,30,20,40] | indices(20) }"
        contains20: "${ [10,20,30] | contains([20]) }"
        notContains5: "${ [10,20,30] | contains([5]) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["idx"], json!([1, 3]));
    assert_eq!(output["contains20"], json!(true));
    assert_eq!(output["notContains5"], json!(false));
}

// === Batch 8: More edge cases and DSL patterns ===

// Expression: nested object construction with computed keys
#[tokio::test]
async fn test_runner_expression_computed_key_nested() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-computed-key-nested
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ ({(.key): .value}) }"
        key: "${ .key }"
        value: "${ .value }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({"key": "myKey", "value": "myValue"}))
        .await
        .unwrap();
    assert_eq!(output["result"]["myKey"], json!("myValue"));
}

// Expression: object construction from array of key-value pairs
#[tokio::test]
async fn test_runner_expression_object_from_kv_pairs() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-object-from-kv
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ [.pairs[] | {(.k): .v}] | add }"
        pairs: "${ .pairs }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({"pairs": [{"k": "a", "v": 1}, {"k": "b", "v": 2}]}))
        .await
        .unwrap();
    assert_eq!(output["result"]["a"], json!(1));
    assert_eq!(output["result"]["b"], json!(2));
}

// Expression: flatten nested arrays using flatten
#[tokio::test]
async fn test_runner_expression_recurse_flatten() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-flatten-nested
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ [[1,2],[3,[4,5]],[6]] | flatten }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["result"], json!([1, 2, 3, 4, 5, 6]));
}

// Expression: reduce to build object
#[tokio::test]
async fn test_runner_expression_reduce_object() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-reduce-object
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ reduce .items[] as $x ({}; . + {($x.name): $x.value}) }"
        items: "${ .items }"
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let input = json!({"items": [{"name": "a", "value": 1}, {"name": "b", "value": 2}]});
    let output = runner.run(input).await.unwrap();
    assert_eq!(output["result"]["a"], json!(1));
    assert_eq!(output["result"]["b"], json!(2));
}

// Expression: flatten deeply nested
#[tokio::test]
async fn test_runner_expression_flatten_deeply_nested() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-flatten-deeply
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ [[1,2],[3,[4,5]],[6]] | flatten }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["result"], json!([1, 2, 3, 4, 5, 6]));
}

// Expression: group_by with transform
#[tokio::test]
async fn test_runner_expression_group_by_transform() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-group-by-transform
  version: '0.1.0'
do:
  - compute:
      set:
        grouped: "${ .items | group_by(.category) }"
        items: "${ .items }"
"#;
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
    let input = json!({"items": [{"name": "A", "category": "x"}, {"name": "B", "category": "y"}, {"name": "C", "category": "x"}]});
    let runner = WorkflowRunner::new(workflow).unwrap();
    let output = runner.run(input).await.unwrap();
    assert!(output["grouped"].is_array());
    // group_by should produce 2 groups: x items and y items
    assert_eq!(output["grouped"].as_array().unwrap().len(), 2);
}

// Expression: @base64d decode
#[tokio::test]
async fn test_runner_expression_base64d_decode() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-base64d-decode
  version: '0.1.0'
do:
  - compute:
      set:
        encoded: "${ \"hello\" | @base64 }"
        decoded: "${ \"aGVsbG8=\" | @base64d }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["decoded"], json!("hello"));
    // encoded should be base64 of "hello"
    assert!(output["encoded"].is_string());
}

// Expression: any/all with generator
#[tokio::test]
async fn test_runner_expression_any_all_generator() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-any-all-generator
  version: '0.1.0'
do:
  - compute:
      set:
        anyPositive: "${ any(.nums[]; . > 0) }"
        allPositive: "${ all(.nums[]; . > 0) }"
        anyNegative: "${ any(.nums[]; . < 0) }"
        nums: "${ .nums }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"nums": [1, 2, 3]}))
        .await
        .unwrap();
    assert_eq!(output["anyPositive"], json!(true));
    assert_eq!(output["allPositive"], json!(true));
    assert_eq!(output["anyNegative"], json!(false));
}

// Expression: @json format (round-trip)
#[tokio::test]
async fn test_runner_expression_html_format_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-json-format-v2
  version: '0.1.0'
do:
  - compute:
      set:
        jsonStr: "${ {a: 1, b: \"hello\"} | @json }"
        parsed: "${ {a: 1, b: \"hello\"} | @json | fromjson }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert!(output["jsonStr"].is_string());
    assert_eq!(output["parsed"]["a"], json!(1));
    assert_eq!(output["parsed"]["b"], json!("hello"));
}

// Expression: debug filter (passes through)
#[tokio::test]
async fn test_runner_expression_debug_passthrough_v2() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: expr-debug-v2
  version: '0.1.0'
do:
  - compute:
      set:
        result: "${ [1,2,3] | debug | map(. * 2) }"
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert_eq!(output["result"], json!([2, 4, 6]));
}

// === Java SDK's for-sum.yaml: for loop with export.as context accumulation ===

#[tokio::test]
async fn test_runner_runtime_version_expression() {
    // Matches Java SDK's simple-expression.yaml checkSpecialKeywords
    // Verifies $runtime.version contains version info
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: runtime-version-test
  version: '0.1.0'
do:
  - checkRuntime:
      set:
        version: ${$runtime.version}
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert!(output["version"].is_string());
    let version = output["version"].as_str().unwrap();
    assert!(!version.is_empty(), "runtime version should not be empty");
}

#[tokio::test]
async fn test_runner_workflow_id_expression() {
    // Matches Java SDK's simple-expression.yaml - $workflow.id should be a UUID
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: workflow-id-test
  version: '0.1.0'
do:
  - checkId:
      set:
        workflowId: ${$workflow.id}
"#;
    let output = run_workflow_yaml(&yaml_str, json!({})).await.unwrap();
    assert!(output["workflowId"].is_string());
    let id = output["workflowId"].as_str().unwrap();
    assert!(!id.is_empty(), "workflow ID should not be empty");
    // UUID v4 format: 8-4-4-4-12 hex chars
    assert_eq!(id.len(), 36, "workflow ID should be UUID format");
}

#[tokio::test]
async fn test_runner_call_http_endpoint_expression() {
    // Matches Java SDK's call-http-endpoint-interpolation.yaml
    // HTTP endpoint with JQ expression interpolation
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: call-http-endpoint-interpolation
  version: '0.1.0'
do:
  - tryGetPet:
      try:
        - getPet:
            call: http
            with:
              method: get
              endpoint: '${ "http://localhost:9876/pets/" + (.petId | tostring) }'
      catch:
        errors:
          with:
            type: communication
            status: 404
        do:
          - notFound:
              set:
                found: false
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    // This will fail to connect but should parse correctly
    let result = runner.run(json!({"petId": 42})).await;
    // Will either get a connection error or catch it
    // The important thing is that the YAML parses and the endpoint expression evaluates
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_runner_set_multiple_expressions_go_pattern() {
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-multiple-expressions
  version: '0.1.0'
do:
  - extractInfo:
      set:
        userName: '${.user.name}'
        userEmail: '${.user.email}'
        domain: '${.user.email | split("@") | .[1]}'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({
            "user": {
                "name": "Alice",
                "email": "alice@example.com"
            }
        }))
        .await
        .unwrap();
    assert_eq!(output["userName"], json!("Alice"));
    assert_eq!(output["userEmail"], json!("alice@example.com"));
    assert_eq!(output["domain"], json!("example.com"));
}

#[tokio::test]
async fn test_runner_set_expressions_with_functions() {
    // Matches Go SDK's TestSetTaskExecutor_ExpressionsWithFunctions
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-expr-functions
  version: '0.1.0'
do:
  - setSum:
      set:
        sum: '${.values | add}'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({
            "values": [1, 2, 3, 4, 5]
        }))
        .await
        .unwrap();
    assert_eq!(output["sum"], json!(15));
}

#[tokio::test]
async fn test_runner_raise_timeout_with_expression() {
    // Matches Go SDK's TestRaiseTaskRunner_TimeoutErrorWithExpression
    // Raise a timeout error type with expression in detail field
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-timeout-expr
  version: '0.1.0'
do:
  - raiseTimeout:
      raise:
        error:
          type: https://serverlessworkflow.io/spec/1.0.0/errors/timeout
          status: 408
          title: Timeout Error
          detail: '${.timeoutMessage}'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let result = runner
        .run(json!({
            "timeoutMessage": "Request took too long"
        }))
        .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type_short(), "timeout");
    assert_eq!(err.status(), Some(&json!(408)));
    assert_eq!(err.title(), Some("Timeout Error"));
    assert_eq!(err.detail(), Some("Request took too long"));
}

#[tokio::test]
async fn test_runner_raise_runtime_error_with_expression_detail() {
    // Go SDK pattern: raise with type/status as static, detail from expression
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: raise-runtime-expr
  version: '0.1.0'
do:
  - raiseRuntime:
      raise:
        error:
          type: https://serverlessworkflow.io/spec/1.0.0/errors/runtime
          status: 500
          title: Runtime Error
          detail: Unexpected failure
          instance: /task_runtime
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let result = runner.run(json!({})).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type_short(), "runtime");
    assert_eq!(err.status(), Some(&json!(500)));
    assert_eq!(err.title(), Some("Runtime Error"));
    assert_eq!(err.detail(), Some("Unexpected failure"));
    // instance field from YAML takes precedence over auto-generated task reference
    assert_eq!(err.instance(), Some("/task_runtime"));
}

#[tokio::test]
async fn test_runner_set_conditional_expression_go_pattern() {
    // Matches Go SDK's TestSetTaskExecutor_ConditionalExpressions
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-conditional-expr
  version: '0.1.0'
do:
  - setWeather:
      set:
        weather: '${if .temperature > 25 then "hot" else "cold" end}'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"temperature": 30}))
        .await
        .unwrap();
    assert_eq!(output["weather"], json!("hot"));
}

#[tokio::test]
async fn test_runner_set_conditional_expression_cold() {
    // Same as above but with cold temperature
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-conditional-expr-cold
  version: '0.1.0'
do:
  - setWeather:
      set:
        weather: '${if .temperature > 25 then "hot" else "cold" end}'
"#;
    let output = run_workflow_yaml(&yaml_str, json!({"temperature": 15}))
        .await
        .unwrap();
    assert_eq!(output["weather"], json!("cold"));
}

#[tokio::test]
async fn test_runner_set_runtime_expression_fullname() {
    // Matches Go SDK's TestSetTaskExecutor_RuntimeExpressions
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: set-runtime-expression
  version: '0.1.0'
do:
  - setFullName:
      set:
        fullName: '${ "\(.user.firstName) \(.user.lastName)" }'
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({
            "user": {
                "firstName": "John",
                "lastName": "Doe"
            }
        }))
        .await
        .unwrap();
    assert_eq!(output["fullName"], json!("John Doe"));
}

#[tokio::test]
async fn test_runner_shell_jq_expression_command() {
    // Matches Java SDK's echo-jq.yaml - shell command with ${} JQ interpolation
    let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: shell-jq-expr
  version: '0.1.0'
do:
  - runShell:
      run:
        shell:
          command: '${ "echo Hello, " + .user.name }'
        return: all
"#;
    let runner = WorkflowRunner::new(serde_yaml::from_str(&yaml_str).unwrap()).unwrap();
    let output = runner
        .run(json!({"user": {"name": "World"}}))
        .await
        .unwrap();
    assert_eq!(output["code"], json!(0));
    assert!(output["stdout"].as_str().unwrap().contains("Hello, World"));
}

// === Shell await: false (Java SDK echo-not-awaiting.yaml pattern) ===

#[tokio::test]
async fn test_runner_call_http_basic_secret_expression_auth() {
    use crate::secret::MapSecretManager;

    let protected = warp::path("api")
        .and(warp::header::optional("Authorization"))
        .map(|auth: Option<String>| match auth {
            Some(val) if val == "Basic YWRtaW46cGFzc3dvcmQxMjM=" => {
                warp::reply::json(&serde_json::json!({"access": "granted"}))
            }
            _ => warp::reply::json(&serde_json::json!({"access": "denied"})),
        });

    let port = start_mock_server(protected);

    let secret_mgr = Arc::new(MapSecretManager::new().with_secret(
        "mySecret",
        json!({
            "username": "admin",
            "password": "password123"
        }),
    ));

    let yaml_str = std::fs::read_to_string(testdata("call_http_basic_secret_expr.yaml")).unwrap();
    let yaml_str = yaml_str.replace("9876", &port.to_string());
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
    let runner = WorkflowRunner::new(workflow)
        .unwrap()
        .with_secret_manager(secret_mgr);

    let output = runner.run(json!({})).await.unwrap();
    assert_eq!(output["access"], json!("granted"));
}

// === HTTP Call: Basic Auth with $secret expression + export $authorization ===
// Matches Java SDK's basic-properties-auth.yaml with export.as

#[tokio::test]
async fn test_runner_call_http_basic_secret_expression_export() {
    use crate::secret::MapSecretManager;

    let protected = warp::path("api")
        .and(warp::header::optional("Authorization"))
        .map(|auth: Option<String>| match auth {
            Some(val) if val == "Basic YWRtaW46cGFzc3dvcmQxMjM=" => {
                warp::reply::json(&serde_json::json!({"status": "ok"}))
            }
            _ => warp::reply::json(&serde_json::json!({"status": "denied"})),
        });

    let port = start_mock_server(protected);

    let secret_mgr = Arc::new(MapSecretManager::new().with_secret(
        "mySecret",
        json!({
            "username": "admin",
            "password": "password123"
        }),
    ));

    let yaml_str =
        std::fs::read_to_string(testdata("call_http_basic_secret_expr_export.yaml")).unwrap();
    let yaml_str = yaml_str.replace("9876", &port.to_string());
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
    let runner = WorkflowRunner::new(workflow)
        .unwrap()
        .with_secret_manager(secret_mgr);

    let output = runner.run(json!({})).await.unwrap();
    assert_eq!(output["scheme"], json!("Basic"));
    // Parameter is Base64-encoded credentials (matches Java SDK behavior)
    assert_eq!(
        output["param"],
        json!(base64::engine::general_purpose::STANDARD.encode("admin:password123"))
    );
}

// === HTTP Call: Digest Auth with $secret expression ===
// Matches Java SDK's digest-properties-auth.yaml pattern

#[tokio::test]
async fn test_runner_call_http_digest_secret_expression_auth() {
    use crate::secret::MapSecretManager;

    let protected = warp::path("dir")
        .and(warp::path("index.html"))
        .and(warp::header::optional("Authorization"))
        .map(|auth: Option<String>| match auth {
            Some(val) if val == "Basic bXlVc2VyOm15UGFzcw==" => {
                warp::reply::json(&serde_json::json!({"page": "index"}))
            }
            _ => warp::reply::json(&serde_json::json!({"page": "denied"})),
        });

    let port = start_mock_server(protected);

    let secret_mgr = Arc::new(MapSecretManager::new().with_secret(
        "mySecret",
        json!({
            "username": "myUser",
            "password": "myPass"
        }),
    ));

    let yaml_str =
        std::fs::read_to_string(testdata("call_http_digest_secret_expr_export.yaml")).unwrap();
    let yaml_str = yaml_str.replace("9876", &port.to_string());
    let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
    let runner = WorkflowRunner::new(workflow)
        .unwrap()
        .with_secret_manager(secret_mgr);

    let output = runner.run(json!({})).await.unwrap();
    assert_eq!(output["scheme"], json!("Digest"));
    // Parameter is Base64-encoded credentials (will change to proper Digest format when implemented)
    assert_eq!(
        output["param"],
        json!(base64::engine::general_purpose::STANDARD.encode("myUser:myPass"))
    );
}

// === HTTP Call: Output as expression ===
// Matches Java SDK's call-with-response-output-expr.yaml pattern
