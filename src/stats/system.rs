//! System-metric sampler: CPU%, RAM, and GPU% on a slow cadence (these change
//! little frame-to-frame, unlike FPS which the overlay computes itself). Runs in
//! a single background thread, no OS service.

use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use sysinfo::System;

use crate::config::Config;
use crate::platform::Features;
use crate::stats::Telemetry;

use gpu::GpuSampler;

/// Cadence for CPU/GPU/RAM sampling. CPU% needs >= ~200ms between refreshes to
/// be meaningful; 500ms is plenty for these slow-moving stats.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

pub fn start(_config: &Config, features: &Features, tele: Telemetry) {
    let gpu_enabled = features.gpu_usage;
    thread::spawn(move || {
        let mut sys = System::new();
        let mut gpu = if gpu_enabled { GpuSampler::new() } else { None };
        if gpu_enabled && gpu.is_none() {
            log::warn!("GPU utilization counter unavailable; GPU row will be hidden");
        }

        loop {
            if tele.shutdown.load(Ordering::Relaxed) {
                break;
            }

            sys.refresh_cpu_usage();
            sys.refresh_memory();

            let cpu = sys.global_cpu_usage();
            let used = sys.used_memory();
            let total = sys.total_memory();
            let gpu_pct = gpu.as_mut().and_then(|g| g.sample());

            if let Ok(mut snap) = tele.snapshot.lock() {
                snap.cpu_percent = Some(cpu);
                snap.gpu_percent = gpu_pct;
                snap.ram_used_mb = Some(used / 1024 / 1024);
                snap.ram_total_mb = Some(total / 1024 / 1024);
            }

            thread::sleep(SAMPLE_INTERVAL);
        }
        log::info!("system sampler stopped");
    });
}

/// GPU utilization via the PDH "GPU Engine" counter set (the same source Task
/// Manager uses, available on Windows 10 1709+).
///
/// NOTE: PDH has the thinnest, most version-sensitive bindings in `windows-rs`.
/// If the crate ever changes these signatures, this is the one module likely to
/// need a small adjustment — it is intentionally self-contained so a failure
/// here only disables the GPU row, never the rest of the app.
mod gpu {
    use windows::core::{w, PCWSTR};
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    };

    /// PDH status meaning "buffer too small; here's the required size".
    const PDH_MORE_DATA: u32 = 0x800007D2;

    pub struct GpuSampler {
        query: isize,
        counter: isize,
    }

    impl GpuSampler {
        pub fn new() -> Option<GpuSampler> {
            unsafe {
                let mut query: isize = 0;
                if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                    return None;
                }
                let mut counter: isize = 0;
                let path = w!("\\GPU Engine(*)\\Utilization Percentage");
                if PdhAddEnglishCounterW(query, path, 0, &mut counter) != 0 {
                    let _ = PdhCloseQuery(query);
                    return None;
                }
                // Prime the query; the first collection produces no delta.
                let _ = PdhCollectQueryData(query);
                Some(GpuSampler { query, counter })
            }
        }

        /// Returns the busiest GPU engine's utilization in 0..=100, or None.
        pub fn sample(&mut self) -> Option<f32> {
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }

                // First call: discover the required buffer size.
                let mut buf_size: u32 = 0;
                let mut item_count: u32 = 0;
                let status = PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut buf_size,
                    &mut item_count,
                    None,
                );
                if status != PDH_MORE_DATA || buf_size == 0 {
                    return None;
                }

                // Second call: fill the array.
                let mut buffer = vec![0u8; buf_size as usize];
                let items = buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
                let status = PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut buf_size,
                    &mut item_count,
                    Some(items),
                );
                if status != 0 {
                    return None;
                }

                let slice = std::slice::from_raw_parts(items, item_count as usize);
                let mut max = 0.0f64;
                for it in slice {
                    let v = it.FmtValue.Anonymous.doubleValue;
                    if v.is_finite() && v > max {
                        max = v;
                    }
                }
                Some(max.min(100.0) as f32)
            }
        }
    }

    impl Drop for GpuSampler {
        fn drop(&mut self) {
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
        }
    }
}
