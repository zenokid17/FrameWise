//! Telemetry orchestration: owns the shared snapshot and spawns the background
//! collectors (PresentMon parser + system-metrics sampler). No background OS
//! service is created — these are plain threads inside our process.

pub mod metrics;
pub mod presentmon;
pub mod system;

pub use metrics::{FrameTimes, StatSnapshot};

use std::collections::HashMap;
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::platform::Features;

/// Shared, cloneable handle to all telemetry state. Cheap to clone (Arcs).
#[derive(Clone)]
pub struct Telemetry {
    /// Latest values rendered by the overlay.
    pub snapshot: Arc<Mutex<StatSnapshot>>,
    /// Per-process frame-time buffers, keyed by PID, filled by the PresentMon
    /// parser and consumed by the system sampler for the current target PID.
    pub frames: Arc<Mutex<HashMap<u32, FrameTimes>>>,
    /// PID of the foreground/fullscreen game whose FPS we report.
    pub target_pid: Arc<AtomicU32>,
    /// Set true to ask the background threads to stop.
    pub shutdown: Arc<AtomicBool>,
    /// Whether FPS telemetry is available (PresentMon found + started).
    pub fps_available: Arc<AtomicBool>,
    /// The PresentMon child process (so we can kill it on exit).
    child: Arc<Mutex<Option<Child>>>,
}

impl Default for Telemetry {
    fn default() -> Self {
        Telemetry {
            snapshot: Arc::new(Mutex::new(StatSnapshot::default())),
            frames: Arc::new(Mutex::new(HashMap::new())),
            target_pid: Arc::new(AtomicU32::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
            fps_available: Arc::new(AtomicBool::new(false)),
            child: Arc::new(Mutex::new(None)),
        }
    }
}

impl Telemetry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn the background collectors. Call once at startup.
    pub fn start(&self, config: &Config, features: &Features) {
        let fps_ok = presentmon::start(
            config,
            self.frames.clone(),
            self.shutdown.clone(),
            self.child.clone(),
        );
        self.fps_available.store(fps_ok, Ordering::Relaxed);
        if !fps_ok {
            log::warn!(
                "PresentMon not available — FPS / 1% low / frame time are disabled. \
                 See README to install PresentMon."
            );
        }

        system::start(config, features, self.clone());
    }

    pub fn set_target_pid(&self, pid: u32) {
        self.target_pid.store(pid, Ordering::Relaxed);
    }

    pub fn fps_available(&self) -> bool {
        self.fps_available.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> StatSnapshot {
        self.snapshot.lock().map(|g| *g).unwrap_or_default()
    }

    /// Compute `(fps, frame_time_ms, low_1_percent_fps)` for the current target
    /// PID from the frame buffer. Returns None if FPS is unavailable, there's no
    /// target, or the game hasn't presented recently (`stale_after`).
    pub fn fps_summary(
        &self,
        recent: std::time::Duration,
        stale_after: std::time::Duration,
    ) -> Option<(f32, f32, f32)> {
        if !self.fps_available() {
            return None;
        }
        let pid = self.target_pid.load(Ordering::Relaxed);
        if pid == 0 {
            return None;
        }
        let map = self.frames.lock().ok()?;
        let ft = map.get(&pid)?;
        if ft.is_stale(stale_after) {
            return None;
        }
        ft.summary(recent)
    }

    /// Signal threads to stop and kill the PresentMon child.
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}
