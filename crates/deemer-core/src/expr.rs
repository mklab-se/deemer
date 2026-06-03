//! Assert-expression evaluation via the CEL interpreter.
//!
//! An authored assert like `{exit_code} == 0 && {ai_passed} == true` is turned
//! into the CEL source `exit_code == 0 && ai_passed == true` (braces stripped),
//! and each referenced variable is bound into the context with its typed value
//! (via [`crate::value::json_to_cel`]). The result must be a boolean.

use crate::value::{Map, json_to_cel};
use cel_interpreter::{Context, Program, Value as Cel};
use regex::Regex;

/// Evaluate an assert expression to a boolean against the given variables.
pub fn evaluate(expr: &str, vars: &Map) -> crate::Result<bool> {
    let re = Regex::new(r"\{(\w+)\}").unwrap();
    let mut names = Vec::new();
    let cel_src = re
        .replace_all(expr, |c: &regex::Captures| {
            names.push(c[1].to_string());
            c[1].to_string()
        })
        .into_owned();

    let mut ctx = Context::default();
    for name in &names {
        let val = vars
            .get(name)
            .ok_or_else(|| crate::Error::Expr(format!("unknown variable {{{name}}}")))?;
        ctx.add_variable_from_value(name.clone(), json_to_cel(val));
    }

    let program = Program::compile(&cel_src).map_err(|e| crate::Error::Expr(e.to_string()))?;
    match program
        .execute(&ctx)
        .map_err(|e| crate::Error::Expr(e.to_string()))?
    {
        Cel::Bool(b) => Ok(b),
        other => Err(crate::Error::Expr(format!(
            "expression did not evaluate to a boolean: {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn v(j: serde_json::Value) -> Map {
        serde_json::from_value(j).unwrap()
    }

    #[test]
    fn evaluates_typed_comparisons() {
        let vars = v(json!({"exit_code": 0, "ai_passed": true, "severity": "none"}));
        assert!(
            evaluate(
                "{exit_code} == 0 && {ai_passed} == true && {severity} != \"high\"",
                &vars
            )
            .unwrap()
        );
    }

    #[test]
    fn string_methods_and_failure() {
        let vars =
            v(json!({"exit_code": 2, "stdout": "Usage: tool", "code": 2, "needle": "Usage"}));
        assert!(
            evaluate(
                "{exit_code} == {code} && {stdout}.contains({needle})",
                &vars
            )
            .unwrap()
        );
        let fail = v(json!({"ai_passed": false}));
        assert!(!evaluate("{ai_passed} == true", &fail).unwrap());
    }

    #[test]
    fn non_bool_result_errors() {
        assert!(evaluate("{n}", &v(json!({"n": 3}))).is_err());
    }
}
