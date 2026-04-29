use super::*;

#[test]
fn test_call_task_with_timeout() {
    // Test CallTaskDefinition with timeout - deserialized as TaskDefinition which holds common fields
    let call_task_json = json!({
        "call": "myFunction",
        "timeout": {
            "after": "PT10S"
        }
    });
    let result: Result<CallTaskDefinition, _> = serde_json::from_value(call_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize call task with timeout: {:?}",
        result.err()
    );
    let call_task = result.unwrap();
    if let CallTaskDefinition::Function(ref f) = call_task {
        assert_eq!(f.call, "myFunction");
    } else {
        panic!("Expected Function variant");
    }
}

#[test]
fn test_call_task_with_all_fields() {
    // Test CallTaskDefinition with all fields - common fields are on TaskDefinition, not CallTaskDefinition
    let call_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "call": "myFunction",
        "with": {
            "param1": "value1",
            "param2": 42
        }
    });
    let result: Result<CallTaskDefinition, _> = serde_json::from_value(call_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize call task: {:?}",
        result.err()
    );
    let call_task = result.unwrap();
    if let CallTaskDefinition::Function(ref f) = call_task {
        assert_eq!(f.call, "myFunction");
        assert!(f.with.is_some());
    } else {
        panic!("Expected Function variant");
    }
}

#[test]
fn test_call_task_roundtrip_serialization() {
    // Test CallTaskDefinition roundtrip serialization
    let call_task = CallTaskDefinition::Function(
        serverless_workflow_core::models::call::CallFunctionDefinition {
            call: "myFunction".to_string(),
            with: None,
            common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
        },
    );

    let json_str = serde_json::to_string(&call_task).expect("Failed to serialize call task");
    let deserialized: CallTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    if let CallTaskDefinition::Function(ref f) = deserialized {
        assert_eq!(f.call, "myFunction");
    } else {
        panic!("Expected Function variant");
    }
}

#[test]
fn test_call_task_with_arguments() {
    // Test CallFunctionDefinition with arguments
    let arguments: std::collections::HashMap<String, serde_json::Value> = vec![
        (
            "param1".to_string(),
            serde_json::Value::String("value1".to_string()),
        ),
        ("param2".to_string(), serde_json::Value::Number(42.into())),
    ]
    .into_iter()
    .collect();

    let call_task = serverless_workflow_core::models::call::CallFunctionDefinition {
        call: "myFunction".to_string(),
        with: Some(arguments),
        common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
    };
    assert_eq!(call_task.call, "myFunction");
    assert!(call_task.with.is_some());
    let with = call_task.with.unwrap();
    assert_eq!(with.get("param1").and_then(|v| v.as_str()), Some("value1"));
    assert_eq!(with.get("param2").and_then(|v| v.as_u64()), Some(42));
}

#[test]
fn test_endpoint_definition_with_uri() {
    use serverless_workflow_core::models::resource::EndpointDefinition;
    let endpoint = EndpointDefinition {
        uri: "http://example.com/{id}".to_string(),
        authentication: None,
    };
    let json_str = serde_json::to_string(&endpoint).expect("Failed to serialize endpoint");
    assert!(json_str.contains("http://example.com/{id}"));
    let deserialized: EndpointDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(endpoint.uri, deserialized.uri);
}

#[test]
fn test_endpoint_definition_with_authentication() {
    use serverless_workflow_core::models::authentication::{
        AuthenticationPolicyDefinition, BasicAuthenticationSchemeDefinition,
        ReferenceableAuthenticationPolicy,
    };
    use serverless_workflow_core::models::resource::EndpointDefinition;
    let endpoint = EndpointDefinition {
        uri: "http://example.com/{id}".to_string(),
        authentication: Some(ReferenceableAuthenticationPolicy::Policy(Box::new(
            AuthenticationPolicyDefinition {
                basic: Some(BasicAuthenticationSchemeDefinition {
                    username: Some("admin".to_string()),
                    password: Some("admin".to_string()),
                    use_: None,
                }),
                ..Default::default()
            },
        ))),
    };
    let json_str = serde_json::to_string(&endpoint).expect("Failed to serialize endpoint");
    assert!(json_str.contains("admin"));
    let deserialized: EndpointDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(endpoint.uri, deserialized.uri);
    assert!(deserialized.authentication.is_some());
}

#[test]
fn test_endpoint_definition_roundtrip() {
    use serverless_workflow_core::models::resource::EndpointDefinition;
    let endpoint = EndpointDefinition {
        uri: "http://example.com/{id}".to_string(),
        authentication: None,
    };
    let json_str = serde_json::to_string(&endpoint).expect("Failed to serialize");
    let deserialized: EndpointDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(endpoint.uri, deserialized.uri);
}

#[test]
fn test_call_task_roundtrip() {
    use serverless_workflow_core::models::task::{CallTaskDefinition, TaskDefinition};
    use std::collections::HashMap;
    let mut with_map = HashMap::new();
    with_map.insert("method".to_string(), serde_json::json!("GET"));
    with_map.insert(
        "endpoint".to_string(),
        serde_json::json!("http://example.com"),
    );
    let call_fn = CallTaskDefinition::Function(
        serverless_workflow_core::models::call::CallFunctionDefinition {
            call: "http".to_string(),
            with: Some(with_map),
            common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
        },
    );
    let task = TaskDefinition::Call(Box::new(call_fn));
    let json_str = serde_json::to_string(&task).expect("Failed to serialize");
    assert!(json_str.contains("http"));
    let deserialized: TaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(matches!(deserialized, TaskDefinition::Call(_)));
}

#[test]
fn test_call_http_with_headers_and_query() {
    // Test call HTTP with headers and query from specification example
    let call_json = json!({
        "call": "http",
        "with": {
            "method": "get",
            "endpoint": "https://swapi.dev/api/people/",
            "headers": {
                "Accept": "application/json"
            },
            "query": {
                "search": "${.searchQuery}"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::CallTaskDefinition, _> =
        serde_json::from_value(call_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize call HTTP with headers/query: {:?}",
        result.err()
    );
}

#[test]
fn test_call_task_with_interpolated_endpoint() {
    // Test call task with interpolated endpoint
    let call_json = json!({
        "call": "http",
        "with": {
            "method": "get",
            "endpoint": "https://petstore.swagger.io/v2/pet/{petId}"
        }
    });

    let result: Result<serverless_workflow_core::models::task::CallTaskDefinition, _> =
        serde_json::from_value(call_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize call task with interpolated endpoint: {:?}",
        result.err()
    );
}

#[test]
fn test_call_openapi_task() {
    // Test call task with OpenAPI
    let call_json = json!({
        "call": "openapi",
        "with": {
            "document": {
                "name": "petstore",
                "endpoint": "https://petstore.swagger.io/v2/api"
            },
            "operationId": "getPetById"
        }
    });

    let result: Result<serverless_workflow_core::models::task::CallTaskDefinition, _> =
        serde_json::from_value(call_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize OpenAPI call: {:?}",
        result.err()
    );
}

#[test]
fn test_call_custom_function() {
    // Test call task with custom function
    let call_json = json!({
        "call": "customFunction",
        "with": {
            "arg1": "value1",
            "arg2": 42
        }
    });

    let result: Result<serverless_workflow_core::models::task::CallTaskDefinition, _> =
        serde_json::from_value(call_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize custom function call: {:?}",
        result.err()
    );
}

#[test]
fn test_call_grpc() {
    // Test call task with grpc
    let call_json = json!({
        "call": "grpc",
        "with": {
            "proto": {
                "endpoint": "file://app/greet.proto"
            },
            "service": {
                "name": "GreeterApi.Greeter",
                "host": "localhost",
                "port": 5011
            },
            "method": "SayHello",
            "arguments": {
                "name": "${ .user.preferredDisplayName }"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::CallTaskDefinition, _> =
        serde_json::from_value(call_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize grpc call: {:?}",
        result.err()
    );
    let call_task = result.unwrap();
    if let CallTaskDefinition::GRPC(ref grpc_def) = call_task {
        assert_eq!(grpc_def.call, "grpc");
    } else {
        panic!("Expected GRPC variant");
    }
}

#[test]
fn test_call_mcp() {
    // Test call task with mcp
    let call_json = json!({
        "call": "mcp",
        "with": {
            "method": "tools/call",
            "parameters": {
                "name": "conversations_add_message",
                "arguments": {
                    "channel_id": "C1234567890",
                    "thread_ts": "1623456789.123456",
                    "payload": "Hello, world!"
                }
            },
            "transport": {
                "stdio": {
                    "command": "npx",
                    "arguments": ["slack-mcp-server@latest", "--transport", "stdio"],
                    "environment": {
                        "SLACK_MCP_TOKEN": "test-slack-token-placeholder"
                    }
                }
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::CallTaskDefinition, _> =
        serde_json::from_value(call_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize mcp call: {:?}",
        result.err()
    );
    let call_task = result.unwrap();
    if let CallTaskDefinition::Function(ref func_def) = call_task {
        assert_eq!(func_def.call, "mcp");
    } else {
        panic!("Expected Function variant for mcp call");
    }
}

#[test]
fn test_call_task_with_export() {
    // Test call task with export (similar to star-wars-homeworld example)
    let call_json = json!({
        "call": "http",
        "with": {
            "method": "get",
            "endpoint": "https://swapi.dev/api/people/{id}"
        },
        "output": {
            "as": "${ .response }"
        },
        "export": {
            "as": {
                "homeworld": "${ .content.homeworld }"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::CallTaskDefinition, _> =
        serde_json::from_value(call_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize call task with export: {:?}",
        result.err()
    );
    let call_task = result.unwrap();
    if let CallTaskDefinition::HTTP(ref http_def) = call_task {
        assert_eq!(http_def.call, "http");
    } else {
        panic!("Expected HTTP variant");
    }
}

#[test]
fn test_call_asyncapi() {
    // Test call task with asyncapi (similar to call-asyncapi-publish example)
    let call_json = json!({
        "call": "asyncapi",
        "with": {
            "document": {
                "endpoint": "https://fake.com/docs/asyncapi.json"
            },
            "operation": "findPetsByStatus",
            "server": {
                "name": "staging"
            },
            "message": {
                "payload": {
                    "petId": "${ .pet.id }"
                }
            },
            "authentication": {
                "bearer": {
                    "token": "${ .token }"
                }
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::CallTaskDefinition, _> =
        serde_json::from_value(call_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize asyncapi call: {:?}",
        result.err()
    );
    let call_task = result.unwrap();
    if let CallTaskDefinition::AsyncAPI(ref asyncapi_def) = call_task {
        assert_eq!(asyncapi_def.call, "asyncapi");
    } else {
        panic!("Expected AsyncAPI variant");
    }
}

#[test]
fn test_http_output_format_constants() {
    assert_eq!(HttpOutputFormat::RAW, "raw");
    assert_eq!(HttpOutputFormat::CONTENT, "content");
    assert_eq!(HttpOutputFormat::RESPONSE, "response");
}

#[test]
fn test_call_http_definition() {
    use serverless_workflow_core::models::call::*;

    let json_str = r#"{
        "call": "http",
        "with": {
            "method": "GET",
            "endpoint": "http://example.com/api",
            "headers": {"Content-Type": "application/json"},
            "output": "content",
            "redirect": false
        }
    }"#;

    let call_task: CallTaskDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize CallHTTP");
    match call_task {
        CallTaskDefinition::HTTP(ref http_def) => {
            assert_eq!(http_def.call, "http");
            assert_eq!(http_def.with.method, "GET");
            assert_eq!(http_def.with.output, Some("content".to_string()));
            assert_eq!(http_def.with.redirect, Some(false));
        }
        _ => panic!("Expected HTTP variant"),
    }

    // Test serialization roundtrip
    let serialized = serde_json::to_string(&call_task).expect("Failed to serialize");
    let deserialized: CallTaskDefinition =
        serde_json::from_str(&serialized).expect("Failed to re-deserialize");
    assert_eq!(call_task, deserialized);
}

#[test]
fn test_call_grpc_definition() {
    use serverless_workflow_core::models::call::*;

    let json_str = r#"{
        "call": "grpc",
        "with": {
            "proto": {
                "endpoint": "http://example.com/proto/service.proto"
            },
            "service": {
                "name": "MyService",
                "host": "localhost"
            },
            "method": "DoSomething",
            "arguments": {"key": "value"}
        }
    }"#;

    let call_task: CallTaskDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize CallGRPC");
    match call_task {
        CallTaskDefinition::GRPC(ref grpc_def) => {
            assert_eq!(grpc_def.call, "grpc");
            assert_eq!(grpc_def.with.method, "DoSomething");
            assert_eq!(grpc_def.with.service.name, "MyService");
            assert_eq!(grpc_def.with.service.host, "localhost");
        }
        _ => panic!("Expected GRPC variant"),
    }
}

#[test]
fn test_call_openapi_definition() {
    use serverless_workflow_core::models::call::*;

    let json_str = r#"{
        "call": "openapi",
        "with": {
            "document": {
                "endpoint": "http://example.com/openapi.json"
            },
            "operationId": "getUsers",
            "output": "response",
            "redirect": true
        }
    }"#;

    let call_task: CallTaskDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize CallOpenAPI");
    match call_task {
        CallTaskDefinition::OpenAPI(ref openapi_def) => {
            assert_eq!(openapi_def.call, "openapi");
            assert_eq!(openapi_def.with.operation_id, "getUsers");
            assert_eq!(openapi_def.with.output, Some("response".to_string()));
            assert_eq!(openapi_def.with.redirect, Some(true));
        }
        _ => panic!("Expected OpenAPI variant"),
    }
}

#[test]
fn test_call_a2a_definition() {
    use serverless_workflow_core::models::call::*;

    let json_str = r#"{
        "call": "a2a",
        "with": {
            "method": "message/send",
            "server": "http://example.com/a2a",
            "agentCard": {
                "endpoint": "http://example.com/agent-card.json"
            },
            "parameters": {"message": "hello"}
        }
    }"#;

    let call_task: CallTaskDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize CallA2A");
    match call_task {
        CallTaskDefinition::A2A(ref a2a_def) => {
            assert_eq!(a2a_def.call, "a2a");
            assert_eq!(a2a_def.with.method, "message/send");
            assert!(a2a_def.with.agent_card.is_some());
        }
        _ => panic!("Expected A2A variant"),
    }
}

#[test]
fn test_call_function_definition() {
    use serverless_workflow_core::models::call::*;

    let json_str = r#"{
        "call": "myFunction",
        "with": {"param1": "value1", "param2": 42}
    }"#;

    let call_task: CallTaskDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize CallFunction");
    match call_task {
        CallTaskDefinition::Function(ref func_def) => {
            assert_eq!(func_def.call, "myFunction");
            assert!(func_def.with.is_some());
        }
        _ => panic!("Expected Function variant"),
    }
}

#[test]
fn test_call_asyncapi_definition() {
    use serverless_workflow_core::models::call::*;

    let json_str = r#"{
        "call": "asyncapi",
        "with": {
            "document": {
                "endpoint": "http://example.com/asyncapi.json"
            },
            "operation": "publishUserCreated",
            "message": {
                "payload": {"userId": "123"}
            }
        }
    }"#;

    let call_task: CallTaskDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize CallAsyncAPI");
    match call_task {
        CallTaskDefinition::AsyncAPI(ref asyncapi_def) => {
            assert_eq!(asyncapi_def.call, "asyncapi");
            assert_eq!(
                asyncapi_def.with.operation,
                Some("publishUserCreated".to_string())
            );
            assert!(asyncapi_def.with.message.is_some());
        }
        _ => panic!("Expected AsyncAPI variant"),
    }
}

#[test]
fn test_asyncapi_message_consumption_policy() {
    use serverless_workflow_core::models::call::*;

    // Test amount-based consumption
    let amount_json = r#"{"amount": 5}"#;
    let amount_policy: AsyncApiMessageConsumptionPolicy =
        serde_json::from_str(amount_json).expect("Failed to deserialize amount policy");
    match amount_policy {
        AsyncApiMessageConsumptionPolicy::Amount { amount } => assert_eq!(amount, 5),
        _ => panic!("Expected Amount variant"),
    }

    // Test while-based consumption
    let while_json = r#"{"while": "${ .count < 10 }"}"#;
    let while_policy: AsyncApiMessageConsumptionPolicy =
        serde_json::from_str(while_json).expect("Failed to deserialize while policy");
    match while_policy {
        AsyncApiMessageConsumptionPolicy::While { while_ } => {
            assert_eq!(while_, "${ .count < 10 }")
        }
        _ => panic!("Expected While variant"),
    }

    // Test until-based consumption
    let until_json = r#"{"until": "${ .done == true }"}"#;
    let until_policy: AsyncApiMessageConsumptionPolicy =
        serde_json::from_str(until_json).expect("Failed to deserialize until policy");
    match until_policy {
        AsyncApiMessageConsumptionPolicy::Until { until } => {
            assert_eq!(until, "${ .done == true }")
        }
        _ => panic!("Expected Until variant"),
    }
}

#[test]
fn test_a2a_method_constants() {
    use serverless_workflow_core::models::call::A2AMethod;
    assert_eq!(A2AMethod::MESSAGE_SEND, "message/send");
    assert_eq!(A2AMethod::MESSAGE_STREAM, "message/stream");
    assert_eq!(A2AMethod::TASKS_GET, "tasks/get");
    assert_eq!(A2AMethod::TASKS_LIST, "tasks/list");
    assert_eq!(A2AMethod::TASKS_CANCEL, "tasks/cancel");
    assert_eq!(A2AMethod::TASKS_RESUBSCRIBE, "tasks/resubscribe");
    assert_eq!(
        A2AMethod::AGENT_GET_AUTHENTICATED_EXTENDED_CARD,
        "agent/getAuthenticatedExtendedCard"
    );
}

#[test]
fn test_asyncapi_protocol_constants() {
    use serverless_workflow_core::models::call::AsyncApiProtocol;
    assert_eq!(AsyncApiProtocol::KAFKA, "kafka");
    assert_eq!(AsyncApiProtocol::AMQP, "amqp");
    assert_eq!(AsyncApiProtocol::MQTT, "mqtt");
    assert_eq!(AsyncApiProtocol::NATS, "nats");
    assert_eq!(AsyncApiProtocol::HTTP, "http");
    assert_eq!(AsyncApiProtocol::WS, "ws");
}

#[test]
fn test_call_type_constants() {
    use serverless_workflow_core::models::call::CallType;
    assert_eq!(CallType::ASYNCAPI, "asyncapi");
    assert_eq!(CallType::GRPC, "grpc");
    assert_eq!(CallType::HTTP, "http");
    assert_eq!(CallType::OPENAPI, "openapi");
    assert_eq!(CallType::A2A, "a2a");
}

#[test]
fn test_endpoint_with_referenceable_auth() {
    use serverless_workflow_core::models::authentication::ReferenceableAuthenticationPolicy;
    use serverless_workflow_core::models::resource::EndpointDefinition;

    // Test endpoint with reference authentication
    let json_str = r#"{
        "uri": "http://example.com",
        "authentication": {"use": "myAuthPolicy"}
    }"#;

    let endpoint: EndpointDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize endpoint with ref auth");
    match endpoint.authentication {
        Some(ReferenceableAuthenticationPolicy::Reference(ref r)) => {
            assert_eq!(r.use_, "myAuthPolicy")
        }
        _ => panic!("Expected Reference variant"),
    }

    // Test endpoint with inline authentication
    let json_str2 = r#"{
        "uri": "http://example.com",
        "authentication": {"basic": {"username": "admin", "password": "secret"}}
    }"#;

    let endpoint2: EndpointDefinition =
        serde_json::from_str(json_str2).expect("Failed to deserialize endpoint with inline auth");
    match endpoint2.authentication {
        Some(ReferenceableAuthenticationPolicy::Policy(ref p)) => assert!(p.basic.is_some()),
        _ => panic!("Expected Policy variant"),
    }
}

#[test]
fn test_http_headers_expression() {
    use serverless_workflow_core::models::call::*;

    // Test with map headers
    let json_str = r#"{
        "call": "http",
        "with": {
            "method": "POST",
            "endpoint": "http://example.com/api",
            "headers": {"Content-Type": "application/json", "Accept": "application/json"}
        }
    }"#;

    let call_task: CallTaskDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize");
    if let CallTaskDefinition::HTTP(http_def) = call_task {
        if let Some(OneOfHeadersOrExpression::Map(ref headers)) = http_def.with.headers {
            assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
        } else {
            panic!("Expected Map headers");
        }
    } else {
        panic!("Expected HTTP variant");
    }

    // Test with expression headers
    let json_str2 = r#"{
        "call": "http",
        "with": {
            "method": "POST",
            "endpoint": "http://example.com/api",
            "headers": "${ .headers }"
        }
    }"#;

    let call_task2: CallTaskDefinition =
        serde_json::from_str(json_str2).expect("Failed to deserialize");
    if let CallTaskDefinition::HTTP(http_def) = call_task2 {
        if let Some(OneOfHeadersOrExpression::Expression(ref expr)) = http_def.with.headers {
            assert_eq!(expr, "${ .headers }");
        } else {
            panic!("Expected Expression headers");
        }
    } else {
        panic!("Expected HTTP variant");
    }
}

#[test]
fn test_call_task_definition_enum_variants() {
    use serverless_workflow_core::models::call::{CallFunctionDefinition, CallTaskDefinition};

    // Test Function variant
    let func = CallFunctionDefinition {
        call: "myFunction".to_string(),
        with: None,
        common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
    };
    let call = CallTaskDefinition::Function(func);
    if let CallTaskDefinition::Function(f) = call {
        assert_eq!(f.call, "myFunction");
    } else {
        panic!("Expected Function variant");
    }
}

// ============== New SDK Comparison Tests ==============

#[test]
fn test_call_http_task_serialization() {
    use serverless_workflow_core::models::call::*;
    let http_task = CallHTTPDefinition {
        call: "http".to_string(),
        with: HTTPArguments {
            method: "GET".to_string(),
            endpoint: OneOfEndpointDefinitionOrUri::Uri("http://example.com/api".to_string()),
            headers: None,
            body: None,
            query: None,
            output: None,
            redirect: None,
        },
        common: TaskDefinitionFields::default(),
    };
    let call = CallTaskDefinition::HTTP(http_task);
    let task = TaskDefinition::Call(Box::new(call));
    let json_str = serde_json::to_string(&task).expect("Failed to serialize");
    assert!(json_str.contains("\"call\":\"http\""));
    assert!(json_str.contains("\"method\":\"GET\""));
}

#[test]
fn test_call_http_task_deserialization() {
    use serverless_workflow_core::models::call::*;
    let json = r#"{"call":"http","with":{"method":"POST","endpoint":"http://api.example.com","body":{"key":"value"}}}"#;
    let call: CallTaskDefinition = serde_json::from_str(json).expect("Failed to deserialize");
    match call {
        CallTaskDefinition::HTTP(http) => {
            assert_eq!(http.with.method, "POST");
            assert!(http.with.body.is_some());
        }
        _ => panic!("Expected HTTP call type"),
    }
}

#[test]
fn test_call_grpc_task_serialization() {
    use serverless_workflow_core::models::call::*;
    let grpc_task = CallGRPCDefinition {
        call: "grpc".to_string(),
        with: GRPCArguments {
            proto: serverless_workflow_core::models::resource::ExternalResourceDefinition {
                name: None,
                endpoint:
                    serverless_workflow_core::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                        "http://proto.example.com/api.proto".to_string(),
                    ),
            },
            service: GRPCServiceDefinition {
                name: "MyService".to_string(),
                host: "localhost".to_string(),
                port: Some(50051),
                authentication: None,
            },
            method: "DoSomething".to_string(),
            arguments: None,
            authentication: None,
        },
        common: TaskDefinitionFields::default(),
    };
    let json_str = serde_json::to_string(&grpc_task).expect("Failed to serialize");
    assert!(json_str.contains("\"call\":\"grpc\""));
    assert!(json_str.contains("\"method\":\"DoSomething\""));
}

#[test]
fn test_call_grpc_task_deserialization() {
    use serverless_workflow_core::models::call::*;
    let json = r#"{"call":"grpc","with":{"proto":{"endpoint":"http://proto.example.com/api.proto"},"service":{"name":"Greeter","host":"localhost","port":50051},"method":"SayHello"}}"#;
    let call: CallTaskDefinition = serde_json::from_str(json).expect("Failed to deserialize");
    match call {
        CallTaskDefinition::GRPC(grpc) => {
            assert_eq!(grpc.with.method, "SayHello");
            assert_eq!(grpc.with.service.name, "Greeter");
        }
        _ => panic!("Expected GRPC call type"),
    }
}

#[test]
fn test_call_openapi_task_serialization() {
    use serverless_workflow_core::models::call::*;
    let openapi_task = CallOpenAPIDefinition {
        call: "openapi".to_string(),
        with: OpenAPIArguments {
            document: serverless_workflow_core::models::resource::ExternalResourceDefinition {
                name: None,
                endpoint:
                    serverless_workflow_core::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                        "http://api.example.com/openapi.json".to_string(),
                    ),
            },
            operation_id: "getUsers".to_string(),
            parameters: None,
            authentication: None,
            output: None,
            redirect: None,
        },
        common: TaskDefinitionFields::default(),
    };
    let json_str = serde_json::to_string(&openapi_task).expect("Failed to serialize");
    assert!(json_str.contains("\"call\":\"openapi\""));
    assert!(json_str.contains("\"operationId\":\"getUsers\""));
}

#[test]
fn test_call_openapi_task_deserialization() {
    use serverless_workflow_core::models::call::*;
    let json = r#"{"call":"openapi","with":{"document":{"endpoint":"http://api.example.com/openapi.json"},"operationId":"getUsers"}}"#;
    let call: CallTaskDefinition = serde_json::from_str(json).expect("Failed to deserialize");
    match call {
        CallTaskDefinition::OpenAPI(openapi) => {
            assert_eq!(openapi.with.operation_id, "getUsers");
        }
        _ => panic!("Expected OpenAPI call type"),
    }
}

#[test]
fn test_call_asyncapi_task_serialization() {
    use serverless_workflow_core::models::call::*;
    let asyncapi_task = CallAsyncAPIDefinition {
        call: "asyncapi".to_string(),
        with: AsyncApiArguments {
            document: serverless_workflow_core::models::resource::ExternalResourceDefinition {
                name: None,
                endpoint:
                    serverless_workflow_core::models::resource::OneOfEndpointDefinitionOrUri::Uri(
                        "http://asyncapi.example.com/asyncapi.json".to_string(),
                    ),
            },
            channel: Some("userUpdates".to_string()),
            operation: None,
            server: None,
            protocol: None,
            message: None,
            subscription: None,
            authentication: None,
        },
        common: TaskDefinitionFields::default(),
    };
    let json_str = serde_json::to_string(&asyncapi_task).expect("Failed to serialize");
    assert!(json_str.contains("\"call\":\"asyncapi\""));
    assert!(json_str.contains("\"channel\":\"userUpdates\""));
}

#[test]
fn test_call_asyncapi_task_deserialization() {
    use serverless_workflow_core::models::call::*;
    let json = r#"{"call":"asyncapi","with":{"document":{"endpoint":"http://asyncapi.example.com/asyncapi.json"},"channel":"userUpdates"}}"#;
    let call: CallTaskDefinition = serde_json::from_str(json).expect("Failed to deserialize");
    match call {
        CallTaskDefinition::AsyncAPI(asyncapi) => {
            assert_eq!(asyncapi.with.channel, Some("userUpdates".to_string()));
        }
        _ => panic!("Expected AsyncAPI call type"),
    }
}

#[test]
fn test_call_a2a_task_serialization() {
    use serverless_workflow_core::models::call::*;
    let a2a_task = CallA2ADefinition {
        call: "a2a".to_string(),
        with: A2AArguments {
            agent_card: None,
            server: None,
            method: "message/send".to_string(),
            parameters: None,
        },
        common: TaskDefinitionFields::default(),
    };
    let json_str = serde_json::to_string(&a2a_task).expect("Failed to serialize");
    assert!(json_str.contains("\"call\":\"a2a\""));
    assert!(json_str.contains("\"method\":\"message/send\""));
}

#[test]
fn test_call_a2a_task_deserialization() {
    use serverless_workflow_core::models::call::*;
    let json = r#"{"call":"a2a","with":{"method":"tasks/get"}}"#;
    let call: CallTaskDefinition = serde_json::from_str(json).expect("Failed to deserialize");
    match call {
        CallTaskDefinition::A2A(a2a) => {
            assert_eq!(a2a.with.method, "tasks/get");
        }
        _ => panic!("Expected A2A call type"),
    }
}
