//! Read-only check: background apps using significant CPU.
//!
//! Reports only — FrameWise never closes processes for you. GPU-per-process
//! attribution is intentionally omitted for now (it needs per-process PDH GPU
//! engine counters); CPU is the reliable, portable signal.

use std::time::Duration;

use sysinfo::{ProcessesToUpdate, System};

use crate::audit::{Finding, Severity};

/// Report processes using at least this fraction of *total* CPU.
const CPU_THRESHOLD_TOTAL: f32 = 8.0;
const MAX_OFFENDERS: usize = 5;

pub fn check() -> Vec<Finding> {
    let mut sys = System::new();
    // Two samples spaced apart so per-process CPU deltas are meaningful.
    sys.refresh_processes(ProcessesToUpdate::All, true);
    std::thread::sleep(Duration::from_millis(400));
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let self_pid = std::process::id();
    let ncpu = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1) as f32;

    let mut offenders: Vec<(String, f32)> = Vec::new();
    for (pid, proc_) in sys.processes() {
        if pid.as_u32() == self_pid {
            continue;
        }
        // sysinfo reports CPU as a percentage of a single core; normalize to a
        // share of the whole machine.
        let total = proc_.cpu_usage() / ncpu;
        if total >= CPU_THRESHOLD_TOTAL {
            offenders.push((proc_.name().to_string_lossy().into_owned(), total));
        }
    }

    if offenders.is_empty() {
        return Vec::new();
    }

    offenders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    offenders.truncate(MAX_OFFENDERS);

    let list = offenders
        .iter()
        .map(|(name, cpu)| format!("{name} ({cpu:.0}%)"))
        .collect::<Vec<_>>()
        .join(", ");

    vec![Finding {
        id: "background_apps",
        title: "Background apps using CPU".into(),
        severity: Severity::Suggestion,
        issue: format!("High background CPU use detected: {list}."),
        recommendation: "Consider closing apps you aren't using before playing. FrameWise will \
                         never close a process for you."
            .into(),
        expected_benefit: "Frees CPU headroom if you choose to close a background hog.".into(),
        applicable: false,
    }]
}
