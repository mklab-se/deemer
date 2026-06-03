//! The results-log models and report rendering.
//!
//! [`RunResults`] is the complete, structured record of one run — the source of
//! truth, serialized to YAML. It mirrors `docs/test-results.md`. The human report
//! is rendered from it (see `render_report`, added alongside).

use crate::value::Map;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
