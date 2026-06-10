use crate::error::{WorkflowError, WorkflowResult};
use crate::events::{CloudEvent, EventBus};
use crate::listener::WorkflowEvent;
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::tasks::define_simple_task_runner;
use crate::tasks::task_name_impl;
use serde_json::Value;
use std::collections::HashMap;
use swf_core::models::event::{
    EventConsumptionStrategyDefinition, EventFilterDefinition,
    OneOfEventConsumptionStrategyDefinitionOrExpression,
};
use swf_core::models::task::ListenTaskDefinition;

define_simple_task_runner!(
    /// Runner for Listen tasks - subscribes to events from the EventBus
    ///
    /// Supports three consumption strategies:
    /// - `any`: Complete when any one of the listed events is received
    /// - `all`: Complete when all listed events have been received at least once
    /// - `one`: Complete when the single specified event is received
    ///
    /// Also supports `until` conditions and `foreach` iterators.
    ListenTaskRunner, ListenTaskDefinition
);

/// Consumption strategy mode for the unified listen loop
enum ListenMode {
    /// Complete when any filter matches
    Any,
    /// Complete when all filters have matched at least once
    All,
    /// Complete when the single filter matches (equivalent to Any with one filter,
    /// but kept for clarity)
    One,
}

impl ListenTaskRunner {
    /// Checks if a CloudEvent matches an EventFilterDefinition
    fn event_matches_filter(
        event: &CloudEvent,
        filter: &EventFilterDefinition,
        vars: &std::collections::HashMap<String, Value>,
    ) -> bool {
        if let Some(ref with) = filter.with {
            for (key, expected) in with {
                let actual = match key.as_str() {
                    "type" => Some(Value::String(event.event_type.clone())),
                    "source" => event.source.as_ref().map(|s| Value::String(s.clone())),
                    "data" => Some(event.data.clone()),
                    _ => event.attributes.get(key).cloned(),
                };
                match (actual, expected) {
                    (Some(actual_val), expected_val) => {
                        // For "data" key, support JQ expression filtering
                        if key == "data" {
                            if let Some(expr) = expected_val.as_str() {
                                if expr.starts_with("${") && expr.ends_with('}') {
                                    if !evaluate_data_filter(expr, &actual_val, vars) {
                                        return false;
                                    }
                                    continue;
                                }
                            }
                        }
                        if !values_match(&actual_val, expected_val) {
                            return false;
                        }
                    }
                    (None, _) => return false,
                }
            }
        }
        true
    }

    /// Returns the read mode for this listen task
    fn read_mode(&self) -> &str {
        self.task
            .listen
            .read
            .as_deref()
            .unwrap_or(swf_core::models::task::constants::EventReadMode::DATA)
    }

    /// Extracts event data according to the read mode
    fn event_to_output(&self, event: &CloudEvent) -> Value {
        match self.read_mode() {
            swf_core::models::task::constants::EventReadMode::ENVELOPE => event.to_json_value(),
            swf_core::models::task::constants::EventReadMode::RAW => {
                // Raw: the serialized JSON string of the full event, wrapped as a string value
                Value::String(event.to_json_value().to_string())
            }
            _ => event.data.clone(), // "data" (default)
        }
    }

    /// Converts consumed events to a JSON Value for expression evaluation
    fn events_to_value(&self, consumed_events: &[CloudEvent]) -> Value {
        if consumed_events.len() == 1 {
            self.event_to_output(&consumed_events[0])
        } else {
            Value::Array(
                consumed_events
                    .iter()
                    .map(|e| self.event_to_output(e))
                    .collect(),
            )
        }
    }

    /// Evaluates an `until` condition to decide whether to stop listening
    async fn evaluate_until(
        &self,
        until: &OneOfEventConsumptionStrategyDefinitionOrExpression,
        consumed_events: &[CloudEvent],
        support: &TaskSupport<'_>,
    ) -> WorkflowResult<bool> {
        match until {
            OneOfEventConsumptionStrategyDefinitionOrExpression::Bool(false) => Ok(false),
            OneOfEventConsumptionStrategyDefinitionOrExpression::Bool(true) => Ok(true),
            OneOfEventConsumptionStrategyDefinitionOrExpression::Expression(expr) => {
                let events_ctx = self.events_to_value(consumed_events);
                support.eval_bool(expr, &events_ctx)
            }
            OneOfEventConsumptionStrategyDefinitionOrExpression::Strategy(strategy) => {
                // Check if the until strategy is satisfied by the consumed events
                let vars = support.get_vars();
                Ok(Self::is_strategy_satisfied(
                    strategy,
                    consumed_events,
                    &vars,
                ))
            }
        }
    }

    /// Checks whether a consumption strategy is satisfied by the consumed events
    fn is_strategy_satisfied(
        strategy: &EventConsumptionStrategyDefinition,
        consumed_events: &[CloudEvent],
        vars: &std::collections::HashMap<String, Value>,
    ) -> bool {
        if let Some(ref any_filters) = strategy.any {
            // "any" means at least one filter must match at least one consumed event
            let any_matched = any_filters.iter().any(|filter| {
                consumed_events
                    .iter()
                    .any(|e| Self::event_matches_filter(e, filter, vars))
            });
            if !any_matched {
                return false;
            }
        }
        if let Some(ref all_filters) = strategy.all {
            // "all" means every filter must match at least one consumed event
            for filter in all_filters {
                let matched = consumed_events
                    .iter()
                    .any(|e| Self::event_matches_filter(e, filter, vars));
                if !matched {
                    return false;
                }
            }
        }
        if let Some(ref one_filter) = strategy.one {
            // "one" means the single filter must match at least one consumed event
            let matched = consumed_events
                .iter()
                .any(|e| Self::event_matches_filter(e, one_filter, vars));
            if !matched {
                return false;
            }
        }
        true
    }

    /// Unified listen loop that handles all consumption strategies
    async fn run_listen_loop(
        &self,
        filters: &[EventFilterDefinition],
        mode: ListenMode,
        event_bus: &dyn EventBus,
        input: &Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        let mut subscription = event_bus.subscribe_all().await;
        let mut consumed_events: Vec<CloudEvent> = Vec::new();
        let cancel_token = support.context.cancellation_token();

        // Emit correlation-started lifecycle event (spec 1.0.3 requirement)
        let correlation_keys = collect_correlation_keys(filters);
        let instance_id = support.context.instance_id().to_string();
        support
            .context
            .emit_event(WorkflowEvent::WorkflowCorrelationStarted {
                instance_id: instance_id.clone(),
                correlation_keys: correlation_keys.clone(),
            });

        let result = self
            .run_listen_inner_loop(
                filters,
                mode,
                event_bus,
                &mut subscription,
                &mut consumed_events,
                &cancel_token,
                support,
            )
            .await;

        // Emit correlation-completed lifecycle event regardless of how the loop exits
        support
            .context
            .emit_event(WorkflowEvent::WorkflowCorrelationCompleted {
                instance_id: instance_id.clone(),
                correlation_keys: extract_resolved_correlation_keys(
                    filters,
                    &consumed_events,
                    &correlation_keys,
                ),
            });

        event_bus.unsubscribe(subscription).await;

        result?;
        self.build_output(&consumed_events, input, support).await
    }

    /// Inner loop body, separated so correlation-completed can fire on every exit path
    #[allow(clippy::too_many_arguments)]
    async fn run_listen_inner_loop(
        &self,
        filters: &[EventFilterDefinition],
        mode: ListenMode,
        event_bus: &dyn EventBus,
        subscription: &mut crate::events::EventSubscription,
        consumed_events: &mut Vec<CloudEvent>,
        cancel_token: &tokio_util::sync::CancellationToken,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<()> {
        loop {
            // For "all" mode, check if all filters are already satisfied before waiting
            if matches!(mode, ListenMode::All) && !consumed_events.is_empty() {
                let vars = support.get_vars();
                let all_satisfied = filters.iter().all(|filter| {
                    consumed_events
                        .iter()
                        .any(|e| Self::event_matches_filter(e, filter, &vars))
                });
                if all_satisfied {
                    if let Some(ref until) = self.task.listen.to.until {
                        if self.evaluate_until(until, consumed_events, support).await? {
                            return Ok(());
                        }
                    } else {
                        return Ok(());
                    }
                }
            }

            tokio::select! {
                event = event_bus.recv(subscription) => {
                    match event {
                        Some(event) => {
                            // Skip lifecycle events (internal to the runtime)
                            if is_lifecycle_event(&event) {
                                continue;
                            }
                            // Check if event matches any of the filters
                            let vars = support.get_vars();
                            let matches = filters.is_empty()
                                || filters.iter().any(|f| Self::event_matches_filter(&event, f, &vars));
                            if matches {
                                consumed_events.push(event);

                                // Check if we should complete
                                let should_complete = match mode {
                                    ListenMode::All => {
                                        // For "all", check if all filters are now satisfied
                                        // (will be checked at top of loop on next iteration)
                                        // But we also check here for the no-until case to avoid extra iteration
                                        if self.task.listen.to.until.is_none() {
                                            filters.iter().all(|filter| {
                                                consumed_events.iter().any(|e| Self::event_matches_filter(e, filter, &vars))
                                            })
                                        } else {
                                            // With until: check at top of loop
                                            false
                                        }
                                    }
                                    ListenMode::Any | ListenMode::One => {
                                        // For "any"/"one" without until: done after first match
                                        self.task.listen.to.until.is_none()
                                    }
                                };

                                if should_complete {
                                    return Ok(());
                                }

                                // Check until condition
                                if let Some(ref until) = self.task.listen.to.until {
                                    if self.evaluate_until(until, consumed_events, support).await? {
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        None => {
                            return Err(WorkflowError::runtime_simple(
                                "event bus closed while listening".to_string(),
                                &self.name,
                            ));
                        }
                    }
                }
                _ = cancel_token.cancelled() => {
                    return Err(WorkflowError::runtime_simple(
                        "workflow cancelled while listening for events".to_string(),
                        &self.name,
                    ));
                }
            }
        }
    }

    /// Builds the output from consumed events, applying foreach iterator if defined
    async fn build_output(
        &self,
        consumed_events: &[CloudEvent],
        _input: &Value,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        let output = self.events_to_value(consumed_events);

        // Process foreach iterator if defined
        if let Some(ref foreach) = self.task.foreach {
            let foreach_input = Value::Array(
                consumed_events
                    .iter()
                    .map(|e| self.event_to_output(e))
                    .collect(),
            );
            self.process_foreach(&foreach_input, foreach, support).await
        } else {
            Ok(output)
        }
    }

    /// Processes the foreach iterator for consumed events
    async fn process_foreach(
        &self,
        events_array: &Value,
        foreach: &swf_core::models::task::SubscriptionIteratorDefinition,
        support: &mut TaskSupport<'_>,
    ) -> WorkflowResult<Value> {
        let items = match events_array.as_array() {
            Some(arr) => arr.clone(),
            None => vec![events_array.clone()],
        };

        let item_var =
            crate::utils::ensure_dollar_prefix(foreach.item.as_deref().unwrap_or("item"), "$item");
        let at_var =
            crate::utils::ensure_dollar_prefix(foreach.at.as_deref().unwrap_or("index"), "$index");
        let mut results = Vec::new();

        if let Some(ref tasks) = foreach.do_ {
            for (idx, item) in items.iter().enumerate() {
                // Set up iterator variables
                let mut local_vars = std::collections::HashMap::new();
                local_vars.insert(item_var.to_string(), item.clone());
                local_vars.insert(at_var.to_string(), serde_json::json!(idx));
                support.add_local_expr_vars(local_vars);

                // Run the tasks in the foreach body
                let task_input = item.clone();
                let mut current_output = task_input;

                for (task_name, task_def) in &tasks.entries {
                    let runner = crate::task_runner::create_task_runner(
                        task_name,
                        task_def,
                        support.workflow,
                    )?;
                    current_output = runner.run(current_output, support).await?;
                }

                // Process output transformation
                if let Some(ref output_def) = foreach.output {
                    current_output = support.process_task_output(
                        Some(output_def),
                        &current_output,
                        &self.name,
                    )?;
                }

                // Process export
                if let Some(ref export_def) = foreach.export {
                    support.process_task_export(Some(export_def), &current_output, &self.name)?;
                }

                results.push(current_output);

                // Clean up iterator variables
                support.remove_local_expr_vars(&[&item_var, &at_var]);
            }
        }

        Ok(Value::Array(results))
    }
}

#[async_trait::async_trait]
impl TaskRunner for ListenTaskRunner {
    task_name_impl!();

    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        let event_bus = support.clone_event_bus().ok_or_else(|| {
            WorkflowError::runtime_simple(
                "listen task requires an EventBus (configure one via WorkflowRunner::with_event_bus())".to_string(),
                &self.name,
            )
        })?;

        let strategy = &self.task.listen.to;

        if let Some(ref any_filters) = strategy.any {
            self.run_listen_loop(
                any_filters,
                ListenMode::Any,
                event_bus.as_ref(),
                &input,
                support,
            )
            .await
        } else if let Some(ref all_filters) = strategy.all {
            self.run_listen_loop(
                all_filters,
                ListenMode::All,
                event_bus.as_ref(),
                &input,
                support,
            )
            .await
        } else if let Some(ref one_filter) = strategy.one {
            self.run_listen_loop(
                std::slice::from_ref(one_filter),
                ListenMode::One,
                event_bus.as_ref(),
                &input,
                support,
            )
            .await
        } else {
            // No consumption strategy defined — return input unchanged
            Ok(input)
        }
    }
}

/// Checks if a CloudEvent is a lifecycle event (internal to the runtime)
fn is_lifecycle_event(event: &CloudEvent) -> bool {
    event.event_type.starts_with("io.serverlessworkflow.")
}

/// Evaluates a JQ expression as a boolean filter against event data
fn evaluate_data_filter(
    expr: &str,
    data: &Value,
    vars: &std::collections::HashMap<String, Value>,
) -> bool {
    let sanitized = crate::expression::prepare_expression(expr);
    match crate::expression::evaluate_jq(&sanitized, data, vars) {
        Ok(result) => result.as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

/// Checks if an actual value matches an expected value (supports string and regex matching)
fn values_match(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::String(a), Value::String(b)) => {
            // If expected looks like a regex pattern, try regex matching
            if b.starts_with('^') || b.ends_with('$') || b.contains(".*") || b.contains(".+") {
                if let Ok(re) = regex::Regex::new(b) {
                    return re.is_match(a);
                }
            }
            a == b
        }
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => actual == expected,
    }
}

/// Collects the declared correlation key names from all filters
fn collect_correlation_keys(filters: &[EventFilterDefinition]) -> HashMap<String, Value> {
    let mut keys = HashMap::new();
    for filter in filters {
        if let Some(ref correlate_map) = filter.correlate {
            for name in correlate_map.keys() {
                keys.entry(name.clone()).or_insert(Value::Null);
            }
        }
    }
    keys
}

/// Resolves correlation keys by extracting their values from the consumed events
fn extract_resolved_correlation_keys(
    filters: &[EventFilterDefinition],
    consumed_events: &[CloudEvent],
    declared_keys: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    if declared_keys.is_empty() {
        return HashMap::new();
    }

    let mut resolved: HashMap<String, Value> = HashMap::new();
    for filter in filters {
        if let Some(ref correlate_map) = filter.correlate {
            for (name, def) in correlate_map {
                if resolved.contains_key(name) {
                    continue;
                }
                // Find the first consumed event that matches this filter and extract
                // the correlation value via the runtime expression in `from`.
                for event in consumed_events {
                    if !event_matches_filter_with(event, filter) {
                        continue;
                    }
                    let extracted = evaluate_correlation_from(&def.from, event);
                    if let Some(value) = extracted {
                        resolved.insert(name.clone(), value);
                        break;
                    }
                }
            }
        }
    }
    resolved
}

/// Synchronous filter check used during correlation-key resolution (no JQ evaluation needed
/// beyond a basic attribute-with match — sufficient for resolving the first matching event).
fn event_matches_filter_with(event: &CloudEvent, filter: &EventFilterDefinition) -> bool {
    let Some(ref with) = filter.with else {
        return true;
    };
    for (k, expected) in with {
        let actual = match k.as_str() {
            "type" => Some(Value::String(event.event_type.clone())),
            "source" => event.source.as_ref().map(|s| Value::String(s.clone())),
            "data" => Some(event.data.clone()),
            _ => event.attributes.get(k).cloned(),
        };
        match actual {
            Some(actual_val) if &actual_val == expected => continue,
            _ => return false,
        }
    }
    true
}

/// Evaluates a `from` runtime expression against an event's data.
/// Supports simple `.{path}` expressions on the event's `data` field.
fn evaluate_correlation_from(expr: &str, event: &CloudEvent) -> Option<Value> {
    let path = expr.trim().trim_start_matches('$').trim_start_matches('.');
    if path.is_empty() {
        return Some(event.data.clone());
    }
    let mut current: Option<&Value> = Some(&event.data);
    for seg in path.split('.') {
        match current {
            Some(Value::Object(map)) => {
                current = map.get(seg);
            }
            _ => return None,
        }
    }
    current.cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::WorkflowContext;
    use crate::default_support;
    use crate::events::InMemoryEventBus;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use swf_core::models::event::EventConsumptionStrategyDefinition;
    use swf_core::models::event::EventFilterDefinition;
    use swf_core::models::task::ListenTaskDefinition;
    use swf_core::models::task::ListenerDefinition;
    use swf_core::models::task::TaskDefinitionFields;
    use swf_core::models::workflow::WorkflowDefinition;

    #[test]
    fn test_listen_runner_new() {
        let listen_def = ListenTaskDefinition::new(ListenerDefinition::new(
            EventConsumptionStrategyDefinition::default(),
        ));
        let runner = ListenTaskRunner::new("testListen", &listen_def);
        assert!(runner.is_ok());
        assert_eq!(runner.unwrap().task_name(), "testListen");
    }

    #[test]
    fn test_event_matches_filter_by_type() {
        let event = CloudEvent::new("com.example.test", json!({"msg": "hello"}));
        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.test"));
        let filter = EventFilterDefinition {
            with: Some(with),
            correlate: None,
        };
        assert!(ListenTaskRunner::event_matches_filter(
            &event,
            &filter,
            &HashMap::new()
        ));
    }

    #[test]
    fn test_event_does_not_match_filter_by_type() {
        let event = CloudEvent::new("com.example.other", json!({}));
        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.test"));
        let filter = EventFilterDefinition {
            with: Some(with),
            correlate: None,
        };
        assert!(!ListenTaskRunner::event_matches_filter(
            &event,
            &filter,
            &HashMap::new()
        ));
    }

    #[test]
    fn test_event_matches_filter_by_source() {
        let event = CloudEvent::new("test.event", json!({})).with_source("https://example.com");
        let mut with = HashMap::new();
        with.insert("source".to_string(), json!("https://example.com"));
        let filter = EventFilterDefinition {
            with: Some(with),
            correlate: None,
        };
        assert!(ListenTaskRunner::event_matches_filter(
            &event,
            &filter,
            &HashMap::new()
        ));
    }

    #[test]
    fn test_event_matches_filter_by_custom_attribute() {
        let event = CloudEvent::new("test.event", json!({}))
            .with_attribute("correlationId", json!("abc-123"));
        let mut with = HashMap::new();
        with.insert("correlationId".to_string(), json!("abc-123"));
        let filter = EventFilterDefinition {
            with: Some(with),
            correlate: None,
        };
        assert!(ListenTaskRunner::event_matches_filter(
            &event,
            &filter,
            &HashMap::new()
        ));
    }

    #[test]
    fn test_values_match_string() {
        assert!(values_match(&json!("hello"), &json!("hello")));
        assert!(!values_match(&json!("hello"), &json!("world")));
    }

    #[test]
    fn test_values_match_regex() {
        assert!(values_match(
            &json!("com.example.test"),
            &json!("com\\.example\\..*")
        ));
        assert!(!values_match(
            &json!("com.other.test"),
            &json!("^com\\.example\\..*")
        ));
    }

    #[test]
    fn test_event_matches_filter_data_expression() {
        // JQ expression filter on data: temperature > 38
        let event = CloudEvent::new("com.hospital.temperature", json!({"temperature": 39.5}));
        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.hospital.temperature"));
        with.insert("data".to_string(), json!("${ .temperature > 38 }"));
        let filter = EventFilterDefinition {
            with: Some(with),
            correlate: None,
        };
        assert!(ListenTaskRunner::event_matches_filter(
            &event,
            &filter,
            &HashMap::new()
        ));

        // Temperature below threshold should not match
        let event2 = CloudEvent::new("com.hospital.temperature", json!({"temperature": 36.5}));
        assert!(!ListenTaskRunner::event_matches_filter(
            &event2,
            &filter,
            &HashMap::new()
        ));
    }

    #[test]
    fn test_event_matches_filter_data_with_vars() {
        // JQ expression filter using $input variables
        let event = CloudEvent::new("com.hospital.temperature", json!({"temperature": 39.5}));
        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.hospital.temperature"));
        with.insert(
            "data".to_string(),
            json!("${ .temperature > $input.threshold }"),
        );
        let filter = EventFilterDefinition {
            with: Some(with),
            correlate: None,
        };
        let mut vars = HashMap::new();
        vars.insert("$input".to_string(), json!({"threshold": 38}));
        assert!(ListenTaskRunner::event_matches_filter(
            &event, &filter, &vars
        ));

        // Below threshold
        let event2 = CloudEvent::new("com.hospital.temperature", json!({"temperature": 36.5}));
        assert!(!ListenTaskRunner::event_matches_filter(
            &event2, &filter, &vars
        ));
    }

    #[test]
    fn test_event_matches_filter_data_literal() {
        // Literal data match (not an expression)
        let event = CloudEvent::new("com.example.test", json!({"status": "ok"}));
        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.test"));
        with.insert("data".to_string(), json!({"status": "ok"}));
        let filter = EventFilterDefinition {
            with: Some(with),
            correlate: None,
        };
        assert!(ListenTaskRunner::event_matches_filter(
            &event,
            &filter,
            &HashMap::new()
        ));
    }

    #[tokio::test]
    async fn test_listen_no_event_bus_returns_error() {
        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.test"));
        let strategy = EventConsumptionStrategyDefinition {
            any: Some(vec![EventFilterDefinition {
                with: Some(with),
                correlate: None,
            }]),
            all: None,
            one: None,
            until: None,
        };
        let listen_def = ListenTaskDefinition {
            listen: ListenerDefinition::new(strategy),
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenNoBus", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        default_support!(workflow, context, support);

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("EventBus"));
    }

    #[tokio::test]
    async fn test_listen_any_single_event() {
        let bus = Arc::new(InMemoryEventBus::new());

        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.test"));
        let strategy = EventConsumptionStrategyDefinition {
            any: Some(vec![EventFilterDefinition {
                with: Some(with),
                correlate: None,
            }]),
            all: None,
            one: None,
            until: None,
        };
        let listen_def = ListenTaskDefinition {
            listen: ListenerDefinition::new(strategy),
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenAny", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_event_bus(bus.clone());
        let mut support = TaskSupport::new(&workflow, &mut context);

        // Publish event in background after a short delay
        let bus_clone = bus.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            bus_clone
                .publish(CloudEvent::new("com.example.test", json!({"msg": "hello"})))
                .await;
        });

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["msg"], "hello");
    }

    #[tokio::test]
    async fn test_listen_one_event() {
        let bus = Arc::new(InMemoryEventBus::new());

        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.target"));
        let strategy = EventConsumptionStrategyDefinition {
            any: None,
            all: None,
            one: Some(EventFilterDefinition {
                with: Some(with),
                correlate: None,
            }),
            until: None,
        };
        let listen_def = ListenTaskDefinition {
            listen: ListenerDefinition::new(strategy),
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenOne", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_event_bus(bus.clone());
        let mut support = TaskSupport::new(&workflow, &mut context);

        let bus_clone = bus.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            bus_clone
                .publish(CloudEvent::new("com.example.other", json!({"skip": true})))
                .await;
            bus_clone
                .publish(CloudEvent::new(
                    "com.example.target",
                    json!({"found": true}),
                ))
                .await;
        });

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["found"], true);
    }

    #[tokio::test]
    async fn test_listen_all_events() {
        let bus = Arc::new(InMemoryEventBus::new());

        let mut with1 = HashMap::new();
        with1.insert("type".to_string(), json!("com.example.type1"));
        let mut with2 = HashMap::new();
        with2.insert("type".to_string(), json!("com.example.type2"));

        let strategy = EventConsumptionStrategyDefinition {
            any: None,
            all: Some(vec![
                EventFilterDefinition {
                    with: Some(with1),
                    correlate: None,
                },
                EventFilterDefinition {
                    with: Some(with2),
                    correlate: None,
                },
            ]),
            one: None,
            until: None,
        };
        let listen_def = ListenTaskDefinition {
            listen: ListenerDefinition::new(strategy),
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenAll", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_event_bus(bus.clone());
        let mut support = TaskSupport::new(&workflow, &mut context);

        let bus_clone = bus.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            bus_clone
                .publish(CloudEvent::new("com.example.type1", json!({"a": 1})))
                .await;
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            bus_clone
                .publish(CloudEvent::new("com.example.type2", json!({"b": 2})))
                .await;
        });

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // "all" with two events returns an array
        let arr = output.as_array().expect("expected array output");
        assert_eq!(arr.len(), 2);
    }

    #[tokio::test]
    async fn test_listen_filters_unmatched_events() {
        let bus = Arc::new(InMemoryEventBus::new());

        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.target"));
        let strategy = EventConsumptionStrategyDefinition {
            any: Some(vec![EventFilterDefinition {
                with: Some(with),
                correlate: None,
            }]),
            all: None,
            one: None,
            until: None,
        };
        let listen_def = ListenTaskDefinition {
            listen: ListenerDefinition::new(strategy),
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenFilter", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_event_bus(bus.clone());
        let mut support = TaskSupport::new(&workflow, &mut context);

        let bus_clone = bus.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            // Publish non-matching events first
            bus_clone
                .publish(CloudEvent::new("com.example.other", json!({"skip": 1})))
                .await;
            bus_clone
                .publish(CloudEvent::new("com.example.unrelated", json!({"skip": 2})))
                .await;
            // Then the matching one
            bus_clone
                .publish(CloudEvent::new(
                    "com.example.target",
                    json!({"matched": true}),
                ))
                .await;
        });

        let output = runner.run(json!({}), &mut support).await.unwrap();
        assert_eq!(output["matched"], true);
    }

    #[tokio::test]
    async fn test_listen_read_mode_data() {
        let bus = Arc::new(InMemoryEventBus::new());

        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.test"));
        let strategy = EventConsumptionStrategyDefinition {
            any: Some(vec![EventFilterDefinition {
                with: Some(with),
                correlate: None,
            }]),
            all: None,
            one: None,
            until: None,
        };
        let mut listener = ListenerDefinition::new(strategy);
        listener.read = Some("data".to_string());
        let listen_def = ListenTaskDefinition {
            listen: listener,
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenReadData", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_event_bus(bus.clone());
        let mut support = TaskSupport::new(&workflow, &mut context);

        let bus_clone = bus.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            bus_clone
                .publish(
                    CloudEvent::new("com.example.test", json!({"msg": "hello"}))
                        .with_source("https://example.com"),
                )
                .await;
        });

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // "data" mode: only the data payload
        assert_eq!(output["msg"], "hello");
        assert!(output.get("type").is_none());
    }

    #[tokio::test]
    async fn test_listen_read_mode_envelope() {
        let bus = Arc::new(InMemoryEventBus::new());

        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.test"));
        let strategy = EventConsumptionStrategyDefinition {
            any: Some(vec![EventFilterDefinition {
                with: Some(with),
                correlate: None,
            }]),
            all: None,
            one: None,
            until: None,
        };
        let mut listener = ListenerDefinition::new(strategy);
        listener.read = Some("envelope".to_string());
        let listen_def = ListenTaskDefinition {
            listen: listener,
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenReadEnvelope", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_event_bus(bus.clone());
        let mut support = TaskSupport::new(&workflow, &mut context);

        let bus_clone = bus.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            bus_clone
                .publish(
                    CloudEvent::new("com.example.test", json!({"msg": "hello"}))
                        .with_source("https://example.com"),
                )
                .await;
        });

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // "envelope" mode: full CloudEvent envelope
        assert_eq!(output["type"], "com.example.test");
        assert_eq!(output["source"], "https://example.com");
        assert_eq!(output["data"]["msg"], "hello");
    }

    #[tokio::test]
    async fn test_listen_read_mode_raw() {
        let bus = Arc::new(InMemoryEventBus::new());

        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.test"));
        let strategy = EventConsumptionStrategyDefinition {
            any: Some(vec![EventFilterDefinition {
                with: Some(with),
                correlate: None,
            }]),
            all: None,
            one: None,
            until: None,
        };
        let mut listener = ListenerDefinition::new(strategy);
        listener.read = Some("raw".to_string());
        let listen_def = ListenTaskDefinition {
            listen: listener,
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenReadRaw", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_event_bus(bus.clone());
        let mut support = TaskSupport::new(&workflow, &mut context);

        let bus_clone = bus.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            bus_clone
                .publish(
                    CloudEvent::new("com.example.test", json!({"msg": "hello"}))
                        .with_source("https://example.com"),
                )
                .await;
        });

        let output = runner.run(json!({}), &mut support).await.unwrap();
        // "raw" mode: serialized JSON string
        let raw_str = output.as_str().expect("expected string output");
        let parsed: Value = serde_json::from_str(raw_str).expect("expected valid JSON");
        assert_eq!(parsed["type"], "com.example.test");
        assert_eq!(parsed["data"]["msg"], "hello");
    }

    // ===== Correlation lifecycle event tests (spec 1.0.3) =====

    #[tokio::test]
    async fn test_correlation_events_emitted_without_keys() {
        use crate::listener::CollectingListener;

        let bus = Arc::new(InMemoryEventBus::new());
        let listener = Arc::new(CollectingListener::new());

        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.signal"));
        let strategy = EventConsumptionStrategyDefinition {
            any: Some(vec![EventFilterDefinition {
                with: Some(with),
                correlate: None,
            }]),
            all: None,
            one: None,
            until: None,
        };
        let listen_def = ListenTaskDefinition {
            listen: ListenerDefinition::new(strategy),
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenWithCorrelation", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_event_bus(bus.clone());
        context.set_listener(listener.clone());
        let mut support = TaskSupport::new(&workflow, &mut context);

        let bus_clone = bus.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            bus_clone
                .publish(CloudEvent::new("com.example.signal", json!({"id": "abc"})))
                .await;
        });

        let _ = runner.run(json!({}), &mut support).await.unwrap();

        let events = listener.events();
        // Should have at least one started and one completed correlation event
        let started_count = events
            .iter()
            .filter(|e| matches!(e, WorkflowEvent::WorkflowCorrelationStarted { .. }))
            .count();
        let completed_count = events
            .iter()
            .filter(|e| matches!(e, WorkflowEvent::WorkflowCorrelationCompleted { .. }))
            .count();
        assert_eq!(
            started_count, 1,
            "expected exactly one CorrelationStarted event"
        );
        assert_eq!(
            completed_count, 1,
            "expected exactly one CorrelationCompleted event"
        );

        // No correlation keys are declared — both events should carry empty maps
        for event in &events {
            match event {
                WorkflowEvent::WorkflowCorrelationStarted {
                    correlation_keys, ..
                }
                | WorkflowEvent::WorkflowCorrelationCompleted {
                    correlation_keys, ..
                } => {
                    assert!(
                        correlation_keys.is_empty(),
                        "expected no declared correlation keys"
                    );
                }
                _ => {}
            }
        }

        // Ensure the started event was emitted BEFORE completed
        let started_idx = events
            .iter()
            .position(|e| matches!(e, WorkflowEvent::WorkflowCorrelationStarted { .. }))
            .unwrap();
        let completed_idx = events
            .iter()
            .position(|e| matches!(e, WorkflowEvent::WorkflowCorrelationCompleted { .. }))
            .unwrap();
        assert!(
            started_idx < completed_idx,
            "CorrelationStarted should be emitted before CorrelationCompleted"
        );
    }

    #[tokio::test]
    async fn test_correlation_events_with_resolved_keys() {
        use crate::listener::CollectingListener;
        use swf_core::models::event::CorrelationKeyDefinition;

        let bus = Arc::new(InMemoryEventBus::new());
        let listener = Arc::new(CollectingListener::new());

        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.order"));
        let mut correlate = HashMap::new();
        correlate.insert(
            "orderId".to_string(),
            CorrelationKeyDefinition::new(".orderId", None),
        );
        let strategy = EventConsumptionStrategyDefinition {
            any: Some(vec![EventFilterDefinition {
                with: Some(with),
                correlate: Some(correlate),
            }]),
            all: None,
            one: None,
            until: None,
        };
        let listen_def = ListenTaskDefinition {
            listen: ListenerDefinition::new(strategy),
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenResolveKeys", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_event_bus(bus.clone());
        context.set_listener(listener.clone());
        let mut support = TaskSupport::new(&workflow, &mut context);

        let bus_clone = bus.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            bus_clone
                .publish(CloudEvent::new(
                    "com.example.order",
                    json!({"orderId": "ORD-42", "amount": 100}),
                ))
                .await;
        });

        let _ = runner.run(json!({}), &mut support).await.unwrap();

        let events = listener.events();
        // CorrelationStarted: declared key with Null value (not yet resolved)
        let started = events
            .iter()
            .find_map(|e| match e {
                WorkflowEvent::WorkflowCorrelationStarted {
                    correlation_keys, ..
                } => Some(correlation_keys.clone()),
                _ => None,
            })
            .expect("CorrelationStarted event missing");
        assert!(
            started.contains_key("orderId"),
            "orderId should be declared"
        );
        assert_eq!(
            started.get("orderId"),
            Some(&Value::Null),
            "started key value should be null before resolution"
        );

        // CorrelationCompleted: resolved key value extracted from the consumed event
        let completed = events
            .iter()
            .find_map(|e| match e {
                WorkflowEvent::WorkflowCorrelationCompleted {
                    correlation_keys, ..
                } => Some(correlation_keys.clone()),
                _ => None,
            })
            .expect("CorrelationCompleted event missing");
        assert_eq!(
            completed.get("orderId"),
            Some(&json!("ORD-42")),
            "orderId should be resolved from the matched event"
        );
    }

    #[tokio::test]
    async fn test_correlation_completed_emitted_on_cancellation() {
        use crate::listener::CollectingListener;

        let bus = Arc::new(InMemoryEventBus::new());
        let listener = Arc::new(CollectingListener::new());

        let mut with = HashMap::new();
        with.insert("type".to_string(), json!("com.example.never-fires"));
        let strategy = EventConsumptionStrategyDefinition {
            any: Some(vec![EventFilterDefinition {
                with: Some(with),
                correlate: None,
            }]),
            all: None,
            one: None,
            until: None,
        };
        let listen_def = ListenTaskDefinition {
            listen: ListenerDefinition::new(strategy),
            foreach: None,
            common: TaskDefinitionFields::new(),
        };
        let runner = ListenTaskRunner::new("listenCancelled", &listen_def).unwrap();

        let workflow = WorkflowDefinition::default();
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_event_bus(bus.clone());
        context.set_listener(listener.clone());
        let cancel_token = context.cancellation_token();
        let mut support = TaskSupport::new(&workflow, &mut context);

        // Cancel after a short delay so the listen loop exits via the cancel branch
        let cancel_clone = cancel_token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            cancel_clone.cancel();
        });

        let result = runner.run(json!({}), &mut support).await;
        assert!(result.is_err(), "expected cancellation error");

        let events = listener.events();
        // CorrelationCompleted MUST be emitted even if the loop exits with an error
        assert!(
            events
                .iter()
                .any(|e| matches!(e, WorkflowEvent::WorkflowCorrelationStarted { .. })),
            "CorrelationStarted should fire even on cancellation"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, WorkflowEvent::WorkflowCorrelationCompleted { .. })),
            "CorrelationCompleted should fire on cancellation path"
        );
    }

    #[test]
    fn test_correlation_event_to_cloud_event_payload() {
        // Spec 1.0.3: started event must carry `name` and `startedAt`
        let started = WorkflowEvent::WorkflowCorrelationStarted {
            instance_id: "wf-instance-1".to_string(),
            correlation_keys: HashMap::new(),
        };
        let started_ce = started.to_cloud_event();
        assert_eq!(
            started_ce.event_type,
            WorkflowEvent::WORKFLOW_CORRELATION_STARTED_TYPE
        );
        assert_eq!(started_ce.data["name"], "wf-instance-1");
        assert!(started_ce.data.get("startedAt").is_some());
        // No correlation keys → field should be absent
        assert!(started_ce.data.get("correlationKeys").is_none());

        // Spec 1.0.3: completed event must carry `name`, `completedAt`, and optional `correlationKeys`
        let mut keys = HashMap::new();
        keys.insert("orderId".to_string(), json!("ORD-42"));
        let completed = WorkflowEvent::WorkflowCorrelationCompleted {
            instance_id: "wf-instance-1".to_string(),
            correlation_keys: keys,
        };
        let completed_ce = completed.to_cloud_event();
        assert_eq!(
            completed_ce.event_type,
            WorkflowEvent::WORKFLOW_CORRELATION_COMPLETED_TYPE
        );
        assert_eq!(completed_ce.data["name"], "wf-instance-1");
        assert!(completed_ce.data.get("completedAt").is_some());
        assert_eq!(completed_ce.data["correlationKeys"]["orderId"], "ORD-42");
    }

    #[test]
    fn test_collect_correlation_keys_helper() {
        use swf_core::models::event::CorrelationKeyDefinition;

        let mut correlate_a = HashMap::new();
        correlate_a.insert("k1".to_string(), CorrelationKeyDefinition::new(".a", None));
        let mut correlate_b = HashMap::new();
        correlate_b.insert("k2".to_string(), CorrelationKeyDefinition::new(".b", None));
        correlate_b.insert("k1".to_string(), CorrelationKeyDefinition::new(".a", None));

        let filters = vec![
            EventFilterDefinition {
                with: None,
                correlate: Some(correlate_a),
            },
            EventFilterDefinition {
                with: None,
                correlate: Some(correlate_b),
            },
        ];

        let collected = collect_correlation_keys(&filters);
        assert_eq!(collected.len(), 2, "should dedupe k1");
        assert!(collected.contains_key("k1"));
        assert!(collected.contains_key("k2"));
        // Initially, all declared keys should map to Null
        assert_eq!(collected.get("k1"), Some(&Value::Null));
    }

    #[test]
    fn test_evaluate_correlation_from_helper() {
        let event = CloudEvent::new(
            "com.example.test",
            json!({"orderId": "ORD-42", "nested": {"value": 99}}),
        );

        assert_eq!(
            evaluate_correlation_from(".orderId", &event),
            Some(json!("ORD-42"))
        );
        assert_eq!(
            evaluate_correlation_from(".nested.value", &event),
            Some(json!(99))
        );
        // Empty path returns the whole data
        assert_eq!(
            evaluate_correlation_from("", &event),
            Some(json!({"orderId": "ORD-42", "nested": {"value": 99}}))
        );
        // Missing path returns None
        assert_eq!(evaluate_correlation_from(".missing", &event), None);
    }
}
