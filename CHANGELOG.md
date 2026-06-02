# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial template: Cargo workspace (`rusty-tmpl` + `rusty-tmpl-core`), clap-derive CLI with
  `-v`/`-q`/`--no-color` global flags, `Hello world!` default command, `ai` subcommand backed by
  Ailloy, shell completions, version banner, and a background crates.io update checker.
- GitHub Actions CI (check / test / clippy / fmt) and a release pipeline that builds cross-platform
  binaries, publishes to crates.io, and updates the Homebrew tap.
- `/release` skill for cutting versioned releases.

[Unreleased]: https://github.com/mklab-se/rusty-tmpl/commits/main
