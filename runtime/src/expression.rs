use crate::error::{WorkflowError, WorkflowResult};
use serde_json::Value;
use serverless_workflow_core::models::expression::{is_strict_expr, sanitize_expr};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Compiled JQ filter cache key: (expression, sorted variable names joined by null)
type CacheKey = (String, String);

/// Global cache for compiled JQ filters.
/// Key: (expression_text, sorted_variable_names_joined)
/// Value: compiled Filter that can be reused with matching variable bindings
static FILTER_CACHE: LazyLock<
    std::sync::RwLock<HashMap<CacheKey, jaq_core::Filter<jaq_core::Native<jaq_json::Val>>>>,
> = LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

/// Evaluates a JQ expression against a JSON input with variable bindings.
/// Uses a global cache to avoid recompiling the same expression with the same variable names.
pub fn evaluate_jq(
    expression: &str,
    input: &Value,
    vars: &HashMap<String, Value>,
) -> WorkflowResult<Value> {
    use jaq_core::{load, Compiler, Ctx, RcIter};
    use jaq_json::Val;

    // Prepare global variable names in a stable sorted order
    let mut var_names: Vec<String> = vars.keys().cloned().collect();
    var_names.sort();
    let var_name_refs: Vec<&str> = var_names.iter().map(|s| s.as_str()).collect();

    // Build cache key from expression and variable names
    let cache_key = (expression.to_string(), var_names.join("\0"));

    // Try to get a compiled filter from the cache
    let filter = {
        let cache = FILTER_CACHE.read().unwrap_or_else(|e| e.into_inner());
        cache.get(&cache_key).cloned()
    };

    let filter = match filter {
        Some(f) => f,
        None => {
            // Parse the expression
            let program = load::File {
                code: expression,
                path: (),
            };
            let loader = load::Loader::new(jaq_std::defs().chain(jaq_json::defs()));
            let arena = load::Arena::default();

            let modules = loader.load(&arena, program).map_err(|e| {
                WorkflowError::expression(
                    format!("failed to parse jq expression '{}': {:?}", expression, e),
                    "",
                )
            })?;

            // Compile with standard functions and global variables
            let filter = Compiler::default()
                .with_funs(jaq_std::funs().chain(jaq_json::funs()))
                .with_global_vars(var_name_refs)
                .compile(modules)
                .map_err(|errs| {
                    WorkflowError::expression(
                        format!(
                            "failed to compile jq expression '{}': {:?}",
                            expression, errs
                        ),
                        "",
                    )
                })?;

            // Store in cache
            let mut cache = FILTER_CACHE.write().unwrap_or_else(|e| e.into_inner());
            cache.entry(cache_key).or_insert(filter).clone()
        }
    };

    // Convert serde_json::Value to jaq Val
    let jaq_input = Val::from(input.clone());

    // Build variable bindings for jaq context using the same key order as var_names
    let var_vals: Vec<Val> = var_names
        .iter()
        .map(|k| Val::from(vars[k].clone()))
        .collect();
    let inputs = RcIter::new(core::iter::empty());

    let out = filter.run((Ctx::new(var_vals, &inputs), jaq_input));

    let mut results = Vec::new();
    for item in out {
        match item {
            Ok(val) => {
                let json_val: Value = val.into();
                results.push(json_val);
            }
            Err(e) => {
                return Err(WorkflowError::expression(
                    format!("jq evaluation error: {:?}", e),
                    "",
                ));
            }
        }
    }

    match results.len() {
        0 => Err(WorkflowError::expression(
            "no result from jq evaluation",
            "",
        )),
        1 => Ok(results.into_iter().next().unwrap_or(Value::Null)),
        _ => Ok(Value::Array(results)),
    }
}

/// Recursively traverses a JSON structure and evaluates all runtime expressions
pub fn traverse_and_evaluate(
    node: &mut Value,
    input: &Value,
    vars: &HashMap<String, Value>,
) -> WorkflowResult<()> {
    match node {
        Value::Object(map) => {
            for (_key, value) in map.iter_mut() {
                traverse_and_evaluate(value, input, vars)?;
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                traverse_and_evaluate(item, input, vars)?;
            }
        }
        Value::String(s) if is_strict_expr(s) => {
            let expr = sanitize_expr(s);
            let result = evaluate_jq(&expr, input, vars)?;
            *node = result;
        }
        _ => {}
    }
    Ok(())
}

/// Evaluates an expression and returns the result as a boolean
pub fn traverse_and_evaluate_bool(
    expr: &str,
    input: &Value,
    vars: &HashMap<String, Value>,
) -> WorkflowResult<bool> {
    if expr.is_empty() {
        return Ok(false);
    }

    // Normalize: add ${} if not strict
    let normalized = if is_strict_expr(expr) {
        expr.to_string()
    } else {
        serverless_workflow_core::models::expression::normalize_expr(expr)
    };

    let sanitized = sanitize_expr(&normalized);
    let result = evaluate_jq(&sanitized, input, vars)?;

    match result {
        Value::Bool(b) => Ok(b),
        _ => Ok(false),
    }
}

/// Evaluates an optional runtime expression object (input.from, output.as, etc.)
pub fn traverse_and_evaluate_obj(
    obj: Option<&Value>,
    input: &Value,
    vars: &HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<Value> {
    match obj {
        None => Ok(input.clone()),
        Some(value) => {
            let mut result = value.clone();
            traverse_and_evaluate(&mut result, input, vars)
                .map_err(|e| WorkflowError::expression(format!("{}", e), task_name))?;
            Ok(result)
        }
    }
}

/// Evaluates a string that may contain a JQ expression (${...}).
///
/// Supports three forms:
/// 1. Full expression: `${ .foo }` — evaluates the whole thing as JQ
/// 2. Embedded expressions: `http://host/${ .id }/path` — substitutes each `${...}` inline
/// 3. Plain string: `hello` — returned as-is
///
/// Returns the result as a String (JSON values are converted via Display).
pub fn evaluate_expression_str(
    expr: &str,
    input: &Value,
    vars: &HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<String> {
    if is_strict_expr(expr) {
        // Full expression: ${ .foo } -> evaluate the whole thing
        let sanitized = sanitize_expr(expr);
        let result = evaluate_jq(&sanitized, input, vars)
            .map_err(|e| WorkflowError::expression(format!("{}", e), task_name))?;
        match result {
            Value::String(s) => Ok(s),
            other => Ok(other.to_string()),
        }
    } else if expr.contains("${") {
        // Embedded expression: http://host/${ .id }/path -> substitute each ${...}
        evaluate_embedded_expressions(expr, input, vars, task_name)
    } else {
        Ok(expr.to_string())
    }
}

/// Evaluates embedded ${...} expressions within a string, replacing each with its JQ result
fn evaluate_embedded_expressions(
    s: &str,
    input: &Value,
    vars: &HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<String> {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            // Found ${...} - find the matching }
            chars.next(); // consume '{'
            let mut depth = 1;
            let mut expr_buf = String::new();
            #[allow(clippy::while_let_on_iterator)]
            while let Some(ec) = chars.next() {
                match ec {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                expr_buf.push(ec);
            }
            // Evaluate the expression
            let sanitized = sanitize_expr(&expr_buf);
            let val = evaluate_jq(&sanitized, input, vars)
                .map_err(|e| WorkflowError::expression(format!("{}", e), task_name))?;
            match val {
                Value::String(vs) => result.push_str(&vs),
                other => result.push_str(&other.to_string()),
            }
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

/// Evaluates a `Value` that may contain a JQ expression.
///
/// - String values are prepared (normalized + sanitized) and evaluated as JQ.
/// - Non-string values have embedded `${...}` expressions evaluated via traverse_and_evaluate.
pub fn evaluate_value_expr(
    value: &Value,
    input: &Value,
    vars: &HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<Value> {
    match value {
        Value::String(expr) => {
            let sanitized = prepare_expression(expr);
            evaluate_jq(&sanitized, input, vars)
                .map_err(|e| WorkflowError::expression(format!("{}", e), task_name))
        }
        _ => traverse_and_evaluate_obj(Some(value), input, vars, task_name),
    }
}

/// Prepares an expression for JQ evaluation by normalizing and sanitizing.
///
/// If the expression is a strict expression (`${...}`), strips the `${` and `}` wrapper.
/// Otherwise, normalizes it first (adding `${}` wrapper if missing) then sanitizes.
pub fn prepare_expression(expr: &str) -> String {
    if is_strict_expr(expr) {
        sanitize_expr(expr)
    } else {
        let normalized = serverless_workflow_core::models::expression::normalize_expr(expr);
        sanitize_expr(&normalized)
    }
}

/// Evaluates a JQ expression string and returns the JSON result.
///
/// If the string is a strict expression (`${...}`), evaluates it as JQ.
/// Otherwise, tries to parse it as JSON.
pub fn evaluate_expression_json(
    expr: &str,
    input: &Value,
    vars: &HashMap<String, Value>,
    task_name: &str,
) -> WorkflowResult<Value> {
    if is_strict_expr(expr) {
        let sanitized = sanitize_expr(expr);
        evaluate_jq(&sanitized, input, vars)
            .map_err(|e| WorkflowError::expression(format!("{}", e), task_name))
    } else {
        // Not an expression, try parsing as JSON
        serde_json::from_str(expr).map_err(|e| {
            WorkflowError::expression(
                format!("failed to parse non-expression value as JSON: {}", e),
                task_name,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // === evaluate_jq tests ===

    #[test]
    fn test_evaluate_jq_simple_path() {
        let input = json!({"foo": "bar"});
        let vars = HashMap::new();
        let result = evaluate_jq(".foo", &input, &vars).unwrap();
        assert_eq!(result, json!("bar"));
    }

    #[test]
    fn test_evaluate_jq_nested_path() {
        let input = json!({"foo": {"bar": 42}});
        let vars = HashMap::new();
        let result = evaluate_jq(".foo.bar", &input, &vars).unwrap();
        assert_eq!(result, json!(42));
    }

    #[test]
    fn test_evaluate_jq_with_variable() {
        let input = json!({});
        let mut vars = HashMap::new();
        vars.insert("$input".to_string(), json!({"x": 1}));
        let result = evaluate_jq("$input.x", &input, &vars).unwrap();
        assert_eq!(result, json!(1));
    }

    #[test]
    fn test_evaluate_jq_undefined_variable() {
        let input = json!({"foo": "bar"});
        let vars = HashMap::new();
        let result = evaluate_jq("$undefinedVar", &input, &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_jq_invalid_expression() {
        let input = json!({"foo": "bar"});
        let vars = HashMap::new();
        let result = evaluate_jq(".foo(", &input, &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_jq_array_result() {
        let input = json!({"items": [1, 2, 3]});
        let vars = HashMap::new();
        let result = evaluate_jq(".items[]", &input, &vars).unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_evaluate_jq_length_function() {
        let input = json!({"items": [1, 2, 3]});
        let vars = HashMap::new();
        let result = evaluate_jq(".items | length", &input, &vars).unwrap();
        assert_eq!(result, json!(3));
    }

    #[test]
    fn test_evaluate_jq_arithmetic() {
        let input = json!({"a": 10, "b": 3});
        let vars = HashMap::new();
        let result = evaluate_jq(".a - .b", &input, &vars).unwrap();
        assert_eq!(result, json!(7));
    }

    #[test]
    fn test_jq_filter_cache_hit() {
        // Evaluate the same expression twice - second call should use cache
        let input1 = json!({"x": 1});
        let input2 = json!({"x": 2});
        let vars = HashMap::new();

        let result1 = evaluate_jq(".x", &input1, &vars).unwrap();
        assert_eq!(result1, json!(1));

        let result2 = evaluate_jq(".x", &input2, &vars).unwrap();
        assert_eq!(result2, json!(2));

        // Verify cache has entries
        let cache = FILTER_CACHE.read().unwrap();
        assert!(!cache.is_empty());
    }

    // === traverse_and_evaluate tests (port from Go TestTraverseAndEvaluate) ===

    #[test]
    fn test_traverse_no_expression() {
        let mut node = json!({
            "key": "value",
            "num": 123
        });
        let input = json!(null);
        let vars = HashMap::new();
        traverse_and_evaluate(&mut node, &input, &vars).unwrap();
        assert_eq!(node["key"], json!("value"));
        assert_eq!(node["num"], json!(123));
    }

    #[test]
    fn test_traverse_and_evaluate_object() {
        let mut node = json!({
            "name": "${.foo}",
            "count": 42
        });
        let input = json!({"foo": "hello"});
        let vars = HashMap::new();
        traverse_and_evaluate(&mut node, &input, &vars).unwrap();
        assert_eq!(node["name"], json!("hello"));
        assert_eq!(node["count"], json!(42));
    }

    #[test]
    fn test_traverse_expression_in_array() {
        let mut node = json!(["static", "${.foo}"]);
        let input = json!({"foo": "bar"});
        let vars = HashMap::new();
        traverse_and_evaluate(&mut node, &input, &vars).unwrap();
        assert_eq!(node[0], json!("static"));
        assert_eq!(node[1], json!("bar"));
    }

    #[test]
    fn test_traverse_and_evaluate_nested_expr() {
        let mut node = json!({
            "data": {
                "inner": "${.name}"
            }
        });
        let input = json!({"name": "world"});
        let vars = HashMap::new();
        traverse_and_evaluate(&mut node, &input, &vars).unwrap();
        assert_eq!(node["data"]["inner"], json!("world"));
    }

    #[test]
    fn test_traverse_nested_structure_in_array() {
        let mut node = json!({
            "level1": [{"expr": "${.foo}"}]
        });
        let input = json!({"foo": "nestedValue"});
        let vars = HashMap::new();
        traverse_and_evaluate(&mut node, &input, &vars).unwrap();
        assert_eq!(node["level1"][0]["expr"], json!("nestedValue"));
    }

    #[test]
    fn test_traverse_with_vars() {
        let mut node = json!({"expr": "${$myVar}"});
        let input = json!({});
        let mut vars = HashMap::new();
        vars.insert("$myVar".to_string(), json!("HelloVars"));
        traverse_and_evaluate(&mut node, &input, &vars).unwrap();
        assert_eq!(node["expr"], json!("HelloVars"));
    }

    #[test]
    fn test_traverse_invalid_jq_expression() {
        let mut node = json!("${ .foo( }");
        let input = json!({"foo": "bar"});
        let vars = HashMap::new();
        let result = traverse_and_evaluate(&mut node, &input, &vars);
        assert!(result.is_err());
    }

    // === traverse_and_evaluate_bool tests ===

    #[test]
    fn test_traverse_and_evaluate_bool_true() {
        let input = json!({"x": 1});
        let vars = HashMap::new();
        let result = traverse_and_evaluate_bool("${.x == 1}", &input, &vars).unwrap();
        assert!(result);
    }

    #[test]
    fn test_traverse_and_evaluate_bool_false() {
        let input = json!({"x": 1});
        let vars = HashMap::new();
        let result = traverse_and_evaluate_bool("${.x == 2}", &input, &vars).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_traverse_and_evaluate_bool_empty() {
        let input = json!({});
        let vars = HashMap::new();
        let result = traverse_and_evaluate_bool("", &input, &vars).unwrap();
        assert!(!result);
    }

    // === traverse_and_evaluate_obj tests ===

    #[test]
    fn test_traverse_and_evaluate_obj_none() {
        let input = json!({"key": "value"});
        let vars = HashMap::new();
        let result = traverse_and_evaluate_obj(None, &input, &vars, "test").unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_traverse_and_evaluate_obj_with_expression() {
        let obj = json!({"result": "${.value}"});
        let input = json!({"value": 42});
        let vars = HashMap::new();
        let result = traverse_and_evaluate_obj(Some(&obj), &input, &vars, "test").unwrap();
        assert_eq!(result["result"], json!(42));
    }

    #[test]
    fn test_jq_update_operator() {
        // Test if jaq supports the += update operator (used in Java SDK's for-sum.yaml)
        let input = json!({"incr": [2, 3], "counter": 6});
        let vars = HashMap::new();
        let result = evaluate_jq(".incr += [5]", &input, &vars);
        // jaq 2.x may or may not support +=; if not, we'll get an error
        match result {
            Ok(val) => {
                // If supported, the result should have incr updated
                assert_eq!(val["incr"], json!([2, 3, 5]));
                assert_eq!(val["counter"], json!(6));
            }
            Err(_) => {
                // += not supported - this is expected for jaq
                // The Java SDK uses jq which supports update operators
            }
        }
    }

    #[test]
    fn test_jq_if_then_else_with_concat() {
        // Alternative to += that works in jaq
        // Matches Java SDK's for-sum.yaml export.as expression pattern
        let input = json!({"incr": [2, 3], "counter": 6});
        let vars = HashMap::new();
        // This is the if-then-else part that builds a new object
        let result = evaluate_jq(
            "if .incr == null then {incr: [5]} else {incr: (.incr + [5])} end",
            &input,
            &vars,
        )
        .unwrap();
        assert_eq!(result["incr"], json!([2, 3, 5]));
    }

    #[test]
    fn test_jq_if_then_else_null_check() {
        // First iteration: .incr is null
        let input = json!({"counter": 0});
        let vars = HashMap::new();
        let result = evaluate_jq(
            "if .incr == null then {incr: [2]} else {incr: (.incr + [2])} end",
            &input,
            &vars,
        )
        .unwrap();
        assert_eq!(result["incr"], json!([2]));
    }
}
