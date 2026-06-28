//! Application wiring: startup checks, building the overlay/tray/hotkey, and the
//! Win32 message loop that also drains tray-menu and hotkey events.

use anyhow::Result;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use tray_icon::menu::MenuEvent;
use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MessageBoxW, TranslateMessage, MB_ICONERROR, MB_ICONWARNING,
    MB_OK, MESSAGEBOX_STYLE, MSG,
};

use crate::audit;
use crate::config::Config;
use crate::hotkey::Hotkeys;
use crate::overlay::Overlay;
use crate::platform::{self, Features, OsInfo};
use crate::stats::Telemetry;
use crate::tray::Tray;

pub fn run() -> Result<()> {
    let config = Config::load_or_create();

    let os = match OsInfo::detect() {
        Ok(o) => o,
        Err(e) => {
            log::warn!("could not detect OS version: {e}");
            OsInfo {
                major: 0,
                build: 0,
                ubr: 0,
                display_version: String::new(),
                product_name: String::new(),
            }
        }
    };
    log::info!("Detected {}", os.version_string());
    log::debug!(
        "registry ProductName: {:?}, major version {}",
        os.product_name,
        os.major
    );

    let features = os.features();
    log::info!(
        "Feature gating: gpu_usage={}, hags={}",
        features.gpu_usage,
        features.hags
    );

    if os.build != 0 && !os.is_supported() {
        message_box(
            &format!(
                "FrameWise targets Windows 10 build 19041+ or Windows 11.\n\n\
                 Detected: {}\n\nIt may still run, but some features could be unavailable.",
                os.version_string()
            ),
            "FrameWise — unsupported Windows version",
            MB_ICONWARNING,
        );
    }

    if !platform::is_elevated() {
        log::warn!("not running elevated; PresentMon FPS telemetry will be unavailable");
        message_box(
            "FrameWise is not running as administrator.\n\n\
             FPS, 1% low and frame time use PresentMon, which needs admin rights. CPU, GPU and \
             RAM stats will still work.\n\nTo enable FPS: right-click FrameWise and choose \
             \"Run as administrator\".",
            "FrameWise",
            MB_ICONWARNING,
        );
    }

    // Start telemetry collectors (background threads, no OS service).
    let tele = Telemetry::new();
    tele.start(&config, &features);

    // Build the overlay window.
    let overlay = Overlay::create(config.clone(), tele.clone())?;

    // System tray.
    let tray = Tray::new()?;

    // Global hotkey (non-fatal if it can't register).
    let hotkeys = match Hotkeys::register(&config.hotkey) {
        Ok(h) => Some(h),
        Err(e) => {
            log::warn!("could not register hotkey: {e}");
            None
        }
    };

    log::info!("FrameWise ready");
    run_message_loop(&overlay, &tray, hotkeys.as_ref(), features);

    log::info!("shutting down");
    tele.stop();
    Ok(())
}

fn run_message_loop(overlay: &Overlay, tray: &Tray, hotkeys: Option<&Hotkeys>, features: Features) {
    let menu_rx = MenuEvent::receiver();
    let hotkey_rx = GlobalHotKeyEvent::receiver();

    let mut msg = MSG::default();
    loop {
        let res = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        match res.0 {
            -1 => {
                log::error!("GetMessageW failed");
                break;
            }
            0 => break, // WM_QUIT
            _ => unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            },
        }

        // Drain tray-menu clicks.
        while let Ok(ev) = menu_rx.try_recv() {
            if ev.id == tray.toggle_id {
                overlay.toggle();
            } else if ev.id == tray.settings_id {
                open_path(&Config::path());
            } else if ev.id == tray.audit_id {
                run_audit_async(features);
            } else if ev.id == tray.quit_id {
                // Triggers WM_DESTROY -> PostQuitMessage -> loop exit.
                overlay.destroy();
            }
        }

        // Drain hotkey presses.
        while let Ok(ev) = hotkey_rx.try_recv() {
            let is_toggle = hotkeys.map(|hk| ev.id == hk.toggle_id).unwrap_or(false);
            if ev.state == HotKeyState::Pressed && is_toggle {
                overlay.toggle();
            }
        }
    }
}

/// Run the read-only audit off the UI thread, write the report, and open it.
fn run_audit_async(features: Features) {
    std::thread::spawn(move || {
        let findings = audit::run_audit(&features);
        log::info!("audit complete: {} finding(s)", findings.len());
        match audit::write_report(&findings) {
            Ok(path) => {
                log::info!("audit report written: {}", path.display());
                open_path(&path);
            }
            Err(e) => log::error!("could not write audit report: {e}"),
        }
    });
}

/// Open a file/path with its default handler (via the shell).
fn open_path(path: &std::path::Path) {
    let p = path.to_string_lossy().to_string();
    if let Err(e) = std::process::Command::new("cmd")
        .args(["/C", "start", "", &p])
        .spawn()
    {
        log::warn!("could not open {p}: {e}");
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn message_box(text: &str, caption: &str, icon: MESSAGEBOX_STYLE) {
    let t = to_wide(text);
    let c = to_wide(caption);
    unsafe {
        MessageBoxW(None, PCWSTR(t.as_ptr()), PCWSTR(c.as_ptr()), MB_OK | icon);
    }
}

/// Used by `main` to surface a fatal startup error to the user.
pub fn fatal_message_box(text: &str) {
    message_box(text, "FrameWise — error", MB_ICONERROR);
}
