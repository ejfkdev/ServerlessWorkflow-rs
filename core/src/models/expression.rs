use serde::{Deserialize, Serialize};

/// Represents a runtime expression following the Serverless Workflow DSL `${...}` syntax.
///
/// Runtime expressions are used throughout the specification to reference workflow
/// data, context, and other dynamic values at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RuntimeExpression(String);

impl RuntimeExpression {
    /// Creates a new RuntimeExpression from a string
    pub fn new(expr: &str) -> Self {
        RuntimeExpression(expr.to_string())
    }

    /// Creates a RuntimeExpression and normalizes it (adds `${}` if missing)
    pub fn normalized(expr: &str) -> Self {
        RuntimeExpression(normalize_expr(expr))
    }

    /// Returns the raw expression value as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Checks if the expression is in strict form (enclosed in `${ }`)
    pub fn is_strict(&self) -> bool {
        is_strict_expr(&self.0)
    }

    /// Checks if the expression appears to be syntactically valid.
    ///
    /// This performs basic structural validation (proper `${}` enclosure
    /// or bare expression form). Full jq syntax validation would require
    /// a jq parser.
    pub fn is_valid(&self) -> bool {
        is_valid_expr(&self.0)
    }

    /// Returns the expression content without the `${}` enclosure.
    /// If the expression is not in strict form, returns it as-is.
    pub fn sanitize(&self) -> String {
        sanitize_expr(&self.0)
    }

    /// Returns the expression in normalized form (with `${}` enclosure).
    pub fn normalize(&self) -> RuntimeExpression {
        RuntimeExpression(normalize_expr(&self.0))
    }
}

impl std::fmt::Display for RuntimeExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for RuntimeExpression {
    fn from(s: &str) -> Self {
        RuntimeExpression(s.to_string())
    }
}

impl From<String> for RuntimeExpression {
    fn from(s: String) -> Self {
        RuntimeExpression(s)
    }
}

impl AsRef<str> for RuntimeExpression {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Checks if the string is a strict runtime expression (enclosed in `${ }`)
pub fn is_strict_expr(expression: &str) -> bool {
    expression.starts_with("${") && expression.ends_with('}')
}

/// Sanitizes the expression by removing `${}` enclosure if present
/// and replacing single quotes with double quotes.
pub fn sanitize_expr(expression: &str) -> String {
    let mut expr = expression.to_string();

    // Remove `${}` enclosure if present, properly handling nested braces
    if expr.starts_with("${") && expr.ends_with('}') {
        // Count braces from the end to find the matching } for ${
        // Walk from the end backwards to find the outermost } that balances with ${
        let chars: Vec<char> = expr.chars().collect();
        let inner = &chars[2..chars.len() - 1]; // strip ${ and last }

        // Check if the inner content has balanced braces
        let mut depth = 0i32;
        let mut balanced = true;
        for &ch in inner {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                balanced = false;
                break;
            }
        }
        if depth != 0 {
            balanced = false;
        }

        if balanced {
            // Simple case: inner braces are balanced, just strip ${ and last }
            expr = expr[2..expr.len() - 1].trim().to_string();
        } else {
            // Complex case: the last } is part of an inner object, not the ${} closer
            // Find the true closing } for ${ by tracking depth from the beginning
            let mut depth = 0i32;
            let mut end_pos = None;
            for (i, &ch) in chars.iter().enumerate().skip(2) {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth < 0 {
                            end_pos = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(pos) = end_pos {
                expr = expr[2..pos].trim().to_string();
            }
        }
    }

    // Replace single-quoted strings with double-quoted strings,
    // but only when the single quotes denote a JQ string literal (not inside a double-quoted string).
    // We must NOT replace single quotes that appear inside double-quoted strings,
    // as they may be part of JQ string interpolation like "Hello '\(.name)'"
    expr = replace_single_quoted_strings(&expr);

    expr
}

/// Normalizes the expression by adding `${}` enclosure if not already present.
pub fn normalize_expr(expr: &str) -> String {
    if expr.starts_with("${") {
        expr.to_string()
    } else {
        format!("${{{}}}", expr)
    }
}

/// Performs basic structural validation of a runtime expression.
///
/// Checks that:
/// - If the expression starts with `${`, it must end with `}`
/// - The expression is not empty after sanitization
/// - Basic bracket matching
pub fn is_valid_expr(expression: &str) -> bool {
    if expression.is_empty() {
        return false;
    }

    // If starts with ${, must end with }
    if expression.starts_with("${") {
        if !expression.ends_with('}') {
            return false;
        }
        // Check for balanced braces inside
        let inner = &expression[2..expression.len() - 1];
        if inner.is_empty() {
            return false;
        }
        return has_balanced_brackets(inner);
    }

    // Non-strict form: just check it's not empty
    !expression.trim().is_empty()
}

/// Replaces single-quoted JQ string literals with double-quoted ones,
/// while preserving single quotes that appear inside double-quoted strings.
///
/// JQ uses single-quoted strings like `'hello'`, but jaq only supports double-quoted
/// strings. However, single quotes inside double-quoted strings (like `"it's"` or
/// `"'\(.name)'"`) must be preserved.
fn replace_single_quoted_strings(expr: &str) -> String {
    let mut result = String::with_capacity(expr.len());
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '"' => {
                // Inside a double-quoted string — copy everything as-is
                result.push('"');
                i += 1;
                while i < chars.len() {
                    result.push(chars[i]);
                    if chars[i] == '"' && (i == 0 || chars[i - 1] != '\\') {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
            '\'' => {
                // Start of a single-quoted string — replace with double quotes
                result.push('"');
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\'' && (i == 0 || chars[i - 1] != '\\') {
                        result.push('"');
                        i += 1;
                        break;
                    }
                    // Escape any double quotes inside the single-quoted string
                    if chars[i] == '"' {
                        result.push_str("\\\"");
                    } else {
                        result.push(chars[i]);
                    }
                    i += 1;
                }
            }
            _ => {
                result.push(chars[i]);
                i += 1;
            }
        }
    }

    result
}

/// Checks if brackets are balanced in an expression
fn has_balanced_brackets(expr: &str) -> bool {
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut escape_next = false;

    for ch in expr.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }
        if ch == '\\' {
            escape_next = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '{' | '(' | '[' => stack.push(ch),
            '}' if stack.pop() != Some('{') => return false,
            ')' if stack.pop() != Some('(') => return false,
            ']' if stack.pop() != Some('[') => return false,
            _ => {}
        }
    }

    stack.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_strict_expr() {
        assert!(is_strict_expr("${.foo}"));
        assert!(is_strict_expr("${ .foo.bar }"));
        assert!(!is_strict_expr(".foo"));
        assert!(!is_strict_expr("$ {.foo}"));
    }

    #[test]
    fn test_sanitize_expr() {
        assert_eq!(sanitize_expr("${.foo}"), ".foo");
        assert_eq!(sanitize_expr("${ .foo }"), ".foo");
        assert_eq!(sanitize_expr(".foo"), ".foo");
        assert_eq!(sanitize_expr("${.foo['bar']}"), ".foo[\"bar\"]");
    }

    #[test]
    fn test_normalize_expr() {
        assert_eq!(normalize_expr(".foo"), "${.foo}");
        assert_eq!(normalize_expr("${.foo}"), "${.foo}");
        assert_eq!(normalize_expr(" .foo "), "${ .foo }");
    }

    #[test]
    fn test_is_valid_expr() {
        assert!(is_valid_expr("${.foo}"));
        assert!(is_valid_expr("${.foo.bar}"));
        assert!(is_valid_expr(".foo"));
        assert!(!is_valid_expr(""));
        assert!(!is_valid_expr("${}"));
        assert!(!is_valid_expr("${.foo"));
        assert!(!is_valid_expr("${.foo]}"));
    }

    #[test]
    fn test_runtime_expression_new() {
        let expr = RuntimeExpression::new("${.foo}");
        assert_eq!(expr.as_str(), "${.foo}");
        assert!(expr.is_strict());
        assert!(expr.is_valid());
    }

    #[test]
    fn test_runtime_expression_normalized() {
        let expr = RuntimeExpression::normalized(".foo");
        assert_eq!(expr.as_str(), "${.foo}");
        assert!(expr.is_strict());
    }

    #[test]
    fn test_runtime_expression_sanitize() {
        let expr = RuntimeExpression::new("${.foo.bar}");
        assert_eq!(expr.sanitize(), ".foo.bar");
    }

    #[test]
    fn test_runtime_expression_normalize() {
        let expr = RuntimeExpression::new(".foo");
        let normalized = expr.normalize();
        assert_eq!(normalized.as_str(), "${.foo}");
    }

    #[test]
    fn test_runtime_expression_display() {
        let expr = RuntimeExpression::new("${.foo}");
        assert_eq!(format!("{}", expr), "${.foo}");
    }

    #[test]
    fn test_runtime_expression_from_str() {
        let expr: RuntimeExpression = "${.bar}".into();
        assert_eq!(expr.as_str(), "${.bar}");
    }

    #[test]
    fn test_runtime_expression_serde() {
        let expr = RuntimeExpression::new("${.foo}");
        let json = serde_json::to_string(&expr).unwrap();
        assert_eq!(json, "\"${.foo}\"");

        let deserialized: RuntimeExpression = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, expr);
    }

    #[test]
    fn test_balanced_brackets() {
        assert!(has_balanced_brackets(".foo.bar"));
        assert!(has_balanced_brackets(".foo[0]"));
        assert!(has_balanced_brackets(".foo[\"bar\"]"));
        assert!(has_balanced_brackets(".foo | {a: .b}"));
        assert!(!has_balanced_brackets(".foo[}"));
        assert!(!has_balanced_brackets(".foo]}"));
    }

    // Additional tests matching Go SDK's runtime_expression_test.go

    #[test]
    fn test_is_strict_expr_edge_cases() {
        // Matches Go SDK IsStrictExpr tests
        assert!(is_strict_expr("${.some.path}"), "strict expr with braces");
        assert!(!is_strict_expr("${.some.path"), "missing closing brace");
        assert!(!is_strict_expr(".some.path}"), "missing opening brace");
        assert!(!is_strict_expr(""), "empty string");
        assert!(!is_strict_expr(".some.path"), "no braces at all");
        assert!(
            is_strict_expr("${  .some.path   }"),
            "with spaces but still correct"
        );
        assert!(is_strict_expr("${}"), "only braces");
    }

    #[test]
    fn test_sanitize_expr_edge_cases() {
        // Matches Go SDK SanitizeExpr tests
        assert_eq!(
            sanitize_expr("${ 'some.path' }"),
            "\"some.path\"",
            "remove braces and replace single quotes"
        );
        assert_eq!(
            sanitize_expr(".some.path"),
            ".some.path",
            "already sanitized, no braces"
        );
        assert_eq!(
            sanitize_expr("${ 'foo' + 'bar' }"),
            "\"foo\" + \"bar\"",
            "multiple single quotes"
        );
        assert_eq!(sanitize_expr("${    }"), "", "only braces with spaces");
        assert_eq!(
            sanitize_expr("'some.path'"),
            "\"some.path\"",
            "no braces, just single quotes to replace"
        );
        assert_eq!(sanitize_expr(""), "", "nothing to sanitize");
    }

    #[test]
    fn test_is_valid_expr_edge_cases() {
        // Matches Go SDK IsValidExpr tests
        assert!(is_valid_expr("${ .foo }"), "valid expression - simple path");
        assert!(
            is_valid_expr("${ .arr[0] }"),
            "valid expression - array slice"
        );
        assert!(
            !is_valid_expr("${ .foo( }"),
            "invalid syntax - unbalanced parens"
        );
        assert!(is_valid_expr(".bar"), "no braces but valid JQ");
        assert!(!is_valid_expr(""), "empty expression");
        assert!(!is_valid_expr("${ .arr[ }"), "invalid bracket usage");
    }

    #[test]
    fn test_sanitize_expr_nested_object() {
        // Nested object literal inside ${} - the key test for balanced braces handling
        assert_eq!(
            sanitize_expr("${ {a:1, b:2, c:3} | del(.a,.c) }"),
            "{a:1, b:2, c:3} | del(.a,.c)"
        );
        assert_eq!(
            sanitize_expr("${ {processed: {colors: [], indexes: []}} }"),
            "{processed: {colors: [], indexes: []}}"
        );
    }

    #[test]
    fn test_sanitize_expr_nested_object_with_pipe() {
        // Object with pipe operator
        assert_eq!(sanitize_expr("${ {x: .foo} | .x }"), "{x: .foo} | .x");
    }

    #[test]
    fn test_sanitize_expr_simple_vs_complex() {
        // Simple expression: inner braces are balanced → strip ${ and last }
        assert_eq!(sanitize_expr("${ .foo.bar }"), ".foo.bar");
        // Complex expression: inner braces unbalanced → use depth tracking
        assert_eq!(sanitize_expr("${ .foo | {a: .b} }"), ".foo | {a: .b}");
    }

    #[test]
    fn test_sanitize_expr_deeply_nested() {
        // Deeply nested objects
        assert_eq!(sanitize_expr("${ {a: {b: {c: 1}}} }"), "{a: {b: {c: 1}}}");
    }

    #[test]
    fn test_sanitize_expr_if_then_else_object() {
        // if-then-else returning an object
        assert_eq!(
            sanitize_expr("${ if .x then {a: 1} else {b: 2} end }"),
            "if .x then {a: 1} else {b: 2} end"
        );
    }
}
