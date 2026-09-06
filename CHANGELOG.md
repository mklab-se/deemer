# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-09-07

### Changed

- Ailloy 2.0 → 2.1 (keyring 4, ratatui 0.30 config dashboard, hardened Azure CLI token parsing),
  plus `cargo update`.

## [0.2.0] - 2026-09-06

### Added

- **v2 test-suite format and the `deemer run` / `deemer check` commands.** `deemer run <suite.yml>`
  runs deterministic and AI-judged tests, writes a results log and a report (default or custom
  template), and exits `0`/`1`/`2` for CI. `deemer check <suite.yml>` validates a suite statically.
  Suite features: templating and log substitution, CEL `assert` expressions, `rate_limit`, AI prompt
  assembly with reply parsing/validation, and a `config_sha256` tying each results log to its suite.
  See `docs/test-suite-config.md`, `docs/test-results.md`, and `docs/samples/`.
- Running `deemer` without a subcommand now prints the banner and usage instead of `Hello world!`.
- Supply-chain transparency for release builds: binaries are built with `cargo auditable`
  (dependency list embedded in the executable, readable with `cargo audit bin` or `syft`), and
  a per-target CycloneDX 1.5 SBOM (`deemer-vX.Y.Z-<target>.cdx.json`) is attached to every
  GitHub release.

### Changed

- Dependencies upgraded to current majors: Ailloy 0.8 → 2.0, `colored` 2 → 3, `dirs` 6 → 7,
  `sha2` 0.10 → 0.11, plus `cargo update` across the lockfile. `reqwest` stays on 0.12 to share a
  single TLS stack with Ailloy.

## [0.1.0] - 2026-06-02

### Added

- Initial scaffold: Cargo workspace (`deemer` + `deemer-core`), clap-derive CLI with
  `-v`/`-q`/`--no-color` global flags, `Hello world!` default command, `ai` subcommand backed by
  Ailloy, shell completions, version banner, and a background crates.io update checker.
- GitHub Actions CI (check / test / clippy / fmt) and a release pipeline that builds cross-platform
  binaries, publishes to crates.io, and updates the Homebrew tap.
- `/release` skill for cutting versioned releases.

[0.2.1]: https://github.com/mklab-se/deemer/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/mklab-se/deemer/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/mklab-se/deemer/releases/tag/v0.1.0
