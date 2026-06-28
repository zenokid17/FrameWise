//! Metric types: the shared snapshot the overlay renders, and the per-process
//! frame-time buffer used to compute FPS / 1% low / frame time.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// The latest computed values, read by the overlay each refresh. `None` means
/// "not available" (feature unsupported, PresentMon missing, or no recent data)
/// and the corresponding row is hidden.
#[derive(Debug, Clone, Copy, Default)]
pub struct StatSnapshot {
    pub fps: Option<f32>,
    pub low_1_percent: Option<f32>,
    pub frame_time_ms: Option<f32>,
    pub cpu_percent: Option<f32>,
    pub gpu_percent: Option<f32>,
    pub ram_used_mb: Option<u64>,
    pub ram_total_mb: Option<u64>,
}

/// A rolling buffer of recent frame intervals (milliseconds between presents)
/// for a single process, each tagged with arrival time so we can compute a
/// time-windowed mean and detect staleness.
pub struct FrameTimes {
    samples: VecDeque<(Instant, f32)>,
    capacity: usize,
}

impl FrameTimes {
    pub fn new(capacity: usize) -> Self {
        FrameTimes {
            samples: VecDeque::with_capacity(capacity.min(4096)),
            capacity,
        }
    }

    pub fn push(&mut self, frame_time_ms: f32) {
        if !frame_time_ms.is_finite() || frame_time_ms <= 0.0 {
            return;
        }
        if self.samples.len() >= self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back((Instant::now(), frame_time_ms));
    }

    /// True if no frame has arrived within `max_age` (e.g. game minimized or not
    /// presenting). Caller treats this as "FPS unknown".
    pub fn is_stale(&self, max_age: Duration) -> bool {
        match self.samples.back() {
            Some((t, _)) => t.elapsed() > max_age,
            None => true,
        }
    }

    fn mean_recent(&self, window: Duration) -> Option<f32> {
        let now = Instant::now();
        let mut sum = 0.0f32;
        let mut count = 0u32;
        for (t, v) in self.samples.iter().rev() {
            if now.duration_since(*t) > window {
                break;
            }
            sum += *v;
            count += 1;
        }
        if count == 0 {
            None
        } else {
            Some(sum / count as f32)
        }
    }

    /// p in 0.0..=1.0; returns the frame time at that percentile over the full
    /// buffer. Requires a minimum number of samples to be meaningful.
    fn percentile(&self, p: f32) -> Option<f32> {
        let n = self.samples.len();
        if n < 30 {
            return None;
        }
        let mut v: Vec<f32> = self.samples.iter().map(|(_, x)| *x).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((p * n as f32).ceil() as usize)
            .saturating_sub(1)
            .min(n - 1);
        Some(v[idx])
    }

    /// Returns `(fps, frame_time_ms, low_1_percent_fps)`.
    ///
    /// - fps / frame time are averaged over `recent` (a short window) so they
    ///   track the present.
    /// - 1% low is `1000 / p99(frame_time)` over the whole buffer: the FPS of
    ///   the worst 1% of frames.
    pub fn summary(&self, recent: Duration) -> Option<(f32, f32, f32)> {
        let mean_ft = self.mean_recent(recent)?;
        if mean_ft <= 0.0 {
            return None;
        }
        let fps = 1000.0 / mean_ft;
        let low = self.percentile(0.99).map(|p99| 1000.0 / p99).unwrap_or(fps);
        Some((fps, mean_ft, low))
    }
}
