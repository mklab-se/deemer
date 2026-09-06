<p align="center"><img src="https://raw.githubusercontent.com/mklab-se/deemer/main/media/deemer-horizontal.png" width="600"></p>

<p align="center">
<a href="https://github.com/mklab-se/deemer/actions/workflows/ci.yml"><img src="https://github.com/mklab-se/deemer/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
<a href="https://crates.io/crates/deemer"><img src="https://img.shields.io/crates/v/deemer.svg" alt="crates.io"></a>
<a href="https://github.com/mklab-se/deemer/releases/latest"><img src="https://img.shields.io/github/v/release/mklab-se/deemer" alt="GitHub Release"></a>
<a href="https://github.com/mklab-se/homebrew-tap/blob/main/Formula/deemer.rb"><img src="https://img.shields.io/badge/dynamic/regex?url=https%3A%2F%2Fraw.githubusercontent.com%2Fmklab-se%2Fhomebrew-tap%2Fmain%2FFormula%2Fdeemer.rb&search=%5Cd%2B%5C.%5Cd%2B%5C.%5Cd%2B&label=homebrew&prefix=v&color=orange" alt="Homebrew"></a>
<a href="https://github.com/mklab-se/deemer/blob/main/LICENSE"><img src="https://img.shields.io/crates/l/deemer.svg" alt="License"></a>
</p>

# deemer

**Integration tests, judged by AI.** Deemer runs a command against a list of inputs and decides whether
each run — and the suite as a whole — passed. When a plain `assertEquals` can't capture "did this
*actually* work?", you let a model judge it, and you still get a deterministic pass/fail and a complete
audit trail.

---

## The problem

Some things are easy to assert: an exit code, an exact string, a JSON field. Many things aren't:

- Did the chatbot **refuse** the unsafe request — or just crash?
- Is the support reply **empathetic and on-topic**, with no invented promises?
- Did the summariser **keep the meaning** while cutting the length?
- Is the generated README **actually helpful** to a newcomer?

These are real test cases, and today they're either skipped, checked by brittle keyword matching, or
left to a human to eyeball. Deemer makes them **first-class, repeatable tests**: you write the
criteria in plain language, an AI evaluates each run, and an `assert` turns the result into a verdict
your CI can gate on.

## How it works

A **test suite** is one YAML file. Deemer runs your `command` once per data row, optionally asks an AI
to evaluate the output, and an `assert` decides pass/fail — for each test and for the suite.

```yaml
test_suite_format: 1
name: "Support reply quality"
description: "supportbot's drafts must be empathetic, on-topic, and make no false promises."

test:
  command: "supportbot --draft"      # the ticket is piped to stdin (see the `stdin:` rows below)
  evaluate:
    ai:
      prompt: |
        Judge the drafted support reply. It passes only if it is empathetic, addresses
        the customer's problem with a concrete next step, and promises nothing it can't keep.
        Ticket: {stdin}
        Reply:  {stdout}
      expected_outputs:
        ai_passed: boolean
        reason: string
    assert:
      expression: '{exit_code} == 0 && {ai_passed} == true'

suite:
  assert:
    expression: '{pass_rate} >= 0.9'     # at least 90% of drafts must pass

data:
  - stdin: "My order is 3 days late and still not here. This is ridiculous."
  - stdin: "I was charged twice this month — please fix it."
  - stdin: "Any plans for a dark mode?"
```

Run it, and Deemer writes two artifacts: a **results log** (a complete, structured YAML record of every
run — the command, the output, the AI's full prompt and parsed verdict, and the assert) and a
human-readable **report** rendered from it. The log is the source of truth; the report is a view.

## Why Deemer

- **AI when you need it, deterministic when you don't.** The same suite can mix AI-judged tests and
  plain `exit_code == 0` checks. No AI? It runs as a fast, ordinary test runner.
- **You write criteria, not JSON wrangling.** Declare the values you want back; Deemer generates the
  response format, parses it, and exposes each as a variable your `assert` can use.
- **One simple syntax.** `{placeholders}` everywhere — in commands, prompts, asserts, and reports.
- **A real audit trail.** Every AI call's *complete* assembled prompt and reply is stored, so a verdict
  is never a black box. Re-render different reports from one run without re-executing.
- **CI-ready.** A single `status: passed|failed|errored` to gate on, plus concurrency and rate-limit
  controls for suites that hit an API.
- **Honest about non-determinism.** AI judgement isn't perfectly repeatable; Deemer is built to surface
  the model's reasoning (and even its own confidence) so you decide how strict to be.

## Status

> **v1 runner landed; early but usable.** `deemer run <suite.yml>` and `deemer check <suite.yml>` are
> implemented — deterministic *and* AI-judged suites run, producing a results log and a report, with a
> `0`/`1`/`2` exit code for CI. AI evaluation uses your configured [Ailloy](https://crates.io/crates/ailloy)
> model. Rough edges remain (e.g. `working_dir`/`env` suite fields and per-test model overrides are not
> yet wired — use the shell command inline for now). Expect changes before a tagged release.

## Getting started

Install (see [INSTALL.md](INSTALL.md) for Homebrew, `cargo install`, `cargo binstall`, from source):

```sh
cargo install deemer
```

Then explore the format — these are the best starting points today:

- **[`docs/samples/annotated.suite.yml`](docs/samples/annotated.suite.yml)** — a fully-commented suite touring every feature.
- **[`docs/samples/`](docs/samples/)** — three focused examples, each paired with the results log it produces.
- **[`docs/test-suite-config.md`](docs/test-suite-config.md)** — the suite format reference.
- **[`docs/test-results.md`](docs/test-results.md)** — the results-log reference.

Configure AI once (shared across all MKLab tools via [Ailloy](https://crates.io/crates/ailloy)):

```sh
deemer ai config   # pick a provider/model interactively
deemer ai test     # send a test message
deemer ai          # show status
```

## Development

```sh
cargo run -- --help                       # CLI help
cargo fmt --all                           # format
cargo clippy --workspace -- -D warnings   # lint (matches CI)
cargo test --workspace                    # test
```

The CLI lives in `crates/deemer`; reusable, framework-agnostic logic (config, the suite/results models,
AI judgement) belongs in `crates/deemer-core`. To add a command: declare it in `cli.rs` (`Commands`
enum), add a module under `commands/`, and wire the dispatch arm in `Cli::run`. See
[CLAUDE.md](CLAUDE.md) for the architecture.

## Releasing

Releases are driven by the [`/release`](.claude/skills/release/SKILL.md) skill (run it in Claude Code
with `major`, `minor`, or `patch`): it bumps the version, updates the changelog, commits, pushes, and
tags `vX.Y.Z`. Pushing the tag triggers [`release.yml`](.github/workflows/release.yml), which re-runs
CI, builds [auditable](https://github.com/rust-secure-code/cargo-auditable) binaries for
Linux/macOS/Windows with a CycloneDX SBOM per target, creates a GitHub Release, publishes
`deemer-core` then `deemer` to crates.io, and updates the Homebrew formula in
[`mklab-se/homebrew-tap`](https://github.com/mklab-se/homebrew-tap). See
[INSTALL.md](INSTALL.md#software-bill-of-materials-sbom) for how to read the SBOM.

Two secrets enable publishing (configured once on the repo): `CARGO_REGISTRY_TOKEN` (in the
`crates-io` environment) and `HOMEBREW_TAP_TOKEN` (a repo secret). If the Homebrew token is missing the
release still succeeds — that step just logs a warning.

## License

[MIT](LICENSE) © MKLab AB
