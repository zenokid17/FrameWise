//! Global show/hide hotkey via the `global-hotkey` crate. Events are delivered
//! through `GlobalHotKeyEvent::receiver()`, polled from the main loop.

use anyhow::{anyhow, Result};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;

use crate::config::Hotkey as HotkeyCfg;

pub struct Hotkeys {
    // Held to keep the registration alive.
    _manager: GlobalHotKeyManager,
    pub toggle_id: u32,
}

impl Hotkeys {
    pub fn register(cfg: &HotkeyCfg) -> Result<Hotkeys> {
        let manager = GlobalHotKeyManager::new()?;
        let hotkey = parse(cfg)?;
        manager.register(hotkey)?;
        log::info!("registered hotkey: {:?}+{}", cfg.modifiers, cfg.key);
        Ok(Hotkeys {
            _manager: manager,
            toggle_id: hotkey.id(),
        })
    }
}

fn parse(cfg: &HotkeyCfg) -> Result<HotKey> {
    let mut mods = Modifiers::empty();
    for m in &cfg.modifiers {
        match m.trim().to_ascii_uppercase().as_str() {
            "ALT" => mods |= Modifiers::ALT,
            "CTRL" | "CONTROL" => mods |= Modifiers::CONTROL,
            "SHIFT" => mods |= Modifiers::SHIFT,
            "SUPER" | "WIN" | "META" => mods |= Modifiers::SUPER,
            other => log::warn!("ignoring unknown hotkey modifier: {other}"),
        }
    }
    let code = parse_code(&cfg.key).ok_or_else(|| anyhow!("unknown hotkey key: {}", cfg.key))?;
    Ok(HotKey::new(Some(mods), code))
}

fn parse_code(key: &str) -> Option<Code> {
    let k = key.trim().to_ascii_uppercase();

    // Function keys F1..F24
    if let Some(n) = k.strip_prefix('F').and_then(|s| s.parse::<u32>().ok()) {
        return function_code(n);
    }

    if k.chars().count() == 1 {
        let c = k.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            return letter_code(c);
        }
        if c.is_ascii_digit() {
            return digit_code(c);
        }
    }
    None
}

fn function_code(n: u32) -> Option<Code> {
    Some(match n {
        1 => Code::F1,
        2 => Code::F2,
        3 => Code::F3,
        4 => Code::F4,
        5 => Code::F5,
        6 => Code::F6,
        7 => Code::F7,
        8 => Code::F8,
        9 => Code::F9,
        10 => Code::F10,
        11 => Code::F11,
        12 => Code::F12,
        13 => Code::F13,
        14 => Code::F14,
        15 => Code::F15,
        16 => Code::F16,
        17 => Code::F17,
        18 => Code::F18,
        19 => Code::F19,
        20 => Code::F20,
        21 => Code::F21,
        22 => Code::F22,
        23 => Code::F23,
        24 => Code::F24,
        _ => return None,
    })
}

fn letter_code(c: char) -> Option<Code> {
    Some(match c {
        'A' => Code::KeyA,
        'B' => Code::KeyB,
        'C' => Code::KeyC,
        'D' => Code::KeyD,
        'E' => Code::KeyE,
        'F' => Code::KeyF,
        'G' => Code::KeyG,
        'H' => Code::KeyH,
        'I' => Code::KeyI,
        'J' => Code::KeyJ,
        'K' => Code::KeyK,
        'L' => Code::KeyL,
        'M' => Code::KeyM,
        'N' => Code::KeyN,
        'O' => Code::KeyO,
        'P' => Code::KeyP,
        'Q' => Code::KeyQ,
        'R' => Code::KeyR,
        'S' => Code::KeyS,
        'T' => Code::KeyT,
        'U' => Code::KeyU,
        'V' => Code::KeyV,
        'W' => Code::KeyW,
        'X' => Code::KeyX,
        'Y' => Code::KeyY,
        'Z' => Code::KeyZ,
        _ => return None,
    })
}

fn digit_code(c: char) -> Option<Code> {
    Some(match c {
        '0' => Code::Digit0,
        '1' => Code::Digit1,
        '2' => Code::Digit2,
        '3' => Code::Digit3,
        '4' => Code::Digit4,
        '5' => Code::Digit5,
        '6' => Code::Digit6,
        '7' => Code::Digit7,
        '8' => Code::Digit8,
        '9' => Code::Digit9,
        _ => return None,
    })
}
