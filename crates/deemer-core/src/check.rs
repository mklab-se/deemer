//! Static validation of a suite (`deemer check`).
//!
//! Catches the mistakes the format docs warn about — reserved-name collisions,
//! malformed rate limits, asserts/templates referencing an unknown variable, and
//! two suites resolving to the same output path — before anything runs.

use crate::suite::{AiSpec, RateLimit, Suite};
use regex::Regex;
use std::collections::HashSet;

/// Names Deemer provides (captured output, deemer-injected report/suite vars).
/// `stdin` is included so it's a *known* variable, but it is explicitly allowed
/// as a data-field name (the documented way to feed stdin — see D4).
const RESERVED: &[&str] = &[
    "stdout",
    "stderr",
    "exit_code",
    "stdin",
    "duration_ms",
    "test_number",
    "name",
    "status",
    "command",
    "expression",
    "assert_result",
    "all_test_results",
    "suite_context",
    "total",
    "passed",
    "failed",
    "errored",
    "pass_rate",
];

/// Validate a suite. Returns a (possibly empty) list of human-readable issues.
pub fn check(suite: &Suite) -> Vec<String> {
    let mut issues = Vec::new();
    let ai_outputs = collect_ai_output_names(suite);

    // (a) data fields may not collide with a reserved name (except `stdin`) or a
    //     declared AI output.
    for (i, row) in suite.data.iter().enumerate() {
        for key in row.keys() {
            if key != "stdin" && RESERVED.contains(&key.as_str()) {
                issues.push(format!(
                    "data row {}: field `{key}` collides with a reserved name",
                    i + 1
                ));
            }
            if ai_outputs.contains(key) {
                issues.push(format!(
                    "data row {}: field `{key}` collides with a declared AI output",
                    i + 1
                ));
            }
        }
    }

    // (b) rate_limit must parse.
    if let Some(rl) = &suite.settings.rate_limit {
        if let Err(e) = RateLimit::parse(rl) {
            issues.push(format!("settings.rate_limit: {e}"));
        }
    }

    // (c) every {var} referenced in an assert/prompt/template must be known.
    let known = known_names(suite);
    check_refs(
        "test.assert",
        &suite.test.evaluate.assert.expression,
        &known,
        &mut issues,
    );
    check_refs(
        "suite.assert",
        &suite.suite.assert.expression,
        &known,
        &mut issues,
    );
    if let Some(ai) = &suite.test.evaluate.ai {
        check_refs("test.ai.prompt", &ai.prompt, &known, &mut issues);
    }
    if let Some(out) = &suite.test.output {
        check_refs("test.output", out, &known, &mut issues);
    }
    if let Some(eval) = &suite.suite.evaluation {
        if let Some(ai) = &eval.ai {
            check_refs("suite.ai.prompt", &ai.prompt, &known, &mut issues);
        }
    }
    if let Some(report) = &suite.report {
        if let Some(tmpl) = &report.template {
            check_refs("report.template", tmpl, &known, &mut issues);
        }
    }

    // (d) report and results must not resolve to the same path.
    let report_path = suite.report.as_ref().and_then(|r| r.path.as_ref());
    let results_path = suite.results.as_ref().and_then(|r| r.path.as_ref());
    if let (Some(a), Some(b)) = (report_path, results_path) {
        if a == b {
            issues.push(format!("report and results resolve to the same path: {a}"));
        }
    }

    issues
}

/// The set of all variable names a template may legitimately reference.
fn known_names(suite: &Suite) -> HashSet<String> {
    let mut k: HashSet<String> = RESERVED.iter().map(|s| s.to_string()).collect();
    for row in &suite.data {
        for key in row.keys() {
            k.insert(key.clone());
        }
    }
    add_output_names(&suite.test.evaluate.ai, &mut k);
    if let Some(eval) = &suite.suite.evaluation {
        add_output_names(&eval.ai, &mut k);
    }
    k
}

/// The set of declared AI output names (test + suite), used for collision checks.
fn collect_ai_output_names(suite: &Suite) -> HashSet<String> {
    let mut s = HashSet::new();
    add_output_names(&suite.test.evaluate.ai, &mut s);
    if let Some(eval) = &suite.suite.evaluation {
        add_output_names(&eval.ai, &mut s);
    }
    s
}

/// Insert an AI step's output names (declared, or the `{ai_passed, reason}` default).
fn add_output_names(ai: &Option<AiSpec>, set: &mut HashSet<String>) {
    if let Some(ai) = ai {
        match &ai.expected_outputs {
            Some(outs) => {
                for key in outs.keys() {
                    set.insert(key.clone());
                }
            }
            None => {
                set.insert("ai_passed".to_string());
                set.insert("reason".to_string());
            }
        }
    }
}

/// Flag any `{var}` in `text` not present in `known`.
fn check_refs(label: &str, text: &str, known: &HashSet<String>, issues: &mut Vec<String>) {
    for name in referenced(text) {
        if !known.contains(&name) {
            issues.push(format!("{label} references unknown variable {{{name}}}"));
        }
    }
}

/// The `{ident}` names referenced in a template (escaped `{{`/`}}` ignored).
fn referenced(text: &str) -> Vec<String> {
    let cleaned = text.replace("{{", "").replace("}}", "");
    let re = Regex::new(r"\{(\w+)\}").unwrap();
    re.captures_iter(&cleaned)
        .map(|c| c[1].to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(y: &str) -> Suite {
        serde_yaml::from_str(y).unwrap()
    }

    #[test]
    fn flags_reserved_collision() {
        let s = parse(
            "test_suite_format: 1\nname: x\ndescription: d\ntest:\n  command: \"t {arg}\"\n  evaluate:\n    assert: {expression: \"{exit_code} == 0\"}\nsuite:\n  assert: {expression: \"{failed} == 0\"}\ndata:\n  - {arg: 1, stdout: oops}",
        );
        assert!(check(&s).iter().any(|i| i.contains("reserved")));
    }

    #[test]
    fn flags_undeclared_var_in_assert() {
        let s = parse(
            "test_suite_format: 1\nname: x\ndescription: d\ntest:\n  command: \"t {arg}\"\n  evaluate:\n    assert: {expression: \"{nope} == true\"}\nsuite:\n  assert: {expression: \"{failed} == 0\"}\ndata:\n  - {arg: 1}",
        );
        assert!(check(&s).iter().any(|i| i.contains("nope")));
    }

    #[test]
    fn flags_bad_rate_limit() {
        let s = parse(
            "test_suite_format: 1\nname: x\ndescription: d\nsettings:\n  rate_limit: 5/fortnight\ntest:\n  command: \"t {arg}\"\n  evaluate:\n    assert: {expression: \"{exit_code} == 0\"}\nsuite:\n  assert: {expression: \"{failed} == 0\"}\ndata:\n  - {arg: 1}",
        );
        assert!(check(&s).iter().any(|i| i.contains("rate")));
    }

    #[test]
    fn stdin_data_field_is_allowed() {
        let s = parse(
            "test_suite_format: 1\nname: x\ndescription: d\ntest:\n  command: \"t\"\n  evaluate:\n    assert: {expression: \"{exit_code} == 0\"}\nsuite:\n  assert: {expression: \"{failed} == 0\"}\ndata:\n  - {stdin: hello}",
        );
        assert!(check(&s).is_empty());
    }

    #[test]
    fn all_samples_are_clean() {
        for yaml in [
            include_str!("../../../docs/samples/safety-checks.suite.yml"),
            include_str!("../../../docs/samples/cli-smoke.suite.yml"),
            include_str!("../../../docs/samples/support-tone.suite.yml"),
            include_str!("../../../docs/samples/annotated.suite.yml"),
        ] {
            let s: Suite = serde_yaml::from_str(yaml).unwrap();
            let issues = check(&s);
            assert!(issues.is_empty(), "expected no issues, got: {issues:?}");
        }
    }
}
