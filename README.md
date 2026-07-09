<h1 align="center">LocalPorter</h1>

<p align="center">
  Native local port monitor and process manager for macOS and Windows.
</p>

<p align="center">
  <code>Rust</code>
  <code>egui</code>
  <code>Native Desktop</code>
  <code>Local Ports</code>
  <code>Process Actions</code>
  <code>macOS</code>
  <code>Windows</code>
</p>

<p align="center">
  <a href="https://github.com/MuzzyLip/localporter/releases">
    <img alt="Release" src="https://img.shields.io/github/v/release/MuzzyLip/localporter?display_name=tag">
  </a>
  <a href="https://github.com/MuzzyLip/localporter/actions/workflows/ci.yml">
    <img alt="CI" src="https://img.shields.io/github/actions/workflow/status/MuzzyLip/localporter/ci.yml?branch=main&label=ci">
  </a>
  <a href="https://github.com/MuzzyLip/localporter/issues">
    <img alt="Issues" src="https://img.shields.io/github/issues/MuzzyLip/localporter">
  </a>
  <a href="https://github.com/MuzzyLip/localporter/releases">
    <img alt="Downloads" src="https://img.shields.io/github/downloads/MuzzyLip/localporter/total">
  </a>
  <a href="https://github.com/MuzzyLip/localporter">
    <img alt="Platforms" src="https://img.shields.io/badge/platforms-macOS%20%7C%20Windows-4c8bf5">
  </a>
</p>

<p align="center">
  <a href="#showcase">Showcase</a> |
  <a href="#features">Features</a> |
  <a href="#boundaries">Boundaries</a> |
  <a href="#installation">Installation</a> |
  <a href="#development">Development</a> |
  <a href="#contributing">Contributing</a>
</p>

LocalPorter is a native desktop utility for inspecting local listening ports, locating the processes behind them, and taking fast action when a port should be opened, revealed, or terminated. It is built with a Rust workspace and an `egui` UI, without Electron or a webview shell.

Currently supported platforms: `macOS` and `Windows`.

## Showcase

<p align="center">
  <img src="./docs/localporter-showcase.gif" alt="LocalPorter showcase" width="100%">
</p>

<p align="center">
  <a href="./docs/localporter-showcase.mp4">Watch MP4</a>
</p>

## Features

- Inspect local TCP listening ports and UDP bindings from a native desktop UI.
- Search by process name, by `:port`, or by a port range like `:3000-:3999`.
- Expand each port row to inspect process details such as `PID` and full command line.
- Open a local service in the browser when the port is meant to be visited.
- Reveal the launcher window when the parent process chain can be resolved to a supported terminal or IDE launcher.
- Kill a single process or batch-kill only the processes that are considered killable.
- Hide noisy Windows `System` entries by default, while still allowing them to appear when `Show all` is enabled.
- Tune behavior from settings, including refresh interval, kill confirmation, launch at startup, and persisted preferences.

## Boundaries

- LocalPorter is a local inspection tool, not a packet sniffer, firewall, or remote host scanner.
- Port ownership and process metadata are best effort and depend on OS capabilities, permissions, and the availability of system tooling.
- Some system-owned or protected processes are intentionally not treated as killable, especially for batch actions.
- `Open in Browser` is useful for web services, but some ports will not serve a browser-friendly response.
- `Reveal launcher` only works when LocalPorter can infer a meaningful launcher from the parent process chain.
- Only `macOS` and `Windows` are currently supported.
- The current packaged targets are macOS universal app bundles and Windows x64 artifacts. Other targets may build from source but are not packaged here.
- The current macOS distribution is unsigned and not notarized, so end users may need to allow it manually in system security settings.

## Installation

### End users

Download a packaged build from [GitHub Releases](https://github.com/MuzzyLip/localporter/releases).

- macOS: `LocalPorter-<version>-macos-universal.dmg`
- Windows: `LocalPorter-<version>-windows-x64.msi`
- Windows portable: `LocalPorter-<version>-windows-x64.zip`

### From source

Prerequisites:

- Rust stable toolchain
- `cargo`
- macOS only: Xcode Command Line Tools
- Windows MSI packaging only: WiX v7 CLI with `WixToolset.UI.wixext`

Run locally:

```bash
cargo run --release -p localporter-app
```

## Development

Workspace layout:

```text
crates/
  localporter-app/   executable entry
  localporter-ui/    egui application and platform UI
  localporter-core/  shared process and port logic
```

Common commands:

```bash
# Run the app
cargo run --release -p localporter-app

# Run tests
cargo test --workspace
```

Platform build scripts:

```powershell
# Windows release artifacts
./scripts/build-windows-release.ps1
```

```bash
# macOS universal DMG
./scripts/build-macos-dmg.sh
```

Artifacts are written under `target/`.

## Contributing

Contributions are welcome. Small focused pull requests are the easiest to review.

Before opening a PR:

- keep changes scoped to one problem or feature
- run `cargo test --workspace`
- include screenshots or a short demo when the UI changes
- describe platform-specific behavior if the change affects Windows or macOS differently

If you are proposing a larger feature, open an issue first so the product shape and platform tradeoffs can be discussed before implementation.

## Changelog

Release notes live in [CHANGELOG.md](./CHANGELOG.md).
