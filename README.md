# FrameWise

A lightweight, honest performance toolkit for Windows 10 & 11. A live FPS / stats overlay plus a transparent system-optimization assistant — no bloat, no background services, no magic-boost lies.

![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-blue)
![Language](https://img.shields.io/badge/built%20with-Rust-orange)
![RAM](https://img.shields.io/badge/RAM-%3C30MB-brightgreen)
![License](https://img.shields.io/badge/license-MIT-green)

## What it does

FrameWise has two parts:

1. **A live overlay** that shows your real frame rate and system stats while you game.
2. **An optimization assistant** that audits your system for genuine, safe performance wins and lets you apply each one individually — and undo it later.

It is built in Rust and stays under 30 MB of RAM. It runs as a single portable executable with no installer and no background services.

## What it does *not* do

Honesty is a feature here. FrameWise will **not**:

- Promise to "double your FPS" or "boost" your PC with one click. Those claims are marketing, not engineering.
- Inject code or hook into game processes (this is what gets people anti-cheat banned).
- "Clean" your RAM, kill random processes, or edit the registry blindly. These give little to no real benefit and can break things.

Every change FrameWise applies is documented, reversible, and only happens with your consent.

---

## Features

### FPS & stats overlay
- FPS, 1% low, and frame time
- CPU, GPU, and RAM usage
- Multi-monitor aware — draws on the monitor your fullscreen game is actually on
- System tray icon with toggle, settings, and exit
- Hotkey to show/hide the overlay
- Configurable position, font size, opacity, refresh rate, and which stats to show
- Settings persist in a config file next to the executable

### Optimization assistant
An **audit-and-apply** model, not a one-click booster. FrameWise detects real, safe wins and explains each one:

- Hardware-Accelerated GPU Scheduling (HAGS) status
- Active Windows power plan
- GPU driver version vs. latest available
- Background apps consuming high CPU/GPU
- Windows Game Mode status

For each finding you see the **issue**, the **recommended action**, the **expected benefit**, and an **Apply** button. Every applied change is logged and can be reverted individually. Nothing is applied automatically.

---

## Anti-cheat safety

FrameWise reads frame-rate telemetry **passively** through Intel PresentMon. It does not inject code into or hook your games, so it does not behave like the tools anti-cheat systems are designed to detect. That said, no third-party overlay can be *guaranteed* safe with every anti-cheat — use your judgment with competitive titles.

## A note on antivirus warnings

Some antivirus engines may flag FrameWise. This is common for hardware-monitoring tools and happens because:

- It accesses **low-level system APIs** to read CPU/GPU stats, which heuristically resembles how some malware probes hardware.
- The released binary is **unsigned** (code-signing certificates are expensive for an open-source project).

The full source is in this repo — you can read it and build it yourself if you prefer not to trust a prebuilt binary.

---

## Requirements

- **Windows 10 (version 2004 / build 19041 or later)** or **Windows 11**
- **Run as Administrator** is recommended — PresentMon needs elevated access to read frame data for all processes
- Some features (e.g. HAGS) require newer Windows builds and are hidden automatically when unavailable

## Installation

1. Download the latest release from the [Releases](../../releases) page.
2. Extract the portable folder anywhere.
3. Right-click `framewise.exe` → **Run as administrator**.
4. The overlay appears over your fullscreen game; right-click the tray icon for settings.

## Build from source

```bash
# Requires the Rust toolchain: https://rustup.rs
git clone https://github.com/YOUR_USERNAME/framewise.git
cd framewise
cargo build --release
# Output: target/release/framewise.exe
```

## Configuration

Settings are stored in `config.toml` next to the executable and are created on first run. You can edit it directly or use the tray → Settings menu.

---

## Contributing

Contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md). Good first issues are tagged for newcomers. The guiding rule: **if an optimization can't be measured and reverted, it doesn't belong here.**

## License

[MIT](LICENSE) © YOUR_NAME
