use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::TaskDefinitionFields;
use crate::models::resource::ExternalResourceDefinition;

/// Represents the configuration of a task used to run a given process
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunTaskDefinition {
    /// Gets/sets the configuration of the process to execute
    #[serde(rename = "run")]
    pub run: ProcessTypeDefinition,

    /// Gets/sets the task's common fields
    #[serde(flatten)]
    pub common: TaskDefinitionFields,
}
impl RunTaskDefinition {
    /// Initializes a new RunTaskDefinition
    pub fn new(run: ProcessTypeDefinition) -> Self {
        Self {
            run,
            common: TaskDefinitionFields::new(),
        }
    }
}

/// Represents the configuration of a process execution
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessTypeDefinition {
    /// Gets/sets the configuration of the container to run
    #[serde(rename = "container", skip_serializing_if = "Option::is_none")]
    pub container: Option<ContainerProcessDefinition>,

    /// Gets/sets the configuration of the script to run
    #[serde(rename = "script", skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptProcessDefinition>,

    /// Gets/sets the configuration of the shell command to run
    #[serde(rename = "shell", skip_serializing_if = "Option::is_none")]
    pub shell: Option<ShellProcessDefinition>,

    /// Gets/sets the configuration of the workflow to run
    #[serde(rename = "workflow", skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowProcessDefinition>,

    /// Gets/sets a boolean indicating whether or not to await the process completion before continuing. Defaults to 'true'
    #[serde(rename = "await", skip_serializing_if = "Option::is_none")]
    pub await_: Option<bool>,

    /// Gets/sets the configuration of the process output. Defaults to 'stdout'.
    /// Possible values: 'stdout', 'stderr', 'code', 'all', 'none'
    #[serde(rename = "return", skip_serializing_if = "Option::is_none")]
    pub return_: Option<String>,
}
impl ProcessTypeDefinition {
    /// Creates a new container process
    pub fn using_container(container: ContainerProcessDefinition, await_: Option<bool>) -> Self {
        Self {
            container: Some(container),
            await_,
            ..Self::default()
        }
    }

    /// Creates a new script process
    pub fn using_script(script: ScriptProcessDefinition, await_: Option<bool>) -> Self {
        Self {
            script: Some(script),
            await_,
            ..Self::default()
        }
    }

    /// Creates a new shell process
    pub fn using_shell(shell: ShellProcessDefinition, await_: Option<bool>) -> Self {
        Self {
            shell: Some(shell),
            await_,
            ..Self::default()
        }
    }

    /// Creates a new workflow process
    pub fn using_workflow(workflow: WorkflowProcessDefinition, await_: Option<bool>) -> Self {
        Self {
            workflow: Some(workflow),
            await_,
            ..Self::default()
        }
    }

    /// Gets the type of the defined process, or None if no process is configured
    pub fn get_process_type(&self) -> Option<&str> {
        if self.container.is_some() {
            Some(super::constants::ProcessType::CONTAINER)
        } else if self.script.is_some() {
            Some(super::constants::ProcessType::SCRIPT)
        } else if self.shell.is_some() {
            Some(super::constants::ProcessType::SHELL)
        } else if self.workflow.is_some() {
            Some(super::constants::ProcessType::WORKFLOW)
        } else {
            None
        }
    }
}

/// Represents the configuration of a container's lifetime
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerLifetimeDefinition {
    /// Gets/sets the container cleanup policy to use
    #[serde(rename = "cleanup")]
    pub cleanup: String,

    /// Gets/sets the duration after which to cleanup the container, in case the cleanup policy has been set to 'eventually'
    #[serde(rename = "after", skip_serializing_if = "Option::is_none")]
    pub after: Option<crate::models::duration::OneOfDurationOrIso8601Expression>,
}

/// Represents the configuration of a container process
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerProcessDefinition {
    /// Gets/sets the name of the container image to run
    #[serde(rename = "image")]
    pub image: String,

    /// Gets/sets the name of the container to run
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Gets/sets the command, if any, to execute on the container
    #[serde(rename = "command", skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Gets/sets a list containing the container's port mappings, if any
    /// Keys and values can be strings or numbers (matches Go SDK's `map[string]interface{}`)
    #[serde(rename = "ports", skip_serializing_if = "Option::is_none")]
    pub ports: Option<HashMap<String, Value>>,

    /// Gets/sets the volume mapping for the container, if any
    /// Matches Go SDK's `map[string]interface{}` which allows both string and object values
    #[serde(rename = "volumes", skip_serializing_if = "Option::is_none")]
    pub volumes: Option<HashMap<String, Value>>,

    /// Gets/sets a key/value mapping of the environment variables, if any, to use when running the configured process
    #[serde(rename = "environment", skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, String>>,

    /// Gets/sets the data to pass to the process via stdin, if any
    #[serde(rename = "stdin", skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,

    /// Gets/sets a list of arguments, if any, to pass to the container (argv)
    #[serde(rename = "arguments", skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<String>>,

    /// Gets/sets the policy that controls how the container's image should be pulled from the registry
    #[serde(rename = "pullPolicy", skip_serializing_if = "Option::is_none")]
    pub pull_policy: Option<String>,

    /// Gets/sets the configuration of the container's lifetime
    #[serde(rename = "lifetime", skip_serializing_if = "Option::is_none")]
    pub lifetime: Option<ContainerLifetimeDefinition>,
}

/// Represents a value that can be either a list of arguments (array) or a key/value mapping (object).
/// Matches Go SDK's RunArguments type which supports both forms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOfRunArguments {
    /// A list of string arguments (e.g., ["arg1=value1", "arg2"])
    Array(Vec<String>),
    /// A key/value mapping of arguments (e.g., {"arg1": "value1"})
    Map(HashMap<String, Value>),
}

impl Default for OneOfRunArguments {
    fn default() -> Self {
        OneOfRunArguments::Array(Vec::new())
    }
}

impl OneOfRunArguments {
    /// Returns the arguments as a slice if this is an Array variant
    pub fn as_array(&self) -> Option<&[String]> {
        match self {
            OneOfRunArguments::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Returns the arguments as a HashMap if this is a Map variant
    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        match self {
            OneOfRunArguments::Map(map) => Some(map),
            _ => None,
        }
    }

    /// Returns true if this is an Array variant
    pub fn is_array(&self) -> bool {
        matches!(self, OneOfRunArguments::Array(_))
    }

    /// Returns true if this is a Map variant
    pub fn is_map(&self) -> bool {
        matches!(self, OneOfRunArguments::Map(_))
    }
}

/// Represents the definition of a script evaluation process
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptProcessDefinition {
    /// Gets/sets the language of the script to run
    #[serde(rename = "language")]
    pub language: String,

    /// Gets/sets the script's code. Required if 'source' has not been set
    #[serde(rename = "code", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Gets the the script's source. Required if 'code' has not been set
    #[serde(rename = "source", skip_serializing_if = "Option::is_none")]
    pub source: Option<ExternalResourceDefinition>,

    /// Gets/sets the data to pass to the process via stdin
    #[serde(rename = "stdin", skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,

    /// Gets/sets the arguments to pass to the script. Can be either a list of strings (argv) or a key/value mapping.
    #[serde(rename = "arguments", skip_serializing_if = "Option::is_none")]
    pub arguments: Option<OneOfRunArguments>,

    /// Gets/sets a key/value mapping of the environment variables, if any, to use when running the configured process
    #[serde(rename = "environment", skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, String>>,
}
impl ScriptProcessDefinition {
    /// Initializes a new script from code
    pub fn from_code(
        language: &str,
        code: String,
        stdin: Option<String>,
        arguments: Option<OneOfRunArguments>,
        environment: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            language: language.to_string(),
            code: Some(code),
            source: None,
            stdin,
            arguments,
            environment,
        }
    }

    /// Initializes a new script from an external resource
    pub fn from_source(
        language: &str,
        source: ExternalResourceDefinition,
        stdin: Option<String>,
        arguments: Option<OneOfRunArguments>,
        environment: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            language: language.to_string(),
            code: None,
            source: Some(source),
            stdin,
            arguments,
            environment,
        }
    }
}

/// Represents the definition of a shell process
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShellProcessDefinition {
    /// Gets/sets the shell command to run
    #[serde(rename = "command")]
    pub command: String,

    /// Gets/sets the arguments of the shell command to run (supports both array and map forms, matches Go SDK's RunArguments)
    #[serde(rename = "arguments", skip_serializing_if = "Option::is_none")]
    pub arguments: Option<OneOfRunArguments>,

    /// Gets/sets a key/value mapping of the environment variables, if any, to use when running the configured process
    #[serde(rename = "environment", skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, String>>,
}
impl ShellProcessDefinition {
    pub fn new(
        command: &str,
        arguments: Option<OneOfRunArguments>,
        environment: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            command: command.to_string(),
            arguments,
            environment,
        }
    }
}

/// Represents the definition of a (sub)workflow process
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowProcessDefinition {
    /// Gets/sets the namespace the workflow to run belongs to
    #[serde(rename = "namespace")]
    pub namespace: String,

    /// Gets/sets the name of the workflow to run
    #[serde(rename = "name")]
    pub name: String,

    /// Gets/sets the version of the workflow to run
    #[serde(rename = "version")]
    pub version: String,

    /// Gets/sets the data, if any, to pass as input to the workflow to execute. The value should be validated against the target workflow's input schema, if specified
    #[serde(rename = "input", skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
}
impl WorkflowProcessDefinition {
    pub fn new(namespace: &str, name: &str, version: &str, input: Option<Value>) -> Self {
        Self {
            namespace: namespace.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            input,
        }
    }
}
