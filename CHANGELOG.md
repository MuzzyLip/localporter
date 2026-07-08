# Changelog

All notable changes to this project will be documented in this file.

This project follows Semantic Versioning. Until `1.0.0`, minor versions may still include breaking changes as the product shape and internal crate boundaries continue to evolve.

## [Unreleased]

### Planned
- Continue stabilizing the cross-platform process and port collection pipeline.
- Expand packaging and release automation around GitHub Releases.

## [0.1.0] - 2026-07-08

### Added
- Initial Rust workspace layout with `localporter-app`, `localporter-ui`, and `localporter-core`.
- Native `egui`/`eframe` desktop shell with standalone window mode and macOS menu bar panel mode.
- Best-effort local port discovery for the current user permission scope.
- Process metadata rendering, including process name, parent launcher name, uptime, CPU usage, memory usage, PID, and command fallback handling.
- Primary port list UI with grouped port presentation, row actions, custom title bar, and platform-specific window controls.
- Basic process actions including opening a local port in the browser and terminating killable processes.
- File logging for core runtime paths, with one log file per app launch under the app configuration directory.
- macOS packaging scripts for universal app and DMG output.
- Windows packaging scripts for ZIP and MSI output.

### Changed
- Standardized first public artifact naming around `LocalPorter`.
- Adopted UTC timestamp-based log filenames for easier issue collection and sorting.

### Notes
- This is the first usable release.
- Port and process collection is best effort and depends on OS version, current permissions, and tool availability.
- macOS distribution is unsigned and not notarized in this release, so end users may need to manually allow the app in system security settings.
