use super::*;

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
