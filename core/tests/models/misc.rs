use super::*;

#[test]
fn test_roundtrip_serialization() {
    let for_loop = ForLoopDefinition::new("item", ".collection", None, None);
    let mut do_tasks = Map::new();
    do_tasks.add(
        "task1".to_string(),
        TaskDefinition::Call(Box::new(CallTaskDefinition::Function(
            swf_core::models::call::CallFunctionDefinition {
                call: "someFunction".to_string(),
                with: None,
                common: swf_core::models::task::TaskDefinitionFields::default(),
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
fn test_oauth2_authentication_with_grant() {
    use swf_core::models::authentication::{
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
    use swf_core::models::authentication::{
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
fn test_try_task_definition() {
    use swf_core::models::task::TryTaskDefinition;
    let try_task = TryTaskDefinition {
        try_: Map::new(),
        ..Default::default()
    };
    let json_str = serde_json::to_string(&try_task).expect("Failed to serialize");
    assert!(json_str.contains("do") || json_str.contains("try"));
}

#[test]
fn test_document_with_tags_and_metadata() {
    // Test Document/WorkflowDefinitionMetadata with tags and metadata (similar to Go SDK TestDocument_JSONMarshal)
    use std::collections::HashMap;
    use swf_core::models::workflow::WorkflowDefinitionMetadata;

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

    let result: Result<swf_core::models::authentication::OAuth2AuthenticationSchemeDefinition, _> =
        serde_json::from_value(oauth2_json);
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
fn test_schedule_with_cron() {
    // Test schedule with cron expression
    let schedule_json = json!({
        "every": {
            "seconds": 10
        },
        "cron": "0 0 * * * *"
    });

    let result: Result<swf_core::models::workflow::WorkflowScheduleDefinition, _> =
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

    let result: Result<swf_core::models::workflow::WorkflowScheduleDefinition, _> =
        serde_json::from_value(schedule_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize event-driven schedule: {:?}",
        result.err()
    );
}

#[test]
fn test_flow_directive_constants() {
    assert_eq!(FlowDirective::CONTINUE, "continue");
    assert_eq!(FlowDirective::EXIT, "exit");
    assert_eq!(FlowDirective::END, "end");
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
fn test_oauth2_endpoints_defaults() {
    use swf_core::models::authentication::OAuth2AuthenticationEndpointsDefinition;

    let json_str = r#"{}"#;
    let endpoints: OAuth2AuthenticationEndpointsDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize");
    assert_eq!(endpoints.token, "/oauth2/token");
    assert_eq!(endpoints.revocation, "/oauth2/revoke");
    assert_eq!(endpoints.introspection, "/oauth2/introspect");
}

#[test]
fn test_oauth2_request_encoding_default() {
    use swf_core::models::authentication::OAuth2AuthenticationRequestDefinition;

    let json_str = r#"{}"#;
    let request: OAuth2AuthenticationRequestDefinition =
        serde_json::from_str(json_str).expect("Failed to deserialize");
    assert_eq!(request.encoding, "application/x-www-form-urlencoded");
}

// ============== New tests for P0/P1 features ==============

#[test]
fn test_oauth2_token_definition_serde() {
    use swf_core::models::authentication::OAuth2TokenDefinition;
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
fn test_runtime_expression_evaluation_mode() {
    use swf_core::models::workflow::RuntimeExpressionEvaluationMode;
    assert_eq!(RuntimeExpressionEvaluationMode::STRICT, "strict");
    assert_eq!(RuntimeExpressionEvaluationMode::LOOSE, "loose");
}

#[test]
fn test_runtime_expressions_constants() {
    use swf_core::models::workflow::RuntimeExpressions;
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
    use swf_core::models::workflow::RuntimeExpressionEvaluationConfiguration;
    let json_str = r#"{"language": "jq", "mode": "strict"}"#;
    let config: RuntimeExpressionEvaluationConfiguration =
        serde_json::from_str(json_str).expect("Failed to deserialize");
    assert_eq!(config.language, "jq");
    assert_eq!(config.mode, Some("strict".to_string()));
}

#[test]
fn test_flow_directive_value_enumerated() {
    use swf_core::models::task::FlowDirectiveType;
    use swf_core::models::task::FlowDirectiveValue;

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
    use swf_core::models::task::FlowDirectiveType;
    use swf_core::models::task::FlowDirectiveValue;

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
    use swf_core::models::schema::SchemaDefinition;
    use swf_core::models::workflow::ContextDataModelDefinition;

    let ctx = ContextDataModelDefinition {
        schema: Some(SchemaDefinition::default()),
        as_: Some(serde_json::json!({"key": "value"})),
    };
    let json = serde_json::to_string(&ctx).unwrap();
    assert!(json.contains("\"as\""));
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
fn test_runtime_expression() {
    use swf_core::models::expression::*;

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
    use swf_core::validation::*;

    assert!(is_valid_semver("1.0.0"));
    assert!(is_valid_hostname("example.com"));
    assert!(swf_core::models::duration::is_iso8601_duration_valid(
        "PT5S"
    ));

    let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default", "test", "1.0.0", None, None, None,
    ));
    let result = validate_workflow(&workflow);
    assert!(result.is_valid());
}

#[test]
fn test_validation_missing_fields() {
    use swf_core::validation::*;

    let workflow = WorkflowDefinition::new(WorkflowDefinitionMetadata::new(
        "default", "", "", None, None, None,
    ));
    let result = validate_workflow(&workflow);
    assert!(!result.is_valid());
    assert!(result.errors.iter().any(|e| e.field == "document.name"));
    assert!(result.errors.iter().any(|e| e.field == "document.version"));
}
