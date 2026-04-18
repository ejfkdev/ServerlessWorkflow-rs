use super::*;

    #[tokio::test]
    async fn test_runner_call_function_http() {
        use warp::Filter;

        let get_pet = warp::path("pets")
            .and(warp::path::param::<i32>())
            .map(|id: i32| {
                warp::reply::json(&serde_json::json!({
                    "id": id,
                    "name": "Doggie"
                }))
            });

        let output = run_workflow_with_mock_server("call_function_http.yaml", get_pet, json!({})).await;
        assert_eq!(output["id"], json!(1));
        assert_eq!(output["name"], json!("Doggie"));
    }

    // === HTTP Call: Bearer Auth with $secret ===

    #[tokio::test]
    async fn test_runner_call_function_with_output_as() {
        use warp::Filter;

        let get_pet = warp::path!("pets" / i32).map(|id: i32| {
            warp::reply::json(&serde_json::json!({
                "id": id,
                "name": "Rex"
            }))
        });

        let output = run_workflow_with_mock_server("call_function_with_input.yaml", get_pet, json!({})).await;
        // output.as: .name should extract just the name
        assert_eq!(output, json!("Rex"));
    }

    // === Try-Catch: communication error ===

    // Call function: with arguments via HTTP (use mock server)
    #[tokio::test]
    async fn test_runner_call_function_with_args() {
        use warp::Filter;

        let add_handler =
            warp::path("add")
                .and(warp::body::json())
                .map(|body: serde_json::Value| {
                    let a = body.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
                    let b = body.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
                    warp::reply::json(&serde_json::json!({"result": a + b}))
                });

        let (addr, server_fn) = warp::serve(add_handler).bind_ephemeral(([127, 0, 0, 1], 0));
        let port = addr.port();
        tokio::spawn(server_fn);

        let yaml_str = format!(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: call-function-args
  version: '0.1.0'
do:
  - compute:
      call: http
      with:
        method: POST
        endpoint: "http://127.0.0.1:{port}/add"
        body:
          a: "${{ .x }}"
          b: "${{ .y }}"
      output:
        as: "${{ .result }}"
"#
        );
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({"x": 3, "y": 7})).await.unwrap();
        assert_eq!(output, json!(10));
    }

    #[tokio::test]
    async fn test_runner_call_function_reference() {
        // Call function defined in use.functions - matches Go/Java SDK pattern
        let yaml_str = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: call-func-ref
  version: '0.1.0'
use:
  functions:
    greet:
      call: http
      with:
        method: get
        endpoint:
          uri: http://localhost:PORT/hello
do:
  - callGreet:
      call: greet
"#;
        use warp::Filter;
        let hello = warp::path("hello").map(|| warp::reply::json(&json!({"greeting": "Hello!"})));
        let (addr, server_fn) = warp::serve(hello).bind_ephemeral(([127, 0, 0, 1], 0));
        let port = addr.port();
        tokio::spawn(server_fn);

        let yaml_str = yaml_str.replace("PORT", &port.to_string());
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml_str).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["greeting"], json!("Hello!"));
    }

    // === Emit with event data and type ===

    #[tokio::test]
    async fn test_runner_call_grpc_with_custom_handler() {
        use crate::handler::CallHandler;

        struct MockGrpcHandler;

        #[async_trait::async_trait]
        impl CallHandler for MockGrpcHandler {
            fn call_type(&self) -> &str {
                "grpc"
            }

            async fn handle(
                &self,
                _task_name: &str,
                _call_config: &Value,
                input: &Value,
            ) -> WorkflowResult<Value> {
                Ok(json!({"grpc_result": input["message"].as_str().unwrap_or("default")}))
            }
        }

        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: grpc-test
  version: '0.1.0'
do:
  - callGrpc:
      call: grpc
      with:
        proto:
          name: TestProto
          endpoint: http://example.com/proto
        service:
          name: TestService
          host: localhost
        method: GetData
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_call_handler(Box::new(MockGrpcHandler));

        let output = runner.run(json!({"message": "hello grpc"})).await.unwrap();
        assert_eq!(output["grpc_result"], json!("hello grpc"));
    }

    #[tokio::test]
    async fn test_runner_call_openapi_with_custom_handler() {
        use crate::handler::CallHandler;

        struct MockOpenApiHandler;

        #[async_trait::async_trait]
        impl CallHandler for MockOpenApiHandler {
            fn call_type(&self) -> &str {
                "openapi"
            }

            async fn handle(
                &self,
                _task_name: &str,
                _call_config: &Value,
                _input: &Value,
            ) -> WorkflowResult<Value> {
                Ok(json!({"openapi_status": "ok"}))
            }
        }

        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: openapi-test
  version: '0.1.0'
do:
  - callApi:
      call: openapi
      with:
        document:
          name: PetStore
          endpoint: http://example.com/openapi.json
        operationId: listPets
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_call_handler(Box::new(MockOpenApiHandler));

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["openapi_status"], json!("ok"));
    }

    #[tokio::test]
    async fn test_runner_call_grpc_without_handler_returns_error() {
        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: grpc-no-handler
  version: '0.1.0'
do:
  - callGrpc:
      call: grpc
      with:
        proto:
          name: TestProto
          endpoint: http://example.com/proto
        service:
          name: TestService
          host: localhost
        method: GetData
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("grpc"), "error should mention 'grpc': {}", err);
        assert!(
            err.contains("CallHandler"),
            "error should mention 'CallHandler': {}",
            err
        );
    }

    #[tokio::test]
    async fn test_runner_multiple_call_handlers() {
        use crate::handler::CallHandler;

        struct MockAsyncApiHandler;

        #[async_trait::async_trait]
        impl CallHandler for MockAsyncApiHandler {
            fn call_type(&self) -> &str {
                "asyncapi"
            }

            async fn handle(
                &self,
                _task_name: &str,
                _call_config: &Value,
                _input: &Value,
            ) -> WorkflowResult<Value> {
                Ok(json!({"asyncapi_channel": "messages"}))
            }
        }

        struct MockA2AHandler;

        #[async_trait::async_trait]
        impl CallHandler for MockA2AHandler {
            fn call_type(&self) -> &str {
                "a2a"
            }

            async fn handle(
                &self,
                _task_name: &str,
                _call_config: &Value,
                _input: &Value,
            ) -> WorkflowResult<Value> {
                Ok(json!({"a2a_agent": "response"}))
            }
        }

        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: multi-handler-test
  version: '0.1.0'
do:
  - callAsync:
      call: asyncapi
      with:
        document:
          name: MyApi
          endpoint: http://example.com/asyncapi.json
        channel: messages
  - callA2A:
      call: a2a
      with:
        method: tasks/get
        agentCard:
          name: MyAgent
          endpoint: http://example.com/agent.json
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_call_handler(Box::new(MockAsyncApiHandler))
            .with_call_handler(Box::new(MockA2AHandler));

        let output = runner.run(json!({})).await.unwrap();
        // Last task output is returned
        assert_eq!(output["a2a_agent"], json!("response"));
    }

    // ---- HTTP output.as Transform Test (Java SDK pattern) ----

    /// Test custom task type with CustomTaskHandler
    #[tokio::test]
    async fn test_runner_custom_task_with_handler() {
        use crate::handler::CustomTaskHandler;

        struct MockGreetHandler;

        #[async_trait::async_trait]
        impl CustomTaskHandler for MockGreetHandler {
            fn task_type(&self) -> &str {
                "greet"
            }

            async fn handle(
                &self,
                task_name: &str,
                _task_type: &str,
                task_config: &Value,
                input: &Value,
            ) -> WorkflowResult<Value> {
                let name = input
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("World");
                Ok(json!({
                    "greeting": format!("Hello, {}!", name),
                    "taskName": task_name,
                    "customField": task_config.get("customField").and_then(|v| v.as_str()).unwrap_or("default")
                }))
            }
        }

        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: custom-task-test
  version: '1.0.0'
do:
  - greetUser:
      type: greet
      customField: myValue
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_custom_task_handler(Box::new(MockGreetHandler));

        let output = runner.run(json!({"name": "Alice"})).await.unwrap();
        assert_eq!(output["greeting"], json!("Hello, Alice!"));
        assert_eq!(output["taskName"], json!("greetUser"));
        assert_eq!(output["customField"], json!("myValue"));
    }

    /// Test custom task type without handler returns error
    #[tokio::test]
    async fn test_runner_custom_task_without_handler_returns_error() {
        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: custom-task-no-handler
  version: '1.0.0'
do:
  - myTask:
      type: unknownType
      someConfig: value
"#;
        let workflow: WorkflowDefinition = serde_yaml::from_str(yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("CustomTaskHandler"),
            "Error should mention CustomTaskHandler, got: {}",
            err_msg
        );
        assert!(
            err_msg.contains("unknownType"),
            "Error should mention the task type, got: {}",
            err_msg
        );
    }

    /// Test call function with inline definition — Java SDK's call-custom-function-inline.yaml
    /// use.functions defines a function inline with HTTP call
    #[tokio::test]
    async fn test_runner_call_function_inline_definition() {
        use warp::Filter;

        let greet = warp::path("api")
            .and(warp::path("greet"))
            .map(|| warp::reply::json(&serde_json::json!({"message": "Hello!"})));

        let (addr, server_fn) = warp::serve(greet).bind_ephemeral(([127, 0, 0, 1], 0));
        let port = addr.port();
        tokio::spawn(server_fn);

        let yaml = r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: call-function-inline
  version: '1.0.0'
use:
  functions:
    greet:
      call: http
      with:
        method: get
        endpoint: http://localhost:PORT/api/greet
do:
  - callGreet:
      call: greet
"#
        .replace("PORT", &port.to_string());
        let workflow: WorkflowDefinition = serde_yaml::from_str(&yaml).unwrap();
        let runner = WorkflowRunner::new(workflow).unwrap();

        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["message"], json!("Hello!"));
    }

    #[tokio::test]
    async fn test_call_function_with_catalog() {
        // Register a function that sets a value, then call it via call: function
        let mut set_map = HashMap::new();
        set_map.insert("greeting".to_string(), json!("hello from catalog"));
        let set_task =
            TaskDefinition::Set(serverless_workflow_core::models::task::SetTaskDefinition {
                set: serverless_workflow_core::models::task::SetValue::Map(set_map),
                common: serverless_workflow_core::models::task::TaskDefinitionFields::new(),
            });
        let workflow: WorkflowDefinition = serde_yaml::from_str(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: catalog-test
  version: '0.1.0'
do:
  - callCatalog:
      call: myFunction
"#,
        )
        .unwrap();

        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_function("myFunction", set_task);
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["greeting"], json!("hello from catalog"));
    }

    #[tokio::test]
    async fn test_call_function_with_catalog_name() {
        // Test functionName@catalogName syntax — catalog name is ignored
        let mut set_map = HashMap::new();
        set_map.insert("source".to_string(), json!("cataloged"));
        let set_task =
            TaskDefinition::Set(serverless_workflow_core::models::task::SetTaskDefinition {
                set: serverless_workflow_core::models::task::SetValue::Map(set_map),
                common: serverless_workflow_core::models::task::TaskDefinitionFields::new(),
            });
        let workflow: WorkflowDefinition = serde_yaml::from_str(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: catalog-name-test
  version: '0.1.0'
do:
  - callWithCatalog:
      call: myFunc@myCatalog
"#,
        )
        .unwrap();

        let runner = WorkflowRunner::new(workflow)
            .unwrap()
            .with_function("myFunc", set_task);
        let output = runner.run(json!({})).await.unwrap();
        assert_eq!(output["source"], json!("cataloged"));
    }

    #[tokio::test]
    async fn test_call_function_not_found_in_catalog() {
        let workflow: WorkflowDefinition = serde_yaml::from_str(
            r#"
document:
  dsl: '1.0.0'
  namespace: test
  name: catalog-missing
  version: '0.1.0'
do:
  - callMissing:
      call: nonexistentFunc
"#,
        )
        .unwrap();

        let runner = WorkflowRunner::new(workflow).unwrap();
        let result = runner.run(json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // === Suspend / Resume ===
