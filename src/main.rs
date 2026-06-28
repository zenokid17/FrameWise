// In release builds, run as a GUI app (no console window). Debug builds keep the
// console so logs are visible while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let log_path = framewise::platform::exe_dir().join("framewise.log");
    framewise::logging::init(&log_path);
    log::info!("FrameWise {} starting", env!("CARGO_PKG_VERSION"));

    if let Err(e) = framewise::app::run() {
        log::error!("fatal: {e:#}");
        framewise::app::fatal_message_box(&format!("FrameWise failed to start:\n\n{e:#}"));
        std::process::exit(1);
    }
}
