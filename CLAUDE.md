# CLAUDE.md

Guidance for Claude Code (and other agents) working in this repository.

## What this is

`deemer` is a CLI for running **AI-assisted integration tests** — for cases where deciding whether a
test or a whole test suite actually "worked" needs AI judgement rather than a plain assertion. The
domain logic (suite model, CEL asserts, AI judgement, results logs, reports) lives in `deemer-core`
and is driven by `deemer run` / `deemer check`, on top of the shared MKLab CLI scaffold described below.

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
  - `cli.rs` — clap-derive `Cli`, `Commands`, `AiCommands`, `Shell`; `Cli::run` dispatches. The
    no-subcommand (`None`) arm prints the banner and usage.
  - `commands/` — one module per command (`ai`, `completion`, `run`, `check`). Add new commands here.
  - `banner.rs` — ASCII block-letter banner + version line.
  - `update.rs` — polls crates.io, caches the result for 24h, notifies on a newer version.
- `crates/deemer-core/` — framework-agnostic library (no clap/tokio). deemer's domain logic lives here.
  - `suite.rs` — the v2 test-suite model (YAML), `check.rs` — static validation (`deemer check`).
  - `expr.rs` / `value.rs` — CEL `assert` evaluation and the value model, `template.rs` — templating
    and log substitution.
  - `judge.rs` — AI prompt assembly and reply parsing/validation, `results.rs` — results log,
    `config_sha256`, and report rendering.
  - `config.rs` — YAML `Config` in `~/.config/deemer/`, a reusable starting point (not yet wired
    into the CLI).
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

- Edition 2024, MSRV 1.88 (`[workspace.package]`; set by Ailloy 2.x).
- All deps are declared in the root `[workspace.dependencies]` and inherited with `.workspace = true`.
  `serde_yaml` stays on 0.9 (see the `# Stays on ...` comment in `Cargo.toml`).
- Building from source on Windows needs NASM and CMake on `PATH` — `aws-lc-rs` (reqwest's TLS crypto
  backend) compiles optimized assembly routines at build time. macOS and Linux need nothing extra.
  The release workflow's Windows leg installs NASM via `ilammy/setup-nasm@v1`; CMake and MSVC are
  already on the `windows-latest` image.
- CI gates: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`. CI runs the latest stable toolchain, so run the gates on an up-to-date
  local toolchain (new clippy lints otherwise surface only in CI).
- Releases go through the `/release` skill (`.claude/skills/release/`) → tag push → `release.yml`. The
  skill updates the toolchain and dependencies first, then watches the workflow. `release.yml` builds
  binaries with `cargo auditable` and attaches a per-target CycloneDX SBOM (`.cdx.json`) to the
  GitHub Release alongside the archives.

## Dependency Policy

We keep this tool's dependencies at their latest compatible versions, not just the versions that
happen to still compile. Staying current is the default, not something we get to eventually —
letting dependencies drift is how technical debt accumulates unnoticed until a security advisory or
a forced breaking upgrade makes it urgent. When a newer major is available and there's no concrete,
documented reason not to take it (see any `# Stays on ...` comments in `Cargo.toml` for the current
exceptions and why), take it during the next maintenance round rather than deferring it. The
cross-repo `maintaining-rust-tools` skill drives this for the whole fleet (ailloy + cosq + deemer +
mdeck + pidge + rigg + rusty-tmpl).
