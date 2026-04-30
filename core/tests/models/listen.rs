use super::*;

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

    let result: Result<swf_core::models::task::ListenTaskDefinition, _> =
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

    let result: Result<swf_core::models::task::ListenTaskDefinition, _> =
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

    let result: Result<swf_core::models::task::ListenTaskDefinition, _> =
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

    let result: Result<swf_core::models::task::ListenTaskDefinition, _> =
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
