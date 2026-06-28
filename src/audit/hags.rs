//! Read-only check: Hardware-Accelerated GPU Scheduling (HAGS).

use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

use crate::audit::{Finding, Severity};
use crate::platform::Features;

fn read_hwsch_mode() -> Option<u32> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers")
        .ok()?;
    key.get_value::<u32, _>("HwSchMode").ok()
}

pub fn check(features: &Features) -> Finding {
    let title = "Hardware-Accelerated GPU Scheduling".to_string();

    if !features.hags {
        return Finding {
            id: "hags",
            title,
            severity: Severity::Info,
            issue: "Not available on this Windows build.".into(),
            recommendation:
                "No action — HAGS requires Windows 10 version 2004 (build 19041) or newer plus a \
                 supporting GPU and driver."
                    .into(),
            expected_benefit: "None on this build.".into(),
            applicable: false,
        };
    }

    let (issue, recommendation, severity) = match read_hwsch_mode() {
        // 2 = enabled, 1 = disabled, 0 = unsupported by driver.
        Some(2) => (
            "HAGS is ON.".to_string(),
            "Leave as-is unless you experience instability. Its effect is hardware-, driver- and \
             game-dependent."
                .to_string(),
            Severity::Info,
        ),
        Some(0) | Some(1) => (
            "HAGS is OFF.".to_string(),
            "You can enable it in Settings > System > Display > Graphics > Default graphics \
             settings, then reboot. Effect varies — benchmark a few games before and after to \
             confirm it helps on your setup."
                .to_string(),
            Severity::Suggestion,
        ),
        Some(other) => (
            format!("HAGS registry value is {other} (unrecognized)."),
            "Check Settings > System > Display > Graphics.".to_string(),
            Severity::Info,
        ),
        None => (
            "HAGS state could not be read from the registry.".to_string(),
            "Check Settings > System > Display > Graphics.".to_string(),
            Severity::Info,
        ),
    };

    Finding {
        id: "hags",
        title,
        severity,
        issue,
        recommendation,
        expected_benefit: "Situational; a small change in scheduling latency. Requires a reboot."
            .into(),
        applicable: false,
    }
}
