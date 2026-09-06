//! `deemer run` — execute a suite, judge each run, and write the log + report.
//!
//! The pure logic lives in `deemer-core`; this module is the async orchestration:
//! it spawns the command per data row (with timeout, concurrency, and rate-limit
//! spacing), calls the model for AI evaluations, and assembles the results log.

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use deemer_core::judge::{build_prompt, default_outputs, parse_reply};
use deemer_core::results::{
    AiInfo, AiRecord, AssertRecord, Execution, RunInfo, RunResults, SettingsInfo, Status,
    SuiteInfo, Summary, TestRecord,
};
use deemer_core::suite::{RateLimit, Suite};
use deemer_core::value::Map;
use deemer_core::{check, expr, template};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::{Mutex, Semaphore};

/// A best-effort label for the model used (ailloy uses the globally configured
/// model; the exact id and per-test `model` override are not wired in v1).
const MODEL_LABEL: &str = "ailloy (configured)";

/// Run a suite end-to-end and exit with a status-derived code (0/1/2).
pub async fn run(
    suite_path: PathBuf,
    report_override: Option<PathBuf>,
    results_override: Option<PathBuf>,
    concurrency_override: Option<usize>,
) -> Result<()> {
    let suite_bytes = std::fs::read(&suite_path)
        .with_context(|| format!("reading suite file {}", suite_path.display()))?;
    let suite = Suite::load(&suite_path)?;

    let issues = check::check(&suite);
    if !issues.is_empty() {
        for issue in &issues {
            eprintln!("✗ {issue}");
        }
        eprintln!("\nSuite is invalid ({} issue(s)); aborting.", issues.len());
        std::process::exit(2);
    }

    let ai_used = suite.test.evaluate.ai.is_some()
        || suite
            .suite
            .evaluation
            .as_ref()
            .and_then(|e| e.ai.as_ref())
            .is_some();
    if ai_used && !crate::commands::ai::is_ai_active() {
        return Err(anyhow!(
            "this suite uses AI evaluation, but AI is not active — run `deemer ai config` then `deemer ai enable`"
        ));
    }

    let started_at = Utc::now();
    let run_start = Instant::now();

    let concurrency = concurrency_override
        .unwrap_or(suite.settings.concurrency)
        .max(1);
    let rate = match &suite.settings.rate_limit {
        Some(r) => Some(RateLimit::parse(r)?),
        None => None,
    };
    let timeout = parse_timeout(suite.test.timeout.as_deref());

    // Run every data row, respecting concurrency and rate-limit spacing.
    let suite = Arc::new(suite);
    let sem = Arc::new(Semaphore::new(concurrency));
    let next_start = Arc::new(Mutex::new(Instant::now()));
    let mut handles = Vec::with_capacity(suite.data.len());
    for (idx, row) in suite.data.iter().enumerate() {
        let suite = Arc::clone(&suite);
        let sem = Arc::clone(&sem);
        let next_start = Arc::clone(&next_start);
        let row = row.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore open");
            if let Some(rate) = rate {
                reserve_rate_slot(&next_start, rate.min_interval).await;
            }
            run_one(&suite, idx, &row, timeout).await
        }));
    }

    // Collected in spawn order == data order (D17), regardless of completion order.
    let mut tests = Vec::with_capacity(handles.len());
    for h in handles {
        tests.push(h.await.context("a test task panicked")?);
    }

    // Aggregate.
    let total = tests.len();
    let passed = tests.iter().filter(|t| t.status == Status::Passed).count();
    let failed = tests.iter().filter(|t| t.status == Status::Failed).count();
    let errored = tests.iter().filter(|t| t.status == Status::Errored).count();
    let pass_rate = if total > 0 {
        passed as f64 / total as f64
    } else {
        0.0
    };

    // Suite-level evaluation.
    let mut svars = Map::new();
    svars.insert("total".to_string(), json!(total));
    svars.insert("passed".to_string(), json!(passed));
    svars.insert("failed".to_string(), json!(failed));
    svars.insert("errored".to_string(), json!(errored));
    svars.insert("pass_rate".to_string(), json!(pass_rate));

    let mut suite_ai = None;
    let mut suite_error: Option<String> = None;
    if let Some(eval) = &suite.suite.evaluation
        && let Some(ai) = &eval.ai
    {
        let outputs = ai.expected_outputs.clone().unwrap_or_else(default_outputs);
        svars.insert(
            "suite_context".to_string(),
            json!(render_suite_context(&tests)),
        );
        match template::render(&ai.prompt, &svars) {
            Ok(rendered) => {
                let full = build_prompt(&rendered, &outputs);
                match call_model(&full).await {
                    Ok(reply) => match parse_reply(&reply, &outputs) {
                        Ok(parsed) => {
                            for (k, v) in &parsed {
                                svars.insert(k.clone(), v.clone());
                            }
                            suite_ai = Some(AiRecord {
                                model: MODEL_LABEL.to_string(),
                                prompt: full,
                                response: reply,
                                outputs: parsed,
                            });
                        }
                        Err(e) => {
                            suite_error = Some(format!("suite AI reply: {e}"));
                            suite_ai = Some(AiRecord {
                                model: MODEL_LABEL.to_string(),
                                prompt: full,
                                response: reply,
                                outputs: Map::new(),
                            });
                        }
                    },
                    Err(e) => suite_error = Some(format!("suite AI call: {e}")),
                }
            }
            Err(e) => suite_error = Some(format!("suite prompt template: {e}")),
        }
    }

    let suite_expr = suite.suite.assert.expression.clone();
    let suite_substituted = template::substitute_for_log(&suite_expr, &svars);
    let suite_result = match expr::evaluate(&suite_expr, &svars) {
        Ok(b) => b,
        Err(e) => {
            suite_error.get_or_insert(format!("suite assert: {e}"));
            false
        }
    };

    let status = if errored > 0 || suite_error.is_some() {
        Status::Errored
    } else if suite_result {
        Status::Passed
    } else {
        Status::Failed
    };

    // Resolve output paths, build the log, and write both artifacts.
    let report_path = resolve_output(
        report_override,
        report_cfg(&suite),
        &suite_path,
        "report.md",
    );
    let results_path = resolve_output(
        results_override,
        results_cfg(&suite),
        &suite_path,
        "results.yml",
    );

    let summary = Summary {
        total,
        passed,
        failed,
        errored,
        pass_rate,
        status,
        assert: AssertRecord {
            expression: suite_expr,
            substituted: suite_substituted,
            result: suite_result,
        },
        ai: suite_ai,
    };

    let results = RunResults {
        results_format: 1,
        suite: SuiteInfo {
            name: suite.name.clone(),
            description: suite.description.clone(),
            config_path: suite_path.display().to_string(),
            config_sha256: deemer_core::results::config_sha256(&suite_bytes),
            report_path: report_path.display().to_string(),
            results_path: results_path.display().to_string(),
        },
        run: RunInfo {
            started_at,
            finished_at: Utc::now(),
            duration_ms: run_start.elapsed().as_millis() as u64,
            deemer_version: env!("CARGO_PKG_VERSION").to_string(),
            settings: SettingsInfo {
                concurrency,
                rate_limit: suite.settings.rate_limit.clone(),
            },
            ai: AiInfo {
                used: ai_used,
                model: ai_used.then(|| MODEL_LABEL.to_string()),
            },
        },
        summary,
        tests,
    };

    let report_md = deemer_core::results::render_report(&results, &suite);
    write_artifact(&results_path, &results.to_yaml()?)?;
    write_artifact(&report_path, &report_md)?;

    println!("{passed}/{total} passed — status: {}", status_str(status));
    println!("results: {}", results_path.display());
    println!("report:  {}", report_path.display());
    if let Some(e) = &suite_error {
        eprintln!("note: {e}");
    }

    std::process::exit(match status {
        Status::Passed => 0,
        Status::Failed => 1,
        Status::Errored => 2,
    });
}

/// Execute and evaluate a single data row into a [`TestRecord`].
async fn run_one(suite: &Suite, idx: usize, row: &Map, timeout: Duration) -> TestRecord {
    let test_number = idx + 1;
    let started = Utc::now();

    let command = match template::render(&suite.test.command, row) {
        Ok(c) => c,
        Err(e) => {
            return errored_record(
                test_number,
                row,
                suite.test.command.clone(),
                None,
                empty_execution(),
                None,
                format!("command template: {e}"),
            );
        }
    };

    let stdin = data_str(row, "stdin");
    let outcome = exec(&command, stdin.clone(), timeout).await;
    let execution = Execution {
        started_at: started,
        exit_code: outcome.exit_code,
        duration_ms: outcome.duration_ms,
        timed_out: outcome.timed_out,
        stdout: outcome.stdout.clone(),
        stderr: outcome.stderr.clone(),
    };

    let mut vars = row.clone();
    vars.insert("exit_code".to_string(), json!(outcome.exit_code));
    vars.insert("stdout".to_string(), json!(outcome.stdout));
    vars.insert("stderr".to_string(), json!(outcome.stderr));
    vars.insert("duration_ms".to_string(), json!(outcome.duration_ms));
    if let Some(s) = &stdin {
        vars.insert("stdin".to_string(), json!(s));
    }

    let mut ai_record = None;
    if let Some(ai) = &suite.test.evaluate.ai {
        let outputs = ai.expected_outputs.clone().unwrap_or_else(default_outputs);
        let rendered = match template::render(&ai.prompt, &vars) {
            Ok(p) => p,
            Err(e) => {
                return errored_record(
                    test_number,
                    row,
                    command,
                    stdin,
                    execution,
                    None,
                    format!("AI prompt template: {e}"),
                );
            }
        };
        let full = build_prompt(&rendered, &outputs);
        match call_model(&full).await {
            Ok(reply) => match parse_reply(&reply, &outputs) {
                Ok(parsed) => {
                    for (k, v) in &parsed {
                        vars.insert(k.clone(), v.clone());
                    }
                    ai_record = Some(AiRecord {
                        model: MODEL_LABEL.to_string(),
                        prompt: full,
                        response: reply,
                        outputs: parsed,
                    });
                }
                Err(e) => {
                    let rec = AiRecord {
                        model: MODEL_LABEL.to_string(),
                        prompt: full,
                        response: reply,
                        outputs: Map::new(),
                    };
                    return errored_record(
                        test_number,
                        row,
                        command,
                        stdin,
                        execution,
                        Some(rec),
                        format!("AI reply parse: {e}"),
                    );
                }
            },
            Err(e) => {
                return errored_record(
                    test_number,
                    row,
                    command,
                    stdin,
                    execution,
                    None,
                    format!("AI call: {e}"),
                );
            }
        }
    }

    let expr_str = suite.test.evaluate.assert.expression.clone();
    let substituted = template::substitute_for_log(&expr_str, &vars);
    match expr::evaluate(&expr_str, &vars) {
        Ok(result) => TestRecord {
            test_number,
            status: if result {
                Status::Passed
            } else {
                Status::Failed
            },
            assert: AssertRecord {
                expression: expr_str,
                substituted,
                result,
            },
            data: row.clone(),
            command,
            stdin,
            execution,
            ai: ai_record,
            error: None,
        },
        Err(e) => TestRecord {
            test_number,
            status: Status::Errored,
            assert: AssertRecord {
                expression: expr_str,
                substituted,
                result: false,
            },
            data: row.clone(),
            command,
            stdin,
            execution,
            ai: ai_record,
            error: Some(format!("assert: {e}")),
        },
    }
}

/// The captured result of running one command.
struct ExecOutcome {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
    duration_ms: u64,
}

/// Spawn a command through the system shell, feed it `stdin`, capture its output,
/// and enforce `timeout` (killing the child if it fires).
async fn exec(command: &str, stdin: Option<String>, timeout: Duration) -> ExecOutcome {
    let start = Instant::now();
    let (sh, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let mut child = match Command::new(sh)
        .arg(flag)
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return ExecOutcome {
                exit_code: None,
                stdout: String::new(),
                stderr: format!("failed to spawn command: {e}"),
                timed_out: false,
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
    };

    // Feed stdin, then close it so the process sees EOF.
    if let Some(mut sin) = child.stdin.take() {
        if let Some(input) = &stdin {
            let _ = sin.write_all(input.as_bytes()).await;
        }
        let _ = sin.shutdown().await;
    }

    // Read both pipes concurrently so a full pipe buffer can't deadlock the wait.
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let out_task = tokio::spawn(read_all(stdout));
    let err_task = tokio::spawn(read_all(stderr));

    let mut timed_out = false;
    let exit_code = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => status.code(),
        Ok(Err(_)) => None,
        Err(_) => {
            timed_out = true;
            let _ = child.start_kill();
            let _ = child.wait().await;
            None
        }
    };

    ExecOutcome {
        exit_code,
        stdout: out_task.await.unwrap_or_default(),
        stderr: err_task.await.unwrap_or_default(),
        timed_out,
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Read an async stream to a `String`, lossily decoding UTF-8.
async fn read_all<R: AsyncReadExt + Unpin>(mut r: R) -> String {
    let mut buf = Vec::new();
    let _ = r.read_to_end(&mut buf).await;
    String::from_utf8_lossy(&buf).to_string()
}

/// Send a single user prompt to the configured model and return the reply text.
async fn call_model(prompt: &str) -> Result<String> {
    use ailloy::{Client, Message};
    let client = Client::from_config()?;
    let response = client.chat(&[Message::user(prompt)]).await?;
    Ok(response.content)
}

/// Reserve the next allowed start slot, sleeping until it (without holding the
/// lock during the sleep), so starts are spaced by at least `interval`.
async fn reserve_rate_slot(next_start: &Mutex<Instant>, interval: Duration) {
    let wait = {
        let mut ns = next_start.lock().await;
        let now = Instant::now();
        let scheduled = (*ns).max(now);
        *ns = scheduled + interval;
        scheduled.saturating_duration_since(now)
    };
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
}

/// A compact rendering of the test records for the suite-AI's `{suite_context}`.
fn render_suite_context(tests: &[TestRecord]) -> String {
    let mut s = format!("{} tests:\n", tests.len());
    for t in tests {
        let reason =
            t.ai.as_ref()
                .and_then(|ai| ai.outputs.get("reason"))
                .and_then(|r| r.as_str())
                .unwrap_or("");
        s.push_str(&format!(
            "- test {} [{}] exit={:?}: {}\n",
            t.test_number,
            status_str(t.status),
            t.execution.exit_code,
            reason
        ));
    }
    s
}

fn report_cfg(suite: &Suite) -> Option<&str> {
    suite.report.as_ref().and_then(|r| r.path.as_deref())
}

fn results_cfg(suite: &Suite) -> Option<&str> {
    suite.results.as_ref().and_then(|r| r.path.as_deref())
}

/// Resolve an output path: CLI override (cwd-relative) → configured path
/// (suite-dir-relative) → `<suite-basename>.<default_ext>` (suite-dir-relative).
fn resolve_output(
    override_: Option<PathBuf>,
    configured: Option<&str>,
    suite_path: &Path,
    default_ext: &str,
) -> PathBuf {
    if let Some(p) = override_ {
        return p;
    }
    let dir = suite_path.parent().unwrap_or_else(|| Path::new("."));
    match configured {
        Some(rel) => dir.join(rel),
        None => dir.join(format!("{}.{default_ext}", suite_basename(suite_path))),
    }
}

/// The suite file's base name with a trailing `.suite.yml`/`.suite.yaml`/`.yml`/
/// `.yaml` removed (e.g. `safety.suite.yml` → `safety`).
fn suite_basename(suite_path: &Path) -> String {
    let name = suite_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "suite".to_string());
    for suffix in [".suite.yml", ".suite.yaml", ".yml", ".yaml"] {
        if let Some(base) = name.strip_suffix(suffix) {
            return base.to_string();
        }
    }
    name
}

/// Write a generated artifact, creating parent directories as needed.
fn write_artifact(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Parse a timeout like `500ms`, `30s`, `2m`. Defaults to 30s.
fn parse_timeout(s: Option<&str>) -> Duration {
    s.and_then(parse_duration)
        .unwrap_or_else(|| Duration::from_secs(30))
}

fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix("ms") {
        return n.trim().parse::<u64>().ok().map(Duration::from_millis);
    }
    if let Some(n) = s.strip_suffix('s') {
        return n.trim().parse::<f64>().ok().map(Duration::from_secs_f64);
    }
    if let Some(n) = s.strip_suffix('m') {
        return n
            .trim()
            .parse::<f64>()
            .ok()
            .map(|m| Duration::from_secs_f64(m * 60.0));
    }
    s.parse::<u64>().ok().map(Duration::from_secs)
}

fn data_str(row: &Map, key: &str) -> Option<String> {
    row.get(key).map(|v| match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

fn status_str(s: Status) -> &'static str {
    match s {
        Status::Passed => "passed",
        Status::Failed => "failed",
        Status::Errored => "errored",
    }
}

fn empty_execution() -> Execution {
    Execution {
        started_at: Utc::now(),
        exit_code: None,
        duration_ms: 0,
        timed_out: false,
        stdout: String::new(),
        stderr: String::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn errored_record(
    test_number: usize,
    row: &Map,
    command: String,
    stdin: Option<String>,
    execution: Execution,
    ai: Option<AiRecord>,
    msg: String,
) -> TestRecord {
    TestRecord {
        test_number,
        status: Status::Errored,
        assert: AssertRecord {
            expression: String::new(),
            substituted: String::new(),
            result: false,
        },
        data: row.clone(),
        command,
        stdin,
        execution,
        ai,
        error: Some(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runs_and_captures() {
        let e = exec("printf hello", None, Duration::from_secs(5)).await;
        assert_eq!(e.exit_code, Some(0));
        assert_eq!(e.stdout, "hello");
        assert!(!e.timed_out);
    }

    #[tokio::test]
    async fn pipes_stdin() {
        let e = exec("cat", Some("piped".to_string()), Duration::from_secs(5)).await;
        assert_eq!(e.stdout, "piped");
    }

    #[tokio::test]
    async fn times_out() {
        let e = exec("sleep 5", None, Duration::from_millis(100)).await;
        assert!(e.timed_out);
    }

    #[test]
    fn suite_basename_strips_suffixes() {
        assert_eq!(suite_basename(Path::new("a/safety.suite.yml")), "safety");
        assert_eq!(suite_basename(Path::new("checks.yaml")), "checks");
    }

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration("500ms"), Some(Duration::from_millis(500)));
        assert_eq!(parse_duration("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration("2m"), Some(Duration::from_secs(120)));
    }
}
