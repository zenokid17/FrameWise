//! FrameWise library crate.
//!
//! All functionality lives here so it can be unit-tested from a target that does
//! *not* link the `requireAdministrator` manifest (that manifest is attached to
//! the `framewise` binary only — see `build.rs` and `Cargo.toml`). The binary in
//! `src/main.rs` is a thin wrapper around [`app::run`].

pub mod app;
pub mod audit;
pub mod config;
pub mod hotkey;
pub mod logging;
pub mod overlay;
pub mod platform;
pub mod stats;
pub mod tray;
