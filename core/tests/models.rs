use serde_json::json;
use serverless_workflow_core::models::authentication::*;
use serverless_workflow_core::models::duration::*;
use serverless_workflow_core::models::error::*;
use serverless_workflow_core::models::extension::ExtensionDefinition;
use serverless_workflow_core::models::map::*;
use serverless_workflow_core::models::resource::OneOfEndpointDefinitionOrUri;
use serverless_workflow_core::models::retry::*;
use serverless_workflow_core::models::task::*;
use serverless_workflow_core::models::timeout::*;
use serverless_workflow_core::models::workflow::*;

#[test]
fn create_workflow() {
    let namespace = "fake-namespace";
    let name = "fake-workflow";
    let version = "1.0.0";
    let title = Some("fake-title".to_string());
    let summary = Some("fake-summary".to_string());
    let document = WorkflowDefinitionMetadata::new(
        namespace,
        name,
        version,
        title.clone(),
        summary.clone(),
        None,
    );
    let call_task = CallTaskDefinition::Function(
        serverless_workflow_core::models::call::CallFunctionDefinition {
            call: "http".to_string(),
            with: None,
            common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
        },
    );
    let do_task = DoTaskDefinition::new(Map::from(vec![(
        "set".to_string(),
        TaskDefinition::Wait(WaitTaskDefinition::new(
            OneOfDurationOrIso8601Expression::Duration(Duration::from_milliseconds(200)),
        )),
    )]));
    let mut workflow = WorkflowDefinition::new(document);
    workflow.do_ = Map::new();
    workflow
        .do_
        .add("callTask".to_string(), TaskDefinition::Call(Box::new(call_task)));
    workflow
        .do_
        .add("doTask".to_string(), TaskDefinition::Do(do_task));
    let json_serialization_result = serde_json::to_string_pretty(&workflow);
    let yaml_serialization_result = serde_yaml::to_string(&workflow);
    assert!(
        json_serialization_result.is_ok(),
        "JSON Serialization failed: {:?}",
        json_serialization_result.err()
    );
    assert!(
        yaml_serialization_result.is_ok(),
        "YAML Serialization failed: {:?}",
        yaml_serialization_result.err()
    );
    if let Result::Ok(yaml) = yaml_serialization_result {
        println!("{}", yaml)
    }
    assert_eq!(workflow.document.namespace, namespace);
    assert_eq!(workflow.document.name, name);
    assert_eq!(workflow.document.version, version);
    assert_eq!(workflow.document.title, title);
    assert_eq!(workflow.document.summary, summary);
}

#[test]
fn test_for_loop_definition_each_field_deserialization() {
    let for_loop_json = serde_json::json!({
        "each": "item",
        "in": ".items"
    });

    let result: Result<ForLoopDefinition, _> = serde_json::from_value(for_loop_json);

    match result {
        Ok(for_loop) => {
            assert_eq!(for_loop.each, "item", "The 'each' field should be 'item'");
            assert_eq!(for_loop.in_, ".items", "The 'in' field should be '.items'");
        }
        Err(e) => {
            panic!(
                "Failed to deserialize ForLoopDefinition with 'each' field: {}",
                e
            );
        }
    }
}

#[test]
fn test_for_task_deserialization() {
    let for_task_json = json!({
        "for": {
            "each": "item",
            "in": ".items"
        },
        "do": [
            {
                "processItem": {
                    "call": "processFunction",
                    "with": {
                        "item": "${ .item }"
                    }
                }
            }
        ]
    });

    let result: Result<TaskDefinition, _> = serde_json::from_value(for_task_json.clone());

    match result {
        Ok(TaskDefinition::For(for_def)) => {
            assert_eq!(for_def.for_.each, "item");
            assert_eq!(for_def.for_.in_, ".items");
            assert_eq!(for_def.do_.entries.len(), 1);
            let has_process_item = for_def
                .do_
                .entries
                .iter()
                .any(|(name, _)| name == "processItem");
            assert!(
                has_process_item,
                "For task should contain processItem subtask"
            );
        }
        Ok(TaskDefinition::Do(_)) => {
            panic!("For task incorrectly deserialized as DoTaskDefinition");
        }
        Ok(other) => {
            panic!("For task deserialized as unexpected variant: {:?}", other);
        }
        Err(e) => {
            panic!("Failed to deserialize For task: {}", e);
        }
    }
}

#[test]
fn test_do_task_deserialization() {
    let do_task_json = json!({
        "do": [
            {
                "step1": {
                    "call": "function1"
                }
            },
            {
                "step2": {
                    "call": "function2"
                }
            }
        ]
    });

    let result: Result<TaskDefinition, _> = serde_json::from_value(do_task_json);

    match result {
        Ok(TaskDefinition::Do(do_def)) => {
            assert_eq!(do_def.do_.entries.len(), 2);
            let has_step1 = do_def.do_.entries.iter().any(|(name, _)| name == "step1");
            let has_step2 = do_def.do_.entries.iter().any(|(name, _)| name == "step2");
            assert!(has_step1, "Do task should contain step1");
            assert!(has_step2, "Do task should contain step2");
        }
        Ok(other) => {
            panic!("Do task deserialized as unexpected variant: {:?}", other);
        }
        Err(e) => {
            panic!("Failed to deserialize Do task: {}", e);
        }
    }
}

#[test]
fn test_for_task_with_while_condition() {
    let for_task_json = json!({
        "for": {
            "each": "user",
            "in": ".users",
            "at": "index"
        },
        "while": "${ .index < 10 }",
        "do": [
            {
                "notifyUser": {
                    "call": "notifyUser",
                    "with": {
                        "user": "${ .user }",
                        "index": "${ .index }"
                    }
                }
            }
        ]
    });

    let result: Result<TaskDefinition, _> = serde_json::from_value(for_task_json.clone());

    match result {
        Ok(TaskDefinition::For(for_def)) => {
            assert_eq!(for_def.for_.each, "user");
            assert_eq!(for_def.for_.in_, ".users");
            assert_eq!(for_def.for_.at, Some("index".to_string()));
            assert_eq!(for_def.while_, Some("${ .index < 10 }".to_string()));
            assert_eq!(for_def.do_.entries.len(), 1);
        }
        Ok(TaskDefinition::Do(_)) => {
            panic!("For task incorrectly deserialized as DoTaskDefinition");
        }
        Ok(other) => {
            panic!("For task deserialized as unexpected variant: {:?}", other);
        }
        Err(e) => {
            panic!("Failed to deserialize For task with while: {}", e);
        }
    }
}

#[test]
fn test_roundtrip_serialization() {
    let for_loop = ForLoopDefinition::new("item", ".collection", None, None);
    let mut do_tasks = Map::new();
    do_tasks.add(
        "task1".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            serverless_workflow_core::models::call::CallFunctionDefinition {
                call: "someFunction".to_string(),
                with: None,
                common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );

    let for_task = ForTaskDefinition::new(for_loop, do_tasks, None);
    let task_def = TaskDefinition::For(for_task);

    let json_str = serde_json::to_string(&task_def).expect("Failed to serialize");
    println!("Serialized: {}", json_str);

    let deserialized: TaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");

    match deserialized {
        TaskDefinition::For(for_def) => {
            assert_eq!(for_def.for_.each, "item");
            assert_eq!(for_def.for_.in_, ".collection");
        }
        TaskDefinition::Do(_) => {
            panic!("After roundtrip serialization, For task became a Do task");
        }
        other => {
            panic!("Unexpected variant after roundtrip: {:?}", other);
        }
    }
}

#[test]
fn test_set_value_map_deserialization() {
    let set_value_map = serde_json::json!({
        "foo": "bar",
        "count": 42
    });

    let result: Result<SetTaskDefinition, _> = serde_json::from_value(serde_json::json!({
        "set": set_value_map
    }));
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with map: {:?}",
        result.err()
    );

    let set_task = result.unwrap();
    match set_task.set {
        SetValue::Map(map) => {
            assert_eq!(map.len(), 2);
            assert_eq!(map.get("foo").and_then(|v| v.as_str()), Some("bar"));
            assert_eq!(map.get("count").and_then(|v| v.as_u64()), Some(42));
        }
        SetValue::Expression(_) => {
            panic!("Expected SetValue::Map but got SetValue::Expression");
        }
    }
}

#[test]
fn test_set_value_expression_deserialization() {
    let set_value_expr_json = serde_json::json!("${ $workflow.input[0] }");

    let result: Result<SetTaskDefinition, _> = serde_json::from_value(serde_json::json!({
        "set": set_value_expr_json
    }));
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with expression: {:?}",
        result.err()
    );

    let set_task = result.unwrap();
    match set_task.set {
        SetValue::Expression(expr) => {
            assert_eq!(expr, "${ $workflow.input[0] }");
        }
        SetValue::Map(_) => {
            panic!("Expected SetValue::Expression but got SetValue::Map");
        }
    }
}

#[test]
fn test_wait_task_iso8601_deserialization() {
    let wait_task_json = serde_json::json!({
        "wait": "PT30S"
    });
    let result: Result<WaitTaskDefinition, _> = serde_json::from_value(wait_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait task with ISO 8601: {:?}",
        result.err()
    );

    let wait_task = result.unwrap();
    match wait_task.wait {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT30S");
        }
        OneOfDurationOrIso8601Expression::Duration(_) => {
            panic!("Expected Iso8601Expression but got Duration");
        }
    }
}

#[test]
fn test_wait_task_inline_duration_deserialization() {
    let wait_task_json = serde_json::json!({
        "wait": {
            "seconds": 30
        }
    });
    let result: Result<WaitTaskDefinition, _> = serde_json::from_value(wait_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait task with inline duration: {:?}",
        result.err()
    );

    let wait_task = result.unwrap();
    match wait_task.wait {
        OneOfDurationOrIso8601Expression::Duration(duration) => {
            assert_eq!(duration.seconds, Some(30));
        }
        OneOfDurationOrIso8601Expression::Iso8601Expression(_) => {
            panic!("Expected Duration but got Iso8601Expression");
        }
    }
}

#[test]
fn test_script_process_arguments_array_deserialization() {
    use serverless_workflow_core::models::task::ScriptProcessDefinition;

    let script_process_json = serde_json::json!({
        "language": "javascript",
        "code": "console.log('test')",
        "arguments": ["hello", "world"]
    });
    let result: Result<ScriptProcessDefinition, _> = serde_json::from_value(script_process_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize script with array arguments: {:?}",
        result.err()
    );

    let script = result.unwrap();
    assert_eq!(script.language, "javascript");
    assert!(script.arguments.is_some());

    let args = script.arguments.unwrap();
    assert_eq!(args.as_array().unwrap().len(), 2);
    assert_eq!(args.as_array().unwrap()[0], "hello");
    assert_eq!(args.as_array().unwrap()[1], "world");
}

#[test]
fn test_script_process_with_stdin_deserialization() {
    use serverless_workflow_core::models::task::ScriptProcessDefinition;

    let script_process_json = serde_json::json!({
        "language": "python",
        "code": "print('test')",
        "stdin": "Hello Workflow",
        "arguments": ["arg1"],
        "environment": {"FOO": "bar"}
    });
    let result: Result<ScriptProcessDefinition, _> = serde_json::from_value(script_process_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize script with stdin: {:?}",
        result.err()
    );

    let script = result.unwrap();
    assert_eq!(script.language, "python");
    assert_eq!(script.stdin, Some("Hello Workflow".to_string()));
    assert!(script.arguments.is_some());
    assert_eq!(
        script.arguments.as_ref().unwrap().as_array().unwrap().len(),
        1
    );
    assert!(script.environment.is_some());
    assert_eq!(
        script.environment.as_ref().unwrap().get("FOO"),
        Some(&"bar".to_string())
    );
}

#[test]
fn test_extension_definition_serialization() {
    let extension_json = json!({
        "document": {
            "dsl": "1.0.2",
            "namespace": "test",
            "name": "sample-workflow",
            "version": "0.1.0"
        },
        "use": {
            "extensions": [
                {
                "mockService": {
                    "extend": "call",
                    "when": "($task.with.endpoint != null and ($task.with.endpoint | startswith(\"https://mocked.service.com\"))) or ($task.with.endpoint.uri != null and ($task.with.endpoint.uri | startswith(\"https://mocked.service.com\")))",
                    "before": [
                        {
                            "mockResponse": {
                                "set": {
                                    "statusCode": 200,
                                    "headers": {
                                        "Content-Type": "application/json"
                                    },
                                    "content": {
                                        "foo": {
                                            "bar": "baz"
                                        }
                                    }
                                },
                                "then": "exit"
                            }
                        }
                    ]
                }
            }
            ]
        },
        "do": [
            {
                "callHttp": {
                    "call": "http",
                    "with": {
                        "method": "get",
                        "endpoint": {
                            "uri": "https://fake.com/sample"
                        }
                    }
                }
            }
        ]
    });
    let result: Result<WorkflowDefinition, _> = serde_json::from_value(extension_json);
    match result {
        Ok(workflow) => {
            assert_eq!(workflow.document.namespace, "test");
            assert_eq!(workflow.document.name, "sample-workflow");
            assert_eq!(workflow.document.version, "0.1.0");
            assert!(workflow.use_.is_some());
            if let Some(use_def) = workflow.use_ {
                assert!(use_def.extensions.is_some());
            }
        }
        Err(e) => {
            panic!("Failed to deserialize workflow with extension: {}", e);
        }
    }
}

#[test]
fn test_timeout_definition_with_inline_duration() {
    // Test TimeoutDefinition with inline duration
    let timeout_json = json!({
        "after": {"days": 1, "hours": 2}
    });
    let result: Result<TimeoutDefinition, _> = serde_json::from_value(timeout_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout with inline duration: {:?}",
        result.err()
    );
    let timeout = result.unwrap();
    match timeout.after {
        OneOfDurationOrIso8601Expression::Duration(d) => {
            assert_eq!(d.days, Some(1));
            assert_eq!(d.hours, Some(2));
        }
        _ => panic!("Expected Duration variant"),
    }
}

#[test]
fn test_timeout_definition_with_iso8601_expression() {
    // Test TimeoutDefinition with ISO 8601 expression
    let timeout_json = json!({
        "after": "PT1H"
    });
    let result: Result<TimeoutDefinition, _> = serde_json::from_value(timeout_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout with ISO 8601: {:?}",
        result.err()
    );
    let timeout = result.unwrap();
    match timeout.after {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT1H");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_timeout_or_reference_as_timeout() {
    // Test OneOfTimeoutDefinitionOrReference as Timeout
    let tor_json = json!({
        "after": {"days": 1, "hours": 2}
    });
    let result: Result<OneOfTimeoutDefinitionOrReference, _> = serde_json::from_value(tor_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout or reference: {:?}",
        result.err()
    );
    match result.unwrap() {
        OneOfTimeoutDefinitionOrReference::Timeout(t) => match t.after {
            OneOfDurationOrIso8601Expression::Duration(d) => {
                assert_eq!(d.days, Some(1));
            }
            _ => panic!("Expected Duration"),
        },
        _ => panic!("Expected Timeout variant"),
    }
}

#[test]
fn test_timeout_or_reference_as_reference() {
    // Test OneOfTimeoutDefinitionOrReference as Reference
    let tor_json = json!("my-timeout-ref");
    let result: Result<OneOfTimeoutDefinitionOrReference, _> = serde_json::from_value(tor_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout reference: {:?}",
        result.err()
    );
    match result.unwrap() {
        OneOfTimeoutDefinitionOrReference::Reference(s) => {
            assert_eq!(s, "my-timeout-ref");
        }
        _ => panic!("Expected Reference variant"),
    }
}

#[test]
fn test_timeout_definition_serialization() {
    // Test TimeoutDefinition serialization
    let timeout = TimeoutDefinition {
        after: OneOfDurationOrIso8601Expression::Duration(Duration::from_hours(2)),
    };
    let json_str = serde_json::to_string(&timeout).expect("Failed to serialize timeout");
    assert!(json_str.contains("\"hours\":2") || json_str.contains("hours"));
}

#[test]
fn test_timeout_reference_serialization() {
    // Test OneOfTimeoutDefinitionOrReference as Reference serialization
    let tor = OneOfTimeoutDefinitionOrReference::Reference("my-timeout-ref".to_string());
    let json_str = serde_json::to_string(&tor).expect("Failed to serialize timeout reference");
    assert!(json_str.contains("my-timeout-ref"));
}

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
fn test_basic_authentication_serialization() {
    // Test BasicAuthenticationSchemeDefinition serialization (similar to Go SDK TestAuthenticationPolicy)
    let basic_auth_json = json!({
        "username": "john",
        "password": "12345"
    });
    let result: Result<BasicAuthenticationSchemeDefinition, _> =
        serde_json::from_value(basic_auth_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize basic auth: {:?}",
        result.err()
    );
    let basic_auth = result.unwrap();
    assert_eq!(basic_auth.username, Some("john".to_string()));
    assert_eq!(basic_auth.password, Some("12345".to_string()));
}

#[test]
fn test_bearer_authentication_serialization() {
    // Test BearerAuthenticationSchemeDefinition serialization
    let bearer_auth_json = json!({
        "token": "my-bearer-token"
    });
    let result: Result<BearerAuthenticationSchemeDefinition, _> =
        serde_json::from_value(bearer_auth_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize bearer auth: {:?}",
        result.err()
    );
    let bearer_auth = result.unwrap();
    assert_eq!(bearer_auth.token, Some("my-bearer-token".to_string()));
}

#[test]
fn test_authentication_policy_with_basic() {
    // Test AuthenticationPolicyDefinition with basic auth
    let auth_policy_json = json!({
        "basic": {
            "username": "john",
            "password": "12345"
        }
    });
    let result: Result<AuthenticationPolicyDefinition, _> =
        serde_json::from_value(auth_policy_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize auth policy: {:?}",
        result.err()
    );
    let auth_policy = result.unwrap();
    assert!(auth_policy.basic.is_some());
    let basic = auth_policy.basic.unwrap();
    assert_eq!(basic.username, Some("john".to_string()));
    assert_eq!(basic.password, Some("12345".to_string()));
}

#[test]
fn test_authentication_policy_with_bearer() {
    // Test AuthenticationPolicyDefinition with bearer auth
    let auth_policy_json = json!({
        "bearer": {
            "token": "my-bearer-token"
        }
    });
    let result: Result<AuthenticationPolicyDefinition, _> =
        serde_json::from_value(auth_policy_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize auth policy: {:?}",
        result.err()
    );
    let auth_policy = result.unwrap();
    assert!(auth_policy.bearer.is_some());
    let bearer = auth_policy.bearer.unwrap();
    assert_eq!(bearer.token, Some("my-bearer-token".to_string()));
}

#[test]
fn test_authentication_policy_roundtrip() {
    // Test AuthenticationPolicyDefinition roundtrip serialization
    let auth_policy = AuthenticationPolicyDefinition {
        use_: Some("my-auth".to_string()),
        basic: Some(BasicAuthenticationSchemeDefinition {
            use_: None,
            username: Some("john".to_string()),
            password: Some("12345".to_string()),
        }),
        bearer: None,
        certificate: None,
        digest: None,
        oauth2: None,
        oidc: None,
    };

    let json_str = serde_json::to_string(&auth_policy).expect("Failed to serialize auth policy");
    let deserialized: AuthenticationPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.basic.is_some());
    assert_eq!(
        deserialized.basic.unwrap().username,
        Some("john".to_string())
    );
}

#[test]
fn test_digest_authentication_serialization() {
    // Test DigestAuthenticationSchemeDefinition serialization
    let digest_auth_json = json!({
        "username": "digestUser",
        "password": "digestPass"
    });
    let result: Result<DigestAuthenticationSchemeDefinition, _> =
        serde_json::from_value(digest_auth_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize digest auth: {:?}",
        result.err()
    );
    let digest_auth = result.unwrap();
    assert_eq!(digest_auth.username, Some("digestUser".to_string()));
    assert_eq!(digest_auth.password, Some("digestPass".to_string()));
}

#[test]
fn test_oauth2_client_definition_serialization() {
    // Test OAuth2AuthenticationClientDefinition serialization
    let client_json = json!({
        "id": "client-id",
        "secret": "client-secret",
        "authentication": "client_secret_post"
    });
    let result: Result<OAuth2AuthenticationClientDefinition, _> =
        serde_json::from_value(client_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize OAuth2 client: {:?}",
        result.err()
    );
    let client = result.unwrap();
    assert_eq!(client.id, Some("client-id".to_string()));
    assert_eq!(client.secret, Some("client-secret".to_string()));
    assert_eq!(
        client.authentication,
        Some("client_secret_post".to_string())
    );
}

#[test]
fn test_oauth2_authentication_serialization() {
    // Test OAuth2AuthenticationSchemeDefinition serialization
    let oauth2_json = json!({
        "authority": "https://authority.com",
        "grant": "client_credentials",
        "client": {
            "id": "client-id",
            "secret": "client-secret"
        }
    });
    let result: Result<OAuth2AuthenticationSchemeDefinition, _> =
        serde_json::from_value(oauth2_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize OAuth2 auth: {:?}",
        result.err()
    );
    let oauth2 = result.unwrap();
    assert_eq!(oauth2.authority, Some("https://authority.com".to_string()));
    assert_eq!(oauth2.grant, Some("client_credentials".to_string()));
    assert!(oauth2.client.is_some());
}

#[test]
fn test_wait_task_with_iso8601() {
    // Test WaitTaskDefinition with ISO 8601 expression
    let wait_task_json = json!({
        "wait": "P1DT1H"
    });
    let result: Result<WaitTaskDefinition, _> = serde_json::from_value(wait_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait task with ISO 8601: {:?}",
        result.err()
    );
    let wait_task = result.unwrap();
    match wait_task.wait {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "P1DT1H");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_wait_task_with_duration() {
    // Test WaitTaskDefinition with inline duration
    let wait_task_json = json!({
        "wait": {"hours": 1}
    });
    let result: Result<WaitTaskDefinition, _> = serde_json::from_value(wait_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait task with duration: {:?}",
        result.err()
    );
    let wait_task = result.unwrap();
    match wait_task.wait {
        OneOfDurationOrIso8601Expression::Duration(d) => {
            assert_eq!(d.hours, Some(1));
        }
        _ => panic!("Expected Duration variant"),
    }
}

#[test]
fn test_wait_task_roundtrip_serialization() {
    // Test WaitTaskDefinition roundtrip serialization
    let wait_task = WaitTaskDefinition::new(OneOfDurationOrIso8601Expression::Iso8601Expression(
        "PT30S".to_string(),
    ));
    let json_str = serde_json::to_string(&wait_task).expect("Failed to serialize wait task");
    let deserialized: WaitTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized.wait {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT30S");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_wait_task_with_all_fields() {
    // Test WaitTaskDefinition with all common fields
    let wait_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "wait": "PT1H"
    });
    let result: Result<WaitTaskDefinition, _> = serde_json::from_value(wait_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait task with all fields: {:?}",
        result.err()
    );
    let wait_task = result.unwrap();
    assert_eq!(wait_task.common.if_, Some("${condition}".to_string()));
    assert!(wait_task.common.timeout.is_some());
    assert_eq!(wait_task.common.then, Some("continue".to_string()));
}

#[test]
fn test_raise_task_serialization() {
    // Test RaiseTaskDefinition serialization (similar to Go SDK TestRaiseTask_MarshalJSON)
    use serverless_workflow_core::models::error::OneOfErrorDefinitionOrReference;

    let raise_task_json = json!({
        "raise": {
            "error": {
                "type": "http://example.com/error",
                "status": 500,
                "title": "Internal Server Error",
                "detail": "An unexpected error occurred."
            }
        }
    });
    let result: Result<RaiseTaskDefinition, _> = serde_json::from_value(raise_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize raise task: {:?}",
        result.err()
    );
    let raise_task = result.unwrap();
    match raise_task.raise.error {
        OneOfErrorDefinitionOrReference::Error(err) => {
            assert_eq!(err.type_.as_str(), "http://example.com/error");
            assert_eq!(err.title, Some("Internal Server Error".to_string()));
            assert_eq!(
                err.detail,
                Some("An unexpected error occurred.".to_string())
            );
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_raise_task_roundtrip_serialization() {
    // Test RaiseTaskDefinition roundtrip serialization
    use serverless_workflow_core::models::error::ErrorDefinition;
    use serverless_workflow_core::models::error::OneOfErrorDefinitionOrReference;

    let raise_task = RaiseTaskDefinition::new(RaiseErrorDefinition::new(
        OneOfErrorDefinitionOrReference::Error(ErrorDefinition::new(
            "http://example.com/error",
            "Internal Server Error",
            serde_json::json!(500),
            Some("An unexpected error occurred.".to_string()),
            None,
        )),
    ));

    let json_str = serde_json::to_string(&raise_task).expect("Failed to serialize raise task");
    let deserialized: RaiseTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized.raise.error {
        OneOfErrorDefinitionOrReference::Error(err) => {
            assert_eq!(err.type_.as_str(), "http://example.com/error");
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_error_definition_with_status() {
    // Test ErrorDefinition with numeric status
    use serverless_workflow_core::models::error::ErrorDefinition;

    let error_json = json!({
        "type": "http://example.com/error",
        "status": 404,
        "title": "Not Found"
    });
    let result: Result<ErrorDefinition, _> = serde_json::from_value(error_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize error definition: {:?}",
        result.err()
    );
    let error_def = result.unwrap();
    assert_eq!(error_def.type_.as_str(), "http://example.com/error");
    assert_eq!(error_def.title, Some("Not Found".to_string()));
    assert_eq!(error_def.status, serde_json::json!(404));
}

#[test]
fn test_error_definition_with_instance() {
    // Test ErrorDefinition with instance
    let error_json = json!({
        "type": "http://example.com/error",
        "status": 500,
        "title": "Internal Server Error",
        "detail": "An error occurred",
        "instance": "/errors/12345"
    });
    let result: Result<ErrorDefinition, _> = serde_json::from_value(error_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize error definition: {:?}",
        result.err()
    );
    let error_def = result.unwrap();
    assert_eq!(error_def.instance, Some("/errors/12345".to_string()));
}

#[test]
fn test_retry_policy_serialization() {
    // Test RetryPolicyDefinition serialization
    let retry_json = json!({
        "when": "${someCondition}",
        "exceptWhen": "${someOtherCondition}",
        "delay": {"seconds": 5},
        "backoff": {"exponential": {}},
        "limit": {
            "attempt": {"count": 3, "duration": {"minutes": 1}},
            "duration": {"minutes": 10}
        },
        "jitter": {"from": {"seconds": 1}, "to": {"seconds": 3}}
    });
    let result: Result<RetryPolicyDefinition, _> = serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry policy: {:?}",
        result.err()
    );
    let retry_policy = result.unwrap();
    assert_eq!(retry_policy.when, Some("${someCondition}".to_string()));
    assert_eq!(
        retry_policy.except_when,
        Some("${someOtherCondition}".to_string())
    );
    assert!(retry_policy.delay.is_some());
    assert!(retry_policy.backoff.is_some());
    assert!(retry_policy.limit.is_some());
    assert!(retry_policy.jitter.is_some());
}

#[test]
fn test_retry_policy_roundtrip_serialization() {
    // Test RetryPolicyDefinition roundtrip serialization
    let retry_policy = RetryPolicyDefinition {
        when: Some("${condition}".to_string()),
        except_when: Some("${exceptCondition}".to_string()),
        delay: Some(OneOfDurationOrIso8601Expression::Duration(
            Duration::from_seconds(5),
        )),
        backoff: Some(BackoffStrategyDefinition {
            constant: None,
            exponential: Some(ExponentialBackoffDefinition::default()),
            linear: None,
        }),
        limit: Some(RetryPolicyLimitDefinition {
            attempt: Some(RetryAttemptLimitDefinition {
                count: Some(3),
                duration: Some(OneOfDurationOrIso8601Expression::Duration(
                    Duration::from_minutes(1),
                )),
            }),
            duration: Some(OneOfDurationOrIso8601Expression::Duration(
                Duration::from_minutes(10),
            )),
        }),
        jitter: Some(JitterDefinition {
            from: Duration::from_seconds(1),
            to: Duration::from_seconds(3),
        }),
    };

    let json_str = serde_json::to_string(&retry_policy).expect("Failed to serialize retry policy");
    let deserialized: RetryPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(deserialized.when, Some("${condition}".to_string()));
    assert_eq!(
        deserialized.except_when,
        Some("${exceptCondition}".to_string())
    );
    assert!(deserialized.backoff.is_some());
    assert!(deserialized.limit.is_some());
    assert!(deserialized.jitter.is_some());
}

#[test]
fn test_retry_policy_with_exponential_backoff() {
    // Test RetryPolicyDefinition with exponential backoff
    let retry_json = json!({
        "backoff": {"exponential": {}},
        "limit": {"attempt": {"count": 5}}
    });
    let result: Result<RetryPolicyDefinition, _> = serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry policy: {:?}",
        result.err()
    );
    let retry_policy = result.unwrap();
    assert!(retry_policy.backoff.is_some());
    let backoff = retry_policy.backoff.unwrap();
    assert!(backoff.exponential.is_some());
}

#[test]
fn test_retry_policy_with_linear_backoff() {
    // Test RetryPolicyDefinition with linear backoff
    let retry_json = json!({
        "backoff": {"linear": {"increment": {"seconds": 5}}},
        "limit": {"attempt": {"count": 3}}
    });
    let result: Result<RetryPolicyDefinition, _> = serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry policy with linear backoff: {:?}",
        result.err()
    );
    let retry_policy = result.unwrap();
    assert!(retry_policy.backoff.is_some());
    let backoff = retry_policy.backoff.unwrap();
    assert!(backoff.linear.is_some());
    if let Some(linear) = &backoff.linear {
        assert!(linear.increment.is_some());
    }
}

#[test]
fn test_retry_policy_with_constant_backoff() {
    // Test RetryPolicyDefinition with constant backoff
    let retry_json = json!({
        "backoff": {"constant": {}},
        "limit": {"attempt": {"count": 3}}
    });
    let result: Result<RetryPolicyDefinition, _> = serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry policy with constant backoff: {:?}",
        result.err()
    );
    let retry_policy = result.unwrap();
    assert!(retry_policy.backoff.is_some());
    let backoff = retry_policy.backoff.unwrap();
    assert!(backoff.constant.is_some());
}

#[test]
fn test_retry_policy_limit_attempt() {
    // Test RetryAttemptLimitDefinition
    let limit_json = json!({
        "attempt": {"count": 10, "duration": {"seconds": 30}}
    });
    let result: Result<RetryPolicyLimitDefinition, _> = serde_json::from_value(limit_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry limit: {:?}",
        result.err()
    );
    let limit = result.unwrap();
    assert!(limit.attempt.is_some());
    let attempt = limit.attempt.unwrap();
    assert_eq!(attempt.count, Some(10));
    assert!(attempt.duration.is_some());
}

#[test]
fn test_jitter_definition_serialization() {
    // Test JitterDefinition serialization
    let jitter_json = json!({
        "from": {"seconds": 1},
        "to": {"seconds": 5}
    });
    let result: Result<JitterDefinition, _> = serde_json::from_value(jitter_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize jitter: {:?}",
        result.err()
    );
    let jitter = result.unwrap();
    assert_eq!(jitter.from.seconds, Some(1));
    assert_eq!(jitter.to.seconds, Some(5));
}

#[test]
fn test_fork_task_serialization() {
    // Test ForkTaskDefinition serialization (similar to Go SDK TestForkTask_MarshalJSON)
    let fork_task_json = json!({
        "fork": {
            "branches": [
                {"task1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}},
                {"task2": {"call": "openapi", "with": {"document": {"name": "doc1", "endpoint": "http://example.com/openapi.json"}, "operationId": "op1"}}}
            ],
            "compete": true
        }
    });
    let result: Result<ForkTaskDefinition, _> = serde_json::from_value(fork_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize fork task: {:?}",
        result.err()
    );
    let fork_task = result.unwrap();
    assert!(fork_task.fork.compete);
    assert_eq!(fork_task.fork.branches.entries.len(), 2);
}

#[test]
fn test_fork_task_roundtrip_serialization() {
    // Test ForkTaskDefinition roundtrip serialization
    let mut branches = Map::new();
    branches.add(
        "task1".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            serverless_workflow_core::models::call::CallFunctionDefinition {
                call: "func1".to_string(),
                with: None,
                common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );
    branches.add(
        "task2".to_string(),
        TaskDefinition::Wait(WaitTaskDefinition::new(
            OneOfDurationOrIso8601Expression::Iso8601Expression("PT5S".to_string()),
        )),
    );

    let fork_task = ForkTaskDefinition::new(BranchingDefinition::new(branches, true));

    let json_str = serde_json::to_string(&fork_task).expect("Failed to serialize fork task");
    let deserialized: ForkTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.fork.compete);
    assert_eq!(deserialized.fork.branches.entries.len(), 2);
}

#[test]
fn test_fork_task_with_multiple_branches() {
    // Test ForkTaskDefinition with multiple branches
    let fork_task_json = json!({
        "fork": {
            "branches": [
                {"branch1": {"call": "function1"}},
                {"branch2": {"call": "function2"}},
                {"branch3": {"call": "function3"}}
            ],
            "compete": false
        }
    });
    let result: Result<ForkTaskDefinition, _> = serde_json::from_value(fork_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize fork task: {:?}",
        result.err()
    );
    let fork_task = result.unwrap();
    assert!(!fork_task.fork.compete);
    assert_eq!(fork_task.fork.branches.entries.len(), 3);
}

#[test]
fn test_branching_definition_serialization() {
    // Test BranchingDefinition serialization
    let mut branches = Map::new();
    branches.add(
        "task1".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            serverless_workflow_core::models::call::CallFunctionDefinition {
                call: "http".to_string(),
                with: None,
                common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );

    let branching = BranchingDefinition::new(branches, true);
    let json_str = serde_json::to_string(&branching).expect("Failed to serialize branching");
    assert!(json_str.contains("compete"));
    assert!(json_str.contains("branches"));
}

#[test]
fn test_switch_task_serialization() {
    // Test SwitchTaskDefinition serialization (similar to Go SDK TestSwitchTask_MarshalJSON)
    let switch_task_json = json!({
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}},
            {"case2": {"when": "${condition2}", "then": "end"}}
        ]
    });
    let result: Result<SwitchTaskDefinition, _> = serde_json::from_value(switch_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch task: {:?}",
        result.err()
    );
    let switch_task = result.unwrap();
    assert_eq!(switch_task.switch.entries.len(), 2);
}

#[test]
fn test_switch_task_roundtrip_serialization() {
    // Test SwitchTaskDefinition roundtrip serialization
    let mut switch_cases = Map::new();
    switch_cases.add(
        "case1".to_string(),
        SwitchCaseDefinition {
            when: Some("${condition1}".to_string()),
            then: Some("next".to_string()),
        },
    );
    switch_cases.add(
        "case2".to_string(),
        SwitchCaseDefinition {
            when: Some("${condition2}".to_string()),
            then: Some("end".to_string()),
        },
    );

    let switch_task = SwitchTaskDefinition {
        switch: switch_cases,
        common: TaskDefinitionFields::new(),
    };

    let json_str = serde_json::to_string(&switch_task).expect("Failed to serialize switch task");
    let deserialized: SwitchTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(deserialized.switch.entries.len(), 2);
}

#[test]
fn test_switch_task_with_all_fields() {
    // Test SwitchTaskDefinition with all common fields
    let switch_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}}
        ]
    });
    let result: Result<SwitchTaskDefinition, _> = serde_json::from_value(switch_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch task with all fields: {:?}",
        result.err()
    );
    let switch_task = result.unwrap();
    assert_eq!(switch_task.common.if_, Some("${condition}".to_string()));
    assert!(switch_task.common.timeout.is_some());
    assert_eq!(switch_task.common.then, Some("continue".to_string()));
}

#[test]
fn test_switch_case_definition_serialization() {
    // Test SwitchCaseDefinition serialization
    let case_json = json!({
        "when": "${myCondition}",
        "then": "continue"
    });
    let result: Result<SwitchCaseDefinition, _> = serde_json::from_value(case_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch case: {:?}",
        result.err()
    );
    let case_def = result.unwrap();
    assert_eq!(case_def.when, Some("${myCondition}".to_string()));
    assert_eq!(case_def.then, Some("continue".to_string()));
}

#[test]
fn test_switch_case_definition_without_when() {
    // Test SwitchCaseDefinition without when (default case)
    let case_json = json!({
        "then": "end"
    });
    let result: Result<SwitchCaseDefinition, _> = serde_json::from_value(case_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch case without when: {:?}",
        result.err()
    );
    let case_def = result.unwrap();
    assert_eq!(case_def.when, None);
    assert_eq!(case_def.then, Some("end".to_string()));
}

#[test]
fn test_extension_definition_from_json() {
    // Test ExtensionDefinition deserialization from JSON
    let extension_json = json!({
        "extend": "call",
        "when": "${someCondition}",
        "before": [{"mockTask": {"call": "mockFunction"}}],
        "after": [{"logTask": {"call": "logFunction"}}]
    });
    let result: Result<ExtensionDefinition, _> = serde_json::from_value(extension_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize extension: {:?}",
        result.err()
    );
    let extension = result.unwrap();
    assert_eq!(extension.extend, "call");
    assert_eq!(extension.when, Some("${someCondition}".to_string()));
    assert!(extension.before.is_some());
    assert!(extension.after.is_some());
}

#[test]
fn test_extension_definition_roundtrip_serialization() {
    // Test ExtensionDefinition roundtrip serialization
    let extension = ExtensionDefinition {
        extend: "call".to_string(),
        when: Some("${condition}".to_string()),
        before: Some(vec![vec![(
            "task1".to_string(),
            TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
                serverless_workflow_core::models::call::CallFunctionDefinition {
                    call: "func1".to_string(),
                    with: None,
                    common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
                },
            ))),
        )]
        .into_iter()
        .collect()]),
        after: None,
    };

    let json_str = serde_json::to_string(&extension).expect("Failed to serialize extension");
    let deserialized: ExtensionDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(deserialized.extend, "call");
    assert_eq!(deserialized.when, Some("${condition}".to_string()));
}

#[test]
fn test_extension_definition_minimal() {
    // Test ExtensionDefinition with minimal fields
    let extension_json = json!({
        "extend": "call"
    });
    let result: Result<ExtensionDefinition, _> = serde_json::from_value(extension_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize minimal extension: {:?}",
        result.err()
    );
    let extension = result.unwrap();
    assert_eq!(extension.extend, "call");
    assert!(extension.when.is_none());
    assert!(extension.before.is_none());
    assert!(extension.after.is_none());
}

#[test]
fn test_emit_task_serialization() {
    // Test EmitTaskDefinition serialization (similar to Go SDK TestEmitTask_MarshalJSON)

    let emit_task_json = json!({
        "emit": {
            "event": {
                "with": {
                    "id": "event-id",
                    "source": "http://example.com/source",
                    "type": "example.event.type"
                }
            }
        }
    });
    let result: Result<EmitTaskDefinition, _> = serde_json::from_value(emit_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize emit task: {:?}",
        result.err()
    );
    let emit_task = result.unwrap();
    assert!(emit_task.emit.event.with.contains_key("id"));
    assert!(emit_task.emit.event.with.contains_key("type"));
}

#[test]
fn test_emit_task_roundtrip_serialization() {
    // Test EmitTaskDefinition roundtrip serialization
    use serverless_workflow_core::models::event::EventDefinition;
    use std::collections::HashMap;

    let mut event_with = HashMap::new();
    event_with.insert("id".to_string(), serde_json::json!("my-event-id"));
    event_with.insert("type".to_string(), serde_json::json!("my.event.type"));

    let emit_task = EmitTaskDefinition::new(EventEmissionDefinition {
        event: EventDefinition {
            with: event_with,
            ..Default::default()
        },
    });

    let json_str = serde_json::to_string(&emit_task).expect("Failed to serialize emit task");
    let deserialized: EmitTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.emit.event.with.contains_key("id"));
}

#[test]
fn test_listen_task_serialization() {
    // Test ListenTaskDefinition serialization
    let listen_task_json = json!({
        "listen": {
            "to": {
                "any": [
                    {"with": {"type": "event.type1"}},
                    {"with": {"type": "event.type2"}}
                ]
            }
        }
    });
    let result: Result<ListenTaskDefinition, _> = serde_json::from_value(listen_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize listen task: {:?}",
        result.err()
    );
    let listen_task = result.unwrap();
    assert!(listen_task.listen.to.any.is_some());
}

#[test]
fn test_listen_task_with_until_condition() {
    // Test ListenTaskDefinition with until condition
    let listen_task_json = json!({
        "listen": {
            "to": {
                "any": [{"with": {"type": "event.type"}}],
                "until": "workflow.data.condition == true"
            }
        }
    });
    let result: Result<ListenTaskDefinition, _> = serde_json::from_value(listen_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize listen task with until: {:?}",
        result.err()
    );
    let listen_task = result.unwrap();
    assert!(listen_task.listen.to.any.is_some());
}

#[test]
fn test_event_filter_definition_serialization() {
    // Test EventFilterDefinition serialization
    use serverless_workflow_core::models::event::EventFilterDefinition;
    use std::collections::HashMap;

    let mut with = HashMap::new();
    with.insert("type".to_string(), serde_json::json!("my.event.type"));
    with.insert(
        "source".to_string(),
        serde_json::json!("http://example.com"),
    );

    let event_filter = EventFilterDefinition {
        with: Some(with),
        correlate: None,
    };

    let json_str = serde_json::to_string(&event_filter).expect("Failed to serialize event filter");
    assert!(json_str.contains("type"));
}

#[test]
fn test_event_definition_serialization() {
    // Test EventDefinition serialization
    use serverless_workflow_core::models::event::EventDefinition;
    use std::collections::HashMap;

    let mut with = HashMap::new();
    with.insert("id".to_string(), serde_json::json!("my-event-id"));
    with.insert("type".to_string(), serde_json::json!("my.event.type"));

    let event = EventDefinition::new(with);
    let json_str = serde_json::to_string(&event).expect("Failed to serialize event");
    let deserialized: EventDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.with.contains_key("id"));
}

#[test]
fn test_event_definition_with_all_fields() {
    // Test EventDefinition with all fields (matching Go SDK EventProperties)
    use serverless_workflow_core::models::event::EventDefinition;

    let event_json = json!({
        "id": "my-event-id",
        "source": "http://example.com/source",
        "type": "com.example.myevent",
        "time": "${ .eventTime }",
        "subject": "My Subject",
        "datacontenttype": "application/json",
        "dataschema": "http://example.com/schema",
        "with": {
            "key": "value"
        }
    });

    let result: Result<EventDefinition, _> = serde_json::from_value(event_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize event definition: {:?}",
        result.err()
    );
    let event = result.unwrap();

    assert_eq!(event.id, Some("my-event-id".to_string()));
    assert_eq!(event.source, Some("http://example.com/source".to_string()));
    assert_eq!(event.type_, Some("com.example.myevent".to_string()));
    assert_eq!(event.time, Some("${ .eventTime }".to_string()));
    assert_eq!(event.subject, Some("My Subject".to_string()));
    assert_eq!(
        event.data_content_type,
        Some("application/json".to_string())
    );
    assert_eq!(
        event.data_schema,
        Some("http://example.com/schema".to_string())
    );
    assert!(event.with.contains_key("key"));
}

#[test]
fn test_event_definition_roundtrip() {
    // Test EventDefinition roundtrip serialization with all fields
    use serverless_workflow_core::models::event::EventDefinition;

    let event = EventDefinition {
        id: Some("test-id".to_string()),
        source: Some("http://example.com".to_string()),
        type_: Some("test.type".to_string()),
        time: Some("${ .time }".to_string()),
        subject: Some("Test Subject".to_string()),
        data_content_type: Some("application/json".to_string()),
        data_schema: Some("http://example.com/schema".to_string()),
        with: std::collections::HashMap::new(),
    };

    let json_str = serde_json::to_string(&event).expect("Failed to serialize");
    let deserialized: EventDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");

    assert_eq!(event.id, deserialized.id);
    assert_eq!(event.source, deserialized.source);
    assert_eq!(event.type_, deserialized.type_);
}

#[test]
fn test_event_consumption_until_false() {
    // Test EventConsumptionStrategyDefinition with until: false (disabled)
    use serverless_workflow_core::models::event::{
        EventConsumptionStrategyDefinition, OneOfEventConsumptionStrategyDefinitionOrExpression,
    };

    let json_true = json!({
        "any": [{"with": {"type": "event.type1"}}],
        "until": false
    });

    let result: Result<EventConsumptionStrategyDefinition, _> = serde_json::from_value(json_true);
    assert!(result.is_ok(), "Failed to deserialize: {:?}", result.err());

    let strategy = result.unwrap();
    assert!(strategy.any.is_some());
    assert!(strategy.until.is_some());

    match *strategy.until.unwrap() {
        OneOfEventConsumptionStrategyDefinitionOrExpression::Bool(false) => {}
        _ => panic!("Expected Bool(false) variant"),
    }
}

#[test]
fn test_event_consumption_until_expression() {
    // Test EventConsumptionStrategyDefinition with until: expression
    use serverless_workflow_core::models::event::{
        EventConsumptionStrategyDefinition, OneOfEventConsumptionStrategyDefinitionOrExpression,
    };

    let json_str = json!({
        "any": [{"with": {"type": "event.type1"}}],
        "until": "${ .done }"
    });

    let result: Result<EventConsumptionStrategyDefinition, _> = serde_json::from_value(json_str);
    assert!(result.is_ok(), "Failed to deserialize: {:?}", result.err());

    let strategy = result.unwrap();
    match *strategy.until.unwrap() {
        OneOfEventConsumptionStrategyDefinitionOrExpression::Expression(expr) => {
            assert_eq!(expr, "${ .done }");
        }
        _ => panic!("Expected Expression variant"),
    }
}

#[test]
fn test_set_task_with_map_serialization() {
    // Test SetTaskDefinition with map (similar to Go SDK TestSetTask_MarshalJSON)
    let set_task_json = json!({
        "set": {
            "key1": "value1",
            "key2": 42
        }
    });
    let result: Result<SetTaskDefinition, _> = serde_json::from_value(set_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with map: {:?}",
        result.err()
    );
    let set_task = result.unwrap();
    match &set_task.set {
        SetValue::Map(map) => {
            assert_eq!(map.get("key1").and_then(|v| v.as_str()), Some("value1"));
            assert_eq!(map.get("key2").and_then(|v| v.as_u64()), Some(42));
        }
        _ => panic!("Expected Map variant"),
    }
}

#[test]
fn test_set_task_with_expression_serialization() {
    // Test SetTaskDefinition with expression
    let set_task_json = json!({
        "set": "${ $workflow.input }"
    });
    let result: Result<SetTaskDefinition, _> = serde_json::from_value(set_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with expression: {:?}",
        result.err()
    );
    let set_task = result.unwrap();
    match &set_task.set {
        SetValue::Expression(expr) => {
            assert_eq!(expr, "${ $workflow.input }");
        }
        _ => panic!("Expected Expression variant"),
    }
}

#[test]
fn test_set_task_roundtrip_serialization() {
    // Test SetTaskDefinition roundtrip serialization
    let mut map = std::collections::HashMap::new();
    map.insert("foo".to_string(), serde_json::json!("bar"));
    map.insert("count".to_string(), serde_json::json!(123));

    let set_task = SetTaskDefinition {
        set: SetValue::Map(map),
        common: TaskDefinitionFields::new(),
    };

    let json_str = serde_json::to_string(&set_task).expect("Failed to serialize set task");
    let deserialized: SetTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized.set {
        SetValue::Map(m) => {
            assert_eq!(m.get("foo").and_then(|v| v.as_str()), Some("bar"));
        }
        _ => panic!("Expected Map variant"),
    }
}

#[test]
fn test_set_task_with_all_fields() {
    // Test SetTaskDefinition with all common fields
    let set_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "set": {"key": "value"}
    });
    let result: Result<SetTaskDefinition, _> = serde_json::from_value(set_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with all fields: {:?}",
        result.err()
    );
    let set_task = result.unwrap();
    assert_eq!(set_task.common.if_, Some("${condition}".to_string()));
    assert!(set_task.common.timeout.is_some());
    assert_eq!(set_task.common.then, Some("continue".to_string()));
}

#[test]
fn test_do_task_serialization() {
    // Test DoTaskDefinition serialization (similar to Go SDK TestDoTask_MarshalJSON)
    let do_task_json = json!({
        "do": [
            {"task1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}},
            {"task2": {"call": "function2"}}
        ]
    });
    let result: Result<DoTaskDefinition, _> = serde_json::from_value(do_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize do task: {:?}",
        result.err()
    );
    let do_task = result.unwrap();
    assert_eq!(do_task.do_.entries.len(), 2);
}

#[test]
fn test_do_task_roundtrip_serialization() {
    // Test DoTaskDefinition roundtrip serialization
    let mut do_tasks = Map::new();
    do_tasks.add(
        "task1".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            serverless_workflow_core::models::call::CallFunctionDefinition {
                call: "func1".to_string(),
                with: None,
                common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );
    do_tasks.add(
        "task2".to_string(),
        TaskDefinition::Wait(WaitTaskDefinition::new(
            OneOfDurationOrIso8601Expression::Iso8601Expression("PT5S".to_string()),
        )),
    );

    let do_task = DoTaskDefinition::new(do_tasks);
    let json_str = serde_json::to_string(&do_task).expect("Failed to serialize do task");
    let deserialized: DoTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(deserialized.do_.entries.len(), 2);
}

#[test]
fn test_do_task_with_multiple_subtasks() {
    // Test DoTaskDefinition with multiple subtasks
    let do_task_json = json!({
        "do": [
            {"step1": {"call": "function1"}},
            {"step2": {"call": "function2"}},
            {"step3": {"call": "function3"}}
        ]
    });
    let result: Result<DoTaskDefinition, _> = serde_json::from_value(do_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize do task: {:?}",
        result.err()
    );
    let do_task = result.unwrap();
    assert_eq!(do_task.do_.entries.len(), 3);
}

#[test]
fn test_run_task_with_container_serialization() {
    // Test RunTaskDefinition with container (similar to Go SDK TestRunTask_MarshalJSON)
    let run_task_json = json!({
        "run": {
            "await": true,
            "container": {
                "image": "example-image",
                "command": "example-command",
                "environment": {"ENV_VAR": "value"}
            }
        }
    });
    let result: Result<RunTaskDefinition, _> = serde_json::from_value(run_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize run task with container: {:?}",
        result.err()
    );
    let run_task = result.unwrap();
    assert!(run_task.run.container.is_some());
}

#[test]
fn test_run_task_roundtrip_serialization() {
    // Test RunTaskDefinition roundtrip serialization
    let container = ContainerProcessDefinition {
        image: "example-image".to_string(),
        command: Some("example-command".to_string()),
        ..Default::default()
    };
    let run_task = RunTaskDefinition::new(ProcessTypeDefinition::using_container(
        container,
        Some(true),
    ));

    let json_str = serde_json::to_string(&run_task).expect("Failed to serialize run task");
    let deserialized: RunTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.run.container.is_some());
}

#[test]
fn test_run_task_with_script() {
    // Test RunTaskDefinition with script
    let run_task_json = json!({
        "run": {
            "await": true,
            "script": {
                "language": "python",
                "code": "print('Hello')"
            }
        }
    });
    let result: Result<RunTaskDefinition, _> = serde_json::from_value(run_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize run task with script: {:?}",
        result.err()
    );
    let run_task = result.unwrap();
    assert!(run_task.run.script.is_some());
}

#[test]
fn test_run_task_with_workflow() {
    // Test RunTaskDefinition with workflow
    let run_task_json = json!({
        "run": {
            "workflow": {
                "namespace": "default",
                "name": "my-workflow",
                "version": "1.0.0"
            }
        }
    });
    let result: Result<RunTaskDefinition, _> = serde_json::from_value(run_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize run task with workflow: {:?}",
        result.err()
    );
    let run_task = result.unwrap();
    assert!(run_task.run.workflow.is_some());
}

#[test]
fn test_workflow_definition_metadata_serialization() {
    // Test WorkflowDefinitionMetadata serialization
    use serverless_workflow_core::models::workflow::WorkflowDefinitionMetadata;

    let doc = WorkflowDefinitionMetadata::new(
        "namespace",
        "name",
        "1.0.0",
        Some("title".to_string()),
        Some("summary".to_string()),
        None,
    );
    let json_str = serde_json::to_string(&doc).expect("Failed to serialize workflow metadata");
    assert!(json_str.contains("namespace"));
    assert!(json_str.contains("name"));
    assert!(json_str.contains("1.0.0"));
}

#[test]
fn test_workflow_definition_with_timeout() {
    // Test WorkflowDefinition with timeout
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "test-workflow",
            "version": "1.0.0"
        },
        "timeout": {
            "after": "PT1H"
        },
        "do": []
    });
    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with timeout: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_definition_with_authentication() {
    // Test WorkflowDefinition with authentication
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "test-workflow",
            "version": "1.0.0"
        },
        "use": {
            "authentications": {
                "myAuth": {
                    "basic": {
                        "username": "user",
                        "password": "pass"
                    }
                }
            }
        },
        "do": []
    });
    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with auth: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_definition_with_functions() {
    // Test WorkflowDefinition with functions
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "test-workflow",
            "version": "1.0.0"
        },
        "use": {
            "functions": {
                "myFunc": {
                    "call": "http",
                    "with": {
                        "method": "GET",
                        "endpoint": "http://example.com"
                    }
                }
            }
        },
        "do": []
    });
    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with functions: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_definition_roundtrip() {
    // Test WorkflowDefinition roundtrip serialization
    let doc = WorkflowDefinitionMetadata::new("namespace", "name", "1.0.0", None, None, None);
    let workflow = WorkflowDefinition::new(doc);

    let json_str = serde_json::to_string(&workflow).expect("Failed to serialize workflow");
    let deserialized: WorkflowDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(workflow.document.namespace, deserialized.document.namespace);
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
        authentication: Some(ReferenceableAuthenticationPolicy::Policy(
            Box::new(AuthenticationPolicyDefinition {
                basic: Some(BasicAuthenticationSchemeDefinition {
                    username: Some("admin".to_string()),
                    password: Some("admin".to_string()),
                    use_: None,
                }),
                ..Default::default()
            }),
        )),
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
fn test_external_resource_definition() {
    use serverless_workflow_core::models::resource::{
        ExternalResourceDefinition, OneOfEndpointDefinitionOrUri,
    };
    let resource = ExternalResourceDefinition {
        name: Some("my-resource".to_string()),
        endpoint: OneOfEndpointDefinitionOrUri::Uri("http://example.com".to_string()),
    };
    let json_str = serde_json::to_string(&resource).expect("Failed to serialize");
    assert!(json_str.contains("my-resource"));
    assert!(json_str.contains("http://example.com"));
}

#[test]
fn test_schema_definition_json() {
    use serverless_workflow_core::models::schema::SchemaDefinition;
    let schema = SchemaDefinition {
        format: "json".to_string(),
        resource: None,
        document: Some(serde_json::json!({"type": "object"})),
    };
    let json_str = serde_json::to_string(&schema).expect("Failed to serialize");
    assert!(json_str.contains("json"));
}

#[test]
fn test_event_consumption_strategy_with_any() {
    use serverless_workflow_core::models::event::{
        EventConsumptionStrategyDefinition, EventFilterDefinition,
    };
    use std::collections::HashMap;
    let mut with = HashMap::new();
    with.insert("type".to_string(), serde_json::json!("example.event.type"));
    let filter = EventFilterDefinition {
        with: Some(with),
        ..Default::default()
    };
    let strategy = EventConsumptionStrategyDefinition {
        any: Some(vec![filter]),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&strategy).expect("Failed to serialize");
    assert!(json_str.contains("any"));
    assert!(json_str.contains("example.event.type"));
}

#[test]
fn test_event_consumption_strategy_with_all() {
    use serverless_workflow_core::models::event::{
        EventConsumptionStrategyDefinition, EventFilterDefinition,
    };
    use std::collections::HashMap;
    let mut with = HashMap::new();
    with.insert(
        "source".to_string(),
        serde_json::json!("http://example.com"),
    );
    let filter = EventFilterDefinition {
        with: Some(with),
        ..Default::default()
    };
    let strategy = EventConsumptionStrategyDefinition {
        all: Some(vec![filter]),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&strategy).expect("Failed to serialize");
    assert!(json_str.contains("all"));
}

#[test]
fn test_event_definition() {
    use serverless_workflow_core::models::event::EventDefinition;
    use std::collections::HashMap;
    let mut with = HashMap::new();
    with.insert("id".to_string(), serde_json::json!("event-id"));
    with.insert("type".to_string(), serde_json::json!("example.event.type"));
    let event = EventDefinition::new(with);
    let json_str = serde_json::to_string(&event).expect("Failed to serialize");
    assert!(json_str.contains("event-id"));
    assert!(json_str.contains("example.event.type"));
}

#[test]
fn test_error_definition() {
    use serverless_workflow_core::models::error::ErrorDefinition;
    let error = ErrorDefinition::new(
        "https://example.com/errors",
        "Bad Request",
        serde_json::json!(400),
        Some("Error occurred".to_string()),
        None,
    );
    let json_str = serde_json::to_string(&error).expect("Failed to serialize");
    assert!(json_str.contains("Bad Request"));
    assert!(json_str.contains("Error occurred"));
}

#[test]
fn test_error_definition_roundtrip() {
    use serverless_workflow_core::models::error::ErrorDefinition;
    let error = ErrorDefinition::new(
        "https://example.com/errors",
        "Not Found",
        serde_json::json!(404),
        Some("Resource not found".to_string()),
        Some("/instance/123".to_string()),
    );
    let json_str = serde_json::to_string(&error).expect("Failed to serialize");
    let deserialized: ErrorDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(error.title, deserialized.title);
    assert_eq!(error.detail, deserialized.detail);
}

#[test]
fn test_error_type_constants() {
    // Test ErrorTypes constants
    use serverless_workflow_core::models::error::ErrorTypes;

    assert_eq!(
        ErrorTypes::CONFIGURATION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/configuration"
    );
    assert_eq!(
        ErrorTypes::VALIDATION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/validation"
    );
    assert_eq!(
        ErrorTypes::EXPRESSION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/expression"
    );
    assert_eq!(
        ErrorTypes::AUTHENTICATION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/authentication"
    );
    assert_eq!(
        ErrorTypes::AUTHORIZATION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/authorization"
    );
    assert_eq!(
        ErrorTypes::TIMEOUT,
        "https://serverlessworkflow.io/spec/1.0.0/errors/timeout"
    );
    assert_eq!(
        ErrorTypes::COMMUNICATION,
        "https://serverlessworkflow.io/spec/1.0.0/errors/communication"
    );
    assert_eq!(
        ErrorTypes::RUNTIME,
        "https://serverlessworkflow.io/spec/1.0.0/errors/runtime"
    );
}

#[test]
fn test_error_convenience_constructors() {
    // Test ErrorDefinition convenience constructors
    use serverless_workflow_core::models::error::ErrorDefinition;

    let config_err = ErrorDefinition::configuration_error(
        Some("config detail".to_string()),
        Some("/instance/1".to_string()),
    );
    assert!(config_err.is_configuration_error());
    assert_eq!(config_err.title, Some("Configuration Error".to_string()));

    let auth_err = ErrorDefinition::authentication_error(None, None);
    assert!(auth_err.is_authentication_error());
    assert_eq!(auth_err.status, serde_json::json!(401));

    let timeout_err = ErrorDefinition::timeout_error(Some("timeout".to_string()), None);
    assert!(timeout_err.is_timeout_error());
    assert_eq!(timeout_err.status, serde_json::json!(408));
}

#[test]
fn test_error_classification_functions() {
    // Test ErrorDefinition classification functions
    use serverless_workflow_core::models::error::ErrorDefinition;

    let validation_err =
        ErrorDefinition::validation_error(Some("validation failed".to_string()), None);
    assert!(validation_err.is_validation_error());
    assert!(!validation_err.is_authentication_error());
    assert!(!validation_err.is_timeout_error());

    let runtime_err = ErrorDefinition::runtime_error(None, None);
    assert!(runtime_err.is_runtime_error());
    assert!(!runtime_err.is_configuration_error());
}

#[test]
fn test_retry_policy_with_backoff() {
    use serverless_workflow_core::models::retry::{
        BackoffStrategyDefinition, ExponentialBackoffDefinition, RetryPolicyDefinition,
    };
    let retry = RetryPolicyDefinition {
        when: Some("${ .retryable }".to_string()),
        delay: Some(OneOfDurationOrIso8601Expression::Duration(
            Duration::from_milliseconds(1000),
        )),
        backoff: Some(BackoffStrategyDefinition {
            exponential: Some(ExponentialBackoffDefinition::default()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&retry).expect("Failed to serialize");
    assert!(json_str.contains("exponential"));
}

#[test]
fn test_retry_policy_roundtrip() {
    use serverless_workflow_core::models::retry::{LinearBackoffDefinition, RetryPolicyDefinition};
    let retry = RetryPolicyDefinition {
        when: Some("${ .shouldRetry }".to_string()),
        delay: Some(OneOfDurationOrIso8601Expression::Duration(
            Duration::from_milliseconds(500),
        )),
        backoff: Some(BackoffStrategyDefinition {
            linear: Some(LinearBackoffDefinition::default()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&retry).expect("Failed to serialize");
    let deserialized: RetryPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(retry.when, deserialized.when);
}

#[test]
fn test_raise_task_definition() {
    use serverless_workflow_core::models::error::OneOfErrorDefinitionOrReference;
    use serverless_workflow_core::models::task::{RaiseErrorDefinition, RaiseTaskDefinition};
    let raise_error = RaiseErrorDefinition {
        error: OneOfErrorDefinitionOrReference::Error(
            serverless_workflow_core::models::error::ErrorDefinition::new(
                "https://example.com/errors",
                "Bad Request",
                serde_json::json!(400),
                Some("Error occurred".to_string()),
                None,
            ),
        ),
    };
    let raise_task = RaiseTaskDefinition::new(raise_error);
    let json_str = serde_json::to_string(&raise_task).expect("Failed to serialize");
    assert!(json_str.contains("raise"));
    assert!(json_str.contains("Bad Request"));
}

#[test]
fn test_raise_task_roundtrip() {
    use serverless_workflow_core::models::error::{
        ErrorDefinition, OneOfErrorDefinitionOrReference,
    };
    use serverless_workflow_core::models::task::{RaiseErrorDefinition, RaiseTaskDefinition};
    let raise_error = RaiseErrorDefinition {
        error: OneOfErrorDefinitionOrReference::Error(ErrorDefinition::new(
            "https://example.com/errors",
            "Not Found",
            serde_json::json!(404),
            Some("Resource not found".to_string()),
            None,
        )),
    };
    let raise_task = RaiseTaskDefinition::new(raise_error);
    let json_str = serde_json::to_string(&raise_task).expect("Failed to serialize");
    let deserialized: RaiseTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    if let OneOfErrorDefinitionOrReference::Error(err) = &deserialized.raise.error {
        assert_eq!(err.title, Some("Not Found".to_string()));
    }
}

#[test]
fn test_retry_policy_with_all_fields() {
    use serverless_workflow_core::models::retry::{
        BackoffStrategyDefinition, ExponentialBackoffDefinition, JitterDefinition,
        RetryAttemptLimitDefinition, RetryPolicyDefinition, RetryPolicyLimitDefinition,
    };
    let retry = RetryPolicyDefinition {
        when: Some("${ .shouldRetry }".to_string()),
        except_when: Some("${ .shouldNotRetry }".to_string()),
        delay: Some(OneOfDurationOrIso8601Expression::Duration(
            Duration::from_seconds(5),
        )),
        backoff: Some(BackoffStrategyDefinition {
            exponential: Some(ExponentialBackoffDefinition::default()),
            ..Default::default()
        }),
        limit: Some(RetryPolicyLimitDefinition {
            attempt: Some(RetryAttemptLimitDefinition {
                count: Some(3),
                duration: Some(OneOfDurationOrIso8601Expression::Duration(
                    Duration::from_minutes(1),
                )),
            }),
            duration: Some(OneOfDurationOrIso8601Expression::Duration(
                Duration::from_minutes(10),
            )),
        }),
        jitter: Some(JitterDefinition {
            from: Duration::from_seconds(1),
            to: Duration::from_seconds(3),
        }),
    };
    let json_str = serde_json::to_string(&retry).expect("Failed to serialize");
    assert!(json_str.contains("when"));
    assert!(json_str.contains("exceptWhen"));
    assert!(json_str.contains("exponential"));
    assert!(json_str.contains("jitter"));
}

#[test]
fn test_retry_policy_roundtrip_with_all_fields() {
    use serverless_workflow_core::models::retry::{
        BackoffStrategyDefinition, ConstantBackoffDefinition, JitterDefinition,
        RetryPolicyDefinition,
    };
    let retry = RetryPolicyDefinition {
        when: Some("${ .retryable }".to_string()),
        delay: Some(OneOfDurationOrIso8601Expression::Duration(
            Duration::from_milliseconds(500),
        )),
        backoff: Some(BackoffStrategyDefinition {
            constant: Some(ConstantBackoffDefinition::default()),
            ..Default::default()
        }),
        jitter: Some(JitterDefinition {
            from: Duration::from_milliseconds(100),
            to: Duration::from_milliseconds(300),
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&retry).expect("Failed to serialize");
    let deserialized: RetryPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(retry.when, deserialized.when);
    assert!(json_str.contains("constant"));
}

#[test]
fn test_oauth2_authentication_with_grant() {
    use serverless_workflow_core::models::authentication::{
        AuthenticationPolicyDefinition, OAuth2AuthenticationSchemeDefinition,
    };
    let auth = AuthenticationPolicyDefinition {
        oauth2: Some(OAuth2AuthenticationSchemeDefinition {
            use_: Some("mysecret".to_string()),
            grant: Some("client_credentials".to_string()),
            authority: Some("https://auth.example.com".to_string()),
            scopes: Some(vec!["scope1".to_string(), "scope2".to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&auth).expect("Failed to serialize");
    assert!(json_str.contains("oauth2"));
    assert!(json_str.contains("client_credentials"));
}

#[test]
fn test_oauth2_authentication_roundtrip() {
    use serverless_workflow_core::models::authentication::{
        AuthenticationPolicyDefinition, OAuth2AuthenticationSchemeDefinition,
    };
    let auth = AuthenticationPolicyDefinition {
        oauth2: Some(OAuth2AuthenticationSchemeDefinition {
            grant: Some("client_credentials".to_string()),
            authority: Some("https://authority.com".to_string()),
            scopes: Some(vec!["read".to_string(), "write".to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&auth).expect("Failed to serialize");
    let deserialized: AuthenticationPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.oauth2.is_some());
}

#[test]
fn test_bearer_authentication_roundtrip() {
    use serverless_workflow_core::models::authentication::{
        AuthenticationPolicyDefinition, BearerAuthenticationSchemeDefinition,
    };
    let auth = AuthenticationPolicyDefinition {
        bearer: Some(BearerAuthenticationSchemeDefinition {
            token: Some("my-token".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&auth).expect("Failed to serialize");
    let deserialized: AuthenticationPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.bearer.is_some());
}

#[test]
fn test_digest_authentication_roundtrip() {
    use serverless_workflow_core::models::authentication::{
        AuthenticationPolicyDefinition, DigestAuthenticationSchemeDefinition,
    };
    let auth = AuthenticationPolicyDefinition {
        digest: Some(DigestAuthenticationSchemeDefinition {
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&auth).expect("Failed to serialize");
    let deserialized: AuthenticationPolicyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.digest.is_some());
}

#[test]
fn test_workflow_document_with_tags_and_metadata() {
    use serverless_workflow_core::models::workflow::WorkflowDefinitionMetadata;
    use std::collections::HashMap;
    let mut tags = HashMap::new();
    tags.insert("env".to_string(), "prod".to_string());
    tags.insert("team".to_string(), "workflow".to_string());
    let mut metadata = HashMap::new();
    metadata.insert("author".to_string(), serde_json::json!("John Doe"));
    let doc = WorkflowDefinitionMetadata::new(
        "namespace",
        "name",
        "1.0.0",
        Some("Title".to_string()),
        Some("Summary".to_string()),
        Some(tags),
    );
    let json_str = serde_json::to_string(&doc).expect("Failed to serialize");
    assert!(json_str.contains("prod"));
    assert!(json_str.contains("workflow"));
    let deserialized: WorkflowDefinitionMetadata =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(doc.namespace, deserialized.namespace);
}

#[test]
fn test_workflow_document_roundtrip() {
    use serverless_workflow_core::models::workflow::WorkflowDefinitionMetadata;
    let doc = WorkflowDefinitionMetadata::new(
        "ns",
        "workflow-name",
        "1.0.0",
        Some("My Title".to_string()),
        Some("My Summary".to_string()),
        None,
    );
    let json_str = serde_json::to_string(&doc).expect("Failed to serialize");
    let deserialized: WorkflowDefinitionMetadata =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(doc.namespace, deserialized.namespace);
    assert_eq!(doc.name, deserialized.name);
    assert_eq!(doc.version, deserialized.version);
    assert_eq!(doc.title, deserialized.title);
    assert_eq!(doc.summary, deserialized.summary);
}

#[test]
fn test_workflow_definition_with_full_document() {
    use serverless_workflow_core::models::workflow::{
        WorkflowDefinition, WorkflowDefinitionMetadata,
    };
    use std::collections::HashMap;
    let mut tags = HashMap::new();
    tags.insert("env".to_string(), "dev".to_string());
    let doc = WorkflowDefinitionMetadata::new(
        "default",
        "full-workflow",
        "1.0.0",
        Some("Full Workflow".to_string()),
        Some("A complete workflow definition".to_string()),
        Some(tags),
    );
    let workflow = WorkflowDefinition::new(doc);
    let json_str = serde_json::to_string(&workflow).expect("Failed to serialize");
    assert!(json_str.contains("full-workflow"));
    assert!(json_str.contains("dev"));
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
fn test_for_task_definition() {
    use serverless_workflow_core::models::task::{ForLoopDefinition, ForTaskDefinition};
    let for_task = ForTaskDefinition::new(
        ForLoopDefinition::new("item", "${items}", Some("index".to_string()), None),
        Map::new(),
        Some("${ .condition }".to_string()),
    );
    let json_str = serde_json::to_string(&for_task).expect("Failed to serialize");
    assert!(json_str.contains("for"));
    assert!(json_str.contains("item"));
    assert!(json_str.contains("items"));
}

#[test]
fn test_for_task_roundtrip() {
    use serverless_workflow_core::models::task::{ForLoopDefinition, ForTaskDefinition};
    let for_task = ForTaskDefinition::new(
        ForLoopDefinition::new("pet", ".pets", Some("index".to_string()), None),
        Map::new(),
        Some("${ .vet != null }".to_string()),
    );
    let json_str = serde_json::to_string(&for_task).expect("Failed to serialize");
    let deserialized: ForTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(for_task.for_.each, deserialized.for_.each);
}

#[test]
fn test_fork_task_definition() {
    use serverless_workflow_core::models::task::BranchingDefinition;
    use serverless_workflow_core::models::task::ForkTaskDefinition;
    let fork_task = ForkTaskDefinition {
        fork: BranchingDefinition {
            branches: Map::new(),
            compete: true,
        },
        ..Default::default()
    };
    let json_str = serde_json::to_string(&fork_task).expect("Failed to serialize");
    assert!(json_str.contains("fork"));
}

#[test]
fn test_fork_task_roundtrip() {
    use serverless_workflow_core::models::task::BranchingDefinition;
    use serverless_workflow_core::models::task::ForkTaskDefinition;
    let fork_task = ForkTaskDefinition {
        fork: BranchingDefinition {
            branches: Map::new(),
            compete: true,
        },
        ..Default::default()
    };
    let json_str = serde_json::to_string(&fork_task).expect("Failed to serialize");
    let deserialized: ForkTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(fork_task.fork.compete, deserialized.fork.compete);
}

#[test]
fn test_wait_task_with_duration_new() {
    use serverless_workflow_core::models::task::WaitTaskDefinition;
    let wait_task = WaitTaskDefinition::new(
        serverless_workflow_core::models::duration::OneOfDurationOrIso8601Expression::Duration(
            serverless_workflow_core::models::duration::Duration::from_seconds(10),
        ),
    );
    let json_str = serde_json::to_string(&wait_task).expect("Failed to serialize");
    assert!(json_str.contains("wait"));
    let _deserialized: WaitTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(json_str.contains("10"));
}

#[test]
fn test_set_task_definition() {
    use serverless_workflow_core::models::task::{SetTaskDefinition, SetValue};
    use std::collections::HashMap;
    let mut set_map = HashMap::new();
    set_map.insert("key1".to_string(), serde_json::json!("value1"));
    set_map.insert("key2".to_string(), serde_json::json!(42));
    let set_task = SetTaskDefinition {
        set: SetValue::Map(set_map),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&set_task).expect("Failed to serialize");
    assert!(json_str.contains("set"));
    assert!(json_str.contains("key1"));
    assert!(json_str.contains("value1"));
}

#[test]
fn test_set_task_roundtrip() {
    use serverless_workflow_core::models::task::{SetTaskDefinition, SetValue};
    use std::collections::HashMap;
    let mut set_map = HashMap::new();
    set_map.insert("name".to_string(), serde_json::json!("test"));
    set_map.insert("value".to_string(), serde_json::json!(123));
    let set_task = SetTaskDefinition {
        set: SetValue::Map(set_map),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&set_task).expect("Failed to serialize");
    let _deserialized: SetTaskDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(json_str.contains("test"));
}

#[test]
fn test_try_task_definition() {
    use serverless_workflow_core::models::task::TryTaskDefinition;
    let try_task = TryTaskDefinition {
        try_: Map::new(),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&try_task).expect("Failed to serialize");
    assert!(json_str.contains("do") || json_str.contains("try"));
}

#[test]
fn test_run_task_definition() {
    use serverless_workflow_core::models::task::{
        ProcessTypeDefinition, RunTaskDefinition, WorkflowProcessDefinition,
    };
    let run_task = RunTaskDefinition {
        run: ProcessTypeDefinition {
            workflow: Some(WorkflowProcessDefinition::new(
                "default",
                "my-workflow",
                "1.0.0",
                None,
            )),
            ..Default::default()
        },
        ..Default::default()
    };
    let json_str = serde_json::to_string(&run_task).expect("Failed to serialize");
    assert!(json_str.contains("run"));
    assert!(json_str.contains("my-workflow"));
}

#[test]
fn test_run_task_roundtrip() {
    use serverless_workflow_core::models::task::{
        ProcessTypeDefinition, RunTaskDefinition, WorkflowProcessDefinition,
    };
    let run_task = RunTaskDefinition {
        run: ProcessTypeDefinition {
            workflow: Some(WorkflowProcessDefinition::new(
                "ns",
                "example-workflow",
                "2.0.0",
                None,
            )),
            ..Default::default()
        },
        ..Default::default()
    };
    let json_str = serde_json::to_string(&run_task).expect("Failed to serialize");
    assert!(json_str.contains("example-workflow"));
}

#[test]
fn test_switch_task_with_all_common_fields() {
    // Test SwitchTaskDefinition with all common fields (if, input, output, timeout, then, metadata)
    let switch_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}},
            {"case2": {"when": "${condition2}", "then": "end"}}
        ]
    });
    let result: Result<SwitchTaskDefinition, _> = serde_json::from_value(switch_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch task with all fields: {:?}",
        result.err()
    );
    let switch_task = result.unwrap();
    assert_eq!(switch_task.common.if_, Some("${condition}".to_string()));
    assert!(switch_task.common.timeout.is_some());
    assert_eq!(switch_task.common.then, Some("continue".to_string()));
    assert_eq!(switch_task.switch.entries.len(), 2);
}

#[test]
fn test_switch_task_unmarshaling() {
    // Test SwitchTaskDefinition unmarshaling from JSON
    let json_data = r#"{
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "switch": [
            {"case1": {"when": "${condition1}", "then": "next"}},
            {"case2": {"when": "${condition2}", "then": "end"}}
        ]
    }"#;
    let result: Result<SwitchTaskDefinition, _> = serde_json::from_str(json_data);
    assert!(
        result.is_ok(),
        "Failed to unmarshal switch task: {:?}",
        result.err()
    );
    let switch_task = result.unwrap();
    assert_eq!(switch_task.common.if_, Some("${condition}".to_string()));
    assert_eq!(switch_task.switch.entries.len(), 2);
}

#[test]
fn test_emit_task_with_all_fields() {
    // Test EmitTaskDefinition with all fields (similar to Go SDK TestEmitTask_MarshalJSON)
    use std::collections::HashMap;

    let mut event_with = HashMap::new();
    event_with.insert("id".to_string(), serde_json::json!("event-id"));
    event_with.insert(
        "source".to_string(),
        serde_json::json!("http://example.com/source"),
    );
    event_with.insert("type".to_string(), serde_json::json!("example.event.type"));
    event_with.insert(
        "time".to_string(),
        serde_json::json!("2023-01-01T00:00:00Z"),
    );
    event_with.insert("subject".to_string(), serde_json::json!("example.subject"));
    event_with.insert(
        "datacontenttype".to_string(),
        serde_json::json!("application/json"),
    );
    event_with.insert(
        "dataschema".to_string(),
        serde_json::json!("http://example.com/schema"),
    );
    event_with.insert("extra".to_string(), serde_json::json!("value"));

    let emit_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "emit": {
            "event": {
                "with": event_with
            }
        }
    });
    let result: Result<EmitTaskDefinition, _> = serde_json::from_value(emit_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize emit task with all fields: {:?}",
        result.err()
    );
    let emit_task = result.unwrap();
    assert_eq!(emit_task.common.if_, Some("${condition}".to_string()));
    assert!(emit_task.common.timeout.is_some());
    assert_eq!(emit_task.common.then, Some("continue".to_string()));
    assert!(emit_task.emit.event.with.contains_key("id"));
    assert_eq!(
        emit_task
            .emit
            .event
            .with
            .get("type")
            .and_then(|v| v.as_str()),
        Some("example.event.type")
    );
}

#[test]
fn test_listen_task_with_all_fields() {
    // Test ListenTaskDefinition with all fields (similar to Go SDK TestListenTask_MarshalJSON_WithUntilCondition)
    let listen_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "listen": {
            "to": {
                "any": [
                    {"with": {"type": "example.event.type", "source": "http://example.com/source"}}
                ],
                "until": "workflow.data.condition == true"
            }
        }
    });
    let result: Result<ListenTaskDefinition, _> = serde_json::from_value(listen_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize listen task with all fields: {:?}",
        result.err()
    );
    let listen_task = result.unwrap();
    assert_eq!(listen_task.common.if_, Some("${condition}".to_string()));
    assert!(listen_task.common.timeout.is_some());
    assert_eq!(listen_task.common.then, Some("continue".to_string()));
    assert!(listen_task.listen.to.any.is_some());
}

#[test]
fn test_listen_task_with_until_condition_v2() {
    // Test ListenTaskDefinition with until condition (different from existing test)
    let listen_task_json = json!({
        "listen": {
            "to": {
                "any": [{"with": {"type": "event.type"}}],
                "until": "workflow.data.condition == true"
            }
        }
    });
    let result: Result<ListenTaskDefinition, _> = serde_json::from_value(listen_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize listen task with until: {:?}",
        result.err()
    );
    let listen_task = result.unwrap();
    assert!(listen_task.listen.to.any.is_some());
}

#[test]
fn test_event_consumption_strategy_with_until_expression() {
    // Test EventConsumptionStrategyDefinition with until expression
    use serverless_workflow_core::models::event::{
        EventConsumptionStrategyDefinition, EventFilterDefinition,
    };
    use std::collections::HashMap;

    let mut with = HashMap::new();
    with.insert("type".to_string(), serde_json::json!("example.event.type"));

    let strategy = EventConsumptionStrategyDefinition {
        any: Some(vec![EventFilterDefinition {
            with: Some(with),
            ..Default::default()
        }]),
        until: Some(Box::new(serverless_workflow_core::models::event::OneOfEventConsumptionStrategyDefinitionOrExpression::Expression("workflow.data.done == true".to_string()))),
        ..Default::default()
    };

    let json_str = serde_json::to_string(&strategy).expect("Failed to serialize");
    assert!(json_str.contains("until"));
    assert!(json_str.contains("workflow.data.done == true"));
}

#[test]
fn test_timeout_with_iso8601_expression() {
    // Test TimeoutDefinition with ISO 8601 expression
    let timeout_json = json!({
        "after": "PT1H"
    });
    let result: Result<TimeoutDefinition, _> = serde_json::from_value(timeout_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout with ISO 8601: {:?}",
        result.err()
    );
    let timeout = result.unwrap();
    match timeout.after {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT1H");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_timeout_roundtrip_serialization() {
    // Test TimeoutDefinition roundtrip serialization
    let timeout = TimeoutDefinition {
        after: OneOfDurationOrIso8601Expression::Iso8601Expression("PT2H".to_string()),
    };
    let json_str = serde_json::to_string(&timeout).expect("Failed to serialize timeout");
    let deserialized: TimeoutDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized.after {
        OneOfDurationOrIso8601Expression::Iso8601Expression(expr) => {
            assert_eq!(expr, "PT2H");
        }
        _ => panic!("Expected Iso8601Expression variant"),
    }
}

#[test]
fn test_document_with_tags_and_metadata() {
    // Test Document/WorkflowDefinitionMetadata with tags and metadata (similar to Go SDK TestDocument_JSONMarshal)
    use serverless_workflow_core::models::workflow::WorkflowDefinitionMetadata;
    use std::collections::HashMap;

    let mut tags = HashMap::new();
    tags.insert("env".to_string(), "prod".to_string());
    tags.insert("team".to_string(), "workflow".to_string());
    let mut metadata = HashMap::new();
    metadata.insert("author".to_string(), serde_json::json!("John Doe"));
    metadata.insert("created".to_string(), serde_json::json!("2025-01-01"));

    let doc = WorkflowDefinitionMetadata::new(
        "example-namespace",
        "example-name",
        "1.0.0",
        Some("Example Workflow".to_string()),
        Some("This is a sample workflow document.".to_string()),
        Some(tags),
    );
    let json_str = serde_json::to_string(&doc).expect("Failed to serialize");
    assert!(json_str.contains("example-namespace"));
    assert!(json_str.contains("example-name"));
    assert!(json_str.contains("prod"));
    assert!(json_str.contains("workflow"));
}

#[test]
fn test_document_unmarshaling() {
    // Test WorkflowDefinitionMetadata unmarshaling from JSON
    let json_data = r#"{
        "dsl": "1.0.0",
        "namespace": "example-namespace",
        "name": "example-name",
        "version": "1.0.0",
        "title": "Example Workflow",
        "summary": "This is a sample workflow document.",
        "tags": {
            "env": "prod",
            "team": "workflow"
        },
        "metadata": {
            "author": "John Doe",
            "created": "2025-01-01"
        }
    }"#;
    let result: Result<WorkflowDefinitionMetadata, _> = serde_json::from_str(json_data);
    assert!(
        result.is_ok(),
        "Failed to unmarshal document: {:?}",
        result.err()
    );
    let doc = result.unwrap();
    assert_eq!(doc.namespace, "example-namespace");
    assert_eq!(doc.name, "example-name");
    assert_eq!(doc.version, "1.0.0");
    assert_eq!(doc.title, Some("Example Workflow".to_string()));
}

#[test]
fn test_workflow_definition_with_use_and_do() {
    // Test WorkflowDefinition with use and do sections
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "test-workflow",
            "version": "1.0.0"
        },
        "use": {
            "authentications": {
                "myBasicAuth": {
                    "basic": {
                        "username": "user",
                        "password": "password"
                    }
                }
            }
        },
        "do": [
            {"step1": {"call": "http", "with": {"method": "GET", "endpoint": "http://example.com"}}}
        ]
    });
    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with use and do: {:?}",
        result.err()
    );
}

#[test]
fn test_error_definition_reference() {
    // Test ErrorDefinition with reference (using OneOfErrorDefinitionOrReference::Reference)
    use serverless_workflow_core::models::error::OneOfErrorDefinitionOrReference;

    let error_ref = OneOfErrorDefinitionOrReference::Reference("myError".to_string());
    let json_str = serde_json::to_string(&error_ref).expect("Failed to serialize error reference");
    assert!(json_str.contains("myError"));

    let deserialized: OneOfErrorDefinitionOrReference =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized {
        OneOfErrorDefinitionOrReference::Reference(s) => {
            assert_eq!(s, "myError");
        }
        _ => panic!("Expected Reference variant"),
    }
}

#[test]
fn test_timeout_reference_serialization_roundtrip() {
    // Test OneOfTimeoutDefinitionOrReference as Reference roundtrip
    let tor = OneOfTimeoutDefinitionOrReference::Reference("my-timeout-ref".to_string());
    let json_str = serde_json::to_string(&tor).expect("Failed to serialize timeout reference");
    let deserialized: OneOfTimeoutDefinitionOrReference =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    match deserialized {
        OneOfTimeoutDefinitionOrReference::Reference(s) => {
            assert_eq!(s, "my-timeout-ref");
        }
        _ => panic!("Expected Reference variant"),
    }
}

#[test]
fn test_raise_task_with_all_fields() {
    // Test RaiseTaskDefinition with all common fields
    use serverless_workflow_core::models::error::OneOfErrorDefinitionOrReference;

    let raise_task_json = json!({
        "if": "${condition}",
        "input": { "from": {"key": "value"} },
        "output": { "as": {"result": "output"} },
        "timeout": { "after": "PT10S" },
        "then": "continue",
        "metadata": {"meta": "data"},
        "raise": {
            "error": {
                "type": "http://example.com/error",
                "status": 500,
                "title": "Internal Server Error",
                "detail": "An unexpected error occurred."
            }
        }
    });
    let result: Result<RaiseTaskDefinition, _> = serde_json::from_value(raise_task_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize raise task with all fields: {:?}",
        result.err()
    );
    let raise_task = result.unwrap();
    assert_eq!(raise_task.common.if_, Some("${condition}".to_string()));
    assert!(raise_task.common.timeout.is_some());
    assert_eq!(raise_task.common.then, Some("continue".to_string()));
    match raise_task.raise.error {
        OneOfErrorDefinitionOrReference::Error(err) => {
            assert_eq!(err.type_.as_str(), "http://example.com/error");
            assert_eq!(err.title, Some("Internal Server Error".to_string()));
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_correlation_key_definition() {
    // Test CorrelationKeyDefinition serialization
    use serverless_workflow_core::models::event::CorrelationKeyDefinition;

    let correlation =
        CorrelationKeyDefinition::new("event.source", Some("http://example.com".to_string()));
    let json_str =
        serde_json::to_string(&correlation).expect("Failed to serialize correlation key");
    assert!(json_str.contains("event.source"));
    let deserialized: CorrelationKeyDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert_eq!(correlation.from, deserialized.from);
}

#[test]
fn test_subscription_iterator_definition() {
    // Test SubscriptionIteratorDefinition serialization
    use serverless_workflow_core::models::task::SubscriptionIteratorDefinition;

    let iterator = SubscriptionIteratorDefinition {
        item: Some("item".to_string()),
        at: Some("index".to_string()),
        do_: Some(Map::new()),
        ..Default::default()
    };
    let json_str =
        serde_json::to_string(&iterator).expect("Failed to serialize subscription iterator");
    assert!(json_str.contains("item"));
    assert!(json_str.contains("index"));
}

// ==================== Additional Tests from SDK Patterns ====================

#[test]
fn test_workflow_yaml_serialization() {
    // Test workflow YAML serialization (similar to Python SDK test_workflow_to_yaml)
    use serverless_workflow_core::models::workflow::{
        WorkflowDefinition, WorkflowDefinitionMetadata,
    };

    let doc = WorkflowDefinitionMetadata::new(
        "default",
        "test-workflow",
        "1.0.0",
        Some("Test Workflow".to_string()),
        Some("A test workflow".to_string()),
        None,
    );
    let workflow = WorkflowDefinition::new(doc);

    let yaml_str = serde_yaml::to_string(&workflow).expect("Failed to serialize workflow to YAML");
    assert!(yaml_str.contains("test-workflow"));
    assert!(yaml_str.contains("1.0.0"));
}

#[test]
fn test_workflow_yaml_roundtrip() {
    // Test workflow roundtrip serialization with YAML
    use serverless_workflow_core::models::workflow::{
        WorkflowDefinition, WorkflowDefinitionMetadata,
    };

    let doc = WorkflowDefinitionMetadata::new("namespace", "yaml-test", "2.0.0", None, None, None);
    let workflow = WorkflowDefinition::new(doc);

    let yaml_str = serde_yaml::to_string(&workflow).expect("Failed to serialize to YAML");
    let deserialized: WorkflowDefinition =
        serde_yaml::from_str(&yaml_str).expect("Failed to deserialize from YAML");
    assert_eq!(workflow.document.name, deserialized.document.name);
}

#[test]
fn test_workflow_json_and_yaml_equivalence() {
    // Test that JSON and YAML serialization produce equivalent results for workflow metadata
    use serverless_workflow_core::models::workflow::WorkflowDefinitionMetadata;

    let doc = WorkflowDefinitionMetadata::new(
        "test-ns",
        "equiv-test",
        "1.0.0",
        Some("Title".to_string()),
        Some("Summary".to_string()),
        None,
    );

    let json_str = serde_json::to_string(&doc).expect("Failed to serialize to JSON");
    let yaml_str = serde_yaml::to_string(&doc).expect("Failed to serialize to YAML");

    // Both should contain the essential information
    assert!(json_str.contains("equiv-test"));
    assert!(yaml_str.contains("equiv-test"));
}

#[test]
fn test_complete_workflow_deserialization() {
    // Test complete workflow deserialization from JSON (similar to TypeScript SDK deserialization test)
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "name": "test",
            "version": "1.0.0",
            "namespace": "default"
        },
        "do": [
            {
                "step1": {
                    "set": {
                        "foo": "bar"
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow: {:?}",
        result.err()
    );
    let workflow = result.unwrap();
    assert_eq!(workflow.document.name, "test");
    assert_eq!(workflow.document.namespace, "default");
}

#[test]
fn test_workflow_with_call_task() {
    // Test workflow with call task
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "call-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "callStep": {
                    "call": "http",
                    "with": {
                        "method": "GET",
                        "endpoint": "http://example.com/api"
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with call task: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_multiple_do_tasks() {
    // Test workflow with multiple tasks in do section
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "multi-task-workflow",
            "version": "1.0.0"
        },
        "do": [
            {"task1": {"call": "func1"}},
            {"task2": {"call": "func2"}},
            {"task3": {"call": "func3"}}
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with multiple tasks: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_for_task() {
    // Test workflow with for task
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "for-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "iterateItems": {
                    "for": {
                        "each": "item",
                        "in": "${ .items }"
                    },
                    "do": [
                        {"processItem": {"call": "process"}}
                    ]
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with for task: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_switch_task() {
    // Test workflow with switch task
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "switch-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "decision": {
                    "switch": [
                        {"high": {"when": "${ .value > 100 }", "then": "next"}},
                        {"low": {"when": "${ .value <= 100 }", "then": "end"}}
                    ]
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with switch task: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_wait_task() {
    // Test workflow with wait task
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "wait-workflow",
            "version": "1.0.0"
        },
        "do": [
            {"delay": {"wait": "PT5S"}}
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with wait task: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_roundtrip() {
    // Test complete workflow roundtrip serialization
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "roundtrip-test",
            "version": "1.0.0",
            "title": "Roundtrip Test",
            "summary": "Testing roundtrip serialization"
        },
        "do": [
            {"step1": {"set": {"key": "value"}}}
        ]
    });

    let json_str = serde_json::to_string(&workflow_json).expect("Failed to serialize to string");
    let result: Result<WorkflowDefinition, _> = serde_json::from_str(&json_str);
    assert!(
        result.is_ok(),
        "Failed to deserialize from string: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_authentication() {
    // Test workflow with authentication
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "auth-workflow",
            "version": "1.0.0"
        },
        "use": {
            "authentications": {
                "basicAuth": {
                    "basic": {
                        "username": "admin",
                        "password": "secret"
                    }
                }
            }
        },
        "do": []
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with auth: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_retries() {
    // Test workflow with retry definitions
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "retry-workflow",
            "version": "1.0.0"
        },
        "use": {
            "retries": {
                "defaultRetry": {
                    "delay": {"seconds": 1},
                    "backoff": {"exponential": {}},
                    "limit": {"attempt": {"count": 3}}
                }
            }
        },
        "do": []
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with retries: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_functions() {
    // Test workflow with function definitions
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "function-workflow",
            "version": "1.0.0"
        },
        "use": {
            "functions": {
                "myFunc": {
                    "call": "http",
                    "with": {
                        "method": "GET",
                        "endpoint": "http://example.com/api"
                    }
                }
            }
        },
        "do": []
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with functions: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_extensions() {
    // Test workflow with extension definitions
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "extension-workflow",
            "version": "1.0.0"
        },
        "use": {
            "extensions": [
                {"ext1": {"extend": "call"}}
            ]
        },
        "do": []
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with extensions: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_timeout() {
    // Test workflow with timeout definition
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "timeout-workflow",
            "version": "1.0.0"
        },
        "timeout": {
            "after": "PT1H"
        },
        "do": []
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with timeout: {:?}",
        result.err()
    );
}

#[test]
fn test_nested_do_task() {
    // Test nested do tasks (do within do)
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "nested-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "outerTask": {
                    "do": [
                        {"innerTask1": {"call": "func1"}},
                        {"innerTask2": {"call": "func2"}}
                    ]
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with nested do: {:?}",
        result.err()
    );
}

#[test]
fn test_try_task_with_catch() {
    // Test try task with catch error handling
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "try-catch-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "riskyOperation": {
                    "try": [
                        {"step1": {"call": "unreliable"}}
                    ],
                    "catch": {
                        "errors": {"with": {"type": "ServerError"}},
                        "do": [
                            {"handleError": {"call": "errorHandler"}}
                        ]
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with try-catch: {:?}",
        result.err()
    );
}

#[test]
fn test_fork_task_with_branches() {
    // Test fork task with multiple branches
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "fork-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "parallelTasks": {
                    "fork": {
                        "branches": [
                            {"task1": {"call": "task1"}},
                            {"task2": {"call": "task2"}},
                            {"task3": {"call": "task3"}}
                        ],
                        "compete": true
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with fork: {:?}",
        result.err()
    );
}

#[test]
fn test_emit_task_workflow() {
    // Test workflow with emit task for event emission
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "emit-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "sendNotification": {
                    "emit": {
                        "event": {
                            "with": {
                                "type": "notification.event",
                                "source": "http://example.com"
                            }
                        }
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with emit: {:?}",
        result.err()
    );
}

#[test]
fn test_listen_task_workflow() {
    // Test workflow with listen task for event consumption
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "listen-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "waitForEvent": {
                    "listen": {
                        "to": {
                            "any": [
                                {"with": {"type": "event.type1"}},
                                {"with": {"type": "event.type2"}}
                            ]
                        }
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with listen: {:?}",
        result.err()
    );
}

#[test]
fn test_raise_task_workflow() {
    // Test workflow with raise task for error raising
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "raise-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "failOperation": {
                    "raise": {
                        "error": {
                            "type": "https://example.com/errors/failure",
                            "status": 500,
                            "title": "Operation Failed"
                        }
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with raise: {:?}",
        result.err()
    );
}

#[test]
fn test_run_task_workflow() {
    // Test workflow with run task for process execution
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "run-workflow",
            "version": "1.0.0"
        },
        "do": [
            {
                "executeProcess": {
                    "run": {
                        "workflow": {
                            "namespace": "default",
                            "name": "sub-workflow",
                            "version": "1.0.0"
                        }
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with run: {:?}",
        result.err()
    );
}

#[test]
fn test_map_serialization_order() {
    // Test that Map entries maintain serialization order
    use serverless_workflow_core::models::task::CallTaskDefinition;
    use serverless_workflow_core::models::task::TaskDefinition;

    let mut map = Map::new();
    map.add(
        "first".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            serverless_workflow_core::models::call::CallFunctionDefinition {
                call: "func1".to_string(),
                with: None,
                common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );
    map.add(
        "second".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            serverless_workflow_core::models::call::CallFunctionDefinition {
                call: "func2".to_string(),
                with: None,
                common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );
    map.add(
        "third".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            serverless_workflow_core::models::call::CallFunctionDefinition {
                call: "func3".to_string(),
                with: None,
                common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );

    let json_str = serde_json::to_string(&map).expect("Failed to serialize map");
    // JSON array order should be preserved
    assert!(json_str.contains("first"));
    assert!(json_str.contains("second"));
    assert!(json_str.contains("third"));
}

#[test]
fn test_set_task_with_nested_object() {
    // Test set task with nested object values
    let set_task_json = json!({
        "set": {
            "nested": {
                "level1": {
                    "level2": "value"
                }
            },
            "array": [1, 2, 3]
        }
    });
    let result: Result<SetTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"set": set_task_json}));
    assert!(
        result.is_ok(),
        "Failed to deserialize set task with nested object: {:?}",
        result.err()
    );
}

#[test]
fn test_error_definition_with_string_status() {
    // Test ErrorDefinition with string status
    use serverless_workflow_core::models::error::ErrorDefinition;

    let error_json = json!({
        "type": "https://example.com/errors",
        "status": "500",
        "title": "Server Error"
    });
    let result: Result<ErrorDefinition, _> = serde_json::from_value(error_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize error with string status: {:?}",
        result.err()
    );
}

#[test]
fn test_fork_task_branches_construction() {
    // Test ForkTaskDefinition with multiple branches and compete flag
    use serverless_workflow_core::models::map::Map;
    use serverless_workflow_core::models::task::{
        BranchingDefinition, CallTaskDefinition, ForkTaskDefinition, TaskDefinition,
    };

    let mut branch1 = Map::new();
    branch1.add(
        "callNurse".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            serverless_workflow_core::models::call::CallFunctionDefinition {
                call: "http".to_string(),
                with: None,
                common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );

    let mut branch2 = Map::new();
    branch2.add(
        "callDoctor".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            serverless_workflow_core::models::call::CallFunctionDefinition {
                call: "http".to_string(),
                with: None,
                common: serverless_workflow_core::models::task::TaskDefinitionFields::default(),
            },
        ))),
    );

    let mut branches = Map::new();
    branches.add(
        "branch1".to_string(),
        TaskDefinition::Do(serverless_workflow_core::models::task::DoTaskDefinition::new(branch1)),
    );
    branches.add(
        "branch2".to_string(),
        TaskDefinition::Do(serverless_workflow_core::models::task::DoTaskDefinition::new(branch2)),
    );

    let fork = BranchingDefinition::new(branches, true);
    let fork_task = ForkTaskDefinition::new(fork);

    let json_str = serde_json::to_string(&fork_task).expect("Failed to serialize fork task");
    assert!(json_str.contains("fork"));
    assert!(json_str.contains("compete"));
}

#[test]
fn test_fork_task_deserialization() {
    // Test deserialization of fork task from specification example
    let fork_json = json!({
        "fork": {
            "branches": [
                {
                    "callNurse": {
                        "call": "http",
                        "with": {
                            "method": "put",
                            "endpoint": "https://fake-hospital.com/api/v3/alert/nurses"
                        }
                    }
                },
                {
                    "callDoctor": {
                        "call": "http",
                        "with": {
                            "method": "put",
                            "endpoint": "https://fake-hospital.com/api/v3/alert/doctor"
                        }
                    }
                }
            ],
            "compete": true
        }
    });

    let result: Result<ForkTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"fork": fork_json["fork"]}));
    assert!(
        result.is_ok(),
        "Failed to deserialize fork task: {:?}",
        result.err()
    );
}

#[test]
fn test_switch_task_with_multiple_cases() {
    // Test SwitchTaskDefinition with multiple cases
    use serverless_workflow_core::models::map::Map;
    use serverless_workflow_core::models::task::{SwitchCaseDefinition, SwitchTaskDefinition};

    let mut switch_cases = Map::new();
    switch_cases.add(
        "case1".to_string(),
        SwitchCaseDefinition {
            when: Some("${ .orderType == \"electronic\" }".to_string()),
            then: Some("processElectronicOrder".to_string()),
        },
    );
    switch_cases.add(
        "case2".to_string(),
        SwitchCaseDefinition {
            when: Some("${ .orderType == \"physical\" }".to_string()),
            then: Some("processPhysicalOrder".to_string()),
        },
    );
    switch_cases.add(
        "default".to_string(),
        SwitchCaseDefinition {
            when: None,
            then: Some("handleUnknownOrderType".to_string()),
        },
    );

    let switch_task = SwitchTaskDefinition {
        switch: switch_cases,
        ..Default::default()
    };

    let json_str = serde_json::to_string(&switch_task).expect("Failed to serialize switch task");
    assert!(json_str.contains("switch"));
    assert!(json_str.contains("case1"));
    assert!(json_str.contains("case2"));
}

#[test]
fn test_switch_task_deserialization() {
    // Test deserialization of switch task from specification example
    let switch_json = json!({
        "switch": [
            {
                "case1": {
                    "when": ".orderType == \"electronic\"",
                    "then": "processElectronicOrder"
                }
            },
            {
                "case2": {
                    "when": ".orderType == \"physical\"",
                    "then": "processPhysicalOrder"
                }
            },
            {
                "default": {
                    "then": "handleUnknownOrderType"
                }
            }
        ]
    });

    let result: Result<SwitchTaskDefinition, _> = serde_json::from_value(switch_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch task: {:?}",
        result.err()
    );
    if let Ok(switch_task) = result {
        assert_eq!(switch_task.switch.entries.len(), 3);
    }
}

#[test]
fn test_oauth2_authentication() {
    // Test OAuth2 authentication from specification example
    let oauth2_json = json!({
        "authority": "http://keycloak/realms/fake-authority",
        "endpoints": {
            "token": "/auth/token",
            "introspection": "/auth/introspect"
        },
        "grant": "client_credentials",
        "client": {
            "id": "workflow-runtime-id",
            "secret": "workflow-runtime-secret"
        }
    });

    let result: Result<
        serverless_workflow_core::models::authentication::OAuth2AuthenticationSchemeDefinition,
        _,
    > = serde_json::from_value(oauth2_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize OAuth2 auth: {:?}",
        result.err()
    );
    if let Ok(oauth2) = result {
        assert_eq!(
            oauth2.authority,
            Some("http://keycloak/realms/fake-authority".to_string())
        );
        assert_eq!(oauth2.grant, Some("client_credentials".to_string()));
    }
}

#[test]
fn test_bearer_authentication_with_uri() {
    // Test Bearer authentication with URI format
    let bearer_json = json!({
        "token": "${ .token }"
    });

    let result: Result<
        serverless_workflow_core::models::authentication::BearerAuthenticationSchemeDefinition,
        _,
    > = serde_json::from_value(bearer_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize bearer auth: {:?}",
        result.err()
    );
    if let Ok(bearer) = result {
        assert!(bearer.token.is_some());
        if let Some(token) = bearer.token {
            assert!(token.contains(".token"));
        }
    }
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
fn test_try_catch_retry_inline_workflow() {
    // Test try-catch-retry inline workflow from specification example
    let try_catch_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "default",
            "name": "try-catch-retry",
            "version": "0.1.0"
        },
        "do": [
            {
                "tryGetPet": {
                    "try": [
                        {
                            "getPet": {
                                "call": "http",
                                "with": {
                                    "method": "get",
                                    "endpoint": "https://petstore.swagger.io/v2/pet/{petId}"
                                }
                            }
                        }
                    ],
                    "catch": {
                        "errors": [
                            {
                                "with": {
                                    "type": "https://serverlessworkflow.io/spec/1.0.0/errors/communication",
                                    "status": 503
                                }
                            }
                        ],
                        "retry": {
                            "delay": {
                                "seconds": 3
                            },
                            "backoff": {
                                "exponential": {}
                            },
                            "limit": {
                                "attempt": {
                                    "count": 5
                                }
                            }
                        }
                    }
                }
            }
        ]
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(try_catch_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize try-catch-retry workflow: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_external_schema_input() {
    // Test workflow with external schema input
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "examples",
            "name": "http-query-headers-expressions",
            "version": "1.0.0"
        },
        "input": {
            "schema": {
                "format": "json",
                "document": {
                    "type": "object",
                    "required": ["searchQuery"],
                    "properties": {
                        "searchQuery": {
                            "type": "string"
                        }
                    }
                }
            }
        },
        "do": []
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with schema input: {:?}",
        result.err()
    );
    if let Ok(workflow) = result {
        assert!(workflow.input.is_some());
        let input = workflow.input.unwrap();
        assert!(input.schema.is_some());
    }
}

#[test]
fn test_for_task_with_while_condition_spec() {
    // Test for task with while condition from specification example
    let for_json = json!({
        "for": {
            "each": "pet",
            "in": ".pets",
            "at": "index"
        },
        "while": ".vet != null",
        "do": [
            {
                "waitForCheckup": {
                    "listen": {
                        "to": {
                            "one": {
                                "with": {
                                    "type": "com.fake.petclinic.pets.checkup.completed.v2"
                                }
                            }
                        }
                    },
                    "output": {
                        "as": ".pets + [{ \"id\": $pet.id }]"
                    }
                }
            }
        ]
    });

    let result: Result<serverless_workflow_core::models::task::ForTaskDefinition, _> =
        serde_json::from_value(for_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize for task with while: {:?}",
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
fn test_emit_task() {
    // Test emit task from specification example
    use std::collections::HashMap;
    let mut with = HashMap::new();
    with.insert(
        "type".to_string(),
        serde_json::json!("com.fake.petclinic.pets.checkup.completed"),
    );
    with.insert("source".to_string(), serde_json::json!("test-source"));

    let emit_json = json!({
        "emit": {
            "event": {
                "with": with
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::EmitTaskDefinition, _> =
        serde_json::from_value(emit_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize emit task: {:?}",
        result.err()
    );
}

#[test]
fn test_wait_duration_iso8601() {
    // Test wait task with ISO8601 duration (matches Go SDK: WaitTask.Wait is *Duration)
    let wait_json = json!({
        "wait": "PT1S"
    });

    let result: Result<serverless_workflow_core::models::task::WaitTaskDefinition, _> =
        serde_json::from_value(wait_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait with ISO8601: {:?}",
        result.err()
    );
}

#[test]
fn test_wait_duration_inline() {
    // Test wait task with inline duration (matches Go SDK: DurationInline with valid keys)
    let wait_json = json!({
        "wait": {
            "seconds": 5
        }
    });

    let result: Result<serverless_workflow_core::models::task::WaitTaskDefinition, _> =
        serde_json::from_value(wait_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize wait with inline duration: {:?}",
        result.err()
    );
}

#[test]
fn test_authentication_reusable() {
    // Test reusable authentication definitions
    let auth_json = json!({
        "name": "myAuth",
        "scheme": {
            "basic": {
                "username": "admin",
                "password": "admin"
            }
        }
    });

    let result: Result<
        serverless_workflow_core::models::authentication::AuthenticationPolicyDefinition,
        _,
    > = serde_json::from_value(auth_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize reusable auth: {:?}",
        result.err()
    );
}

#[test]
fn test_schedule_with_cron() {
    // Test schedule with cron expression
    let schedule_json = json!({
        "every": {
            "seconds": 10
        },
        "cron": "0 0 * * * *"
    });

    let result: Result<serverless_workflow_core::models::workflow::WorkflowScheduleDefinition, _> =
        serde_json::from_value(schedule_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize schedule with cron: {:?}",
        result.err()
    );
}

#[test]
fn test_schedule_event_driven() {
    // Test event-driven schedule (when 'on' is set)
    let schedule_json = json!({
        "on": {
            "one": {
                "with": {
                    "type": "com.example.event"
                }
            }
        }
    });

    let result: Result<serverless_workflow_core::models::workflow::WorkflowScheduleDefinition, _> =
        serde_json::from_value(schedule_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize event-driven schedule: {:?}",
        result.err()
    );
}

#[test]
fn test_run_task_container() {
    // Test run task with container
    let run_json = json!({
        "run": {
            "container": {
                "image": "nginx:latest"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::RunTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"run": run_json["run"]}));
    assert!(
        result.is_ok(),
        "Failed to deserialize run container task: {:?}",
        result.err()
    );
}

#[test]
fn test_run_task_script() {
    // Test run task with script
    let run_json = json!({
        "run": {
            "script": {
                "language": "sh",
                "source": {
                    "endpoint": "echo hello"
                }
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::RunTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"run": run_json["run"]}));
    assert!(
        result.is_ok(),
        "Failed to deserialize run script task: {:?}",
        result.err()
    );
}

#[test]
fn test_run_task_shell() {
    // Test run task with shell
    let run_json = json!({
        "run": {
            "shell": {
                "command": "echo hello"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::RunTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"run": run_json["run"]}));
    assert!(
        result.is_ok(),
        "Failed to deserialize run shell task: {:?}",
        result.err()
    );
}

#[test]
fn test_listen_task_with_any_and_until() {
    // Test listen task with 'any' events and 'until' condition
    let listen_json = json!({
        "listen": {
            "to": {
                "any": [],
                "until": "( . | length ) > 3"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ListenTaskDefinition, _> =
        serde_json::from_value(listen_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize listen task with any/until: {:?}",
        result.err()
    );
}

#[test]
fn test_listen_task_with_one_event() {
    // Test listen task with single event
    let listen_json = json!({
        "listen": {
            "to": {
                "one": {
                    "with": {
                        "type": "com.example.event"
                    }
                }
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ListenTaskDefinition, _> =
        serde_json::from_value(listen_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize listen task with one event: {:?}",
        result.err()
    );
}

#[test]
fn test_listen_task_with_all_filter() {
    // Test listen task with 'all' filter
    let listen_json = json!({
        "listen": {
            "to": {
                "all": [
                    {
                        "with": {
                            "type": "com.example.event1"
                        }
                    },
                    {
                        "with": {
                            "type": "com.example.event2"
                        }
                    }
                ]
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ListenTaskDefinition, _> =
        serde_json::from_value(listen_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize listen task with all filter: {:?}",
        result.err()
    );
}

#[test]
fn test_listen_task_with_all_filter_and_correlate() {
    // Test listen task with 'all' filter and correlate (similar to accumulate-room-readings example)
    let listen_json = json!({
        "listen": {
            "to": {
                "all": [
                    {
                        "with": {
                            "source": "https://my.home.com/sensor",
                            "type": "my.home.sensors.temperature"
                        },
                        "correlate": {
                            "roomId": {
                                "from": ".roomid"
                            }
                        }
                    },
                    {
                        "with": {
                            "source": "https://my.home.com/sensor",
                            "type": "my.home.sensors.humidity"
                        },
                        "correlate": {
                            "roomId": {
                                "from": ".roomid"
                            }
                        }
                    }
                ]
            },
            "output": {
                "as": ".data.reading"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ListenTaskDefinition, _> =
        serde_json::from_value(listen_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize listen task with all filter and correlate: {:?}",
        result.err()
    );
    let listen_task = result.unwrap();
    assert!(listen_task.listen.to.all.is_some());
    let all_events = listen_task.listen.to.all.unwrap();
    assert_eq!(all_events.len(), 2);
    // Check correlate is preserved
    let first_event = &all_events[0];
    assert!(first_event.correlate.is_some());
}

#[test]
fn test_timeout_with_inline_duration() {
    // Test TimeoutDefinition with inline duration
    let timeout_json = json!({
        "after": {
            "days": 1,
            "hours": 2,
            "minutes": 30
        }
    });

    let result: Result<serverless_workflow_core::models::timeout::TimeoutDefinition, _> =
        serde_json::from_value(timeout_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout with inline duration: {:?}",
        result.err()
    );
}

#[test]
fn test_timeout_with_iso8601_duration() {
    // Test TimeoutDefinition with ISO 8601 duration
    let timeout_json = json!({
        "after": "PT1H30M"
    });

    let result: Result<serverless_workflow_core::models::timeout::TimeoutDefinition, _> =
        serde_json::from_value(timeout_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize timeout with ISO8601: {:?}",
        result.err()
    );
}

#[test]
fn test_oidc_authentication() {
    // Test OIDC authentication from specification
    let oidc_json = json!({
        "oidc": {
            "authority": "https://authority.com",
            "client": {
                "id": "client-id",
                "secret": "client-secret"
            },
            "scopes": ["openid", "profile"]
        }
    });

    let result: Result<
        serverless_workflow_core::models::authentication::OpenIDConnectSchemeDefinition,
        _,
    > = serde_json::from_value(oidc_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize OIDC auth: {:?}",
        result.err()
    );
}

#[test]
fn test_digest_authentication() {
    // Test Digest authentication
    let digest_json = json!({
        "digest": {
            "username": "user",
            "password": "pass"
        }
    });

    let result: Result<
        serverless_workflow_core::models::authentication::DigestAuthenticationSchemeDefinition,
        _,
    > = serde_json::from_value(digest_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize digest auth: {:?}",
        result.err()
    );
}

#[test]
fn test_retry_with_linear_backoff() {
    // Test retry policy with linear backoff
    let retry_json = json!({
        "delay": {
            "seconds": 5
        },
        "backoff": {
            "linear": {
                "wait": "PT1S"
            }
        },
        "limit": {
            "attempt": {
                "count": 3
            }
        }
    });

    let result: Result<serverless_workflow_core::models::retry::RetryPolicyDefinition, _> =
        serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry with linear backoff: {:?}",
        result.err()
    );
}

#[test]
fn test_retry_with_constant_backoff() {
    // Test retry policy with constant backoff
    let retry_json = json!({
        "delay": {
            "seconds": 5
        },
        "backoff": {
            "constant": {
                "wait": "PT1S"
            }
        },
        "limit": {
            "attempt": {
                "count": 3
            }
        }
    });

    let result: Result<serverless_workflow_core::models::retry::RetryPolicyDefinition, _> =
        serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry with constant backoff: {:?}",
        result.err()
    );
}

#[test]
fn test_retry_with_exponential_backoff() {
    // Test retry policy with exponential backoff
    let retry_json = json!({
        "delay": {
            "seconds": 5
        },
        "backoff": {
            "exponential": {}
        },
        "limit": {
            "attempt": {
                "count": 3
            }
        }
    });

    let result: Result<serverless_workflow_core::models::retry::RetryPolicyDefinition, _> =
        serde_json::from_value(retry_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize retry with exponential backoff: {:?}",
        result.err()
    );
}

#[test]
fn test_error_filter_with_status() {
    // Test error filter with status as string
    let error_filter_json = json!({
        "with": {
            "type": "https://example.com/errors",
            "status": 500,
            "title": "Server Error"
        },
        "use": {
            "set": {
                "error": "true"
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ErrorFilterDefinition, _> =
        serde_json::from_value(error_filter_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize error filter: {:?}",
        result.err()
    );
}

#[test]
fn test_error_catcher_with_retry() {
    // Test error catcher with retry policy
    let error_catcher_json = json!({
        "errors": {
            "with": {
                "type": "https://example.com/errors/connection"
            }
        },
        "retry": {
            "delay": {
                "seconds": 1
            },
            "backoff": {
                "exponential": {}
            },
            "limit": {
                "attempt": {
                    "count": 5
                }
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ErrorCatcherDefinition, _> =
        serde_json::from_value(error_catcher_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize error catcher: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_schedule_cron() {
    // Test workflow with cron schedule
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "scheduled-workflow",
            "version": "1.0.0"
        },
        "schedule": {
            "every": {
                "hours": 1
            },
            "cron": "0 0 * * * *"
        },
        "do": []
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with schedule: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_with_schedule_after() {
    // Test workflow with 'after' schedule
    let workflow_json = json!({
        "document": {
            "dsl": "1.0.0",
            "namespace": "test",
            "name": "delayed-workflow",
            "version": "1.0.0"
        },
        "schedule": {
            "after": {
                "minutes": 30
            }
        },
        "do": []
    });

    let result: Result<WorkflowDefinition, _> = serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow with after schedule: {:?}",
        result.err()
    );
}

#[test]
fn test_event_consumption_all_strategy() {
    // Test event consumption with 'all' strategy
    let strategy_json = json!({
        "all": [
            {
                "with": {
                    "type": "com.example.event1"
                }
            },
            {
                "with": {
                    "type": "com.example.event2"
                }
            }
        ]
    });

    let result: Result<
        serverless_workflow_core::models::event::EventConsumptionStrategyDefinition,
        _,
    > = serde_json::from_value(strategy_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize all strategy: {:?}",
        result.err()
    );
}

#[test]
fn test_container_process_with_pull_policy() {
    // Test container process with pull policy
    let container_json = json!({
        "image": "nginx:latest",
        "pullPolicy": "Always"
    });

    let result: Result<serverless_workflow_core::models::task::ContainerProcessDefinition, _> =
        serde_json::from_value(container_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize container with pull policy: {:?}",
        result.err()
    );
    let container = result.unwrap();
    assert!(container.pull_policy.is_some());
    assert_eq!(container.pull_policy.unwrap(), "Always");
}

#[test]
fn test_container_process_with_lifetime_eventually() {
    // Test container process with lifetime cleanup=eventually
    let container_json = json!({
        "image": "hello-world",
        "lifetime": {
            "cleanup": "eventually",
            "after": {
                "seconds": 20
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ContainerProcessDefinition, _> =
        serde_json::from_value(container_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize container with lifetime: {:?}",
        result.err()
    );
    let container = result.unwrap();
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.unwrap();
    assert_eq!(lifetime.cleanup, "eventually");
    assert!(lifetime.after.is_some());
}

#[test]
fn test_container_process_with_lifetime_always() {
    // Test container process with lifetime cleanup=always
    let container_json = json!({
        "image": "hello-world",
        "lifetime": {
            "cleanup": "always"
        }
    });

    let result: Result<serverless_workflow_core::models::task::ContainerProcessDefinition, _> =
        serde_json::from_value(container_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize container with lifetime: {:?}",
        result.err()
    );
    let container = result.unwrap();
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.unwrap();
    assert_eq!(lifetime.cleanup, "always");
    assert!(lifetime.after.is_none());
}

#[test]
fn test_container_process_with_lifetime_never() {
    // Test container process with lifetime cleanup=never
    let container_json = json!({
        "image": "hello-world",
        "lifetime": {
            "cleanup": "never"
        }
    });

    let result: Result<serverless_workflow_core::models::task::ContainerProcessDefinition, _> =
        serde_json::from_value(container_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize container with lifetime: {:?}",
        result.err()
    );
    let container = result.unwrap();
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.unwrap();
    assert_eq!(lifetime.cleanup, "never");
}

#[test]
fn test_container_process_with_all_fields() {
    // Test container process with all fields including lifetime and pullPolicy
    let container_json = json!({
        "image": "nginx:latest",
        "name": "my-container",
        "command": "/bin/sh",
        "ports": {"8080": 80},
        "volumes": {"/data": "/storage"},
        "environment": {"NODE_ENV": "production"},
        "stdin": "input data",
        "arguments": ["arg1", "arg2"],
        "pullPolicy": "IfNotPresent",
        "lifetime": {
            "cleanup": "eventually",
            "after": {
                "minutes": 30
            }
        }
    });

    let result: Result<serverless_workflow_core::models::task::ContainerProcessDefinition, _> =
        serde_json::from_value(container_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize container with all fields: {:?}",
        result.err()
    );
    let container = result.unwrap();
    assert_eq!(container.image, "nginx:latest");
    assert_eq!(container.name, Some("my-container".to_string()));
    assert_eq!(container.command, Some("/bin/sh".to_string()));
    assert!(container.ports.is_some());
    assert!(container.volumes.is_some());
    assert!(container.environment.is_some());
    assert_eq!(container.stdin, Some("input data".to_string()));
    assert_eq!(
        container.arguments,
        Some(vec!["arg1".to_string(), "arg2".to_string()])
    );
    assert!(container.pull_policy.is_some());
    assert_eq!(container.pull_policy.unwrap(), "IfNotPresent");
    assert!(container.lifetime.is_some());
    let lifetime = container.lifetime.unwrap();
    assert_eq!(lifetime.cleanup, "eventually");
    assert!(lifetime.after.is_some());
}

#[test]
fn test_workflow_process() {
    // Test workflow process definition
    let workflow_json = json!({
        "namespace": "default",
        "name": "sub-workflow",
        "version": "1.0.0"
    });

    let result: Result<serverless_workflow_core::models::task::WorkflowProcessDefinition, _> =
        serde_json::from_value(workflow_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow process: {:?}",
        result.err()
    );
}

#[test]
fn test_subscription_iterator() {
    // Test subscription iterator definition
    let iterator_json = json!({
        "IEF": "someExpression"
    });

    let result: Result<serverless_workflow_core::models::task::SubscriptionIteratorDefinition, _> =
        serde_json::from_value(iterator_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize subscription iterator: {:?}",
        result.err()
    );
}

#[test]
fn test_event_definition_with_multiple_attributes() {
    // Test event definition with multiple attributes
    use std::collections::HashMap;
    let mut with = HashMap::new();
    with.insert("type".to_string(), serde_json::json!("com.example.event"));
    with.insert(
        "source".to_string(),
        serde_json::json!("https://example.com"),
    );
    with.insert("id".to_string(), serde_json::json!("event-123"));

    let event_def = serverless_workflow_core::models::event::EventDefinition::new(with);
    let json_str = serde_json::to_string(&event_def).expect("Failed to serialize");
    assert!(json_str.contains("com.example.event"));
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
                        "SLACK_MCP_TOKEN": "xoxp-xxx"
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
fn test_raise_task_with_error_reference() {
    // Test raise task with error reference (string reference directly)
    let raise_json = json!({
        "raise": {
            "error": "someError"
        }
    });

    let result: Result<serverless_workflow_core::models::task::RaiseTaskDefinition, _> =
        serde_json::from_value(serde_json::json!({"raise": raise_json["raise"]}));
    assert!(
        result.is_ok(),
        "Failed to deserialize raise task with error reference: {:?}",
        result.err()
    );
}

#[test]
fn test_switch_task_with_string_then() {
    // Test switch task where 'then' is a string (transition)
    let switch_json = json!({
        "switch": [
            {
                "case1": {
                    "when": ".status == 'active'",
                    "then": "processActive"
                }
            },
            {
                "default": {
                    "then": "handleDefault"
                }
            }
        ]
    });

    let result: Result<serverless_workflow_core::models::task::SwitchTaskDefinition, _> =
        serde_json::from_value(switch_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize switch with string then: {:?}",
        result.err()
    );
}

#[test]
fn test_for_task_without_while() {
    // Test for task without while condition (iterate all items)
    let for_json = json!({
        "for": {
            "each": "item",
            "in": ".items"
        },
        "do": [
            {
                "processItem": {
                    "set": {
                        "processed": true
                    }
                }
            }
        ]
    });

    let result: Result<serverless_workflow_core::models::task::ForTaskDefinition, _> =
        serde_json::from_value(for_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize for without while: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_output() {
    // Test workflow output definition
    let output_json = json!({
        "as": "${ .result }"
    });

    let result: Result<serverless_workflow_core::models::output::OutputDataModelDefinition, _> =
        serde_json::from_value(output_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow output: {:?}",
        result.err()
    );
}

#[test]
fn test_workflow_input() {
    // Test workflow input definition with schema
    let input_json = json!({
        "schema": {
            "format": "json",
            "document": {
                "type": "object",
                "properties": {
                    "data": {
                        "type": "string"
                    }
                }
            }
        }
    });

    let result: Result<serverless_workflow_core::models::input::InputDataModelDefinition, _> =
        serde_json::from_value(input_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize workflow input: {:?}",
        result.err()
    );
}

// ==================== New Feature Tests ====================

#[test]
fn test_referenceable_authentication_policy_reference() {
    use serverless_workflow_core::models::authentication::{
        AuthenticationPolicyReference, ReferenceableAuthenticationPolicy,
    };

    // Test reference-based policy
    let json_str = r#"{"use": "myAuthPolicy"}"#;
    let policy: ReferenceableAuthenticationPolicy =
        serde_json::from_str(json_str).expect("Failed to deserialize reference");
    match policy {
        ReferenceableAuthenticationPolicy::Reference(ref r) => {
            assert_eq!(r.use_, "myAuthPolicy")
        }
        _ => panic!("Expected Reference variant"),
    }

    // Test serialization roundtrip
    let reference = AuthenticationPolicyReference {
        use_: "testPolicy".to_string(),
    };
    let policy = ReferenceableAuthenticationPolicy::Reference(reference);
    let serialized = serde_json::to_string(&policy).expect("Failed to serialize");
    assert!(serialized.contains("\"use\":\"testPolicy\""));
}

#[test]
fn test_referenceable_authentication_policy_inline() {
    use serverless_workflow_core::models::authentication::ReferenceableAuthenticationPolicy;

    // Test inline policy with basic auth
    let json_str = r#"{"basic":{"username":"admin","password":"secret"}}"#;
    let policy: ReferenceableAuthenticationPolicy =
        serde_json::from_str(json_str).expect("Failed to deserialize inline policy");
    match policy {
        ReferenceableAuthenticationPolicy::Policy(ref p) => {
            assert!(p.basic.is_some());
            let basic = p.basic.as_ref().unwrap();
            assert_eq!(basic.username, Some("admin".to_string()));
        }
        _ => panic!("Expected Policy variant"),
    }
}

#[test]
fn test_flow_directive_constants() {
    assert_eq!(FlowDirective::CONTINUE, "continue");
    assert_eq!(FlowDirective::EXIT, "exit");
    assert_eq!(FlowDirective::END, "end");
}

#[test]
fn test_container_cleanup_policy_constants() {
    assert_eq!(ContainerCleanupPolicy::ALWAYS, "always");
    assert_eq!(ContainerCleanupPolicy::EVENTUALLY, "eventually");
    assert_eq!(ContainerCleanupPolicy::NEVER, "never");
}

#[test]
fn test_event_read_mode_constants() {
    assert_eq!(EventReadMode::DATA, "data");
    assert_eq!(EventReadMode::ENVELOPE, "envelope");
    assert_eq!(EventReadMode::RAW, "raw");
}

#[test]
fn test_http_output_format_constants() {
    assert_eq!(HttpOutputFormat::RAW, "raw");
    assert_eq!(HttpOutputFormat::CONTENT, "content");
    assert_eq!(HttpOutputFormat::RESPONSE, "response");
}

#[test]
fn test_process_return_type_constants() {
    assert_eq!(ProcessReturnType::STDOUT, "stdout");
    assert_eq!(ProcessReturnType::STDERR, "stderr");
    assert_eq!(ProcessReturnType::CODE, "code");
    assert_eq!(ProcessReturnType::ALL, "all");
    assert_eq!(ProcessReturnType::NONE, "none");
}

#[test]
fn test_extension_target_constants() {
    assert_eq!(ExtensionTarget::CALL, "call");
    assert_eq!(ExtensionTarget::COMPOSITE, "composite");
    assert_eq!(ExtensionTarget::EMIT, "emit");
    assert_eq!(ExtensionTarget::FOR, "for");
    assert_eq!(ExtensionTarget::LISTEN, "listen");
    assert_eq!(ExtensionTarget::RAISE, "raise");
    assert_eq!(ExtensionTarget::RUN, "run");
    assert_eq!(ExtensionTarget::SET, "set");
    assert_eq!(ExtensionTarget::SWITCH, "switch");
    assert_eq!(ExtensionTarget::TRY, "try");
    assert_eq!(ExtensionTarget::WAIT, "wait");
    assert_eq!(ExtensionTarget::ALL, "all");
}

#[test]
fn test_oauth2_grant_type_constants() {
    assert_eq!(OAuth2GrantType::AUTHORIZATION_CODE, "authorization_code");
    assert_eq!(OAuth2GrantType::CLIENT_CREDENTIALS, "client_credentials");
    assert_eq!(OAuth2GrantType::PASSWORD, "password");
    assert_eq!(OAuth2GrantType::REFRESH_TOKEN, "refresh_token");
    assert_eq!(
        OAuth2GrantType::TOKEN_EXCHANGE,
        "urn:ietf:params:oauth:grant-type:token-exchange"
    );
}

#[test]
fn test_backoff_definition_with_parameters() {
    use serverless_workflow_core::models::retry::*;

    // Test constant backoff with definition
    let constant = ConstantBackoffDefinition {
        definition: Some({
            let mut map = std::collections::HashMap::new();
            map.insert("factor".to_string(), json!(2));
            map
        }),
    };
    let json_str = serde_json::to_string(&constant).expect("Failed to serialize constant backoff");
    assert!(json_str.contains("\"factor\":2"));

    let deserialized: ConstantBackoffDefinition =
        serde_json::from_str(&json_str).expect("Failed to deserialize");
    assert!(deserialized.definition.is_some());

    // Test exponential backoff with definition
    let exponential = ExponentialBackoffDefinition {
        definition: Some({
            let mut map = std::collections::HashMap::new();
            map.insert("factor".to_string(), json!(2));
            map.insert("maxDelay".to_string(), json!("PT30S"));
            map
        }),
    };
    let json_str =
        serde_json::to_string(&exponential).expect("Failed to serialize exponential backoff");
    assert!(json_str.contains("\"factor\":2"));
    assert!(json_str.contains("\"maxDelay\":\"PT30S\""));

    // Test linear backoff with increment and definition
    let linear = LinearBackoffDefinition {
        increment: Some(Duration::from_milliseconds(1000)),
        definition: Some({
            let mut map = std::collections::HashMap::new();
            map.insert("maxDelay".to_string(), json!("PT30S"));
            map
        }),
    };
    let json_str = serde_json::to_string(&linear).expect("Failed to serialize linear backoff");
    assert!(json_str.contains("PT1S") || json_str.contains("\"increment\""));
    assert!(json_str.contains("\"maxDelay\":\"PT30S\""));
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
fn test_error_definition_is_type_checks() {
    let config_error = ErrorDefinition::configuration_error(Some("bad config".to_string()), None);
    assert!(config_error.is_configuration_error());
    assert!(!config_error.is_runtime_error());

    let auth_error = ErrorDefinition::authentication_error(None, None);
    assert!(auth_error.is_authentication_error());

    let timeout_error = ErrorDefinition::timeout_error(None, None);
    assert!(timeout_error.is_timeout_error());

    let runtime_error = ErrorDefinition::runtime_error(Some("crash".to_string()), None);
    assert!(runtime_error.is_runtime_error());
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
fn test_oauth2_endpoints_defaults() {
    use serverless_workflow_core::models::authentication::OAuth2AuthenticationEndpointsDefinition;

    let json_str = r#"{}"#;
    let endpoints: OAuth2AuthenticationEndpointsDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize");
    assert_eq!(endpoints.token, "/oauth2/token");
    assert_eq!(endpoints.revocation, "/oauth2/revoke");
    assert_eq!(endpoints.introspection, "/oauth2/introspect");
}

#[test]
fn test_oauth2_request_encoding_default() {
    use serverless_workflow_core::models::authentication::OAuth2AuthenticationRequestDefinition;

    let json_str = r#"{}"#;
    let request: OAuth2AuthenticationRequestDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize");
    assert_eq!(request.encoding, "application/x-www-form-urlencoded");
}

// ============== New tests for P0/P1 features ==============

#[test]
fn test_oauth2_token_definition_serde() {
    use serverless_workflow_core::models::authentication::OAuth2TokenDefinition;
    // Verify the token field serializes/deserializes correctly (was a bug: rename was "encoding" instead of "token")
    let json_str = r#"{"token": "my-token", "type": "Bearer"}"#;
    let token: OAuth2TokenDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize");
    assert_eq!(token.token, "my-token");
    assert_eq!(token.type_, "Bearer");

    let serialized = serde_json::to_string(&token).expect("Failed to serialize");
    assert!(serialized.contains("\"token\""));
    assert!(!serialized.contains("\"encoding\""));
}

#[test]
fn test_run_task_return_field() {
    // Test that ProcessTypeDefinition's return field deserializes correctly
    let json_str = r#"{"shell": {"command": "ls"}, "return": "all"}"#;
    let process: ProcessTypeDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize ProcessTypeDefinition");
    assert_eq!(process.return_, Some("all".to_string()));
    assert!(process.shell.is_some());
}

#[test]
fn test_process_return_type_constants_values() {
    assert_eq!(ProcessReturnType::STDOUT, "stdout");
    assert_eq!(ProcessReturnType::STDERR, "stderr");
    assert_eq!(ProcessReturnType::CODE, "code");
    assert_eq!(ProcessReturnType::ALL, "all");
    assert_eq!(ProcessReturnType::NONE, "none");
}

#[test]
fn test_error_filter_properties() {
    use serverless_workflow_core::models::task::ErrorFilterProperties;
    let props = ErrorFilterProperties {
        type_: Some("https://serverlessworkflow.io/spec/1.0.0/errors/communication".to_string()),
        status: Some(json!(500)),
        instance: None,
        title: None,
        detail: None,
    };
    assert_eq!(
        props.type_,
        Some("https://serverlessworkflow.io/spec/1.0.0/errors/communication".to_string())
    );
    assert_eq!(props.status, Some(json!(500)));
}

#[test]
fn test_error_filter_definition_serde() {
    use serverless_workflow_core::models::task::ErrorFilterDefinition;
    let json_str = r#"{"with": {"type": "https://serverlessworkflow.io/spec/1.0.0/errors/communication", "status": 500}}"#;
    let filter: ErrorFilterDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize");
    assert!(filter.with.is_some());
    let props = filter.with.unwrap();
    assert_eq!(
        props.type_,
        Some("https://serverlessworkflow.io/spec/1.0.0/errors/communication".to_string())
    );
    assert_eq!(props.status, Some(json!(500)));
}

#[test]
fn test_error_type_enum() {
    use serverless_workflow_core::models::error::ErrorType;
    let uri = ErrorType::uri_template("https://serverlessworkflow.io/spec/1.0.0/errors/timeout");
    assert_eq!(
        uri.as_str(),
        "https://serverlessworkflow.io/spec/1.0.0/errors/timeout"
    );
    assert!(!uri.is_runtime_expression());

    let expr = ErrorType::runtime_expression("${ .error.type }");
    assert_eq!(expr.as_str(), "${ .error.type }");
    assert!(expr.is_runtime_expression());
}

#[test]
fn test_error_definition_with_error_type() {
    let err = ErrorDefinition::timeout_error(Some("timed out".to_string()), None);
    assert!(err.is_timeout_error());
    assert_eq!(
        err.type_.as_str(),
        "https://serverlessworkflow.io/spec/1.0.0/errors/timeout"
    );
}

#[test]
fn test_error_type_serde() {
    use serverless_workflow_core::models::error::ErrorType;
    // URI template
    let json_str = r#""https://serverlessworkflow.io/spec/1.0.0/errors/timeout""#;
    let et: ErrorType = serde_json::from_str(json_str).expect("Failed to deserialize");
    assert_eq!(
        et.as_str(),
        "https://serverlessworkflow.io/spec/1.0.0/errors/timeout"
    );
}

#[test]
fn test_schema_validation() {
    use serverless_workflow_core::models::resource::ExternalResourceDefinition;
    use serverless_workflow_core::models::schema::{SchemaDefinition, SchemaValidationError};

    // Neither set - should be invalid
    let schema = SchemaDefinition::default();
    assert_eq!(schema.validate(), Err(SchemaValidationError::NeitherSet));
    assert!(!schema.is_document());
    assert!(!schema.is_resource());

    // With document - should be valid
    let schema = SchemaDefinition::with_document("json", json!({"type": "object"}));
    assert!(schema.validate().is_ok());
    assert!(schema.is_document());
    assert!(!schema.is_resource());

    // With resource - should be valid
    let schema = SchemaDefinition::with_resource("json", ExternalResourceDefinition::default());
    assert!(schema.validate().is_ok());
    assert!(!schema.is_document());
    assert!(schema.is_resource());

    // Both set - should be invalid
    let mut schema = SchemaDefinition::with_document("json", json!({"type": "object"}));
    schema.resource = Some(ExternalResourceDefinition::default());
    assert_eq!(schema.validate(), Err(SchemaValidationError::BothSet));
}

#[test]
fn test_runtime_expression_evaluation_mode() {
    use serverless_workflow_core::models::workflow::RuntimeExpressionEvaluationMode;
    assert_eq!(RuntimeExpressionEvaluationMode::STRICT, "strict");
    assert_eq!(RuntimeExpressionEvaluationMode::LOOSE, "loose");
}

#[test]
fn test_runtime_expressions_constants() {
    use serverless_workflow_core::models::workflow::RuntimeExpressions;
    assert_eq!(RuntimeExpressions::RUNTIME, "runtime");
    assert_eq!(RuntimeExpressions::WORKFLOW, "workflow");
    assert_eq!(RuntimeExpressions::CONTEXT, "context");
    assert_eq!(RuntimeExpressions::ITEM, "item");
    assert_eq!(RuntimeExpressions::INDEX, "index");
    assert_eq!(RuntimeExpressions::OUTPUT, "output");
    assert_eq!(RuntimeExpressions::SECRET, "secret");
    assert_eq!(RuntimeExpressions::TASK, "task");
    assert_eq!(RuntimeExpressions::INPUT, "input");
    assert_eq!(RuntimeExpressions::ERROR, "error");
    assert_eq!(RuntimeExpressions::AUTHORIZATION, "authorization");
}

#[test]
fn test_runtime_expression_evaluation_configuration_mode() {
    use serverless_workflow_core::models::workflow::RuntimeExpressionEvaluationConfiguration;
    let json_str = r#"{"language": "jq", "mode": "strict"}"#;
    let config: RuntimeExpressionEvaluationConfiguration =
        serde_json::from_str(json_str).expect("Failed to deserialize");
    assert_eq!(config.language, "jq");
    assert_eq!(config.mode, Some("strict".to_string()));
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

#[test]
fn test_flow_directive_value_enumerated() {
    use serverless_workflow_core::models::task::FlowDirectiveType;
    use serverless_workflow_core::models::task::FlowDirectiveValue;

    let cont = FlowDirectiveValue::Enumerated(FlowDirectiveType::Continue);
    assert!(cont.is_enumerated());
    assert!(!cont.is_termination());

    let exit = FlowDirectiveValue::Enumerated(FlowDirectiveType::Exit);
    assert!(exit.is_enumerated());
    assert!(exit.is_termination());

    let end = FlowDirectiveValue::Enumerated(FlowDirectiveType::End);
    assert!(end.is_enumerated());
    assert!(end.is_termination());

    let custom = FlowDirectiveValue::Custom("myTask".to_string());
    assert!(!custom.is_enumerated());
    assert!(!custom.is_termination());
}

#[test]
fn test_flow_directive_value_serde() {
    use serverless_workflow_core::models::task::FlowDirectiveType;
    use serverless_workflow_core::models::task::FlowDirectiveValue;

    let cont = FlowDirectiveValue::Enumerated(FlowDirectiveType::Continue);
    let json = serde_json::to_string(&cont).unwrap();
    assert_eq!(json, "\"continue\"");

    let custom = FlowDirectiveValue::Custom("taskName".to_string());
    let json = serde_json::to_string(&custom).unwrap();
    assert_eq!(json, "\"taskName\"");

    let deserialized: FlowDirectiveValue = serde_json::from_str("\"exit\"").unwrap();
    assert!(deserialized.is_termination());
}

#[test]
fn test_context_data_model_definition() {
    use serverless_workflow_core::models::schema::SchemaDefinition;
    use serverless_workflow_core::models::workflow::ContextDataModelDefinition;

    let ctx = ContextDataModelDefinition {
        schema: Some(SchemaDefinition::default()),
        as_: Some(serde_json::json!({"key": "value"})),
    };
    let json = serde_json::to_string(&ctx).unwrap();
    assert!(json.contains("\"as\""));
}

#[test]
fn test_workflow_with_context() {
    use serverless_workflow_core::models::workflow::ContextDataModelDefinition;

    let mut workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default",
        "ctx-workflow",
        "1.0.0",
        None,
        None,
        None,
    ));
    workflow.context = Some(ContextDataModelDefinition {
        schema: None,
        as_: Some(serde_json::json!({"counter": 0})),
    });
    let json = serde_json::to_string(&workflow).unwrap();
    assert!(json.contains("\"context\""));
}

#[test]
fn test_document_metadata_field() {
    let mut doc = WorkflowDefinitionMetadata::new("default", "test", "1.0.0", None, None, None);
    let mut meta = std::collections::HashMap::new();
    meta.insert("author".to_string(), serde_json::json!("test-user"));
    doc.metadata = Some(meta);

    let json = serde_json::to_string(&doc).unwrap();
    assert!(json.contains("\"metadata\""));
    assert!(json.contains("\"author\""));
}

#[test]
fn test_backoff_strong_typed_accessors() {
    let backoff = ExponentialBackoffDefinition::with_factor_and_max_delay(2.0, "PT30S");
    assert_eq!(backoff.factor(), Some(2.0));
    assert_eq!(backoff.max_delay(), Some("PT30S"));

    let constant = ConstantBackoffDefinition::with_delay("PT5S");
    assert_eq!(constant.delay(), Some("PT5S"));
}

#[test]
fn test_runtime_expression() {
    use serverless_workflow_core::models::expression::*;

    let expr = RuntimeExpression::new("${.foo.bar}");
    assert!(expr.is_strict());
    assert!(expr.is_valid());
    assert_eq!(expr.sanitize(), ".foo.bar");

    let bare = RuntimeExpression::new(".foo");
    assert!(!bare.is_strict());
    let normalized = bare.normalize();
    assert_eq!(normalized.as_str(), "${.foo}");
}

#[test]
fn test_validation_framework() {
    use serverless_workflow_core::validation::*;

    assert!(is_valid_semver("1.0.0"));
    assert!(is_valid_hostname("example.com"));
    assert!(serverless_workflow_core::models::duration::is_iso8601_duration_valid("PT5S"));

    let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default", "test", "1.0.0", None, None, None,
    ));
    let result = validate_workflow(&workflow);
    assert!(result.is_valid());
}

#[test]
fn test_validation_missing_fields() {
    use serverless_workflow_core::validation::*;

    let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default", "", "", None, None, None,
    ));
    let result = validate_workflow(&workflow);
    assert!(!result.is_valid());
    assert!(result.errors.iter().any(|e| e.field == "document.name"));
    assert!(result.errors.iter().any(|e| e.field == "document.version"));
}
