# Installing deemer

> **Note:** `deemer` hasn't shipped its first release yet — these install paths go live once
> `v0.1.0` is tagged and the release pipeline publishes to crates.io and the Homebrew tap.

## Homebrew (macOS / Linux)

```sh
brew install mklab-se/tap/deemer
```

Or add the tap once, then install:

```sh
brew tap mklab-se/tap
brew install deemer
```

Upgrade with `brew upgrade deemer`.

## Cargo (from crates.io)

```sh
cargo install deemer
```

## cargo-binstall (prebuilt binaries, no compilation)

```sh
cargo binstall deemer
```

## Prebuilt binaries (GitHub Releases)

Download the archive for your platform from the
[latest release](https://github.com/mklab-se/deemer/releases/latest), extract it, and put the
`deemer` binary somewhere on your `PATH`:

| Platform | Archive |
| --- | --- |
| Linux (x86-64) | `deemer-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `deemer-vX.Y.Z-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `deemer-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| Windows (x86-64) | `deemer-vX.Y.Z-x86_64-pc-windows-msvc.zip` |

## Software bill of materials (SBOM)

Every release asset above has a matching CycloneDX 1.5 SBOM listing the exact crate versions
compiled into that platform's binary:

```
deemer-vX.Y.Z-<target>.cdx.json
```

The binaries are also built with [`cargo auditable`](https://github.com/rust-secure-code/cargo-auditable),
so the dependency list travels inside the executable itself. Check a downloaded binary against the
RustSec advisory database with:

```sh
cargo install cargo-audit --features=fix
cargo audit bin ./deemer
```

`syft` and `trivy` also understand this format.

## From source

```sh
git clone https://github.com/mklab-se/deemer
cd deemer
cargo install --path crates/deemer
```

## Shell completions

```sh
# Static script (write it where your shell loads completions)
deemer completion zsh > ~/.zfunc/_deemer

# Or dynamic completions (re-evaluated on each tab)
source <(COMPLETE=zsh deemer)
```
