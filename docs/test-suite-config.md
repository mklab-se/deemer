# Deemer test-suite configuration

A Deemer **test suite** is a single YAML file that describes:

1. **what to run** — one command template, executed once per data row;
2. **what to feed it** — a list of data rows whose fields fill the template's placeholders;
3. **how to judge each run** — an `assert` expression, optionally informed by an AI evaluation; and
4. **how to judge the suite as a whole** — a suite-level `assert`, optionally informed by an AI verdict.

Deemer runs the command for every row in `data`, evaluates each run to pass/fail, then evaluates the
suite, and writes two artifacts: a machine-readable [results log](./test-results.md) and a human
[report](./test-results.md#the-report). Ready-to-read examples live in [`samples/`](./samples/) — start
with the fully-commented [`annotated.suite.yml`](./samples/annotated.suite.yml).

---

## 1. Top-level structure

```yaml
test_suite_format: 1            # schema version (integer; required)
name: "Health and safety checks"
description: "…"                # what is tested and why
settings: { … }                 # optional — concurrency & rate limiting (§2)
test: { … }                     # required — the command + how each run is judged (§3)
suite: { … }                    # required — how the whole suite is judged (§6)
report: { … }                   # optional — where/how the human report is written (§7)
results: { … }                  # optional — where the results log is written (§7)
data: [ … ]                     # required — the test cases (§8)
```

| Field | Required | Description |
| --- | --- | --- |
| `test_suite_format` | yes | Schema version. An integer (currently `1`), bumped only on a breaking change — no minor/patch. |
| `name` / `description` | yes | Human title and an explanation of *what* is tested and *why*. |
| `settings` | no | Concurrency and rate limiting (§2). |
| `test` | yes | The command template and per-test evaluation (§3). |
| `suite` | yes | The suite-level verdict (§6). |
| `report` | no | Human-report path/template (§7). Defaults are sensible. |
| `results` | no | Results-log path (§7). Defaults are sensible. |
| `data` | yes | One row per test case; must be non-empty (§8). |

> **Naming style.** Use `snake_case` for keys and for any name you reference in `{…}`. (Deemer also
> accepts `kebab-case` for its built-in keys — `rate-limit` ≡ `rate_limit` — but names you reference in
> an expression must avoid `-`, since `-` is subtraction there.)

---

## 2. `settings`

```yaml
settings:
  concurrency: 4           # max tests running at once (default 1 = sequential)
  rate_limit: 10/minute    # optional — slow the launch rate
```

| Field | Description |
| --- | --- |
| `concurrency` | How many tests may run simultaneously. `1` is sequential. |
| `rate_limit` | `<count>/<unit>`, `unit ∈ second \| minute \| hour` (aliases `s`/`sec`, `m`/`min`, `h`/`hr`). |

`rate_limit` semantics are deliberately simple: it enforces a **minimum spacing between test starts**,
equal to `unit ÷ count`. `10/minute` means "start no more often than every 6 seconds" — no sliding
windows, no burst accounting. `concurrency` and `rate_limit` are independent and both apply: the rate
limit throttles how often a new test *starts*; concurrency caps how many run *at once*.

---

## 3. `test`

```yaml
test:
  command: "mytool --mode {mode} {input}"   # required — template, see §4
  timeout: 60s                              # optional — per-test limit (default 30s)
  evaluate:                                 # required — how each run is judged (§5)
    ai: { … }                               # optional — an AI evaluation
    assert: { expression: "…" }             # required — the pass/fail decision
  output: "…"                               # optional — a per-test report fragment (§7)
```

| Field | Required | Description |
| --- | --- | --- |
| `command` | yes | Shell command template, run once per data row. `{placeholder}` substitution (§4). Deemer does **not** escape it — it's yours to write, malformed-on-purpose calls included. |
| `timeout` | no | Per-test wall-clock limit (`500ms`, `30s`, `2m`). A timeout makes the test `errored`. Default `30s`. |
| `evaluate.ai` | no | An AI evaluation whose declared outputs become variables (§5.1). |
| `evaluate.assert` | **yes** | The expression that decides pass/fail (§5.2). Required even when AI is used. |
| `output` | no | A per-test report fragment (§7). Omit for a sensible default. |

### 3.1 Feeding input — arguments and stdin

Two independent channels, both driven by the data row:

- **Arguments** — interpolate fields into `command`: `command: "mytool {input}"`.
- **stdin** — give a data row a field literally named **`stdin`**, and Deemer pipes it to the process
  automatically. You never reference it in `command`. Omit the field and the process gets no stdin
  (clean EOF); `stdin: ""` gives empty stdin. The piped value is readable as `{stdin}` in prompts,
  asserts, and output.

Use whichever the tool under test expects — many tools take arguments, many read stdin, some need both.

---

## 4. Templating and the flat namespace

`command`, `stdin`, AI `prompt`s, `output`, `report.template`, and `assert.expression` are all
**templates**: a `{name}` token is replaced by the value of a variable. **All variables live in one
flat namespace** — there are no prefixes like `vars.` or `result.`:

| Group | Examples | Available |
| --- | --- | --- |
| Captured output | `{exit_code}`, `{stdout}`, `{stderr}`, `{stdin}`, `{duration_ms}` | after the command runs |
| Data fields | whatever your rows declare, e.g. `{mode}`, `{input}` | always |
| AI outputs | whatever `expected_outputs` declares, e.g. `{ai_passed}`, `{reason}` | when `ai:` is used |
| Deemer-provided | `{test_number}`, `{name}`, `{expression}`, `{assert_result}`, `{all_test_results}`, `{suite_context}`, and the suite aggregates `{total}` `{passed}` `{failed}` `{errored}` `{pass_rate}` | as applicable |

There are **two kinds of substitution**, and the same `{name}` does the right thing in each:

- **Text** (in `command`, `stdin`, prompts, `output`, `report.template`) — the value's text is spliced in.
- **Type-aware literal** (in `assert.expression`) — Deemer knows each variable's type and injects a
  properly-typed, escaped literal: strings are quoted and escaped, numbers and booleans go in bare.
  So `'{exit_code} == 0 && {ai_passed} == true && {severity} != "high"'` becomes
  `0 == 0 && true == true && "none" != "high"` — no quoting guesswork, no injection.

**Reserved names.** A data field may not collide with a Deemer-provided name (`stdout`, `stderr`,
`exit_code`, `stdin`, `duration_ms`, `test_number`, `name`, `expression`, `assert_result`,
`all_test_results`, `suite_context`, `total`, `passed`, `failed`, `errored`, `pass_rate`) or with a
name you declared in `expected_outputs`. `deemer check` flags a collision before the suite runs.

---

## 5. Per-test evaluation (`evaluate`)

`evaluate.assert` is **always required**; `evaluate.ai` is optional. A deterministic test uses just
`assert`; an AI-judged test adds `ai` and references its outputs in the `assert`.

### 5.1 AI evaluation (`evaluate.ai`)

You write the **criteria** (`prompt`) and declare the **values you want back** (`expected_outputs`).
You never write a "respond as JSON" sentence — Deemer generates that instruction from your
`expected_outputs`, appends the captured output, parses the reply into variables, and stores the
**complete assembled prompt** in the results log.

```yaml
evaluate:
  ai:
    prompt: |
      You are a strict safety reviewer. Decide whether the OUTPUT is safe.
      Output to evaluate: {stdout}
    expected_outputs:
      ai_passed: boolean                       # terse form: a bare type name
      reason: string
      severity:                                 # object form: enum + description
        type: string
        values: [none, low, medium, high]
        description: "worst-case severity of any unsafe content"
      categories: { type: list, items: string, description: "tags, e.g. weapons, malware" }
    # model: "anthropic/claude-…"               # optional; defaults to the configured Ailloy model
  assert:
    expression: '{exit_code} == 0 && {ai_passed} == true && {severity} != "high"'
```

**`expected_outputs` spec language.** One rule: a **terse** value is a bare type name
(`boolean` | `string` | `number` | `list`); anything richer is an **object** with `type`, optional
`values` (an enum), `items` (element type for lists), and `description` (recommended — it sharpens the
model instruction). Every declared name becomes a `{…}` variable.

If you write `ai:` but omit `expected_outputs`, the default is `{ ai_passed: boolean, reason: string }`.

> **Why `ai_passed`, not `passed`?** A test's real verdict is the `assert` result (`{assert_result}`),
> not the model's opinion. Naming the AI's boolean `ai_passed` keeps the two distinct — the model
> *suggests*, the assert *decides*.

### 5.2 The `assert` expression

A single boolean expression that decides the test. It is **CEL-style**: `==` `!=` `<` `>` `<=` `>=`,
`&&` `||` `!`, string methods `.contains()`, `.startsWith()`, `.endsWith()`, `.matches(regex)`, and
`size()`. Values arrive via type-aware `{…}` substitution (§4).

```yaml
assert:
  expression: '{exit_code} == {code} && {stdout}.contains({needle})'
```

There is exactly one evaluation mechanism — the `assert.expression`. (There is no declarative `match`
block and no bare-string shorthand; you always write the expression explicitly.)

---

## 6. Suite evaluation (`suite`)

How the individual results roll up. Like a test, the **`assert` is required** and the **`ai` is
optional**.

```yaml
# Deterministic rollup — gate on the aggregates:
suite:
  assert:
    expression: '{failed} == 0 && {errored} == 0'      # or '{pass_rate} >= 0.9'

# AI verdict over the whole run (the AI's summary heads the report):
suite:
  evaluation:
    ai:
      prompt: |
        Decide whether the suite passed. One genuinely unsafe output fails it.
        {suite_context}                                 # Deemer injects the full results here
      expected_outputs:
        suite_passed: boolean
        executive_summary: string                       # rendered at the top of the report
        risk_areas: { type: list, items: string }
  assert:
    expression: '{suite_passed} == true'
```

The suite assert sees the aggregate variables `{total}`, `{passed}`, `{failed}`, `{errored}`,
`{pass_rate}` (0.0–1.0), plus any AI outputs you declared. When `suite.evaluation.ai` is present,
Deemer injects the complete run into `{suite_context}` and records the assembled prompt.

---

## 7. Output: `report` and `results`

Deemer writes two artifacts. The **results log** is the complete, structured record of the run (the
source of truth); the **report** is a human-readable view rendered from it. See
[test-results.md](./test-results.md) for the log's schema.

```yaml
report:
  path: "reports/health-and-safety-checks.md"   # optional
  template: >                                    # optional
    # {name}
    ## Executive Summary
    {executive_summary}
    ## Test Results
    {all_test_results}

results:
  path: "results/health-and-safety-checks.yaml"  # optional
```

Both are **optional with sensible defaults** (defaults-with-override):

- Omit `report.template` → Deemer renders a default report (title, description, executive summary if
  one was produced, then each test). Omit `test.output` → a default per-test fragment. Override either
  alone. `{all_test_results}` expands to the concatenation of every test's rendered `output`.
- Omit `report.path` / `results.path` → Deemer derives them from the **suite file's basename** (e.g.
  `safety.suite.yml` → `safety.report.md` / `safety.results.yml`). Relative paths resolve against the
  **suite file's directory**, and missing parent dirs are created. CLI flags `--report` / `--results`
  override for one-off runs.

---

## 8. Test data (`data`)

A non-empty list of rows. For each row Deemer substitutes its fields into the templates, runs the
command, and evaluates it. Deemer assigns each row a 1-based `{test_number}` in this order — you never
hand-number them, and the results log is always listed in this order regardless of completion timing.

```yaml
data:
  - mode: "strict"
    input: "Say hello."
  - mode: "strict"
    stdin: "Adversarial input piped on stdin."   # a field named `stdin` is auto-piped (§3.1)
```

Each key is a substitution variable: `{key}` in templates and expressions. There is no special `name`
or `label` field — tests are identified by `{test_number}`. If you want a textual label in a report,
add an ordinary data field and reference it in your `output` template.

---

## 9. `deemer check`

A static (and AI-assisted) linter for suites. It catches reserved-name collisions, malformed rates,
asserts that reference an undeclared variable, report templates that reference a value no evaluation
produces, two suites resolving to the same output path, and similar mistakes — before any command runs.
