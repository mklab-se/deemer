//! AI evaluation: assembling the prompt and parsing the model's reply.
//!
//! The author's `prompt` is a template that already places `{stdout}`, `{stdin}`,
//! `{suite_context}` etc. wherever they want them — the caller renders it first.
//! This module appends the *response instruction* (generated from the declared
//! `expected_outputs`) so the author never writes "respond as JSON", and parses
//! the model's reply back into typed variables.

use crate::suite::OutputSpec;
use indexmap::IndexMap;
use serde_json::Value;

/// The declared outputs of an AI evaluation, in declaration order.
pub type Outputs = IndexMap<String, OutputSpec>;

/// The default outputs when an `ai:` block declares none: a verdict + a reason.
pub fn default_outputs() -> Outputs {
    let mut m = Outputs::new();
    m.insert("ai_passed".to_string(), OutputSpec::Terse("boolean".to_string()));
    m.insert("reason".to_string(), OutputSpec::Terse("string".to_string()));
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
}
