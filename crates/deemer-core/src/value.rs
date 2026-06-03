//! The universal variable value (`serde_json::Value`) and conversions used by
//! templating, expression evaluation, and AI reply parsing.
//!
//! Every variable Deemer exposes — captured output, data fields, AI outputs —
//! is a [`serde_json::Value`]. This module bridges those values to the CEL
//! interpreter (for asserts) and renders them as display literals (for logs).

use serde_json::Value;

/// The Deemer type name for a value, matching the `expected_outputs` vocabulary.
pub fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "list",
        Value::Object(_) => "object",
        Value::Null => "null",
    }
}

/// Maximum number of characters of a string value shown in a substituted-expression
/// log line before it is elided. The full value always lives in `execution`.
const MAX_LITERAL: usize = 60;

/// Render a value as a typed literal for the `substituted` field of the results log.
///
/// Strings are quoted (and elided with `…` when long); numbers, booleans, null and
/// arrays render bare. This is for display only — it is never parsed or evaluated.
pub fn value_to_literal(v: &Value) -> String {
    match v {
        Value::String(s) if s.chars().count() > MAX_LITERAL => {
            let truncated: String = s.chars().take(MAX_LITERAL).collect();
            format!("\"{truncated}…\"")
        }
        Value::String(s) => format!("{s:?}"), // quoted + escaped
        other => other.to_string(),
    }
}

/// Convert a [`serde_json::Value`] into a [`cel_interpreter::Value`] for binding
/// into an assert's evaluation context.
pub fn json_to_cel(v: &Value) -> cel_interpreter::Value {
    use cel_interpreter::Value as Cel;
    match v {
        Value::Null => Cel::Null,
        Value::Bool(b) => Cel::Bool(*b),
        Value::Number(n) if n.is_i64() => Cel::Int(n.as_i64().unwrap()),
        Value::Number(n) if n.is_u64() => Cel::UInt(n.as_u64().unwrap()),
        Value::Number(n) => Cel::Float(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => Cel::String(s.clone().into()),
        Value::Array(a) => Cel::List(a.iter().map(json_to_cel).collect::<Vec<_>>().into()),
        // Objects aren't used as scalar assert operands in v1.
        Value::Object(_) => Cel::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn type_names() {
        assert_eq!(type_name(&json!(true)), "boolean");
        assert_eq!(type_name(&json!(3)), "number");
        assert_eq!(type_name(&json!("x")), "string");
        assert_eq!(type_name(&json!([1, 2])), "list");
        assert_eq!(type_name(&json!({"a": 1})), "object");
        assert_eq!(type_name(&Value::Null), "null");
    }

    #[test]
    fn literal_elides_long_strings() {
        assert_eq!(value_to_literal(&json!(0)), "0");
        assert_eq!(value_to_literal(&json!(true)), "true");
        assert_eq!(value_to_literal(&json!("hi")), "\"hi\"");
        let long = "x".repeat(80);
        assert!(value_to_literal(&json!(long)).ends_with("…\""));
    }

    #[test]
    fn converts_to_cel() {
        assert!(matches!(json_to_cel(&json!(5)), cel_interpreter::Value::Int(5)));
        assert!(matches!(
            json_to_cel(&json!(true)),
            cel_interpreter::Value::Bool(true)
        ));
    }
}
