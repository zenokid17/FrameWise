# Optimization assistant — catalog

The assistant uses an **audit-and-apply** model. The **audit** (read-only) is
implemented today. **Apply/revert** is intentionally not implemented yet because
it modifies system settings; it will land per-finding behind explicit consent,
with every change journaled to `framewise-changes.jsonl` for individual revert.

Each finding reports four things:

- **Issue** — what was detected.
- **Recommended action** — the specific, documented change.
- **Expected benefit** — honest, bounded; no "magic boost" claims.
- **Apply** — *(future)* performs the change and journals a revert entry.

Severity is informational only (`Info` / `Suggestion` / `Opportunity`).

---

## 1. Hardware-Accelerated GPU Scheduling (HAGS)

- **Detect:** read `HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers`
  value `HwSchMode` (`2` = on, `1` = off). Availability also depends on the
  Windows build (Win10 2004+) and GPU driver support.
- **Recommended action:** HAGS can reduce scheduling latency on supported
  hardware. Effect varies by GPU/driver and game — sometimes positive, sometimes
  negligible, occasionally negative. The assistant reports current state and lets
  *you* decide; it does not assert it is universally better.
- **Expected benefit:** situational; small latency change. Requires a reboot to
  take effect.
- **Apply (future):** set `HwSchMode` and prompt for reboot. Revert restores the
  previous value.

## 2. Active Windows power plan

- **Detect:** `PowerGetActiveScheme` + `PowerReadFriendlyName`.
- **Recommended action:** on desktops, ensure the active plan isn't
  "Power saver", which caps CPU frequency. "Balanced" (modern CPUs idle down
  fine) or "High performance" are reasonable. On laptops this trades battery for
  performance — reported, not forced.
- **Expected benefit:** can prevent CPU down-clocking under load on machines
  stuck on Power saver.
- **Apply (future):** `PowerSetActiveScheme`. Revert restores the previous GUID.

## 3. Game Mode

- **Detect:** `HKCU\Software\Microsoft\GameBar` value `AutoGameModeEnabled`.
- **Recommended action:** Game Mode prioritizes the foreground game and limits
  background interference. Generally fine to leave on.
- **Expected benefit:** small, situational.
- **Apply (future):** toggle the value. Revert restores it.

## 4. GPU driver version

- **Detect:** current driver version via the OS (WMI `Win32_VideoController` /
  registry). FrameWise reports the installed version and the vendor's update
  page. It does **not** silently download drivers.
- **Recommended action:** if substantially behind, update via the vendor's
  official tool (NVIDIA App / AMD Adrenalin / Intel). Comparing against "latest"
  requires a vendor-specific online lookup and is treated as an optional,
  clearly-labelled online check — never an automatic install.
- **Expected benefit:** game-specific; newer drivers often include
  per-title optimizations.
- **Apply (future):** none — driver installation is delegated to the vendor tool
  by design.

## 5. Background apps with high GPU/CPU use

- **Detect:** `sysinfo` process sampling; report processes with sustained high
  CPU or GPU usage while a game is foregrounded.
- **Recommended action:** *report only.* FrameWise lists the offenders and what
  they are, so you can close them yourself. It never kills processes silently.
- **Expected benefit:** frees CPU/GPU headroom if you choose to close a
  background hog (e.g. a stray encoder or browser tab).
- **Apply (future):** none automatic. At most, an explicit "this will ask the app
  to close" action with confirmation — never a silent kill.

---

## What the assistant will never do

- Edit the registry beyond the specific, documented values above — and only with
  consent.
- "Clean" or "free" RAM, defragment memory, or trim working sets of other apps.
- Kill processes without an explicit, confirmed action.
- Disable Windows services, telemetry, or scheduled tasks wholesale.
- Make changes it cannot revert.
