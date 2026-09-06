//! AI evaluation: assembling the prompt and parsing the model's reply.
//!
//! The author's `prompt` is a template that already places `{stdout}`, `{stdin}`,
//! `{suite_context}` etc. wherever they want them — the caller renders it first.
//! This module appends the *response instruction* (generated from the declared
//! `expected_outputs`) so the author never writes "respond as JSON", and parses
//! the model's reply back into typed variables.

use crate::suite::OutputSpec;
use crate::value::Map;
use indexmap::IndexMap;
use serde_json::Value;

/// The declared outputs of an AI evaluation, in declaration order.
pub type Outputs = IndexMap<String, OutputSpec>;

/// The default outputs when an `ai:` block declares none: a verdict + a reason.
pub fn default_outputs() -> Outputs {
    let mut m = Outputs::new();
    m.insert(
        "ai_passed".to_string(),
        OutputSpec::Terse("boolean".to_string()),
    );
    m.insert(
        "reason".to_string(),
        OutputSpec::Terse("string".to_string()),
    );
    m
}

/// Render the "respond with ONLY this JSON object" instruction from the declared
/// outputs — naming each field, its type, any enum values, list element type, and
/// description.
pub fn response_instruction(outputs: &Outputs) -> String {
    let mut lines = Vec::new();
    for (name, spec) in outputs {
        let mut desc = spec.type_name().to_string();
        if let OutputSpec::Rich {
            items: Some(it), ..
        } = spec
        {
            desc = format!("{desc} of {it}");
        }
        if let Some(vals) = spec.values() {
            let opts: Vec<String> = vals
                .iter()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect();
            desc = format!("{desc}, one of [{}]", opts.join(", "));
        }
        if let OutputSpec::Rich {
            description: Some(d),
            ..
        } = spec
        {
            desc = format!("{desc} — {d}");
        }
        lines.push(format!("  \"{name}\": {desc}"));
    }
    format!(
        "=== RESPOND WITH ONLY this JSON object (no prose, no code fence) ===\n{{\n{}\n}}",
        lines.join(",\n")
    )
}

/// Assemble the full prompt: the already-rendered author prompt, then the
/// generated response instruction.
pub fn build_prompt(rendered_prompt: &str, outputs: &Outputs) -> String {
    format!("{rendered_prompt}\n\n{}", response_instruction(outputs))
}

/// Parse a model reply into the declared outputs. Tolerates ```` ```json ```` /
/// ```` ``` ```` code fences. Every declared output must be present with the
/// declared type (and an allowed enum value, if constrained); otherwise the test
/// is recorded as `errored`.
pub fn parse_reply(raw: &str, outputs: &Outputs) -> crate::Result<Map> {
    let json = strip_fences(raw);
    let parsed: Value = serde_json::from_str(json)
        .map_err(|e| crate::Error::AiParse(format!("reply is not valid JSON: {e}")))?;
    let obj = parsed
        .as_object()
        .ok_or_else(|| crate::Error::AiParse("reply is not a JSON object".to_string()))?;

    for (name, spec) in outputs {
        let val = obj
            .get(name)
            .ok_or_else(|| crate::Error::AiParse(format!("missing field `{name}`")))?;
        if !type_matches(spec.type_name(), val) {
            return Err(crate::Error::AiParse(format!(
                "field `{name}` should be {} but got {}",
                spec.type_name(),
                crate::value::type_name(val)
            )));
        }
        if let Some(allowed) = spec.values()
            && !allowed.iter().any(|a| a == val)
        {
            return Err(crate::Error::AiParse(format!(
                "field `{name}` value {val} is not one of the allowed values"
            )));
        }
    }
    Ok(obj.clone())
}

/// Strip a leading ```` ```json ```` / ```` ``` ```` fence and a trailing ```` ``` ````.
fn strip_fences(raw: &str) -> &str {
    let t = raw.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t)
        .trim_start();
    t.strip_suffix("```").unwrap_or(t).trim()
}

/// Whether a JSON value satisfies a declared Deemer type name.
fn type_matches(declared: &str, v: &Value) -> bool {
    match declared {
        "boolean" => v.is_boolean(),
        "string" => v.is_string(),
        "number" => v.is_number(),
        "list" => v.is_array(),
        "object" => v.is_object(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_outputs(s: &str) -> Outputs {
        serde_yaml::from_str(s).unwrap()
    }

    #[test]
    fn builds_instruction_from_outputs() {
        let outputs =
            parse_outputs("{ai_passed: boolean, severity: {type: string, values: [none, high]}}");
        let instr = response_instruction(&outputs);
        assert!(instr.contains("\"ai_passed\""));
        assert!(instr.contains("boolean"));
        assert!(instr.contains("none"));
        assert!(instr.contains("high"));
        assert!(instr.contains("JSON"));
    }

    #[test]
    fn build_prompt_appends_instruction() {
        let p = build_prompt("Judge safety. Output: stdout-here", &default_outputs());
        assert!(p.contains("Judge safety."));
        assert!(p.contains("stdout-here"));
        assert!(p.contains("JSON"));
        assert!(p.contains("ai_passed"));
    }

    #[test]
    fn parses_fenced_json_and_validates() {
        let outputs = parse_outputs("{ai_passed: boolean, reason: string}");
        let m = parse_reply(
            "```json\n{\"ai_passed\": true, \"reason\": \"ok\"}\n```",
            &outputs,
        )
        .unwrap();
        assert_eq!(m["ai_passed"], serde_json::json!(true));
        assert_eq!(m["reason"], serde_json::json!("ok"));
    }

    #[test]
    fn missing_field_errors() {
        let outputs = parse_outputs("{ai_passed: boolean, reason: string}");
        assert!(parse_reply("{\"ai_passed\": true}", &outputs).is_err());
    }

    #[test]
    fn wrong_type_errors() {
        let outputs = parse_outputs("{ai_passed: boolean}");
        assert!(parse_reply("{\"ai_passed\": \"yes\"}", &outputs).is_err());
    }

    #[test]
    fn enum_value_enforced() {
        let outputs = parse_outputs("{severity: {type: string, values: [none, high]}}");
        assert!(parse_reply("{\"severity\": \"none\"}", &outputs).is_ok());
        assert!(parse_reply("{\"severity\": \"medium\"}", &outputs).is_err());
    }
}
