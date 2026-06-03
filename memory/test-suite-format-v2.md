---
name: test-suite-format-v2
description: The v2 deemer test-suite YAML format — designed, documented, and reflected across docs+samples
metadata:
  type: project
---

The deemer test-suite YAML format was redesigned and AGREED with the user (Jun 2026) through an
iterative 18-decision design discussion, then fully consolidated. State of the repo:
- `docs/test-suite-config.md` (suite reference) and `docs/test-results.md` (results-log reference)
  are REWRITTEN for v2 — these are now the authoritative spec.
- `docs/samples/annotated.suite.yml` is the fully-commented reference suite (this is the renamed,
  decisions-log-stripped successor to the old `my-test-suite-suggestion-v2.yml`, which was DELETED
  along with the original `my-test-suite-suggestion.yml`).
- Three focused paired samples (safety-checks, cli-smoke, support-tone) — `.suite.yml` + `.results.yml`
  — plus `docs/samples/README.md` all conform to v2.
- Root `README.md` was rewritten as a sales/landing page for the tool.
The design-decision history (D1–D18) no longer lives in any file; it was distilled into the docs.

Key shape: one `{}` syntax everywhere (text in templates, type-aware literal substitution in
expressions); fully flat variable namespace + reserved-name guard; `expected_outputs` terse-type +
object escape hatch; `command` free-form (no escaping) + auto-wired per-row `stdin`; report
defaults-with-override; `settings.concurrency` + `settings.rate_limit: count/unit`;
`test_suite_format: 1` (integer); snake_case throughout; AI clause optional (deterministic suites
allowed); auto `{test_number}`; explicit optional `report.path` + new `results.path` block
(default to suite-file basename, resolved against suite dir, auto-mkdir). An AI-driven
`deemer check` linter is part of the design.

Later decisions (D13–D18): AI verdict boolean is `ai_passed` (distinct from the test's real
verdict `{assert_result}`); `assert` is REQUIRED at both test and suite level (no default
verdict); single eval mechanism — `match:` dropped, only `assert.expression` (CEL-style
methods); tests identified by `{test_number}` only (no label/name field); results log lists
tests in DATA-ENTRY order regardless of concurrency; the results file is the complete
structured log (source of truth) and records each assert as authored + substituted + boolean
(large values like stdout elided in the substituted form).

**Why:** the user wanted a more human-friendly format than the original scaffold's.
**How to apply:** the design + docs are DONE. Remaining real work is in the Rust code — the
test-*running* commands don't exist yet (the binary still prints "Hello world!"). When implementing,
treat `docs/test-suite-config.md` + `docs/test-results.md` as the spec, and `deemer-core` as the home
for the suite/results models + AI judgement.
