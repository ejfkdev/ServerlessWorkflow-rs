use crate::tasks::task_name_impl;
use crate::error::{WorkflowError, WorkflowResult};
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::tasks::{define_simple_task_runner};
use serde_json::Value;
use serverless_workflow_core::models::task::{
    OneOfRunArguments, RunTaskDefinition, ShellProcessDefinition, WorkflowProcessDefinition,
};

define_simple_task_runner!(
    /// Runner for Run tasks - executes processes (shell, container, script, workflow)
    RunTaskRunner, RunTaskDefinition
);

#[async_trait::async_trait]
impl TaskRunner for RunTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        let process = &self.task.run;

        if let Some(ref shell) = process.shell {
            return self.run_shell(shell, &input, support).await;
        }

        if let Some(ref container) = process.container {
            let config =
                crate::error::serialize_to_value(container, "container config", &self.name)?;
            return self
                .run_with_handler("container", &config, &input, support)
                .await;
        }

        if let Some(ref script) = process.script {
            let config = crate::error::serialize_to_value(script, "script config", &self.name)?;
            return self
                .run_with_handler("script", &config, &input, support)
                .await;
        }

        if let Some(ref workflow) = process.workflow {
            return self.run_workflow(workflow, &input, support).await;
        }

        Err(WorkflowError::validation(
            "no process configuration provided",
            &self.name,
        ))
    }

    task_name_impl!();
}

impl RunTaskRunner {
    /// Runs a process using a registered RunHandler, or returns an error if no handler is found
    async fn run_with_handler(
        &self,
        run_type: &str,
        config: &Value,
        input: &Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        let handler = support.get_handler_registry().get_run_handler(run_type);
        match handler {
            Some(handler) => handler.handle(&self.name, config, input).await,
            None => Err(WorkflowError::runtime_simple(
                format!("{} process requires a custom RunHandler (register one via WorkflowRunner::with_run_handler())", run_type),
                &self.name,
            )),
        }
    }

    async fn run_workflow(
        &self,
        workflow_def: &WorkflowProcessDefinition,
        input: &Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        // Look up the sub-workflow from the registry
        let child_workflow = support
            .context
            .get_sub_workflow(
                &workflow_def.namespace,
                &workflow_def.name,
                &workflow_def.version,
            )
            .ok_or_else(|| {
                WorkflowError::runtime_simple(
                    format!(
                        "sub-workflow '{}/{}/{}' not found in registry",
                        workflow_def.namespace, workflow_def.name, workflow_def.version
                    ),
                    &self.name,
                )
            })?;

        // Determine sub-workflow input: use workflow.input if provided, otherwise pass current input
        let sub_input = if let Some(ref workflow_input) = workflow_def.input {
            // Evaluate expressions in the input object
            support.eval_obj(Some(workflow_input), input, &self.name)?
        } else {
            input.clone()
        };

        // Create a child WorkflowRunner and execute the sub-workflow
        let child_runner = crate::runner::WorkflowRunner::new(child_workflow.clone())?;

        // Propagate secret manager and listener to child runner
        let mut child_runner = child_runner;
        if let Some(mgr) = support.context.clone_secret_manager() {
            child_runner = child_runner.with_secret_manager(mgr);
        }
        if let Some(listener) = support.context.clone_listener() {
            child_runner = child_runner.with_listener(listener);
        }
        // Propagate sub-workflow registry for nested sub-workflow support
        let sub_workflows = support.context.clone_sub_workflows();
        for (_, wf) in sub_workflows {
            child_runner = child_runner.with_sub_workflow(wf);
        }

        // Propagate handler registry for custom call/run handlers in sub-workflows
        // Since HandlerRegistry uses Arc internally, clone is cheap
        child_runner = child_runner.with_handler_registry(support.context.clone_handler_registry());

        child_runner.run(sub_input).await
    }

    async fn run_shell(
        &self,
        shell: &ShellProcessDefinition,
        input: &Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        // Evaluate command expression if needed
        let command = support.eval_str(&shell.command, input, &self.name)?;

        // Check await flag — if false, spawn and return null immediately
        let await_flag = self.task.run.await_.unwrap_or(true);
        if !await_flag {
            tokio::spawn(async move {
                let _ = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .output()
                    .await;
            });
            return Ok(Value::Null);
        }

        // Build the full command string including arguments
        let full_command = if let Some(ref args) = shell.arguments {
            let mut parts = vec![command.clone()];
            match args {
                OneOfRunArguments::Map(map) => {
                    for (key, value) in map {
                        // Evaluate the key as a potential expression
                        let evaluated_key = support.eval_str(key, input, &self.name)?;

                        // Value can be a string (literal or expression), or null (key-only positional arg)
                        match value {
                            Value::String(s) => {
                                // Evaluate the value as a potential expression
                                let evaluated_value =
                                    support.eval_str(s, input, &self.name)?;
                                // Combine key + value: e.g., "--user" + "john" → "--user john"
                                parts.push(shell_escape(&evaluated_key));
                                if !evaluated_value.is_empty() {
                                    parts.push(shell_escape(&evaluated_value));
                                }
                            }
                            Value::Null => {
                                // Key-only argument (positional): just add the evaluated key
                                parts.push(shell_escape(&evaluated_key));
                            }
                            _ => {
                                // Other value types: convert to string
                                let str_val = value.to_string();
                                parts.push(shell_escape(&evaluated_key));
                                parts.push(shell_escape(&str_val));
                            }
                        }
                    }
                }
                OneOfRunArguments::Array(arr) => {
                    for arg in arr {
                        let evaluated = support.eval_str(arg, input, &self.name)?;
                        parts.push(shell_escape(&evaluated));
                    }
                }
            }
            parts.join(" ")
        } else {
            command.clone()
        };

        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(&full_command);

        // Add environment variables
        if let Some(ref env) = shell.environment {
            for (key, value) in env {
                let evaluated_value = support.eval_str(value, input, &self.name)?;
                cmd.env(key, evaluated_value);
            }
        }

        // Set working directory from task metadata if available
        if let Some(ref metadata) = self.task.common.metadata {
            if let Some(dir) = metadata.get("workingDir").and_then(|v| v.as_str()) {
                cmd.current_dir(dir);
            }
        }

        let output = cmd.output().await.map_err(|e| {
            WorkflowError::runtime_simple(
                format!("failed to execute shell command '{}': {}", command, e),
                &self.name,
            )
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code();

        // Check for non-zero exit code
        if !output.status.success() {
            let return_type = self.task.run.return_.as_deref().unwrap_or("stdout");
            // Only error if return type is not explicitly set to capture the error
            if return_type != "all" && return_type != "code" && return_type != "stderr" {
                return Err(WorkflowError::runtime_simple(
                    format!(
                        "shell command '{}' failed with exit code {}: {}",
                        command,
                        code.unwrap_or(-1),
                        stderr.trim()
                    ),
                    &self.name,
                ));
            }
        }

        // Determine return format
        let return_type = self.task.run.return_.as_deref().unwrap_or("stdout");

        match return_type {
            "stdout" => {
                // Try to parse as JSON, otherwise return as string
                match serde_json::from_str::<Value>(stdout.trim()) {
                    Ok(v) => Ok(v),
                    Err(_) => Ok(Value::String(stdout)),
                }
            }
            "stderr" => Ok(Value::String(stderr)),
            "code" => Ok(Value::from(code.unwrap_or(-1))),
            "all" => Ok(serde_json::json!({
                "code": code,
                "stdout": stdout,
                "stderr": stderr,
            })),
            "none" => Ok(Value::Null),
            _ => Ok(Value::String(stdout)),
        }
    }
}

/// Escapes a string for safe inclusion in a shell command
fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    // If the string contains only safe characters, no quoting needed
    let safe = s
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/' || c == ':');
    if safe {
        return s.to_string();
    }
    // Use single-quote escaping
    format!("'{}'", s.replace("'", "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::WorkflowContext;
    use crate::default_support;
    use serde_json::json;
    use serverless_workflow_core::models::task::{
        ProcessTypeDefinition, RunTaskDefinition, ShellProcessDefinition,
    };
    use serverless_workflow_core::models::workflow::WorkflowDefinition;
    use std::collections::HashMap;

    fn make_shell_task(command: &str) -> RunTaskDefinition {
        RunTaskDefinition {
            run: ProcessTypeDefinition {
                shell: Some(ShellProcessDefinition {
                    command: command.to_string(),
                    arguments: None,
                    environment: None,
                }),
                container: None,
                script: None,
                workflow: None,
                await_: None,
                return_: None,
            },
            common: TaskDefinitionFields::new(),
        }
    }

    use serverless_workflow_core::models::task::TaskDefinitionFields;

    #[tokio::test]
    async fn test_run_shell_echo() {
        let task = make_shell_task("echo hello");
        let runner = RunTaskRunner::new("shellTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output, Value::String("hello\n".to_string()));
    }

    #[tokio::test]
    async fn test_run_shell_json_output() {
        let task = make_shell_task("echo '{\"key\": \"value\"}'");
        let runner = RunTaskRunner::new("jsonTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output, json!({"key": "value"}));
    }

    #[tokio::test]
    async fn test_run_shell_failure() {
        let task = make_shell_task("exit 1");
        let runner = RunTaskRunner::new("failTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_shell_return_code() {
        let mut task = make_shell_task("exit 42");
        task.run.return_ = Some("code".to_string());
        let runner = RunTaskRunner::new("codeTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output, json!(42));
    }

    #[tokio::test]
    async fn test_run_shell_return_all() {
        let mut task = make_shell_task("echo stdout_output; echo stderr_output >&2");
        task.run.return_ = Some("all".to_string());
        let runner = RunTaskRunner::new("allTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["code"], json!(0));
        assert!(output["stdout"].as_str().unwrap().contains("stdout_output"));
    }

    #[tokio::test]
    async fn test_run_shell_with_env() {
        let mut task = make_shell_task("echo $MY_VAR");
        if let Some(ref mut shell) = task.run.shell {
            let mut env = HashMap::new();
            env.insert("MY_VAR".to_string(), "test_value".to_string());
            shell.environment = Some(env);
        }
        let runner = RunTaskRunner::new("envTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output, Value::String("test_value\n".to_string()));
    }

    #[tokio::test]
    async fn test_run_shell_with_expression() {
        // Command as a full JQ expression
        let task = make_shell_task("${ .cmd }");
        let runner = RunTaskRunner::new("exprTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"cmd": "echo hello"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output, Value::String("hello\n".to_string()));
    }

    #[tokio::test]
    async fn test_run_container_unsupported() {
        use serverless_workflow_core::models::task::ContainerProcessDefinition;

        let task = RunTaskDefinition {
            run: ProcessTypeDefinition {
                shell: None,
                container: Some(ContainerProcessDefinition::default()),
                script: None,
                workflow: None,
                await_: None,
                return_: None,
            },
            common: TaskDefinitionFields::new(),
        };
        let runner = RunTaskRunner::new("containerTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("custom RunHandler"));
    }

    #[tokio::test]
    async fn test_run_script_unsupported() {
        use serverless_workflow_core::models::task::ScriptProcessDefinition;

        let task = RunTaskDefinition {
            run: ProcessTypeDefinition {
                shell: None,
                container: None,
                script: Some(ScriptProcessDefinition::default()),
                workflow: None,
                await_: None,
                return_: None,
            },
            common: TaskDefinitionFields::new(),
        };
        let runner = RunTaskRunner::new("scriptTest", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("custom RunHandler"));
    }

    #[tokio::test]
    async fn test_run_container_with_custom_handler() {
        use crate::handler::RunHandler;
        use serverless_workflow_core::models::task::ContainerProcessDefinition;

        struct MockContainerHandler;

        #[async_trait::async_trait]
        impl RunHandler for MockContainerHandler {
            fn run_type(&self) -> &str {
                "container"
            }

            async fn handle(
                &self,
                _task_name: &str,
                _config: &Value,
                input: &Value,
            ) -> WorkflowResult<Value> {
                Ok(json!({"containerOutput": input}))
            }
        }

        let task = RunTaskDefinition {
            run: ProcessTypeDefinition {
                shell: None,
                container: Some(ContainerProcessDefinition::default()),
                script: None,
                workflow: None,
                await_: None,
                return_: None,
            },
            common: TaskDefinitionFields::new(),
        };
        let runner = RunTaskRunner::new("containerWithHandler", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut registry = crate::handler::HandlerRegistry::new();
        registry.register_run_handler(Box::new(MockContainerHandler));
        context.set_handler_registry(registry);
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner
            .run(json!({"data": "test"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["containerOutput"]["data"], json!("test"));
    }

    #[tokio::test]
    async fn test_run_script_with_custom_handler() {
        use crate::handler::RunHandler;
        use serverless_workflow_core::models::task::ScriptProcessDefinition;

        struct MockScriptHandler;

        #[async_trait::async_trait]
        impl RunHandler for MockScriptHandler {
            fn run_type(&self) -> &str {
                "script"
            }

            async fn handle(
                &self,
                _task_name: &str,
                _config: &Value,
                input: &Value,
            ) -> WorkflowResult<Value> {
                Ok(json!({"scriptResult": input}))
            }
        }

        let task = RunTaskDefinition {
            run: ProcessTypeDefinition {
                shell: None,
                container: None,
                script: Some(ScriptProcessDefinition::default()),
                workflow: None,
                await_: None,
                return_: None,
            },
            common: TaskDefinitionFields::new(),
        };
        let runner = RunTaskRunner::new("scriptWithHandler", &task).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        let mut registry = crate::handler::HandlerRegistry::new();
        registry.register_run_handler(Box::new(MockScriptHandler));
        context.set_handler_registry(registry);
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner
            .run(json!({"name": "world"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["scriptResult"]["name"], json!("world"));
    }
}
