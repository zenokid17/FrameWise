# Overlay & configuration

The overlay is a native Win32 **layered** window (`WS_EX_LAYERED |
WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`). It is
click-through, never steals focus, and never appears in the Alt-Tab list. It is
drawn with GDI into an off-screen buffer and blitted to avoid flicker; window
opacity is applied with `SetLayeredWindowAttributes`.

## Multi-monitor behaviour

FrameWise polls the foreground window. When the foreground window covers an
entire monitor (i.e. looks like a fullscreen or borderless game), the overlay is
repositioned to that monitor at the configured corner. Otherwise it stays on the
primary monitor. This keeps the overlay on the screen you're actually playing on
without any per-game configuration.

## Configuration file

`framewise.toml` is written next to `framewise.exe` on first run. Example with
defaults:

```toml
[overlay]
# Corner the overlay anchors to: "top_left" | "top_right" | "bottom_left" | "bottom_right"
position = "top_left"
# Pixel margin from the chosen corner
margin = 16
# Font point size
font_size = 18
# Window opacity, 0.10 - 1.00
opacity = 0.85
# Overlay refresh rate in Hz (how often the displayed numbers update)
refresh_hz = 10
# Start with the overlay visible
visible_on_start = true

[overlay.stats]
# Toggle individual rows
fps = true
low_1_percent = true
frame_time = true
cpu = true
gpu = true
ram = true

[hotkey]
# Modifiers: any of "ALT", "CTRL", "SHIFT", "SUPER"
modifiers = ["ALT", "SHIFT"]
# Key: function keys "F1".."F24", or a single letter "A".."Z", or a digit "0".."9"
key = "F10"

[telemetry]
# Path to Intel PresentMon. If empty, FrameWise looks for "PresentMon.exe" next
# to framewise.exe. If not found, FPS stats are disabled gracefully.
presentmon_path = ""
# Extra CLI args passed to PresentMon (advanced). Leave empty for defaults.
presentmon_extra_args = []
# Window (in frames) used to compute the 1% low.
low_sample_window = 1000
```

Invalid values are clamped to safe ranges at load time (e.g. opacity to
`0.10..=1.00`, `refresh_hz` to `1..=60`).

## Stats definitions

| Stat        | Source                              | Definition |
|-------------|-------------------------------------|------------|
| FPS         | PresentMon frame intervals          | `1000 / mean(frame_time_ms)` over the last refresh window |
| 1% low      | PresentMon frame intervals          | `1000 / p99(frame_time_ms)` over `low_sample_window` frames — the FPS of the worst 1% of frames |
| Frame time  | PresentMon frame intervals          | mean milliseconds between presents |
| CPU         | `sysinfo` (OS performance counters) | system-wide CPU utilization % |
| GPU         | PDH `GPU Engine` counter            | busiest engine utilization % (the value Task Manager headlines) |
| RAM         | `sysinfo`                           | used / total physical memory |

GPU usage uses the PDH `\GPU Engine(*)\Utilization Percentage` counter set,
available on Windows 10 1709+. On systems where it is unavailable the GPU row is
hidden automatically.
