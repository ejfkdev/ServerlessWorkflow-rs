#![doc = include_str!("../README.md")]

pub mod services;

// Re-export the main builder type for convenience
pub use services::workflow::WorkflowBuilder;

#[cfg(test)]
mod unit_tests {

    use crate::services::workflow::WorkflowBuilder;
    use serde_json::json;
    use serde_json::Value;
    use serverless_workflow_core::models::duration::*;
    use serverless_workflow_core::models::error::OneOfErrorDefinitionOrReference;
    use serverless_workflow_core::models::task::*;
    use serverless_workflow_core::models::timeout::*;
    use std::collections::HashMap;

    #[test]
    fn build_workflow_should_work() {
        // Minimal test - fully chained
        let workflow = WorkflowBuilder::new().use_dsl("1.0.0").build();
        assert_eq!(workflow.document.dsl, "1.0.0");
    }

    #[test]
    fn build_workflow_should_work_full() {
        //arrange
        let dsl_version = "1.0.0";
        let namespace = "namespace";
        let name = "fake-name";
        let version = "1.0.0";
        let title = "fake-title";
        let summary = "fake-summary";
        let tags: HashMap<String, String> = vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]
        .into_iter()
        .collect();
        let timeout_duration = Duration {
            minutes: Some(69),
            ..Duration::default()
        };
        let basic_name = "fake-basic";
        let username = "fake-username";
        let password = "fake-password";
        let call_task_name = "call-task";
        let call_function_name = "fake-function";
        let call_task_with: HashMap<String, Value> = vec![
            ("key1".to_string(), Value::String("value1".to_string())),
            ("key2".to_string(), Value::String("value2".to_string())),
        ]
        .into_iter()
        .collect();
        let do_task_name = "do-task";
        let emit_task_name = "emit-task";
        let emit_event_attributes: HashMap<String, Value> = vec![
            ("key1".to_string(), Value::String("value1".to_string())),
            ("key2".to_string(), Value::String("value2".to_string())),
        ]
        .into_iter()
        .collect();
        let for_task_name = "for-task";
        let for_each = "item";
        let for_each_in = "items";
        let for_each_at = "index";
        let fork_task_name = "fork-task";
        let listen_task_name = "listen-task";
        let raise_task_name = "raise-task-name";
        let raise_error_type = "error-type";
        let raise_error_status = json!(400);
        let raise_error_title = "error-title";
        let raise_error_detail = "error-detail";
        let raise_error_instance = "error-instance";
        let run_container_task_name = "run-container-task-name";
        let container_image = "container-image-name";
        let container_command = "container-command";
        let container_ports: HashMap<String, serde_json::Value> = vec![
            ("8080".to_string(), json!(8081)),
            ("8082".to_string(), json!(8083)),
        ]
        .into_iter()
        .collect();
        let container_volumes: HashMap<String, serde_json::Value> =
            vec![("volume-1".to_string(), json!("/some/fake/path"))]
                .into_iter()
                .collect();
        let container_environment: HashMap<String, String> = vec![
            ("env1-name".to_string(), "env1-value".to_string()),
            ("env2-name".to_string(), "env2-value".to_string()),
        ]
        .into_iter()
        .collect();
        let run_script_task_name = "run-script-task-name";
        let script_code = "script-code";
        let run_shell_task_name = "run-shell-task-name";
        let shell_command_name = "run-shell-command";
        let run_workflow_task_name = "run-workflow-task-name";
        let workflow_namespace = "workflow-namespace";
        let workflow_name = "workflow-name";
        let workflow_version = "workflow-version";
        let workflow_input = json!({"hello": "world"});
        let set_task_name = "set-task-name";
        let set_task_variables: HashMap<String, Value> = vec![
            ("var1-name".to_string(), json!("var1-value".to_string())),
            ("var2-name".to_string(), json!(69)),
        ]
        .into_iter()
        .collect();
        let switch_task_name = "switch-task-name";
        let switch_case_name = "switch-case-name";
        let switch_case_when = "true";
        let switch_case_then = "continue";
        let try_task_name = "try-task-name";
        let catch_when = "catch-when";
        let catch_error_type = "https://serverlessworkflow.io/spec/1.0.0/errors/communication";
        let catch_error_status = json!(500);
        let retry_except_when = "retry-except-when";
        let wait_task_name = "wait-task";
        let wait_duration = OneOfDurationOrIso8601Expression::Duration(Duration::from_days(3));

        //act
        let workflow = WorkflowBuilder::new()
            .use_dsl(dsl_version)
            .with_namespace(namespace)
            .with_name(name)
            .with_version(version)
            .with_title(title)
            .with_summary(summary)
            .with_tags(tags.clone())
            .with_timeout(|t| {
                t.after(timeout_duration.clone());
            })
            .use_authentication(basic_name, |a| {
                a.basic().with_username(username).with_password(password);
            })
            .do_(call_task_name, |task| {
                task.call(call_function_name)
                    .with_arguments(call_task_with.clone());
            })
            .do_(do_task_name, |task| {
                task.do_().do_("fake-wait-task", |st| {
                    st.wait(OneOfDurationOrIso8601Expression::Duration(
                        Duration::from_seconds(25),
                    ));
                });
            })
            .do_(emit_task_name, |task| {
                task.emit(|e| {
                    e.with_attributes(emit_event_attributes.clone());
                });
            })
            .do_(for_task_name, |task| {
                task.for_()
                    .each(for_each)
                    .in_(for_each_in)
                    .at(for_each_at)
                    .do_("fake-wait-task", |st| {
                        st.wait(OneOfDurationOrIso8601Expression::Duration(
                            Duration::from_seconds(25),
                        ));
                    });
            })
            .do_(fork_task_name, |task| {
                task.fork().branch(|b| {
                    b.do_().do_("fake-wait-task", |st| {
                        st.wait(OneOfDurationOrIso8601Expression::Duration(
                            Duration::from_seconds(25),
                        ));
                    });
                });
            })
            .do_(listen_task_name, |task| {
                task.listen().to(|e| {
                    e.with("key", Value::String("value".to_string()));
                });
            })
            .do_(raise_task_name, |task| {
                task.raise()
                    .error()
                    .with_type(raise_error_type)
                    .with_status(raise_error_status)
                    .with_title(raise_error_title)
                    .with_detail(raise_error_detail)
                    .with_instance(raise_error_instance);
            })
            // .do_(run_container_task_name, |task|{
            .do_(run_container_task_name, |task| {
                task.run()
                    .container()
                    .with_image(container_image)
                    .with_command(container_command)
                    .with_ports(container_ports.clone())
                    .with_volumes(container_volumes.clone())
                    .with_environment_variables(container_environment.clone());
            })
            .do_(run_script_task_name, |task| {
                task.run().script().with_code(script_code);
            })
            .do_(run_shell_task_name, |task| {
                task.run().shell().with_command(shell_command_name);
            })
            .do_(run_workflow_task_name, |task| {
                task.run()
                    .workflow()
                    .with_namespace(workflow_namespace)
                    .with_name(workflow_name)
                    .with_version(workflow_version)
                    .with_input(workflow_input.clone());
            })
            .do_(set_task_name, |task| {
                task.set().variables(set_task_variables.clone());
            })
            .do_(switch_task_name, |task| {
                task.switch().case_(switch_case_name, |case| {
                    case.when(switch_case_when).then(switch_case_then);
                });
            })
            .do_(try_task_name, |task| {
                task.try_()
                    .do_("fake-wait-task", |subtask| {
                        subtask.wait(OneOfDurationOrIso8601Expression::Duration(
                            Duration::from_seconds(5),
                        ));
                    })
                    .catch(|catch| {
                        catch
                            .errors(|errors| {
                                errors
                                    .with_type(catch_error_type)
                                    .with_status(catch_error_status.clone());
                            })
                            .when(catch_when)
                            .retry(|retry| {
                                retry
                                    .except_when(retry_except_when)
                                    .delay(Duration::from_seconds(1))
                                    .backoff(|backoff| {
                                        backoff
                                            .linear()
                                            .with_increment(Duration::from_milliseconds(500));
                                    })
                                    .jitter(|jitter| {
                                        jitter
                                            .from(Duration::from_seconds(1))
                                            .to(Duration::from_seconds(3));
                                    });
                            });
                    });
            })
            .do_(wait_task_name, |task| {
                task.wait(wait_duration.clone());
            })
            .build();

        //assert
        assert_eq!(workflow.document.dsl, dsl_version);
        assert_eq!(workflow.document.namespace, namespace);
        assert_eq!(workflow.document.name, name);
        assert_eq!(workflow.document.version, version);
        assert_eq!(workflow.document.title, Some(title.to_string()));
        assert_eq!(workflow.document.summary, Some(summary.to_string()));
        assert_eq!(workflow.document.tags, Some(tags));
        assert_eq!(
            workflow.timeout.as_ref().and_then(|t| match t {
                OneOfTimeoutDefinitionOrReference::Timeout(definition) => match &definition.after {
                    OneOfDurationOrIso8601Expression::Duration(duration) => Some(duration),
                    OneOfDurationOrIso8601Expression::Iso8601Expression(_) => None,
                },
                OneOfTimeoutDefinitionOrReference::Reference(_) => None,
            }),
            Some(&timeout_duration)
        );
        assert!(
            workflow.use_.as_ref()
                .and_then(|component_collection| component_collection.authentications.as_ref())
                .and_then(|authentications| authentications.get(basic_name))
                .map(|auth_policy| matches!(auth_policy, serverless_workflow_core::models::authentication::ReferenceableAuthenticationPolicy::Policy(p) if p.basic.is_some()))
                .unwrap_or(false),
            "Expected authentications to contain an entry with the name '{}' and a non-null `basic` property.",
            basic_name);
        assert!(
            workflow
                .do_
                .entries
                .iter()
                .any(|(name, task)| name == call_task_name && {
                    if let TaskDefinition::Call(call_def) = task {
                        if let serverless_workflow_core::models::call::CallTaskDefinition::Function(ref f) = call_def.as_ref() {
                            f.call == call_function_name && f.with == Some(call_task_with.clone())
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }),
            "Expected a task with key '{}' and a CallTaskDefinition with 'call'={} and 'with'={:?}",
            call_task_name,
            call_function_name,
            call_task_with
        );
        assert!(
            workflow
                .do_
                .entries
                .iter()
                .any(|(name, task)| name == do_task_name && matches!(task, TaskDefinition::Do(_))),
            "Expected a do task with key '{}'",
            do_task_name
        );
        assert!(
            workflow.do_
                .entries
                .iter()
                .any(|(name, task)| name == emit_task_name && {
                    if let TaskDefinition::Emit(emit_task) = task {
                        emit_task.emit.event.with == emit_event_attributes.clone()
                    } else {
                        false
                    }
                }),
            "Expected a task with key '{}' and a EmitTaskDefinition with 'emit.event.with' matching supplied attributes",
            emit_task_name);
        assert!(
            workflow.do_
                .entries
                .iter()
                .any(|(name, task)| name == for_task_name && {
                    if let TaskDefinition::For(for_task) = task {
                        for_task.for_.each == for_each && for_task.for_.in_ == for_each_in && for_task.for_.at == Some(for_each_at.to_string())
                    } else {
                        false
                    }
                }),
            "Expected a task with key '{}' and a ForTaskDefinition with 'for.each'={}, 'for.in'={}' and 'for.at'={}'",
            for_task_name,
            for_each,
            for_each_in,
            for_each_at);
        assert!(
            workflow
                .do_
                .entries
                .iter()
                .any(|(name, task)| name == fork_task_name
                    && matches!(task, TaskDefinition::Fork(_))),
            "Expected a fork task with key '{}'",
            fork_task_name,
        );
        assert!(
            workflow
                .do_
                .entries
                .iter()
                .any(|(name, task)| name == listen_task_name
                    && matches!(task, TaskDefinition::Listen(_))),
            "Expected a listen task with key '{}'",
            listen_task_name
        );
        assert!(
            workflow.do_
                .entries
                .iter()
                .any(|(name, task)| name == raise_task_name && {
                    if let TaskDefinition::Raise(raise_task) = task {
                        if let OneOfErrorDefinitionOrReference::Error(error) = &raise_task.raise.error {
                            error.type_.as_str() == raise_error_type
                                && error.title == Some(raise_error_title.to_string())
                                && error.detail == Some(raise_error_detail.to_string())
                                && error.instance == Some(raise_error_instance.to_string())
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }),
            "Expected a task with key '{}' and a RaiseTaskDefinition with 'raise.error.type'={}, 'raise.error.title'={}, 'raise.error.detail'={} and 'raise.error.instance'={}",
            raise_task_name,
            raise_error_type,
            raise_error_title,
            raise_error_detail,
            raise_error_instance);
        assert!(
            workflow
                .do_
                .entries
                .iter()
                .any(|(name, task)| name == run_container_task_name && {
                    if let TaskDefinition::Run(run_task) = task {
                        if let Some(container) = &run_task.run.container {
                            container.image == container_image
                                && container.command == Some(container_command.to_string())
                                && container.ports == Some(container_ports.clone())
                                && container.volumes == Some(container_volumes.clone())
                                && container.environment == Some(container_environment.clone())
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }),
            "Expected a task with key '{}' and a RunTaskDefinition with 'container.image'={}, 'container.command'={}, 'container.ports'={:?}, 'container.volumes'={:?}, and 'container.environment'={:?}",
            run_container_task_name,
            container_image,
            container_command,
            container_ports,
            container_volumes,
            container_environment);
        assert!(
            workflow
                .do_
                .entries
                .iter()
                .any(|(name, task)| name == run_workflow_task_name && {
                    if let TaskDefinition::Run(run_task) = task {
                        if let Some(subflow) = &run_task.run.workflow{
                            subflow.namespace == workflow_namespace
                                && subflow.name == workflow_name
                                && subflow.version == workflow_version
                                && subflow.input == Some(workflow_input.clone())
                        }
                        else{
                            false
                        }
                    } else {
                        false
                    }
                }),
            "Expected a task with key '{}' and a RunTaskDefinition with 'workflow.namespace'={}, 'workflow.name'={}, 'workflow.version'={}, and 'workflow.input'={:?}",
            run_workflow_task_name,
            workflow_namespace,
            workflow_name,
            workflow_version,
            workflow_input);
        assert!(
            workflow
                .do_
                .entries
                .iter()
                .any(|(name, task)| name == set_task_name && {
                    if let TaskDefinition::Set(set_task) = task {
                        match &set_task.set {
                            SetValue::Map(map) => map == &set_task_variables,
                            _ => false,
                        }
                    } else {
                        false
                    }
                }),
            "Expected a task with key '{}' and a SetTaskDefinition with specified variables",
            set_task_name
        );
        assert!(
            workflow
                .do_
                .entries
                .iter()
                .any(|(name, task)| name == switch_task_name && {
                    if let TaskDefinition::Switch(switch_task) = task {
                        switch_task
                            .switch
                            .entries
                            .iter()
                            .any(|(case_name, _)| case_name == switch_case_name)
                    } else {
                        false
                    }
                }),
            "Expected a task with key '{}' and a SwitchTaskDefinition with a case named '{}'",
            switch_task_name,
            switch_case_name
        );
        assert!(
            workflow
                .do_
                .entries
                .iter()
                .any(|(name, task)| name == try_task_name && {
                    if let TaskDefinition::Try(try_task) = task {
                        try_task.catch.when == Some(catch_when.to_string())
                    } else {
                        false
                    }
                }),
            "Expected a task with key '{}' and a TryTaskDefinition with 'catch.when'={}",
            try_task_name,
            catch_when
        );
        assert!(
            workflow
                .do_
                .entries
                .iter()
                .any(|(name, task)| name == wait_task_name && {
                    if let TaskDefinition::Wait(wait_task) = task {
                        wait_task.wait == wait_duration
                    } else {
                        false
                    }
                }),
            "Expected a task with key '{}' and a WaitTaskDefinition with 'wait'={}",
            wait_task_name,
            wait_duration
        );
    }
}
