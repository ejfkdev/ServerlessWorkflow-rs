use super::*;

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
