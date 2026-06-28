//! Pure formatting for the overlay: turns a `StatSnapshot` into the lines of
//! text to draw, with a colour hint per line. Kept free of Win32 so it can be
//! unit-tested (and so the drawing code stays small).

use crate::config::Stats;
use crate::stats::StatSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Fps,
    Low,
    Normal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub kind: LineKind,
}

/// RGB colour for a line kind.
pub fn color_for(kind: LineKind) -> (u8, u8, u8) {
    match kind {
        LineKind::Fps => (120, 230, 130),
        LineKind::Low => (230, 200, 120),
        LineKind::Normal => (225, 228, 235),
    }
}

const LABEL_WIDTH: usize = 8;

fn row(label: &str, value: String, kind: LineKind) -> Line {
    Line {
        text: format!("{label:<LABEL_WIDTH$}{value}"),
        kind,
    }
}

/// Build the visible overlay lines from the enabled stats and the latest values.
///
/// `fps_available` reflects whether PresentMon is running. When it isn't, the
/// FPS-derived rows show a short "n/a" notice instead of disappearing silently,
/// so the user understands why.
pub fn build_lines(stats: &Stats, snap: &StatSnapshot, fps_available: bool) -> Vec<Line> {
    let mut lines = Vec::new();

    if stats.fps {
        match snap.fps {
            Some(fps) => lines.push(row("FPS", format!("{:.0}", fps.round()), LineKind::Fps)),
            None if !fps_available => {
                lines.push(row("FPS", "n/a (PresentMon)".into(), LineKind::Normal))
            }
            None => lines.push(row("FPS", "—".into(), LineKind::Fps)),
        }
    }

    if stats.low_1_percent {
        match snap.low_1_percent {
            Some(low) => lines.push(row("1% low", format!("{:.0}", low.round()), LineKind::Low)),
            None if fps_available => lines.push(row("1% low", "—".into(), LineKind::Low)),
            None => {}
        }
    }

    if stats.frame_time {
        match snap.frame_time_ms {
            Some(ft) => lines.push(row("Frame", format!("{ft:.1} ms"), LineKind::Normal)),
            None if fps_available => lines.push(row("Frame", "—".into(), LineKind::Normal)),
            None => {}
        }
    }

    if stats.cpu {
        if let Some(cpu) = snap.cpu_percent {
            lines.push(row("CPU", format!("{:.0}%", cpu.round()), LineKind::Normal));
        }
    }

    if stats.gpu {
        // Hidden entirely when unavailable (older build / no PDH counter).
        if let Some(gpu) = snap.gpu_percent {
            lines.push(row("GPU", format!("{:.0}%", gpu.round()), LineKind::Normal));
        }
    }

    if stats.ram {
        if let (Some(used), Some(total)) = (snap.ram_used_mb, snap.ram_total_mb) {
            let used_gb = used as f64 / 1024.0;
            let total_gb = total as f64 / 1024.0;
            lines.push(row(
                "RAM",
                format!("{used_gb:.1} / {total_gb:.1} GB"),
                LineKind::Normal,
            ));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_on() -> Stats {
        Stats {
            fps: true,
            low_1_percent: true,
            frame_time: true,
            cpu: true,
            gpu: true,
            ram: true,
        }
    }

    #[test]
    fn shows_presentmon_notice_when_unavailable() {
        let snap = StatSnapshot::default();
        let lines = build_lines(&all_on(), &snap, false);
        assert!(lines[0].text.contains("n/a (PresentMon)"));
        // With FPS unavailable, low/frame rows are omitted, not shown as "—".
        assert!(!lines.iter().any(|l| l.text.starts_with("1% low")));
    }

    #[test]
    fn formats_values() {
        let snap = StatSnapshot {
            fps: Some(143.6),
            low_1_percent: Some(98.2),
            frame_time_ms: Some(6.94),
            cpu_percent: Some(23.4),
            gpu_percent: Some(61.0),
            ram_used_mb: Some(12_800),
            ram_total_mb: Some(32_768),
        };
        let lines = build_lines(&all_on(), &snap, true);
        let joined: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert!(joined.iter().any(|t| t.contains("144"))); // fps rounded
        assert!(joined.iter().any(|t| t.contains("6.9 ms")));
        assert!(joined.iter().any(|t| t.contains("23%")));
        assert!(joined.iter().any(|t| t.contains("12.5 / 32.0 GB")));
    }

    #[test]
    fn gpu_hidden_when_unavailable() {
        let snap = StatSnapshot {
            cpu_percent: Some(10.0),
            gpu_percent: None,
            ram_used_mb: Some(1024),
            ram_total_mb: Some(8192),
            ..Default::default()
        };
        let lines = build_lines(&all_on(), &snap, true);
        assert!(!lines.iter().any(|l| l.text.starts_with("GPU")));
    }
}
