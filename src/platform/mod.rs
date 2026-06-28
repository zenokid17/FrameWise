//! Platform layer: OS version detection, feature gating, and elevation checks.

pub mod admin;
pub mod os_info;

pub use admin::is_elevated;
pub use os_info::{Features, OsInfo};

use std::path::PathBuf;

/// Directory containing the running executable. All runtime files (config, log,
/// change journal) live here, per the project spec ("next to the executable").
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}
