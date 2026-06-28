//! Passive FPS telemetry via Intel PresentMon.
//!
//! We spawn `PresentMon.exe` as a child process and parse its CSV output from
//! stdout. PresentMon uses ETW to observe `Present()` calls system-wide — we do
//! **not** inject code, hook graphics APIs, or read game memory. This is what
//! keeps FrameWise anti-cheat safe.
//!
//! The CSV header is parsed by *name* (case-insensitive) so we tolerate column
//! differences across PresentMon versions.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// CREATE_NO_WINDOW: spawn PresentMon without flashing a console window (we're a
/// GUI app and capture its output via a pipe).
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

use crate::config::Config;
use crate::platform::exe_dir;
use crate::stats::FrameTimes;

/// Candidate column names for the per-frame interval, newest-first. PresentMon
/// has used several names across versions.
const FRAME_TIME_COLUMNS: &[&str] = &["msbetweenpresents", "frametime", "msbetweendisplaychange"];
const PID_COLUMNS: &[&str] = &["processid", "processid()", "pid"];

/// Resolve the PresentMon executable: explicit config path, else next to our
/// exe. Returns None if not found.
fn resolve_presentmon(config: &Config) -> Option<PathBuf> {
    let configured = config.telemetry.presentmon_path.trim();
    if !configured.is_empty() {
        let p = PathBuf::from(configured);
        if p.exists() {
            return Some(p);
        }
        log::warn!("configured presentmon_path does not exist: {configured}");
    }
    // Common bundled names next to framewise.exe.
    for name in ["PresentMon.exe", "presentmon.exe"] {
        let p = exe_dir().join(name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Start the PresentMon parser. Returns true if PresentMon was found and
/// launched. On false, FPS stats are simply unavailable (graceful).
pub fn start(
    config: &Config,
    frames: Arc<Mutex<HashMap<u32, FrameTimes>>>,
    shutdown: Arc<AtomicBool>,
    child_slot: Arc<Mutex<Option<Child>>>,
) -> bool {
    let exe = match resolve_presentmon(config) {
        Some(p) => p,
        None => return false,
    };

    // Default args: stream CSV to stdout, take over any existing session.
    // Override-able / extendable via config for version differences.
    let mut args: Vec<String> = vec!["--output_stdout".into(), "--stop_existing_session".into()];
    args.extend(config.telemetry.presentmon_extra_args.iter().cloned());

    let capacity = config.telemetry.low_sample_window;

    let mut command = Command::new(&exe);
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            log::error!("failed to launch PresentMon ({}): {e}", exe.display());
            return false;
        }
    };
    log::info!("PresentMon started: {} {args:?}", exe.display());

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            log::error!("PresentMon produced no stdout pipe");
            let _ = child.kill();
            return false;
        }
    };

    // Drain stderr so PresentMon doesn't block on a full pipe; log it.
    if let Some(stderr) = child.stderr.take() {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if !line.trim().is_empty() {
                    log::debug!("presentmon: {line}");
                }
            }
        });
    }

    // Hand the child to the owner so it can be killed on shutdown.
    if let Ok(mut guard) = child_slot.lock() {
        *guard = Some(child);
    }

    thread::spawn(move || parser_loop(stdout, frames, shutdown, capacity));
    true
}

fn find_column(headers: &[String], candidates: &[&str]) -> Option<usize> {
    headers
        .iter()
        .position(|h| candidates.iter().any(|c| h.eq_ignore_ascii_case(c)))
}

fn parser_loop(
    stdout: std::process::ChildStdout,
    frames: Arc<Mutex<HashMap<u32, FrameTimes>>>,
    shutdown: Arc<AtomicBool>,
    capacity: usize,
) {
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    // First non-empty line is the CSV header.
    let header = loop {
        if shutdown.load(Ordering::Relaxed) {
            return;
        }
        match lines.next() {
            Some(Ok(l)) if !l.trim().is_empty() => break l,
            Some(Ok(_)) => continue,
            Some(Err(e)) => {
                log::error!("presentmon stdout error: {e}");
                return;
            }
            None => return,
        }
    };

    let headers: Vec<String> = header.split(',').map(|s| s.trim().to_string()).collect();
    let pid_idx = find_column(&headers, PID_COLUMNS);
    let ft_idx = find_column(&headers, FRAME_TIME_COLUMNS);

    let (pid_idx, ft_idx) = match (pid_idx, ft_idx) {
        (Some(p), Some(f)) => (p, f),
        _ => {
            log::error!(
                "could not find ProcessID / frame-time columns in PresentMon header: {header}"
            );
            return;
        }
    };
    log::info!("PresentMon columns: pid={pid_idx} frame_time={ft_idx}");

    let mut cleanup_counter: u32 = 0;

    for line in lines {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                log::error!("presentmon stdout error: {e}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }

        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() <= pid_idx.max(ft_idx) {
            continue;
        }
        let pid: u32 = match cols[pid_idx].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ft: f32 = match cols[ft_idx].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Ok(mut map) = frames.lock() {
            map.entry(pid)
                .or_insert_with(|| FrameTimes::new(capacity))
                .push(ft);

            // Periodically drop processes that stopped presenting to bound memory.
            cleanup_counter = cleanup_counter.wrapping_add(1);
            if cleanup_counter % 2000 == 0 {
                map.retain(|_, ft| !ft.is_stale(std::time::Duration::from_secs(10)));
            }
        }
    }

    log::info!("PresentMon parser stopped");
}
