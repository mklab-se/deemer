//! `{name}` template substitution and the elided substituted-expression renderer.
//!
//! Two surfaces share one scanner:
//! - [`render`] does text substitution for commands, prompts, output and reports
//!   (a `{name}` is replaced by its value's text; missing names are an error).
//! - [`substitute_for_log`] renders an assert with typed literals for the results
//!   log (strings quoted/elided); it never fails and leaves unknown names as-is.
//!
//! Literal braces are written `{{` and `}}`.

use crate::value::{Map, value_to_literal};
use serde_json::Value;

/// Render a template, substituting `{name}` with each variable's text value.
/// Returns an error if a referenced variable is missing or a brace is unclosed.
pub fn render(template: &str, vars: &Map) -> crate::Result<String> {
    scan(template, |name| vars.get(name).map(value_text)).map_err(crate::Error::Template)
}

/// Render an assert expression with typed literals substituted in, for display in
/// the results log. Strings are quoted and long ones elided; unknown names are
/// left as `{name}`. Never fails.
pub fn substitute_for_log(expr: &str, vars: &Map) -> String {
    scan(expr, |name| {
        Some(match vars.get(name) {
            Some(v) => value_to_literal(v),
            None => format!("{{{name}}}"),
        })
    })
    .unwrap_or_else(|_| expr.to_string())
}

/// The text form of a value for `render`: strings bare, everything else via JSON.
fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Scan a template, replacing `{name}` via `resolve` and collapsing `{{`/`}}`.
/// `resolve` returning `None` for a name is reported as an `Err(name)`.
fn scan<F>(template: &str, mut resolve: F) -> std::result::Result<String, String>
where
    F: FnMut(&str) -> Option<String>,
{
    let mut out = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '{' => {
                let mut name = String::new();
                let mut closed = false;
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc == '}' {
                        closed = true;
                        break;
                    }
                    name.push(nc);
                }
                if !closed {
                    return Err(format!("unclosed '{{' near \"{name}\""));
                }
                let name = name.trim();
                match resolve(name) {
                    Some(v) => out.push_str(&v),
                    None => return Err(format!("unknown variable {{{name}}}")),
                }
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vars() -> Map {
        serde_json::from_value(json!({"name": "Ada", "n": 3})).unwrap()
    }

    #[test]
    fn substitutes_and_escapes() {
        assert_eq!(render("hi {name}, n={n}", &vars()).unwrap(), "hi Ada, n=3");
        assert_eq!(
            render("literal {{name}}", &vars()).unwrap(),
            "literal {name}"
        );
    }

    #[test]
    fn missing_var_errors() {
        assert!(render("{nope}", &vars()).is_err());
    }

    #[test]
    fn substitute_for_log_elides() {
        let v: Map =
            serde_json::from_value(json!({"exit_code": 0, "stdout": "x".repeat(80)})).unwrap();
        let s = substitute_for_log("{exit_code} == 0 && {stdout}.contains(\"x\")", &v);
        assert!(s.starts_with("0 == 0"));
        assert!(s.contains("…"));
    }
}
