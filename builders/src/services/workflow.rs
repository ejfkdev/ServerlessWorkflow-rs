use swf_core::models::authentication::ReferenceableAuthenticationPolicy;
use swf_core::models::timeout::*;
use swf_core::models::workflow::*;
use std::collections::HashMap;

use super::authentication::AuthenticationPolicyDefinitionBuilder;
use super::task::TaskDefinitionBuilder;
use super::timeout::TimeoutDefinitionBuilder;

/// Builder for WorkflowDefinition
pub struct WorkflowBuilder {
    workflow: WorkflowDefinition,
}

impl WorkflowBuilder {
    pub fn new() -> Self {
        Self {
            workflow: WorkflowDefinition::default(),
        }
    }

    pub fn use_dsl(&mut self, version: &str) -> &mut Self {
        self.workflow.document.dsl = version.to_string();
        self
    }

    pub fn with_namespace(&mut self, namespace: &str) -> &mut Self {
        self.workflow.document.namespace = namespace.to_string();
        self
    }

    pub fn with_name(&mut self, name: &str) -> &mut Self {
        self.workflow.document.name = name.to_string();
        self
    }

    pub fn with_version(&mut self, version: &str) -> &mut Self {
        self.workflow.document.version = version.to_string();
        self
    }

    pub fn with_title(&mut self, title: &str) -> &mut Self {
        self.workflow.document.title = Some(title.to_string());
        self
    }

    pub fn with_summary(&mut self, summary: &str) -> &mut Self {
        self.workflow.document.summary = Some(summary.to_string());
        self
    }

    pub fn with_tags(&mut self, tags: HashMap<String, String>) -> &mut Self {
        self.workflow.document.tags = Some(tags);
        self
    }

    pub fn with_timeout(
        &mut self,
        configure: impl FnOnce(&mut TimeoutDefinitionBuilder),
    ) -> &mut Self {
        let mut timeout_builder = TimeoutDefinitionBuilder::new();
        configure(&mut timeout_builder);
        self.workflow.timeout = Some(OneOfTimeoutDefinitionOrReference::Timeout(
            timeout_builder.build(),
        ));
        self
    }

    pub fn use_authentication(
        &mut self,
        name: &str,
        configure: impl FnOnce(&mut AuthenticationPolicyDefinitionBuilder),
    ) -> &mut Self {
        let mut auth_builder = AuthenticationPolicyDefinitionBuilder::new();
        configure(&mut auth_builder);
        let auth_policy = auth_builder.build();

        if self.workflow.use_.is_none() {
            self.workflow.use_ = Some(ComponentDefinitionCollection::default());
        }
        if let Some(ref mut use_) = self.workflow.use_ {
            if use_.authentications.is_none() {
                use_.authentications = Some(HashMap::new());
            }
            if let Some(ref mut authentications) = use_.authentications {
                authentications.insert(
                    name.to_string(),
                    ReferenceableAuthenticationPolicy::Policy(Box::new(auth_policy)),
                );
            }
        }
        self
    }

    pub fn do_(
        &mut self,
        name: &str,
        configure: impl FnOnce(&mut TaskDefinitionBuilder),
    ) -> &mut Self {
        let mut task_builder = TaskDefinitionBuilder::new();
        configure(&mut task_builder);
        let task = task_builder.build();
        self.workflow.do_.add(name.to_string(), task);
        self
    }

    pub fn build(&mut self) -> WorkflowDefinition {
        std::mem::take(&mut self.workflow)
    }
}

impl Default for WorkflowBuilder {
    fn default() -> Self {
        Self::new()
    }
}
