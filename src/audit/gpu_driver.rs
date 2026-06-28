//! Read-only check: installed GPU driver version.
//!
//! Read from the Display device class in the registry. We report the installed
//! version; we deliberately do **not** auto-compare to or download "latest" —
//! that's vendor-specific and is left to the vendor's own tool.

use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

use crate::audit::{Finding, Severity};

const DISPLAY_CLASS: &str =
    r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";

pub fn check() -> Vec<Finding> {
    let mut out = Vec::new();

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let class = match hklm.open_subkey(DISPLAY_CLASS) {
        Ok(k) => k,
        Err(_) => return out,
    };

    for sub in class.enum_keys().flatten() {
        // Adapter instances are 4-digit subkeys like "0000", "0001".
        if sub.len() != 4 || !sub.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let Ok(k) = class.open_subkey(&sub) else {
            continue;
        };

        let desc: String = k.get_value("DriverDesc").unwrap_or_default();
        let ver: String = k.get_value("DriverVersion").unwrap_or_default();
        if desc.is_empty() && ver.is_empty() {
            continue;
        }
        // Skip the Microsoft Basic Render Driver pseudo-adapter.
        if desc.to_lowercase().contains("microsoft basic") {
            continue;
        }

        let name = if desc.is_empty() { "GPU" } else { &desc };
        let version = if ver.is_empty() { "unknown" } else { &ver };

        out.push(Finding {
            id: "gpu_driver",
            title: "GPU driver version".into(),
            severity: Severity::Info,
            issue: format!("{name} — driver version {version}."),
            recommendation:
                "Keep drivers current via the vendor's official tool (NVIDIA App / AMD Adrenalin / \
                 Intel). FrameWise does not auto-download drivers; checking for the latest version \
                 requires a vendor-specific online lookup."
                    .into(),
            expected_benefit: "Game-specific; newer drivers often add per-title optimizations."
                .into(),
            applicable: false,
        });
    }

    if out.is_empty() {
        out.push(Finding {
            id: "gpu_driver",
            title: "GPU driver version".into(),
            severity: Severity::Info,
            issue: "Could not read GPU driver info from the registry.".into(),
            recommendation: "Check Device Manager > Display adapters.".into(),
            expected_benefit: "n/a".into(),
            applicable: false,
        });
    }

    out
}
