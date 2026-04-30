use crate::error::WorkflowResult;
use crate::task_runner::{TaskRunner, TaskSupport};
use crate::tasks::task_name_impl;
use crate::tasks::DoTaskRunner;
use serde_json::Value;
use std::collections::HashMap;
use swf_core::models::input::InputDataModelDefinition;
use swf_core::models::task::{DoTaskDefinition, ForTaskDefinition};

/// Runner for For tasks - iterates over collections
pub struct ForTaskRunner {
    name: String,
    each: String,
    at: String,
    in_expr: String,
    while_expr: Option<String>,
    do_tasks: DoTaskRunner,
    for_input: Option<InputDataModelDefinition>,
}

impl ForTaskRunner {
    pub fn new(name: &str, task: &ForTaskDefinition) -> WorkflowResult<Self> {
        let each = crate::utils::ensure_dollar_prefix(&task.for_.each, "$item");
        let at =
            crate::utils::ensure_dollar_prefix(task.for_.at.as_deref().unwrap_or(""), "$index");

        let do_tasks = DoTaskRunner::new(name, &DoTaskDefinition::new(task.do_.clone()))?;

        Ok(Self {
            name: name.to_string(),
            each,
            at,
            in_expr: task.for_.in_.clone(),
            while_expr: task.while_.clone(),
            do_tasks,
            for_input: task.for_.input.clone(),
        })
    }
}

#[async_trait::async_trait]
impl TaskRunner for ForTaskRunner {
    async fn run(&self, input: Value, support: &mut TaskSupport<'_>) -> WorkflowResult<Value> {
        // Evaluate the 'in' expression to get the collection
        let collection = support.eval_jq_expr(&self.in_expr, &input, &self.name)?;

        // Apply for-input transformation once before iteration starts
        // This matches Java SDK behavior where task input.from is applied once to the for-loop's overall input
        let for_input = if self.for_input.is_some() {
            support.process_task_input(self.for_input.as_ref(), &input, &self.name)?
        } else {
            input
        };

        let mut for_output = for_input;

        match &collection {
            Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    let mut for_vars = HashMap::new();
                    for_vars.insert(self.at.clone(), Value::from(i as i64));
                    for_vars.insert(self.each.clone(), item.clone());

                    support.add_local_expr_vars(for_vars);

                    for_output = self.do_tasks.run(for_output, support).await?;

                    // Check while condition BEFORE removing local vars ($item, $index)
                    // so that while expressions can reference them
                    let should_break = if let Some(ref while_expr) = self.while_expr {
                        let should_continue = support.eval_bool(while_expr, &for_output)?;
                        !should_continue
                    } else {
                        false
                    };

                    support.remove_local_expr_vars(&[&self.at, &self.each]);

                    if should_break {
                        break;
                    }
                }
            }
            Value::Null => {
                // Null collection, no iteration
            }
            other => {
                // Single value iteration
                let mut for_vars = HashMap::new();
                for_vars.insert(self.at.clone(), Value::from(0i64));
                for_vars.insert(self.each.clone(), other.clone());

                support.add_local_expr_vars(for_vars);
                for_output = self.do_tasks.run(for_output, support).await?;
                support.remove_local_expr_vars(&[&self.at, &self.each]);
            }
        }

        Ok(for_output)
    }

    task_name_impl!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::WorkflowContext;
    use crate::default_support;
    use serde_json::json;
    use std::collections::HashMap;
    use swf_core::models::map::Map;
    use swf_core::models::task::{
        ForLoopDefinition, ForTaskDefinition, SetTaskDefinition, SetValue, TaskDefinition,
        TaskDefinitionFields,
    };
    use swf_core::models::workflow::WorkflowDefinition;

    #[tokio::test]
    async fn test_for_basic_iteration() {
        let mut set_map = HashMap::new();
        set_map.insert("total".to_string(), json!("${ .total + $item }"));
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("addNumber".to_string(), set_task)];

        let for_def = ForLoopDefinition {
            in_: "${ .numbers }".to_string(),
            each: "item".to_string(),
            at: None,
            input: None,
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("sumLoop", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner
            .run(
                json!({"numbers": [1, 2, 3, 4, 5], "total": 0}),
                &mut support,
            )
            .await
            .unwrap();
        assert_eq!(output["total"], json!(15));
    }

    #[tokio::test]
    async fn test_for_with_custom_each_and_at() {
        let mut set_map = HashMap::new();
        set_map.insert("result".to_string(), json!("${ .result + [$fruit] }"));
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("collectFruit".to_string(), set_task)];

        let for_def = ForLoopDefinition {
            in_: "${ .fruits }".to_string(),
            each: "fruit".to_string(),
            at: Some("fruitIdx".to_string()),
            input: None,
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("fruitLoop", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner
            .run(
                json!({"fruits": ["apple", "banana"], "result": []}),
                &mut support,
            )
            .await
            .unwrap();
        assert_eq!(output["result"], json!(["apple", "banana"]));
    }

    #[tokio::test]
    async fn test_for_empty_collection() {
        let mut set_map = HashMap::new();
        set_map.insert("touched".to_string(), json!(true));
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("inner".to_string(), set_task)];

        let for_def = ForLoopDefinition {
            in_: "${ .items }".to_string(),
            each: "item".to_string(),
            at: None,
            input: None,
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("emptyLoop", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"items": [], "original": "data"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["original"], json!("data"));
        assert!(output.get("touched").is_none());
    }

    #[tokio::test]
    async fn test_for_with_while_condition() {
        let mut set_map = HashMap::new();
        set_map.insert("total".to_string(), json!("${ .total + $item }"));
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("addNumber".to_string(), set_task)];

        let for_def = ForLoopDefinition {
            in_: "${ .numbers }".to_string(),
            each: "item".to_string(),
            at: None,
            input: None,
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: Some("${ .total < 10 }".to_string()),
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("whileLoop", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"numbers": [3, 5, 7, 9], "total": 0}), &mut support)
            .await
            .unwrap();
        // After 3: total=3 (<10 continue), after 5: total=8 (<10 continue), after 7: total=15 (>=10 stop)
        assert_eq!(output["total"], json!(15));
    }

    #[tokio::test]
    async fn test_for_with_at_index() {
        // Matches Java SDK's for-collect - using index variable to build indexed output
        let mut set_map = HashMap::new();
        set_map.insert("indices".to_string(), json!("${ .indices + [$index] }"));
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("collectIndex".to_string(), set_task)];

        let for_def = ForLoopDefinition {
            in_: "${ .input }".to_string(),
            each: "number".to_string(),
            at: Some("index".to_string()),
            input: None,
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("collectLoop", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"input": [10, 20, 30], "indices": []}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["indices"], json!([0, 1, 2]));
    }

    #[tokio::test]
    async fn test_for_single_value_iteration() {
        // Test iterating over a non-array value (single value)
        let mut set_map = HashMap::new();
        set_map.insert("collected".to_string(), json!("${ [$value] }"));
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("collectVal".to_string(), set_task)];

        let for_def = ForLoopDefinition {
            in_: "${ .single }".to_string(),
            each: "value".to_string(),
            at: None,
            input: None,
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("singleValLoop", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"single": "hello"}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["collected"], json!(["hello"]));
    }

    #[tokio::test]
    async fn test_for_with_at_index_accumulate() {
        // Matches Java SDK's for-sum - sum values using each and at
        let mut set_map = HashMap::new();
        set_map.insert("counter".to_string(), json!("${ .counter + $item }"));
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("accumulate".to_string(), set_task)];

        let for_def = ForLoopDefinition {
            in_: "${ .input }".to_string(),
            each: "item".to_string(),
            at: None,
            input: None,
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("sumLoop", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner
            .run(
                json!({"input": [1, 2, 3, 4, 5], "counter": 0}),
                &mut support,
            )
            .await
            .unwrap();
        assert_eq!(output["counter"], json!(15));
    }

    #[tokio::test]
    async fn test_for_colors_with_index() {
        // Matches Go SDK's for_colors.yaml - loop over colors, collect with index
        let mut set_map = HashMap::new();
        set_map.insert("processed".to_string(), json!("${ {colors: (.processed.colors + [$color]), indexes: (.processed.indexes + [$index])} }"));
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("markProcessed".to_string(), set_task)];

        let for_def = ForLoopDefinition {
            in_: "${ .colors }".to_string(),
            each: "color".to_string(),
            at: None, // uses default $index
            input: None,
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("loopColors", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner.run(json!({"colors": ["red", "green", "blue"], "processed": {"colors": [], "indexes": []}}), &mut support).await.unwrap();
        assert_eq!(
            output["processed"]["colors"],
            json!(["red", "green", "blue"])
        );
        assert_eq!(output["processed"]["indexes"], json!([0, 1, 2]));
    }

    #[tokio::test]
    async fn test_for_sum_numbers() {
        // Matches Go SDK's for_sum_numbers.yaml - sum all numbers in array
        let mut set_map = HashMap::new();
        set_map.insert("total".to_string(), json!("${ .total + $number }"));
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("sumNumber".to_string(), set_task)];

        let for_def = ForLoopDefinition {
            in_: "${ .numbers }".to_string(),
            each: "number".to_string(),
            at: None,
            input: None,
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("sumNumbers", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner
            .run(
                json!({"numbers": [10, 20, 30, 40], "total": 0}),
                &mut support,
            )
            .await
            .unwrap();
        assert_eq!(output["total"], json!(100));
    }

    #[tokio::test]
    async fn test_for_nested_loops() {
        // Test nested for loops where inner loop references outer loop variable
        // Outer: iterate over prefixes, Inner: iterate over suffixes
        // Uses only loop variables ($prefix, $suffix) in the set expression
        let mut set_map = HashMap::new();
        set_map.insert(
            "combined".to_string(),
            json!("${ .combined + [($prefix + \"-\" + $suffix)] }"),
        );
        let inner_set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let inner_do_entries = vec![("pair".to_string(), inner_set_task)];

        // Inner loop iterates over a simple array with just 2 items
        // We can't use .suffixes since Set replaces output, so we use a variable-based approach
        let inner_for_def = ForLoopDefinition {
            in_: "${ [\"X\", \"Y\"] }".to_string(),
            each: "suffix".to_string(),
            at: None,
            input: None,
        };

        let inner_for_task = TaskDefinition::For(ForTaskDefinition {
            for_: inner_for_def,
            do_: Map {
                entries: inner_do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        });

        let outer_do_entries = vec![("innerLoop".to_string(), inner_for_task)];

        let outer_for_def = ForLoopDefinition {
            in_: "${ [\"A\", \"B\"] }".to_string(),
            each: "prefix".to_string(),
            at: None,
            input: None,
        };

        let task = ForTaskDefinition {
            for_: outer_for_def,
            do_: Map {
                entries: outer_do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("nestedLoop", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"combined": []}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["combined"], json!(["A-X", "A-Y", "B-X", "B-Y"]));
    }

    #[tokio::test]
    async fn test_for_nested_loops_with_input_variable() {
        // Matches Go SDK's for_nested_loops.yaml - inner loop uses $input.colors
        // to reference original input, since Set replaces output each iteration
        let mut set_map = HashMap::new();
        set_map.insert(
            "matrix".to_string(),
            json!("${ .matrix + [[$fruit, $color]] }"),
        );
        let inner_set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let inner_do_entries = vec![("combinePair".to_string(), inner_set_task)];

        // Inner loop uses $input.colors to access original input
        let inner_for_def = ForLoopDefinition {
            in_: "${ $input.colors }".to_string(),
            each: "color".to_string(),
            at: Some("colorIdx".to_string()),
            input: None,
        };

        let inner_for_task = TaskDefinition::For(ForTaskDefinition {
            for_: inner_for_def,
            do_: Map {
                entries: inner_do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        });

        let outer_do_entries = vec![("innerLoop".to_string(), inner_for_task)];

        let outer_for_def = ForLoopDefinition {
            in_: "${ .fruits }".to_string(),
            each: "fruit".to_string(),
            at: Some("fruitIdx".to_string()),
            input: None,
        };

        let task = ForTaskDefinition {
            for_: outer_for_def,
            do_: Map {
                entries: outer_do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("nestedLoopInput", &task).unwrap();

        let input = json!({
            "fruits": ["apple", "banana"],
            "colors": ["red", "green"],
            "matrix": []
        });
        let mut context = WorkflowContext::new(&workflow).unwrap();
        context.set_input(input.clone());
        let mut support = TaskSupport::new(&workflow, &mut context);

        let output = runner.run(input, &mut support).await.unwrap();
        assert_eq!(
            output["matrix"],
            json!([
                ["apple", "red"],
                ["apple", "green"],
                ["banana", "red"],
                ["banana", "green"]
            ])
        );
    }

    #[tokio::test]
    async fn test_for_collect_with_input_from() {
        // Matches Java SDK's for-collect.yaml
        // Uses for.input.from to reset iteration input each time,
        // collecting results into an output array
        let mut set_map = HashMap::new();
        set_map.insert(
            "output".to_string(),
            json!("${ .output + [ $number + $index + 1 ] }"),
        );
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("sumIndex".to_string(), set_task)];

        let for_input = swf_core::models::input::InputDataModelDefinition {
            from: Some(json!("${ {input: .input, output: []} }")),
            schema: None,
        };

        let for_def = ForLoopDefinition {
            in_: ".input".to_string(),
            each: "number".to_string(),
            at: Some("index".to_string()),
            input: Some(for_input),
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("sumAll", &task).unwrap();

        default_support!(workflow, context, support);

        // input=[1,2,3]: each iteration resets to {input:[1,2,3], output:[]}
        // iter 0: output += 1+0+1 = [2], iter 1: output += 2+1+1 = [2,4], iter 2: output += 3+2+1 = [2,4,6]
        let output = runner
            .run(json!({"input": [1, 2, 3]}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["output"], json!([2, 4, 6]));
    }

    #[tokio::test]
    async fn test_for_collect_transform_items() {
        // For-collect pattern: transform each item in collection
        // Uses input.from to pass both the original collection and accumulator
        let mut set_map = HashMap::new();
        set_map.insert(
            "results".to_string(),
            json!("${ .results + [ $item * 2 ] }"),
        );
        let set_task = TaskDefinition::Set(SetTaskDefinition {
            set: SetValue::Map(set_map),
            common: TaskDefinitionFields::new(),
        });

        let do_entries = vec![("doubleIt".to_string(), set_task)];

        let for_input = swf_core::models::input::InputDataModelDefinition {
            from: Some(json!("${ {items: .items, results: []} }")),
            schema: None,
        };

        let for_def = ForLoopDefinition {
            in_: "${ .items }".to_string(),
            each: "item".to_string(),
            at: None,
            input: Some(for_input),
        };

        let task = ForTaskDefinition {
            for_: for_def,
            do_: Map {
                entries: do_entries,
            },
            while_: None,
            common: TaskDefinitionFields::new(),
        };

        let workflow = WorkflowDefinition::default();
        let runner = ForTaskRunner::new("doubleAll", &task).unwrap();

        default_support!(workflow, context, support);

        let output = runner
            .run(json!({"items": [1, 2, 3, 4, 5]}), &mut support)
            .await
            .unwrap();
        assert_eq!(output["results"], json!([2, 4, 6, 8, 10]));
    }
}
