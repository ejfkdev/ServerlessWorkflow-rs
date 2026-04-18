use serde_json::Value;
use serverless_workflow_core::models::authentication::{
    AuthenticationPolicyReference, ReferenceableAuthenticationPolicy,
};
use serverless_workflow_core::models::call::CallTaskDefinition;
use serverless_workflow_core::models::call::{
    A2AArguments, AsyncApiArguments, CallA2ADefinition, CallAsyncAPIDefinition,
    CallFunctionDefinition, CallGRPCDefinition, CallHTTPDefinition, CallOpenAPIDefinition,
    GRPCArguments, GRPCServiceDefinition, HTTPArguments, OneOfA2AParametersOrExpression,
    OneOfHeadersOrExpression, OneOfQueryOrExpression, OpenAPIArguments,
};
use serverless_workflow_core::models::duration::*;
use serverless_workflow_core::models::error::ErrorType;
use serverless_workflow_core::models::error::*;
use serverless_workflow_core::models::event::*;
use serverless_workflow_core::models::input::*;
use serverless_workflow_core::models::output::*;
use serverless_workflow_core::models::resource::{
    ExternalResourceDefinition, OneOfEndpointDefinitionOrUri,
};
use serverless_workflow_core::models::retry::*;
use serverless_workflow_core::models::task::*;
use serverless_workflow_core::models::timeout::*;
use std::collections::HashMap;

use super::timeout::TimeoutDefinitionBuilder;

/// Macro to generate the `pub fn variant()` method for TaskDefinitionBuilder,
/// creating the builder, storing it, and returning a mutable reference.
macro_rules! task_variant_method {
    ($method:ident, $variant:ident, $builder:ident $(, $arg:ident: $arg_ty:ty)*) => {
        pub fn $method(&mut self $(, $arg: $arg_ty)*) -> &mut $builder {
            let builder = $builder::new($($arg),*);
            self.builder = Some(TaskBuilderVariant::$variant(Box::new(builder)));
            match &mut self.builder {
                Some(TaskBuilderVariant::$variant(ref mut builder)) => builder,
                _ => unreachable!(concat!("Builder should always be set to ", stringify!($variant))),
            }
        }
    };
}

/// Implements `TaskDefinitionBuilderBase` for a builder struct, generating
/// the 7 common methods (if_, with_timeout_reference, with_timeout,
/// with_input, with_output, with_export, then) that all delegate to
/// `self.$field.common`. The `build()` method is unique per builder
/// and must be provided via the `$build_expr` expression, which will
/// be evaluated in a context where `v` holds the inner field value.
macro_rules! impl_task_definition_builder_base {
    ($builder:ident, $field:ident, $build_expr:expr) => {
        impl TaskDefinitionBuilderBase for $builder {
            fn if_(&mut self, condition: &str) -> &mut Self {
                self.$field.common.if_ = Some(condition.to_string());
                self
            }

            fn with_timeout_reference(&mut self, reference: &str) -> &mut Self {
                self.$field.common.timeout = Some(OneOfTimeoutDefinitionOrReference::Reference(
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
                self.$field.common.timeout =
                    Some(OneOfTimeoutDefinitionOrReference::Timeout(timeout));
                self
            }

            fn with_input<F>(&mut self, setup: F) -> &mut Self
            where
                F: FnOnce(&mut InputDataModelDefinitionBuilder),
            {
                let mut builder = InputDataModelDefinitionBuilder::new();
                setup(&mut builder);
                self.$field.common.input = Some(builder.build());
                self
            }

            fn with_output<F>(&mut self, setup: F) -> &mut Self
            where
                F: FnOnce(&mut OutputDataModelDefinitionBuilder),
            {
                let mut builder = OutputDataModelDefinitionBuilder::new();
                setup(&mut builder);
                self.$field.common.output = Some(builder.build());
                self
            }

            fn with_export<F>(&mut self, setup: F) -> &mut Self
            where
                F: FnOnce(&mut OutputDataModelDefinitionBuilder),
            {
                let mut builder = OutputDataModelDefinitionBuilder::new();
                setup(&mut builder);
                self.$field.common.export = Some(builder.build());
                self
            }

            fn then(&mut self, directive: &str) -> &mut Self {
                self.$field.common.then = Some(directive.to_string());
                self
            }

            fn build(self) -> TaskDefinition {
                $build_expr(self.$field)
            }
        }
    };
}

mod call;
mod common;
mod do_task;
mod emit;
mod for_loop;
mod fork;
mod listen;
mod raise;
mod run;
mod set;
mod switch;
mod task_builder;
mod try_catch;
mod wait;

pub use call::*;
pub use common::*;
pub use do_task::*;
pub use emit::*;
pub use for_loop::*;
pub use fork::*;
pub use listen::*;
pub use raise::*;
pub use run::*;
pub use set::*;
pub use switch::*;
pub use task_builder::*;
pub use try_catch::*;
pub use wait::*;
