mod call_runner;
mod custom_runner;
mod do_runner;
mod emit_runner;
mod for_runner;
mod fork_runner;
mod listen_runner;
mod raise_runner;
mod run_runner;
mod set_runner;
mod switch_runner;
mod try_runner;
mod wait_runner;

pub(crate) use call_runner::CallTaskRunner;
pub(crate) use custom_runner::CustomTaskRunner;
pub(crate) use do_runner::DoTaskRunner;
pub(crate) use emit_runner::EmitTaskRunner;
pub(crate) use for_runner::ForTaskRunner;
pub(crate) use fork_runner::ForkTaskRunner;
pub(crate) use listen_runner::ListenTaskRunner;
pub(crate) use raise_runner::RaiseTaskRunner;
pub(crate) use run_runner::RunTaskRunner;
pub(crate) use set_runner::SetTaskRunner;
pub(crate) use switch_runner::SwitchTaskRunner;
pub(crate) use try_runner::TryTaskRunner;
pub(crate) use wait_runner::WaitTaskRunner;

/// Macro to define a simple task runner with the standard `name + task` struct pattern.
/// Generates the struct and `new()`.
/// Note: Each runner must still provide its own `impl TaskRunner` with `task_name()` and `run()`.
macro_rules! define_simple_task_runner {
    ($( #[$meta:meta] )* $runner:ident, $task_ty:ty) => {
        $( #[$meta] )*
        pub struct $runner {
            name: String,
            task: $task_ty,
        }

        impl $runner {
            pub fn new(name: &str, task: &$task_ty) -> crate::error::WorkflowResult<Self> {
                Ok(Self {
                    name: name.to_string(),
                    task: task.clone(),
                })
            }
        }
    };
}

/// Generates the `task_name()` method for a TaskRunner impl.
/// Use inside `impl TaskRunner for X { ... }` to avoid repeating `fn task_name() -> &str { &self.name }`.
macro_rules! task_name_impl {
    () => {
        fn task_name(&self) -> &str {
            &self.name
        }
    };
}

/// Creates default WorkflowContext and TaskSupport from a workflow reference.
/// Use in tests to avoid repeating the 2-line initialization pattern.
#[macro_export]
macro_rules! default_support {
    ($workflow:expr, $context:ident, $support:ident) => {
        let mut $context = $crate::context::WorkflowContext::new(&$workflow).unwrap();
        let mut $support = $crate::task_runner::TaskSupport::new(&$workflow, &mut $context);
    };
}

pub(crate) use define_simple_task_runner;
pub(crate) use task_name_impl;
