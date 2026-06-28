//! Optimization assistant — **read-only audit**.
//!
//! Each detector inspects the system and returns a `Finding` describing the
//! issue, the recommended action, and the honest expected benefit. Nothing here
//! changes system state. Applying a finding (which *would* modify settings) is
//! intentionally not implemented — see `docs/optimizations.md` and the project
//! safety rules. When apply lands it must be per-finding, consented, and logged
//! to `framewise-changes.jsonl` for individual revert.

pub mod background_apps;
pub mod game_mode;
pub mod gpu_driver;
pub mod hags;
pub mod power_plan;

use std::path::PathBuf;

use crate::platform::{exe_dir, Features};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Informational; nothing to do.
    Info,
    /// Worth considering.
    Suggestion,
    /// A likely, documented win. Reserved for future detectors; part of the
    /// documented severity taxonomy (see docs/optimizations.md).
    #[allow(dead_code)]
    Opportunity,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Suggestion => "SUGGESTION",
            Severity::Opportunity => "OPPORTUNITY",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub id: &'static str,
    pub title: String,
    pub severity: Severity,
    pub issue: String,
    pub recommendation: String,
    pub expected_benefit: String,
    /// Whether an automated apply action exists. Always `false` today: applies
    /// modify system settings and are gated behind explicit, reviewed work.
    pub applicable: bool,
}

/// Run every read-only detector and collect findings.
pub fn run_audit(features: &Features) -> Vec<Finding> {
    let mut findings = Vec::new();

    findings.push(hags::check(features));
    if let Some(f) = power_plan::check() {
        findings.push(f);
    }
    if let Some(f) = game_mode::check() {
        findings.push(f);
    }
    findings.extend(gpu_driver::check());
    findings.extend(background_apps::check());

    findings
}

/// Render findings to a human-readable report and write it next to the exe.
/// Returns the path written.
pub fn write_report(findings: &[Finding]) -> std::io::Result<PathBuf> {
    let path = exe_dir().join("framewise-audit.txt");
    let mut out = String::new();
    out.push_str("FrameWise — optimization audit (read-only)\n");
    out.push_str("==========================================\n\n");
    out.push_str(
        "This report only *describes* findings. FrameWise does not change any\n\
         system setting without your explicit, per-item consent. Applying changes\n\
         is not yet implemented in this build.\n\n",
    );

    if findings.is_empty() {
        out.push_str("No findings.\n");
    }
    for f in findings {
        out.push_str(&format!(
            "[{}] {} ({})\n",
            f.severity.label(),
            f.title,
            f.id
        ));
        out.push_str(&format!("  Issue:           {}\n", f.issue));
        out.push_str(&format!("  Recommendation:  {}\n", f.recommendation));
        out.push_str(&format!("  Expected impact: {}\n", f.expected_benefit));
        out.push_str(&format!(
            "  Apply available: {}\n\n",
            if f.applicable {
                "yes"
            } else {
                "no (report only)"
            }
        ));
    }

    std::fs::write(&path, out)?;
    Ok(path)
}
