//! Read-only check: Windows Game Mode.

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

use crate::audit::{Finding, Severity};

fn game_mode_enabled() -> Option<u32> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(r"Software\Microsoft\GameBar").ok()?;
    key.get_value::<u32, _>("AutoGameModeEnabled").ok()
}

pub fn check() -> Option<Finding> {
    let (issue, recommendation, severity) = match game_mode_enabled() {
        Some(1) => (
            "Game Mode is ON.",
            "Leave it on; it prioritizes the foreground game and reduces background interference.",
            Severity::Info,
        ),
        Some(_) => (
            "Game Mode is OFF.",
            "You can enable it in Settings > Gaming > Game Mode. The effect is small and \
             situational.",
            Severity::Suggestion,
        ),
        None => (
            "Game Mode state unknown (it defaults to ON in modern Windows).",
            "Check Settings > Gaming > Game Mode.",
            Severity::Info,
        ),
    };

    Some(Finding {
        id: "game_mode",
        title: "Game Mode".into(),
        severity,
        issue: issue.into(),
        recommendation: recommendation.into(),
        expected_benefit: "Small and situational.".into(),
        applicable: false,
    })
}
