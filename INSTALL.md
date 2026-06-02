# Installing rusty-tmpl

> `rusty-tmpl` is a template — these instructions become real once you publish your renamed tool.
> Until then they document the install paths the release pipeline sets up.

## Homebrew (macOS / Linux)

```sh
brew install mklab-se/tap/rusty-tmpl
```

Or add the tap once, then install:

```sh
brew tap mklab-se/tap
brew install rusty-tmpl
```

Upgrade with `brew upgrade rusty-tmpl`.

## Cargo (from crates.io)

```sh
cargo install rusty-tmpl
```

## cargo-binstall (prebuilt binaries, no compilation)

```sh
cargo binstall rusty-tmpl
```

## Prebuilt binaries (GitHub Releases)

Download the archive for your platform from the
[latest release](https://github.com/mklab-se/rusty-tmpl/releases/latest), extract it, and put the
`rusty-tmpl` binary somewhere on your `PATH`:

| Platform | Archive |
| --- | --- |
| Linux (x86-64) | `rusty-tmpl-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `rusty-tmpl-vX.Y.Z-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `rusty-tmpl-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| Windows (x86-64) | `rusty-tmpl-vX.Y.Z-x86_64-pc-windows-msvc.zip` |

## From source

```sh
git clone https://github.com/mklab-se/rusty-tmpl
cd rusty-tmpl
cargo install --path crates/rusty-tmpl
```

## Shell completions

```sh
# Static script (write it where your shell loads completions)
rusty-tmpl completion zsh > ~/.zfunc/_rusty-tmpl

# Or dynamic completions (re-evaluated on each tab)
source <(COMPLETE=zsh rusty-tmpl)
```
