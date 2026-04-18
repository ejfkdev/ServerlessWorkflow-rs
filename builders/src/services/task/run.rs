use super::*;

// ============== RunTaskDefinitionBuilder ==============
/// Internal enum tracking which process type variant is being built for a run task.
pub(crate) enum ProcessDefinitionBuilder {
    /// Container process builder.
    Container(Box<ContainerProcessDefinitionBuilder>),
    /// Script process builder.
    Script(Box<ScriptProcessDefinitionBuilder>),
    /// Shell command process builder.
    Shell(Box<ShellProcessDefinitionBuilder>),
    /// Sub-workflow process builder.
    Workflow(Box<WorkflowProcessDefinitionBuilder>),
}

/// Builder for constructing a run task that executes a container, script, shell command, or sub-workflow.
#[derive(Default)]
pub struct RunTaskDefinitionBuilder {
    task: RunTaskDefinition,
    builder: Option<ProcessDefinitionBuilder>,
}

/// Macro to generate process variant methods for RunTaskDefinitionBuilder.
macro_rules! process_variant_method {
    ($method:ident, $variant:ident, $builder:ident) => {
        pub fn $method(&mut self) -> &mut $builder {
            self.builder = Some(ProcessDefinitionBuilder::$variant(Box::default()));
            match &mut self.builder {
                Some(ProcessDefinitionBuilder::$variant(ref mut builder)) => builder,
                _ => unreachable!(concat!("Builder should always be set to ", stringify!($variant))),
            }
        }
    };
}

impl RunTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    process_variant_method!(container, Container, ContainerProcessDefinitionBuilder);
    process_variant_method!(script, Script, ScriptProcessDefinitionBuilder);
    process_variant_method!(shell, Shell, ShellProcessDefinitionBuilder);
    process_variant_method!(workflow, Workflow, WorkflowProcessDefinitionBuilder);
}

impl TaskDefinitionBuilderBase for RunTaskDefinitionBuilder {
    fn if_(&mut self, condition: &str) -> &mut Self {
        self.task.common.if_ = Some(condition.to_string());
        self
    }

    fn with_timeout_reference(&mut self, reference: &str) -> &mut Self {
        self.task.common.timeout = Some(OneOfTimeoutDefinitionOrReference::Reference(
            reference.to_string(),
        ));
        self
    }

    fn with_timeout<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TimeoutDefinitionBuilder),
    {
        let mut builder = TimeoutDefinitionBuilder::new();
        setup(&mut builder);
        let timeout = builder.build();
        self.task.common.timeout = Some(OneOfTimeoutDefinitionOrReference::Timeout(timeout));
        self
    }

    fn with_input<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut InputDataModelDefinitionBuilder),
    {
        let mut builder = InputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.input = Some(builder.build());
        self
    }

    fn with_output<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut OutputDataModelDefinitionBuilder),
    {
        let mut builder = OutputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.output = Some(builder.build());
        self
    }

    fn with_export<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut OutputDataModelDefinitionBuilder),
    {
        let mut builder = OutputDataModelDefinitionBuilder::new();
        setup(&mut builder);
        self.task.common.export = Some(builder.build());
        self
    }

    fn then(&mut self, directive: &str) -> &mut Self {
        self.task.common.then = Some(directive.to_string());
        self
    }

    fn build(mut self) -> TaskDefinition {
        if let Some(builder) = self.builder.take() {
            match builder {
                ProcessDefinitionBuilder::Container(b) => self.task.run.container = Some(b.build()),
                ProcessDefinitionBuilder::Script(b) => self.task.run.script = Some(b.build()),
                ProcessDefinitionBuilder::Shell(b) => self.task.run.shell = Some(b.build()),
                ProcessDefinitionBuilder::Workflow(b) => self.task.run.workflow = Some(b.build()),
            }
        }
        TaskDefinition::Run(Box::new(self.task))
    }
}

/// Builder for constructing a container process definition.
pub struct ContainerProcessDefinitionBuilder {
    container: ContainerProcessDefinition,
}

impl ContainerProcessDefinitionBuilder {
    fn new() -> Self {
        Self {
            container: ContainerProcessDefinition::default(),
        }
    }

    /// Sets the container image to run.
    pub fn with_image(&mut self, image: &str) -> &mut Self {
        self.container.image = image.to_string();
        self
    }

    /// Sets the command to execute inside the container.
    pub fn with_command(&mut self, command: &str) -> &mut Self {
        self.container.command = Some(command.to_string());
        self
    }

    /// Sets the port mappings for the container.
    pub fn with_ports(&mut self, ports: HashMap<String, serde_json::Value>) -> &mut Self {
        self.container.ports = Some(ports);
        self
    }

    /// Sets the volume mounts for the container.
    pub fn with_volumes(&mut self, volumes: HashMap<String, serde_json::Value>) -> &mut Self {
        self.container.volumes = Some(volumes);
        self
    }

    /// Replaces all environment variables with the provided map.
    pub fn with_environment_variables(&mut self, env: HashMap<String, String>) -> &mut Self {
        self.container.environment = Some(env);
        self
    }

    /// Adds a single environment variable to the container.
    pub fn with_env(&mut self, key: &str, value: &str) -> &mut Self {
        self.container
            .environment
            .get_or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Builds the `ContainerProcessDefinition`.
    pub fn build(self) -> ContainerProcessDefinition {
        self.container
    }
}

impl Default for ContainerProcessDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a script process definition.
pub struct ScriptProcessDefinitionBuilder {
    script: ScriptProcessDefinition,
}

impl ScriptProcessDefinitionBuilder {
    fn new() -> Self {
        Self {
            script: ScriptProcessDefinition::default(),
        }
    }

    /// Sets the script source code to execute.
    pub fn with_code(&mut self, code: &str) -> &mut Self {
        self.script.code = Some(code.to_string());
        self
    }

    /// Builds the `ScriptProcessDefinition`.
    pub fn build(self) -> ScriptProcessDefinition {
        self.script
    }
}

impl Default for ScriptProcessDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a shell process definition.
pub struct ShellProcessDefinitionBuilder {
    shell: ShellProcessDefinition,
}

impl ShellProcessDefinitionBuilder {
    fn new() -> Self {
        Self {
            shell: ShellProcessDefinition::default(),
        }
    }

    /// Sets the shell command to execute.
    pub fn with_command(&mut self, command: &str) -> &mut Self {
        self.shell.command = command.to_string();
        self
    }

    /// Sets the shell command arguments.
    pub fn with_arguments(&mut self, arguments: OneOfRunArguments) -> &mut Self {
        self.shell.arguments = Some(arguments);
        self
    }

    /// Sets the shell command arguments as a key-value map.
    pub fn with_map_arguments(&mut self, args: HashMap<String, Value>) -> &mut Self {
        self.shell.arguments = Some(OneOfRunArguments::Map(args));
        self
    }

    /// Sets the shell command arguments as an ordered array of strings.
    pub fn with_array_arguments(&mut self, args: Vec<String>) -> &mut Self {
        self.shell.arguments = Some(OneOfRunArguments::Array(args));
        self
    }

    /// Builds the `ShellProcessDefinition`.
    pub fn build(self) -> ShellProcessDefinition {
        self.shell
    }
}

impl Default for ShellProcessDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a sub-workflow process definition.
pub struct WorkflowProcessDefinitionBuilder {
    workflow: WorkflowProcessDefinition,
}

impl WorkflowProcessDefinitionBuilder {
    fn new() -> Self {
        Self {
            workflow: WorkflowProcessDefinition::default(),
        }
    }

    /// Sets the namespace of the target workflow.
    pub fn with_namespace(&mut self, namespace: &str) -> &mut Self {
        self.workflow.namespace = namespace.to_string();
        self
    }

    /// Sets the name of the target workflow.
    pub fn with_name(&mut self, name: &str) -> &mut Self {
        self.workflow.name = name.to_string();
        self
    }

    /// Sets the version of the target workflow.
    pub fn with_version(&mut self, version: &str) -> &mut Self {
        self.workflow.version = version.to_string();
        self
    }

    /// Sets the input data for the target workflow.
    pub fn with_input(&mut self, input: Value) -> &mut Self {
        self.workflow.input = Some(input);
        self
    }

    /// Builds the `WorkflowProcessDefinition`.
    pub fn build(self) -> WorkflowProcessDefinition {
        self.workflow
    }
}

impl Default for WorkflowProcessDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
