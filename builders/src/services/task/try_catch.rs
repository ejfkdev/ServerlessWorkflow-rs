use super::*;

// ============== TryTaskDefinitionBuilder ==============
/// Builder for constructing a try task with error catching.
#[derive(Default)]
pub struct TryTaskDefinitionBuilder {
    task: TryTaskDefinition,
}

impl TryTaskDefinitionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a named sub-task to the try block.
    pub fn do_<F>(&mut self, name: &str, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TaskDefinitionBuilder),
    {
        let mut builder = TaskDefinitionBuilder::new();
        setup(&mut builder);
        let task = builder.build();
        self.task.try_.add(name.to_string(), task);
        self
    }

    /// Configures the error catcher for this try task using a builder callback.
    pub fn catch<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut ErrorCatcherDefinitionBuilder),
    {
        let mut builder = ErrorCatcherDefinitionBuilder::new(&mut self.task);
        setup(&mut builder);
        self
    }
}

impl_task_definition_builder_base!(TryTaskDefinitionBuilder, task, TaskDefinition::Try);

/// Builder for constructing an error catcher within a try task.
pub struct ErrorCatcherDefinitionBuilder<'a> {
    parent: &'a mut TryTaskDefinition,
}

impl<'a> ErrorCatcherDefinitionBuilder<'a> {
    fn new(parent: &'a mut TryTaskDefinition) -> Self {
        parent.catch = ErrorCatcherDefinition::default();
        Self { parent }
    }

    /// Configures the error filter for this catcher using a builder callback.
    pub fn errors<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut ErrorFilterDefinitionBuilder),
    {
        let mut builder = ErrorFilterDefinitionBuilder::new();
        setup(&mut builder);
        self.parent.catch.errors = Some(builder.build());
        self
    }

    /// Sets the condition expression for when the catcher applies.
    pub fn when(&mut self, when: &str) -> &mut Self {
        self.parent.catch.when = Some(when.to_string());
        self
    }

    /// Configures the retry policy for this catcher using a builder callback.
    pub fn retry<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut RetryPolicyDefinitionBuilder),
    {
        let mut builder = RetryPolicyDefinitionBuilder::new();
        setup(&mut builder);
        self.parent.catch.retry = Some(OneOfRetryPolicyDefinitionOrReference::Retry(Box::new(
            builder.build(),
        )));
        self
    }

    /// Sets the variable name to store the caught error.
    pub fn as_(&mut self, as_var: &str) -> &mut Self {
        self.parent.catch.as_ = Some(as_var.to_string());
        self
    }

    /// Sets the condition expression for when the catcher should not apply.
    pub fn except_when(&mut self, except_when: &str) -> &mut Self {
        self.parent.catch.except_when = Some(except_when.to_string());
        self
    }

    /// Adds a named sub-task to execute when the error is caught.
    pub fn do_<F>(&mut self, name: &str, setup: F) -> &mut Self
    where
        F: FnOnce(&mut TaskDefinitionBuilder),
    {
        let mut builder = TaskDefinitionBuilder::new();
        setup(&mut builder);
        let task = builder.build();
        self.parent
            .catch
            .do_
            .get_or_insert_with(serverless_workflow_core::models::map::Map::default)
            .add(name.to_string(), task);
        self
    }
}

/// Builder for constructing an error filter definition.
pub struct ErrorFilterDefinitionBuilder {
    filter: ErrorFilterDefinition,
}

impl ErrorFilterDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            filter: ErrorFilterDefinition::default(),
        }
    }

    /// Sets the error type to match.
    pub fn with_type(&mut self, type_: &str) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(ErrorFilterProperties::default)
            .type_ = Some(type_.to_string());
        self
    }

    /// Sets the error status to match.
    pub fn with_status(&mut self, status: Value) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(ErrorFilterProperties::default)
            .status = Some(status);
        self
    }

    /// Sets the error instance to match.
    pub fn with_instance(&mut self, instance: &str) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(ErrorFilterProperties::default)
            .instance = Some(instance.to_string());
        self
    }

    /// Sets the error title to match.
    pub fn with_title(&mut self, title: &str) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(ErrorFilterProperties::default)
            .title = Some(title.to_string());
        self
    }

    /// Sets the error detail to match.
    pub fn with_detail(&mut self, details: &str) -> &mut Self {
        self.filter
            .with
            .get_or_insert_with(ErrorFilterProperties::default)
            .detail = Some(details.to_string());
        self
    }

    /// Builds the `ErrorFilterDefinition`.
    pub fn build(self) -> ErrorFilterDefinition {
        self.filter
    }
}

impl Default for ErrorFilterDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a retry policy definition.
pub struct RetryPolicyDefinitionBuilder {
    policy: RetryPolicyDefinition,
}

impl RetryPolicyDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            policy: RetryPolicyDefinition::default(),
        }
    }

    /// Sets the condition for when the retry policy should not apply.
    pub fn except_when(&mut self, except_when: &str) -> &mut Self {
        self.policy.except_when = Some(except_when.to_string());
        self
    }

    /// Sets the condition for when the retry policy applies.
    pub fn when(&mut self, when: &str) -> &mut Self {
        self.policy.when = Some(when.to_string());
        self
    }

    /// Configures the retry limit using a builder callback.
    pub fn limit<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut RetryLimitDefinitionBuilder),
    {
        let mut builder = RetryLimitDefinitionBuilder::new();
        setup(&mut builder);
        self.policy.limit = Some(builder.build());
        self
    }

    /// Sets the initial delay before the first retry.
    pub fn delay(&mut self, delay: Duration) -> &mut Self {
        self.policy.delay = Some(OneOfDurationOrIso8601Expression::Duration(delay));
        self
    }

    /// Configures the backoff strategy using a builder callback.
    pub fn backoff<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut BackoffStrategyDefinitionBuilder),
    {
        let mut builder = BackoffStrategyDefinitionBuilder::new();
        setup(&mut builder);
        self.policy.backoff = Some(builder.build());
        self
    }

    /// Configures the jitter settings using a builder callback.
    pub fn jitter<F>(&mut self, setup: F) -> &mut Self
    where
        F: FnOnce(&mut JitterDefinitionBuilder),
    {
        let mut builder = JitterDefinitionBuilder::new();
        setup(&mut builder);
        self.policy.jitter = Some(builder.build());
        self
    }

    /// Builds the `RetryPolicyDefinition`.
    pub fn build(self) -> RetryPolicyDefinition {
        self.policy
    }
}

impl Default for RetryPolicyDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a retry policy limit definition.
pub struct RetryLimitDefinitionBuilder {
    limit: RetryPolicyLimitDefinition,
}

impl RetryLimitDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            limit: RetryPolicyLimitDefinition::default(),
        }
    }

    /// Sets the maximum number of retry attempts.
    pub fn attempt_count(&mut self, count: u16) -> &mut Self {
        self.limit
            .attempt
            .get_or_insert_with(RetryAttemptLimitDefinition::default)
            .count = Some(count);
        self
    }

    /// Sets the maximum duration for a single retry attempt.
    pub fn attempt_duration(&mut self, duration: Duration) -> &mut Self {
        self.limit
            .attempt
            .get_or_insert_with(RetryAttemptLimitDefinition::default)
            .duration = Some(OneOfDurationOrIso8601Expression::Duration(duration));
        self
    }

    /// Sets the overall maximum duration for all retries combined.
    pub fn duration(&mut self, duration: Duration) -> &mut Self {
        self.limit.duration = Some(OneOfDurationOrIso8601Expression::Duration(duration));
        self
    }

    /// Builds the `RetryPolicyLimitDefinition`.
    pub fn build(self) -> RetryPolicyLimitDefinition {
        self.limit
    }
}

impl Default for RetryLimitDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a backoff strategy definition.
pub struct BackoffStrategyDefinitionBuilder {
    strategy: BackoffStrategyDefinition,
    linear_builder: Option<LinearBackoffDefinitionBuilder>,
    constant_builder: Option<ConstantBackoffDefinitionBuilder>,
    exponential_builder: Option<ExponentialBackoffDefinitionBuilder>,
}

impl BackoffStrategyDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            strategy: BackoffStrategyDefinition::default(),
            linear_builder: None,
            constant_builder: None,
            exponential_builder: None,
        }
    }

    /// Configures a linear backoff strategy.
    pub fn linear(&mut self) -> &mut LinearBackoffDefinitionBuilder {
        self.linear_builder = Some(LinearBackoffDefinitionBuilder::new());
        self.get_linear_builder()
    }

    fn get_linear_builder(&mut self) -> &mut LinearBackoffDefinitionBuilder {
        if let Some(ref mut builder) = self.linear_builder {
            builder
        } else {
            unreachable!("Linear builder should be set")
        }
    }

    /// Configures a constant backoff strategy.
    pub fn constant(&mut self) -> &mut ConstantBackoffDefinitionBuilder {
        self.constant_builder = Some(ConstantBackoffDefinitionBuilder::new());
        self.get_constant_builder()
    }

    fn get_constant_builder(&mut self) -> &mut ConstantBackoffDefinitionBuilder {
        if let Some(ref mut builder) = self.constant_builder {
            builder
        } else {
            unreachable!("Constant builder should be set")
        }
    }

    /// Configures an exponential backoff strategy.
    pub fn exponential(&mut self) -> &mut ExponentialBackoffDefinitionBuilder {
        self.exponential_builder = Some(ExponentialBackoffDefinitionBuilder::new());
        self.get_exponential_builder()
    }

    fn get_exponential_builder(&mut self) -> &mut ExponentialBackoffDefinitionBuilder {
        if let Some(ref mut builder) = self.exponential_builder {
            builder
        } else {
            unreachable!("Exponential builder should be set")
        }
    }

    /// Builds the `BackoffStrategyDefinition`.
    pub fn build(mut self) -> BackoffStrategyDefinition {
        if let Some(builder) = self.linear_builder.take() {
            self.strategy.linear = Some(builder.build());
        }
        if let Some(builder) = self.constant_builder.take() {
            self.strategy.constant = Some(builder.build());
        }
        if let Some(builder) = self.exponential_builder.take() {
            self.strategy.exponential = Some(builder.build());
        }
        self.strategy
    }
}

impl Default for BackoffStrategyDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a linear backoff definition.
pub struct LinearBackoffDefinitionBuilder {
    linear: LinearBackoffDefinition,
}

impl LinearBackoffDefinitionBuilder {
    fn new() -> Self {
        Self {
            linear: LinearBackoffDefinition::default(),
        }
    }

    /// Sets the increment duration added on each retry.
    pub fn with_increment(&mut self, increment: Duration) -> &mut Self {
        self.linear.increment = Some(increment);
        self
    }

    /// Builds the `LinearBackoffDefinition`.
    pub fn build(self) -> LinearBackoffDefinition {
        self.linear
    }
}

impl Default for LinearBackoffDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a constant backoff definition.
pub struct ConstantBackoffDefinitionBuilder {
    constant: ConstantBackoffDefinition,
}

impl ConstantBackoffDefinitionBuilder {
    fn new() -> Self {
        Self {
            constant: ConstantBackoffDefinition::default(),
        }
    }

    /// Sets the constant delay duration between retries.
    pub fn with_delay(&mut self, delay: &str) -> &mut Self {
        self.constant = ConstantBackoffDefinition::with_delay(delay);
        self
    }

    /// Builds the `ConstantBackoffDefinition`.
    pub fn build(self) -> ConstantBackoffDefinition {
        self.constant
    }
}

impl Default for ConstantBackoffDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing an exponential backoff definition.
pub struct ExponentialBackoffDefinitionBuilder {
    exponential: ExponentialBackoffDefinition,
}

impl ExponentialBackoffDefinitionBuilder {
    fn new() -> Self {
        Self {
            exponential: ExponentialBackoffDefinition::default(),
        }
    }

    /// Sets the exponential growth factor.
    pub fn with_factor(&mut self, factor: f64) -> &mut Self {
        self.exponential = ExponentialBackoffDefinition::with_factor(factor);
        self
    }

    /// Sets the exponential growth factor and maximum delay cap.
    pub fn with_factor_and_max_delay(&mut self, factor: f64, max_delay: &str) -> &mut Self {
        self.exponential =
            ExponentialBackoffDefinition::with_factor_and_max_delay(factor, max_delay);
        self
    }

    /// Builds the `ExponentialBackoffDefinition`.
    pub fn build(self) -> ExponentialBackoffDefinition {
        self.exponential
    }
}

impl Default for ExponentialBackoffDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a jitter definition for retry delays.
pub struct JitterDefinitionBuilder {
    jitter: JitterDefinition,
}

impl JitterDefinitionBuilder {
    pub fn new() -> Self {
        Self {
            jitter: JitterDefinition::default(),
        }
    }

    /// Sets the minimum jitter duration.
    pub fn from(&mut self, from: Duration) -> &mut Self {
        self.jitter.from = from;
        self
    }

    /// Sets the maximum jitter duration.
    pub fn to(&mut self, to: Duration) -> &mut Self {
        self.jitter.to = to;
        self
    }

    /// Builds the `JitterDefinition`.
    pub fn build(self) -> JitterDefinition {
        self.jitter
    }
}

impl Default for JitterDefinitionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
