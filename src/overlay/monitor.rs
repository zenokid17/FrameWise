//! Multi-monitor / fullscreen-game detection.
//!
//! We look at the foreground window: if it covers an entire monitor it's almost
//! certainly the game (exclusive fullscreen or borderless), so the overlay is
//! drawn on *that* monitor. Otherwise we fall back to the primary monitor.

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
};

/// The screen FrameWise should draw the overlay on, plus the PID whose FPS we
/// should report.
#[derive(Debug, Clone, Copy)]
pub struct GameTarget {
    /// Full bounds of the target monitor (virtual-screen coordinates).
    pub monitor_rect: RECT,
    /// PID of the foreground window (0 if unknown).
    pub pid: u32,
}

fn monitor_rect(hmon: HMONITOR) -> Option<RECT> {
    unsafe {
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            Some(mi.rcMonitor)
        } else {
            None
        }
    }
}

/// Bounds of the primary monitor.
pub fn primary_monitor_rect() -> RECT {
    unsafe {
        let hmon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        monitor_rect(hmon).unwrap_or(RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        })
    }
}

/// Inspect the foreground window and decide where the overlay goes.
pub fn detect() -> GameTarget {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return GameTarget {
                monitor_rect: primary_monitor_rect(),
                pid: 0,
            };
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mon = monitor_rect(hmon).unwrap_or_else(primary_monitor_rect);

        let mut wr = RECT::default();
        let is_fullscreen = if GetWindowRect(hwnd, &mut wr).is_ok() {
            wr.left <= mon.left
                && wr.top <= mon.top
                && wr.right >= mon.right
                && wr.bottom >= mon.bottom
        } else {
            false
        };

        // When fullscreen, draw on the game's monitor; otherwise use primary so
        // the overlay stays put while you're on the desktop.
        let monitor_rect = if is_fullscreen {
            mon
        } else {
            primary_monitor_rect()
        };

        GameTarget { monitor_rect, pid }
    }
}
