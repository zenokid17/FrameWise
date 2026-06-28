//! Minimal file + stderr logger. Avoids pulling a logging framework so we keep
//! the dependency surface (and binary) small. Writes to `framewise.log` next to
//! the executable; also echoes to stderr (visible in debug builds / when run
//! from a console).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use log::{LevelFilter, Metadata, Record};

struct FileLogger {
    file: Mutex<Option<std::fs::File>>,
}

static LOGGER: FileLogger = FileLogger {
    file: Mutex::new(None),
};

fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // HH:MM:SS.mmm in UTC — enough to correlate events without pulling chrono.
    let total = now.as_secs();
    let millis = now.subsec_millis();
    let h = (total / 3600) % 24;
    let m = (total / 60) % 60;
    let s = total % 60;
    format!("{h:02}:{m:02}:{s:02}.{millis:03}")
}

impl log::Log for FileLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "{} [{:<5}] {}: {}\n",
            timestamp(),
            record.level(),
            record.target(),
            record.args()
        );
        eprint!("{line}");
        if let Ok(mut guard) = self.file.lock() {
            if let Some(f) = guard.as_mut() {
                let _ = f.write_all(line.as_bytes());
                let _ = f.flush();
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = self.file.lock() {
            if let Some(f) = guard.as_mut() {
                let _ = f.flush();
            }
        }
    }
}

/// Initialize logging. Safe to call once at startup. If the log file can't be
/// opened we still log to stderr.
pub fn init(log_path: &Path) {
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(log_path) {
        if let Ok(mut guard) = LOGGER.file.lock() {
            *guard = Some(file);
        }
    }
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    });
}
