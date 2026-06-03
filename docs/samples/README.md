# Sample suites & results

Runnable-shaped examples of the Deemer **v2** formats. The commands are **fake** (`mockbot`,
`widgetctl`, `supportbot`) — these exist to illustrate the file format, not to execute.

New here? Read [`annotated.suite.yml`](./annotated.suite.yml) first — a fully-commented tour of every
feature. The three focused samples below are minimal and each is paired with the results **log** it
would produce. The results file is the complete, structured record of a run (the source of truth); a
human report is a *derived view* rendered from it. Together they cover the matrix of strategies:

| Sample | Per-test evaluation | Suite evaluation | Input |
| --- | --- | --- | --- |
| **safety-checks** | AI eval (`ai_passed`, `reason`, `severity`, `categories`), combined in the `assert` with a clean exit + a severity ceiling | AI eval over `{suite_context}` | per-row `stdin` |
| **cli-smoke** | Deterministic `assert.expression` reading each row's data fields — no AI | Deterministic `assert` (require all to pass) | command argument |
| **support-tone** | AI eval (`ai_passed`, `reason`) | Deterministic `assert` on `{pass_rate} >= 0.9` | per-row `stdin` |

Files:

- [`annotated.suite.yml`](./annotated.suite.yml) — the fully-commented reference (no paired log)
- [`safety-checks.suite.yml`](./safety-checks.suite.yml) → [`safety-checks.results.yml`](./safety-checks.results.yml)
- [`cli-smoke.suite.yml`](./cli-smoke.suite.yml) → [`cli-smoke.results.yml`](./cli-smoke.results.yml)
- [`support-tone.suite.yml`](./support-tone.suite.yml) → [`support-tone.results.yml`](./support-tone.results.yml)

What to look for while reviewing:

- **safety-checks** — the author declares the values they want back (`ai_passed`, `reason`, `severity`,
  `categories`) but never writes a "respond as JSON" sentence; the results log stores the *full*
  assembled prompt — including the response instruction Deemer generated from `expected_outputs` — for
  every AI call. Each test's verdict is its `assert` result (recorded authored + substituted +
  boolean), and the suite's AI `executive_summary` heads the report.
- **cli-smoke** — a deterministic suite with no AI anywhere: one `assert.expression` reads each row's
  own data fields (`{code}`, `{needle}`), and the suite `assert` requires all tests to pass.
- **support-tone** — AI judging each draft, but a deterministic suite `assert` (`{pass_rate} >= 0.9`)
  decides the suite; note `status: failed` because 80% < 90%.

Format reference: [`../test-suite-config.md`](../test-suite-config.md) (the suite) and
[`../test-results.md`](../test-results.md) (the results log).
