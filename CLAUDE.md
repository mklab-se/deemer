# CLAUDE.md

Guidance for Claude Code (and other agents) working in this repository.

## What this is

`deemer` is a CLI for running **AI-assisted integration tests** — for cases where deciding whether a
test or a whole test suite actually "worked" needs AI judgement rather than a plain assertion. The
binary currently still prints `Hello world!` with no subcommand; the domain logic (test-run models,
AI judgement, verdicts) is being built on top of the shared MKLab CLI scaffold described below.

## Template lineage

> **Note for agents:** `deemer` was scaffolded from the `rusty-tmpl` template and then renamed. The
> references in this section intentionally still point at the upstream template, not at this tool.

- **Upstream template:** https://github.com/mklab-se/rusty-tmpl
- **Likely local clone:** `../rusty-tmpl/` (sibling directory under the same parent)

That template carries the shared MKLab CLI conventions (workspace layout, clap CLI, Ailloy `ai`
command, update checker, CI/release pipeline, `/release` skill). Use the lineage in both directions:

- **Pulling improvements in** — when the template gains a fix or new convention, compare against
  `../rusty-tmpl/` and port the relevant change here, adapting the `rusty-tmpl` name to `deemer`.
- **Pushing improvements back** — if you discover a fix or better pattern here that is *generic*
  (not specific to deemer's domain), consider contributing it upstream to `mklab-se/rusty-tmpl`
  so every future tool benefits. Generalize it (strip deemer-specific names/logic) before doing so.

When working from the local clone, prefer reading `../rusty-tmpl/` directly to diff conventions; fall
back to the GitHub URL if the sibling directory isn't present.

## Architecture

A two-crate Cargo workspace:

- `crates/deemer/` — the CLI binary.
  - `main.rs` — `#[tokio::main]`; sets up logging, dynamic-completion env, the `--no-color` override,
    and spawns the background update check, then calls `Cli::run`.
  - `cli.rs` — clap-derive `Cli`, `Commands`, `AiCommands`, `Shell`; `Cli::run` dispatches. **The
    no-subcommand (`None`) arm currently prints `Hello world!`** (inherited from the scaffold) — replace
    it with deemer's real default once the test-running commands exist. `--help`/`-h` still work via clap.
  - `commands/` — one module per command (`ai`, `completion`). Add new commands here.
  - `banner.rs` — ASCII block-letter banner + version line.
  - `update.rs` — polls crates.io, caches the result for 24h, notifies on a newer version.
- `crates/deemer-core/` — framework-agnostic library (no clap/tokio). deemer's domain logic belongs here.
  - `config.rs` — YAML `Config` in `~/.config/deemer/`, a reusable starting point (unused so far).
  - `error.rs` — `thiserror` `Error` enum + `Result` alias.

## Adding a command

1. Add a variant to `Commands` in `cli.rs` (with a doc comment — it becomes the help text).
2. Add a `pub mod <name>;` in `commands/mod.rs` and implement `pub async fn run(...) -> anyhow::Result<()>`.
3. Add the dispatch arm in `Cli::run`.

## AI integration

`commands/ai.rs` wraps [Ailloy](https://crates.io/crates/ailloy) via its `config_tui` helpers and the
shared global config (`~/.config/ailloy/config.yaml`). To call a model from a command, use
`ailloy::Client`. The capability list is the `CAPABILITIES` const in `ai.rs` (`["chat"]`).

## Conventions

- Edition 2024, MSRV 1.85 (`[workspace.package]`).
- All deps are declared in the root `[workspace.dependencies]` and inherited with `.workspace = true`.
- CI gates: `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`.
- Releases go through the `/release` skill (`.claude/skills/release/`) → tag push → `release.yml`.
