# icnsane

Apply PNG and ICNS icons to third-party macOS apps in bulk.

## Install

Requires Rust and the Xcode Command Line Tools.

```sh
cargo install --locked --path .
```

## Usage

Create `icnsane.toml` in your working directory.

```toml
icons_dir = "./icons"

[apps]
"/Applications/Firefox.app" = "firefox.icns"
"~/Applications/Obsidian.app" = "obsidian.png"
```

A relative `icons_dir` starts from the config directory. Icon paths are relative to `icons_dir`. Use `--config PATH` to load a different TOML file.

```sh
icnsane apply --dry-run
icnsane apply
icnsane reset "/Applications/Firefox.app"
icnsane reset --all
```

`--dry-run` validates the apps and images without writing changes. Reset restores the default icon. `reset --all` covers every app listed in the config.

On macOS 27.2, the tested bundles passed `codesign --verify` after customization with Finder or `icnsane`. Strict verification (`--deep --strict`) failed until the icon was reset.
