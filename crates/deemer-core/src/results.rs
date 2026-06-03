//! The results-log models and report rendering.
//!
//! [`RunResults`] is the complete, structured record of one run — the source of
//! truth, serialized to YAML. It mirrors `docs/test-results.md`. The human report
//! is rendered from it (see `render_report`, added alongside).

use crate::suite::Suite;
use crate::value::Map;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

/// The outcome of a test or suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Passed,
    Failed,
    Errored,
}

/// A complete results log for one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResults {
    pub results_format: u32,
    pub suite: SuiteInfo,
    pub run: RunInfo,
    pub summary: Summary,
    pub tests: Vec<TestRecord>,
}

/// Identity of the suite that produced this log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteInfo {
    pub name: String,
    pub description: String,
    pub config_path: String,
    pub config_sha256: String,
    pub report_path: String,
    pub results_path: String,
}

/// When and how this run happened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInfo {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub deemer_version: String,
    pub settings: SettingsInfo,
    pub ai: AiInfo,
}

/// The settings that governed the run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsInfo {
    pub concurrency: usize,
    pub rate_limit: Option<String>,
}

/// Whether AI was used and which model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiInfo {
    pub used: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Aggregate counts and the suite verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub errored: usize,
    pub pass_rate: f64,
    pub status: Status,
    pub assert: AssertRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<AiRecord>,
}

/// An assert as authored, after substitution, and its boolean result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertRecord {
    pub expression: String,
    pub substituted: String,
    pub result: bool,
}

/// The record of an AI evaluation (per test or suite).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRecord {
    pub model: String,
    pub prompt: String,
    pub response: String,
    pub outputs: Map,
}

/// One test's complete record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRecord {
    pub test_number: usize,
    pub status: Status,
    pub assert: AssertRecord,
    pub data: Map,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
    pub execution: Execution,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<AiRecord>,
    pub error: Option<String>,
}

/// The captured execution of a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub started_at: DateTime<Utc>,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub timed_out: bool,
    pub stdout: String,
    pub stderr: String,
}

impl RunResults {
    /// Serialize the log to YAML.
    pub fn to_yaml(&self) -> crate::Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }
}

/// Hex-encoded SHA-256 of the suite file's raw bytes, tying a log to its config.
pub fn config_sha256(file_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(file_bytes);
    format!("{:x}", hasher.finalize())
}

/// The lowercase string form of a status.
fn status_str(s: Status) -> &'static str {
    match s {
        Status::Passed => "passed",
        Status::Failed => "failed",
        Status::Errored => "errored",
    }
}

/// Render the human report from a results log, honouring the suite's optional
/// `test.output` and `report.template` overrides (falling back to sensible
/// defaults).
pub fn render_report(results: &RunResults, suite: &Suite) -> String {
    let fragments: Vec<String> = results
        .tests
        .iter()
        .map(|t| match &suite.test.output {
            Some(tmpl) => {
                crate::template::render(tmpl, &test_vars(t)).unwrap_or_else(|_| default_fragment(t))
            }
            None => default_fragment(t),
        })
        .collect();
    let all_test_results = fragments.join("\n\n");

    match suite.report.as_ref().and_then(|r| r.template.as_ref()) {
        Some(tmpl) => {
            let vars = report_vars(suite, &results.summary, &all_test_results);
            crate::template::render(tmpl, &vars)
                .unwrap_or_else(|_| default_report(suite, &results.summary, &all_test_results))
        }
        None => default_report(suite, &results.summary, &all_test_results),
    }
}

/// The combined variable bag for rendering one test's `output` fragment.
fn test_vars(t: &TestRecord) -> Map {
    let mut m = t.data.clone();
    m.insert("test_number".to_string(), json!(t.test_number));
    m.insert("status".to_string(), json!(status_str(t.status)));
    m.insert("command".to_string(), json!(t.command));
    m.insert("exit_code".to_string(), json!(t.execution.exit_code));
    m.insert("stdout".to_string(), json!(t.execution.stdout));
    m.insert("stderr".to_string(), json!(t.execution.stderr));
    m.insert("duration_ms".to_string(), json!(t.execution.duration_ms));
    if let Some(s) = &t.stdin {
        m.insert("stdin".to_string(), json!(s));
    }
    m.insert("expression".to_string(), json!(t.assert.expression));
    m.insert("assert_result".to_string(), json!(t.assert.result));
    if let Some(ai) = &t.ai {
        for (k, v) in &ai.outputs {
            m.insert(k.clone(), v.clone());
        }
    }
    m
}

/// The default per-test report fragment.
fn default_fragment(t: &TestRecord) -> String {
    let exit = t
        .execution
        .exit_code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "—".to_string());
    let ai_line = t
        .ai
        .as_ref()
        .and_then(|ai| ai.outputs.get("reason"))
        .and_then(|r| r.as_str())
        .map(|r| format!(", AI: {r}"))
        .unwrap_or_default();
    format!(
        "### Test {} — {}\n- Command: `{}`\n- Exit {} ({} ms){}\n- `{}` → {}",
        t.test_number,
        status_str(t.status),
        t.command,
        exit,
        t.execution.duration_ms,
        ai_line,
        t.assert.expression,
        t.assert.result,
    )
}

/// The variable bag for rendering a custom `report.template`.
fn report_vars(suite: &Suite, summary: &Summary, all_test_results: &str) -> Map {
    let mut m = Map::new();
    m.insert("name".to_string(), json!(suite.name));
    m.insert("description".to_string(), json!(suite.description));
    m.insert("total".to_string(), json!(summary.total));
    m.insert("passed".to_string(), json!(summary.passed));
    m.insert("failed".to_string(), json!(summary.failed));
    m.insert("errored".to_string(), json!(summary.errored));
    m.insert("pass_rate".to_string(), json!(summary.pass_rate));
    m.insert("status".to_string(), json!(status_str(summary.status)));
    if let Some(ai) = &summary.ai {
        for (k, v) in &ai.outputs {
            m.insert(k.clone(), v.clone());
        }
    }
    m.insert("all_test_results".to_string(), json!(all_test_results));
    m
}

/// The default whole-suite report.
fn default_report(suite: &Suite, summary: &Summary, all_test_results: &str) -> String {
    let mut out = format!(
        "# {}\n\n{}\n\n**{}/{} passed — status: {}**\n\n",
        suite.name,
        suite.description,
        summary.passed,
        summary.total,
        status_str(summary.status),
    );
    if let Some(es) = summary
        .ai
        .as_ref()
        .and_then(|ai| ai.outputs.get("executive_summary"))
        .and_then(|v| v.as_str())
    {
        out.push_str(&format!("## Executive Summary\n{es}\n\n"));
    }
    out.push_str("## Test Results\n");
    out.push_str(all_test_results);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ts(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn sample_run_results() -> RunResults {
        let mk = |n: usize, status: Status, result: bool| TestRecord {
            test_number: n,
            status,
            assert: AssertRecord {
                expression: "{exit_code} == 0".to_string(),
                substituted: format!("{} == 0", if result { 0 } else { 1 }),
                result,
            },
            data: serde_json::from_value(json!({"arg": n})).unwrap(),
            command: format!("tool {n}"),
            stdin: None,
            execution: Execution {
                started_at: ts("2026-06-02T20:55:01Z"),
                exit_code: Some(if result { 0 } else { 1 }),
                duration_ms: 10,
                timed_out: false,
                stdout: "out".to_string(),
                stderr: String::new(),
            },
            ai: None,
            error: None,
        };
        RunResults {
            results_format: 1,
            suite: SuiteInfo {
                name: "Demo".to_string(),
                description: "d".to_string(),
                config_path: "demo.suite.yml".to_string(),
                config_sha256: config_sha256(b"demo"),
                report_path: "reports/demo.md".to_string(),
                results_path: "results/demo.yaml".to_string(),
            },
            run: RunInfo {
                started_at: ts("2026-06-02T20:55:01Z"),
                finished_at: ts("2026-06-02T20:55:02Z"),
                duration_ms: 1000,
                deemer_version: "0.1.0".to_string(),
                settings: SettingsInfo {
                    concurrency: 1,
                    rate_limit: None,
                },
                ai: AiInfo {
                    used: false,
                    model: None,
                },
            },
            summary: Summary {
                total: 2,
                passed: 1,
                failed: 1,
                errored: 0,
                pass_rate: 0.5,
                status: Status::Failed,
                assert: AssertRecord {
                    expression: "{failed} == 0".to_string(),
                    substituted: "1 == 0".to_string(),
                    result: false,
                },
                ai: None,
            },
            tests: vec![mk(1, Status::Passed, true), mk(2, Status::Failed, false)],
        }
    }

    #[test]
    fn run_results_serializes_expected_keys() {
        let r = sample_run_results();
        let yaml = r.to_yaml().unwrap();
        assert!(yaml.contains("results_format: 1"));
        assert!(yaml.contains("status: failed"));
        assert!(yaml.contains("test_number: 1"));
        assert!(yaml.contains("substituted:"));
        let back: RunResults = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back.summary.total, 2);
        assert_eq!(back.tests.len(), 2);
    }

    #[test]
    fn config_sha256_is_stable_hex() {
        assert_eq!(config_sha256(b"demo"), config_sha256(b"demo"));
        assert_eq!(config_sha256(b"demo").len(), 64);
        assert_ne!(config_sha256(b"a"), config_sha256(b"b"));
    }

    fn suite_no_templates() -> Suite {
        serde_yaml::from_str(
            "test_suite_format: 1\nname: Demo\ndescription: d\ntest:\n  command: \"tool {arg}\"\n  evaluate:\n    assert: {expression: \"{exit_code} == 0\"}\nsuite:\n  assert: {expression: \"{failed} == 0\"}\ndata: [{arg: 1}]",
        )
        .unwrap()
    }

    fn suite_custom_templates() -> Suite {
        serde_yaml::from_str(
            "test_suite_format: 1\nname: Demo\ndescription: d\ntest:\n  command: \"tool {arg}\"\n  evaluate:\n    assert: {expression: \"{exit_code} == 0\"}\nsuite:\n  assert: {expression: \"{failed} == 0\"}\nreport:\n  template: \"# {name}\\n## Executive Summary\\n{executive_summary}\\n## Tests\\n{all_test_results}\"\ndata: [{arg: 1}]",
        )
        .unwrap()
    }

    #[test]
    fn default_report_lists_tests_and_status() {
        let r = sample_run_results();
        let suite = suite_no_templates();
        let md = render_report(&r, &suite);
        assert!(md.contains("# "));
        assert!(md.contains("Test 1"));
        assert!(md.contains("status"));
    }

    #[test]
    fn custom_template_uses_all_test_results_and_exec_summary() {
        let mut r = sample_run_results();
        r.summary.ai = Some(AiRecord {
            model: "m".to_string(),
            prompt: "p".to_string(),
            response: "{}".to_string(),
            outputs: serde_json::from_value(json!({"executive_summary": "All good."})).unwrap(),
        });
        let suite = suite_custom_templates();
        let md = render_report(&r, &suite);
        assert!(md.contains("## Executive Summary"));
        assert!(md.contains("All good."));
        assert!(md.contains("Test 1"));
    }

    #[test]
    fn parses_sample_result_logs() {
        for yaml in [
            include_str!("../../../docs/samples/safety-checks.results.yml"),
            include_str!("../../../docs/samples/cli-smoke.results.yml"),
            include_str!("../../../docs/samples/support-tone.results.yml"),
        ] {
            let r: RunResults = serde_yaml::from_str(yaml).unwrap();
            assert_eq!(r.results_format, 1);
            assert_eq!(r.summary.total, r.tests.len());
        }
    }
}
