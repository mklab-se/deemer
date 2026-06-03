//! The test-suite model and YAML parsing.
//!
//! Mirrors the format documented in `docs/test-suite-config.md`. Only the pieces
//! the runtime needs are modelled; unknown keys (and comments) are ignored.

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::path::Path;
use std::time::Duration;

/// A complete test suite, deserialized from a `*.suite.yml` file.
#[derive(Debug, Clone, Deserialize)]
pub struct Suite {
    #[serde(alias = "test-suite-format")]
    pub test_suite_format: u32,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub settings: Settings,
    pub test: TestSpec,
    pub suite: SuiteSpec,
    #[serde(default)]
    pub report: Option<ReportSpec>,
    #[serde(default)]
    pub results: Option<ResultsSpec>,
    pub data: Vec<Map<String, Value>>,
}

/// Run-wide knobs: concurrency and rate limiting.
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default, alias = "rate-limit")]
    pub rate_limit: Option<String>,
}

fn default_concurrency() -> usize {
    1
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            concurrency: 1,
            rate_limit: None,
        }
    }
}

/// The command template and how each run is judged.
#[derive(Debug, Clone, Deserialize)]
pub struct TestSpec {
    pub command: String,
    #[serde(default)]
    pub timeout: Option<String>,
    pub evaluate: Evaluate,
    #[serde(default)]
    pub output: Option<String>,
}

/// Per-test evaluation: an optional AI step plus the required assert.
#[derive(Debug, Clone, Deserialize)]
pub struct Evaluate {
    #[serde(default)]
    pub ai: Option<AiSpec>,
    pub assert: Assert,
}

/// An AI evaluation: the criteria, the declared outputs, and an optional model.
#[derive(Debug, Clone, Deserialize)]
pub struct AiSpec {
    pub prompt: String,
    #[serde(default, alias = "expected-outputs")]
    pub expected_outputs: Option<IndexMap<String, OutputSpec>>,
    #[serde(default)]
    pub model: Option<String>,
}

/// The pass/fail decision.
#[derive(Debug, Clone, Deserialize)]
pub struct Assert {
    pub expression: String,
}

/// The suite-level verdict: an optional AI evaluation plus the required assert.
#[derive(Debug, Clone, Deserialize)]
pub struct SuiteSpec {
    #[serde(default)]
    pub evaluation: Option<SuiteEval>,
    pub assert: Assert,
}

/// The suite-level AI evaluation wrapper.
#[derive(Debug, Clone, Deserialize)]
pub struct SuiteEval {
    #[serde(default)]
    pub ai: Option<AiSpec>,
}

/// Where/how the human report is written.
#[derive(Debug, Clone, Deserialize)]
pub struct ReportSpec {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub template: Option<String>,
}

/// Where the results log is written.
#[derive(Debug, Clone, Deserialize)]
pub struct ResultsSpec {
    #[serde(default)]
    pub path: Option<String>,
}

/// A declared AI output: either a bare type name, or an object with metadata.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OutputSpec {
    /// A bare type name, e.g. `boolean`, `string`, `number`, `list`.
    Terse(String),
    /// The opened-up form with an enum, list element type, and/or description.
    Rich {
        #[serde(rename = "type")]
        ty: String,
        #[serde(default)]
        values: Option<Vec<Value>>,
        #[serde(default)]
        items: Option<String>,
        #[serde(default)]
        description: Option<String>,
    },
}

impl OutputSpec {
    /// The declared type name (`boolean` | `string` | `number` | `list` | …).
    pub fn type_name(&self) -> &str {
        match self {
            OutputSpec::Terse(t) => t,
            OutputSpec::Rich { ty, .. } => ty,
        }
    }

    /// The allowed enum values, if this output constrains them.
    pub fn values(&self) -> Option<&[Value]> {
        match self {
            OutputSpec::Rich {
                values: Some(v), ..
            } => Some(v),
            _ => None,
        }
    }
}

impl Suite {
    /// Load and parse a suite from a YAML file.
    pub fn load(path: &Path) -> crate::Result<Suite> {
        let yaml = std::fs::read_to_string(path)?;
        let suite = serde_yaml::from_str(&yaml)?;
        Ok(suite)
    }
}

/// A parsed `settings.rate_limit`: a minimum spacing between test starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimit {
    pub min_interval: Duration,
}

impl RateLimit {
    /// Parse a `<count>/<unit>` rate (e.g. `10/minute`, `1/s`) into a minimum
    /// start-spacing of `unit ÷ count`.
    pub fn parse(s: &str) -> crate::Result<RateLimit> {
        let err = || crate::Error::RateLimit(s.to_string());
        let (count_str, unit_str) = s.split_once('/').ok_or_else(err)?;
        let count: f64 = count_str.trim().parse().map_err(|_| err())?;
        if count <= 0.0 || !count.is_finite() {
            return Err(err());
        }
        let unit_secs = match unit_str.trim() {
            "second" | "sec" | "s" => 1.0,
            "minute" | "min" | "m" => 60.0,
            "hour" | "hr" | "h" => 3600.0,
            _ => return Err(err()),
        };
        Ok(RateLimit {
            min_interval: Duration::from_secs_f64(unit_secs / count),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_safety_sample() {
        let yaml = include_str!("../../../docs/samples/safety-checks.suite.yml");
        let s: Suite = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(s.test_suite_format, 1);
        assert_eq!(s.data.len(), 4);
        assert!(s.test.evaluate.ai.is_some());
        assert_eq!(
            s.test.evaluate.assert.expression,
            "{exit_code} == 0 && {ai_passed} == true && {severity} != \"high\""
        );
        // No settings block in this sample -> concurrency defaults to 1.
        assert_eq!(s.settings.concurrency, 1);
    }

    #[test]
    fn parses_all_samples() {
        for yaml in [
            include_str!("../../../docs/samples/cli-smoke.suite.yml"),
            include_str!("../../../docs/samples/support-tone.suite.yml"),
            include_str!("../../../docs/samples/annotated.suite.yml"),
        ] {
            let s: Suite = serde_yaml::from_str(yaml).unwrap();
            assert!(!s.data.is_empty());
        }
    }

    #[test]
    fn output_spec_terse_and_rich() {
        let terse: OutputSpec = serde_yaml::from_str("boolean").unwrap();
        assert!(matches!(terse, OutputSpec::Terse(t) if t == "boolean"));
        let rich: OutputSpec = serde_yaml::from_str("{type: string, values: [a, b]}").unwrap();
        assert!(matches!(rich, OutputSpec::Rich { .. }));
        assert_eq!(rich.type_name(), "string");
        assert_eq!(rich.values().unwrap().len(), 2);
    }

    #[test]
    fn rate_limit_to_interval() {
        assert_eq!(
            RateLimit::parse("10/minute").unwrap().min_interval,
            Duration::from_secs(6)
        );
        assert_eq!(
            RateLimit::parse("1/s").unwrap().min_interval,
            Duration::from_secs(1)
        );
        assert_eq!(
            RateLimit::parse("120/hour").unwrap().min_interval,
            Duration::from_secs(30)
        );
        assert!(RateLimit::parse("0/minute").is_err());
        assert!(RateLimit::parse("nonsense").is_err());
        assert!(RateLimit::parse("5/fortnight").is_err());
    }

    #[test]
    fn missing_assert_is_error() {
        let bad = "test_suite_format: 1\nname: x\ndescription: y\ntest:\n  command: t\n  evaluate: {}\nsuite:\n  assert: {expression: 'true'}\ndata: [{}]";
        assert!(serde_yaml::from_str::<Suite>(bad).is_err());
    }
}
