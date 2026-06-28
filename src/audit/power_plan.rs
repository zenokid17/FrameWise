//! Read-only check: active Windows power plan.
//!
//! The active scheme GUID lives in the registry, so we don't need the power
//! management API. We map the well-known GUIDs to friendly names.

use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

use crate::audit::{Finding, Severity};

fn active_scheme_guid() -> Option<String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes")
        .ok()?;
    let raw: String = key.get_value("ActivePowerScheme").ok()?;
    Some(
        raw.trim()
            .trim_matches(|c| c == '{' || c == '}')
            .to_lowercase(),
    )
}

pub fn check() -> Option<Finding> {
    let guid = active_scheme_guid()?;

    let (name, severity, recommendation) = match guid.as_str() {
        "a1841308-3541-4fab-bc81-f71556f20b4a" => (
            "Power saver",
            Severity::Suggestion,
            "On a desktop, switch to Balanced or High performance — Power saver caps CPU \
             frequency and can hurt frame rates. On a laptop this is a battery vs. performance \
             trade-off, so decide based on whether you're plugged in.",
        ),
        "381b4222-f694-41f0-9685-ff5bb260df2e" => (
            "Balanced",
            Severity::Info,
            "Fine for most systems; modern CPUs still boost fully on Balanced.",
        ),
        "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c" => (
            "High performance",
            Severity::Info,
            "Good for desktop gaming; uses a bit more power at idle.",
        ),
        "e9a42b02-d5df-448d-aa00-03f14749eb61" => (
            "Ultimate Performance",
            Severity::Info,
            "Maximum performance; highest idle power draw.",
        ),
        _ => (
            "Custom / OEM plan",
            Severity::Info,
            "A custom plan is active; make sure it doesn't cap maximum CPU state for gaming.",
        ),
    };

    let expected_benefit = if severity == Severity::Suggestion {
        "Prevents CPU down-clocking under load on machines stuck on Power saver."
    } else {
        "No change recommended."
    };

    Some(Finding {
        id: "power_plan",
        title: "Windows power plan".into(),
        severity,
        issue: format!("Active power plan: {name} ({guid})."),
        recommendation: recommendation.into(),
        expected_benefit: expected_benefit.into(),
        applicable: false,
    })
}
