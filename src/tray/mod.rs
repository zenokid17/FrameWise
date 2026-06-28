//! System tray icon + context menu. Built on the `tray-icon` crate so we don't
//! hand-roll `Shell_NotifyIcon`. Menu clicks are delivered through
//! `MenuEvent::receiver()`, which we poll from the main message loop.

use anyhow::Result;
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct Tray {
    // Kept alive for the lifetime of the app; dropping it removes the icon.
    _tray: TrayIcon,
    pub toggle_id: MenuId,
    pub audit_id: MenuId,
    pub settings_id: MenuId,
    pub quit_id: MenuId,
}

impl Tray {
    pub fn new() -> Result<Tray> {
        let menu = Menu::new();
        let toggle = MenuItem::new("Show / hide overlay", true, None);
        let audit = MenuItem::new("Run optimization audit…", true, None);
        let settings = MenuItem::new("Open config file…", true, None);
        let quit = MenuItem::new("Exit FrameWise", true, None);

        menu.append(&toggle)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&audit)?;
        menu.append(&settings)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit)?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("FrameWise — FPS overlay & optimization assistant")
            .with_icon(make_icon())
            .build()?;

        Ok(Tray {
            _tray: tray,
            toggle_id: toggle.id().clone(),
            audit_id: audit.id().clone(),
            settings_id: settings.id().clone(),
            quit_id: quit.id().clone(),
        })
    }
}

/// Generate a small icon at runtime (dark panel + green "bar chart") so we don't
/// have to ship a binary asset.
fn make_icon() -> Icon {
    const S: u32 = 32;
    let mut rgba = vec![0u8; (S * S * 4) as usize];

    let put = |rgba: &mut [u8], x: u32, y: u32, c: [u8; 4]| {
        if x < S && y < S {
            let i = ((y * S + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&c);
        }
    };

    // Rounded-ish dark panel.
    for y in 2..30 {
        for x in 2..30 {
            put(&mut rgba, x, y, [20, 22, 28, 235]);
        }
    }
    // Three rising bars.
    let bars = [(6u32, 22u32), (13, 16), (20, 10)];
    for (bx, top) in bars {
        for y in top..26 {
            for x in bx..bx + 6 {
                put(&mut rgba, x, y, [120, 230, 130, 255]);
            }
        }
    }

    Icon::from_rgba(rgba, S, S).expect("valid icon dimensions")
}
