<p align="center"><img src="https://raw.githubusercontent.com/mklab-se/deemer/main/media/deemer-horizontal.png" width="600"></p>

<p align="center">
<a href="https://github.com/mklab-se/deemer/actions/workflows/ci.yml"><img src="https://github.com/mklab-se/deemer/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
<a href="https://crates.io/crates/deemer"><img src="https://img.shields.io/crates/v/deemer.svg" alt="crates.io"></a>
<a href="https://github.com/mklab-se/deemer/releases/latest"><img src="https://img.shields.io/github/v/release/mklab-se/deemer" alt="GitHub Release"></a>
<a href="https://github.com/mklab-se/homebrew-tap/blob/main/Formula/deemer.rb"><img src="https://img.shields.io/badge/dynamic/regex?url=https%3A%2F%2Fraw.githubusercontent.com%2Fmklab-se%2Fhomebrew-tap%2Fmain%2FFormula%2Fdeemer.rb&search=%5Cd%2B%5C.%5Cd%2B%5C.%5Cd%2B&label=homebrew&prefix=v&color=orange" alt="Homebrew"></a>
<a href="https://github.com/mklab-se/deemer/blob/main/LICENSE"><img src="https://img.shields.io/crates/l/deemer.svg" alt="License"></a>
</p>

# deemer

**Run AI-assisted integration tests that judge whether your tests passed.** `deemer` is for the cases
where deciding whether a test — or a whole test suite — actually *worked* needs AI judgement rather
than a plain pass/fail assertion.

> **Status: early.** The binary currently prints `Hello world!` with no subcommand — the
> test-running and AI-judgement commands are being built on top of the plumbing below. The
> architecture and full release pipeline are already wired up.

Built on MKLab's shared Rust CLI conventions (the same scaffold behind
[cosq](https://github.com/mklab-se/cosq), [pidge](https://github.com/mklab-se/pidge), and
[rigg](https://github.com/mklab-se/rigg)):

- 📦 A Cargo **workspace** (`deemer` binary + `deemer-core` library)
- 🧰 A [clap](https://docs.rs/clap)-derive CLI with global flags (`-v`, `-q`, `--no-color`) and `--help`
- 🤖 An `ai` subcommand backed by [**Ailloy**](https://crates.io/crates/ailloy), MKLab's shared AI config
- 🐚 Static **and** dynamic shell completions
- 🔔 A background **crates.io update checker**
- ⚙️ **GitHub Actions** CI (check / test / clippy / fmt) and a release pipeline
- 🍺 Automated publishing to **crates.io** and **Homebrew**
- 🪄 A `/release` skill that drives the whole release flow

## Build & run

```sh
cargo run                    # prints "Hello world!"
cargo run -- --help          # show the CLI help
cargo run -- version         # banner + version
cargo run -- ai              # AI status (via Ailloy)
cargo run -- completion zsh  # generate a zsh completion script
cargo test --workspace       # run the unit tests
```

### Install

See [INSTALL.md](INSTALL.md) for Homebrew, `cargo install`, `cargo binstall`, and from-source instructions.

## AI integration (Ailloy)

The `ai` subcommand reuses MKLab's shared [Ailloy](https://crates.io/crates/ailloy) configuration
(`~/.config/ailloy/config.yaml`), so every tool shares the same providers and API keys.

```sh
deemer ai          # show status
deemer ai config   # interactively configure a provider/model
deemer ai test     # send a test message
deemer ai enable   # / disable — toggle AI for this tool
```

To call a model from your own commands, use `ailloy::Client` — see `crates/deemer/src/commands/ai.rs`
for where the integration lives.

## Releasing

Releases are driven by the [`/release`](.claude/skills/release/SKILL.md) skill (run it in Claude
Code with `major`, `minor`, or `patch`). It bumps the version, updates the changelog, then commits,
pushes, and tags `vX.Y.Z`. Pushing the tag triggers `.github/workflows/release.yml`, which:

1. Re-runs the full CI suite
2. Builds binaries for Linux, macOS (Intel + ARM), and Windows
3. Creates a GitHub Release with the artifacts
4. Publishes `deemer-core` then `deemer` to crates.io
5. Updates the Homebrew formula in [`mklab-se/homebrew-tap`](https://github.com/mklab-se/homebrew-tap)

### Required secrets

Configure these once on the GitHub repository (these are the same secrets used by the other MKLab tools):

| Secret | Where | Purpose | How to create |
| --- | --- | --- | --- |
| `CARGO_REGISTRY_TOKEN` | Environment **`crates-io`** | Publish to crates.io | [crates.io/settings/tokens](https://crates.io/settings/tokens) → new token with publish scope |
| `HOMEBREW_TAP_TOKEN` | Repository secret | Push the formula to the tap | A GitHub PAT with `repo` scope for `mklab-se/homebrew-tap` |

If `HOMEBREW_TAP_TOKEN` is missing, the release still succeeds — the Homebrew step just logs a warning.

## Development

```sh
cargo fmt --all              # format
cargo clippy --workspace -- -D warnings   # lint (matches CI)
cargo test --workspace       # test
```

The CLI lives in `crates/deemer` and reusable logic in `crates/deemer-core`. To add a
command: declare it in `cli.rs` (`Commands` enum), add a module under `commands/`, and wire the
dispatch arm in `Cli::run`. See [CLAUDE.md](CLAUDE.md) for the architecture in more detail.

## License

[MIT](LICENSE) © Kristofer Liljeblad
