# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres
to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Project scaffold: Cargo manifest, MIT license, CI (fmt + clippy + build + test
  on Windows), contributing guide, docs.
- OS detection and feature gating for Windows 10 (build 19041+) and Windows 11.
- Config persistence (`framewise.toml`) next to the executable.
- **FPS & stats overlay**: native Win32/GDI layered overlay showing FPS, 1% low,
  frame time, CPU, GPU and RAM usage.
  - Multi-monitor aware: follows the monitor the fullscreen game is on.
  - System tray icon (toggle / settings / exit) and global show/hide hotkey.
  - Passive FPS telemetry via Intel PresentMon (no injection, anti-cheat safe).
- **Optimization assistant (read-only audit)**: detects HAGS status, active power
  plan, Game Mode status, GPU driver version, and noisy background apps. Reports
  the issue, recommended action and expected benefit for each finding.

### Not yet implemented
- Applying / reverting optimization findings (modifies system settings; gated
  behind explicit consent and a dedicated, reviewed change set).

[Unreleased]: https://github.com/your-org/framewise/commits/main
