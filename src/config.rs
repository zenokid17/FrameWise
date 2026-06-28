//! Configuration model + persistence. The config file (`framewise.toml`) lives
//! next to the executable, is human-editable, and is reloaded on launch. Invalid
//! values are clamped to safe ranges rather than rejected.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::platform::exe_dir;

pub const CONFIG_FILE: &str = "framewise.toml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub overlay: Overlay,
    pub hotkey: Hotkey,
    pub telemetry: Telemetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Overlay {
    pub position: Position,
    pub margin: i32,
    pub font_size: i32,
    pub opacity: f32,
    pub refresh_hz: u32,
    pub visible_on_start: bool,
    pub stats: Stats,
}

impl Default for Overlay {
    fn default() -> Self {
        Overlay {
            position: Position::TopLeft,
            margin: 14,
            font_size: 15,
            opacity: 0.85,
            refresh_hz: 10,
            visible_on_start: true,
            stats: Stats::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub fps: bool,
    pub low_1_percent: bool,
    pub frame_time: bool,
    pub cpu: bool,
    pub gpu: bool,
    pub ram: bool,
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            fps: true,
            low_1_percent: true,
            frame_time: true,
            cpu: true,
            gpu: true,
            ram: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkey {
    /// Any of "ALT", "CTRL", "SHIFT", "SUPER".
    pub modifiers: Vec<String>,
    /// "F1".."F24", a single letter "A".."Z", or a digit "0".."9".
    pub key: String,
}

impl Default for Hotkey {
    fn default() -> Self {
        Hotkey {
            // Ctrl+Alt+O ("O" = Overlay). Chosen to avoid common conflicts.
            modifiers: vec!["CTRL".into(), "ALT".into()],
            key: "O".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Telemetry {
    /// Path to PresentMon.exe. Empty = look next to framewise.exe.
    pub presentmon_path: String,
    /// Advanced: extra CLI args appended to the PresentMon invocation.
    pub presentmon_extra_args: Vec<String>,
    /// Frames retained for the 1% low calculation.
    pub low_sample_window: usize,
}

impl Default for Telemetry {
    fn default() -> Self {
        Telemetry {
            presentmon_path: String::new(),
            presentmon_extra_args: Vec::new(),
            low_sample_window: 1000,
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        exe_dir().join(CONFIG_FILE)
    }

    /// Load the config from disk, writing defaults first if it doesn't exist.
    /// Always returns a valid, clamped config (never errors out the app).
    pub fn load_or_create() -> Config {
        let path = Self::path();
        let mut cfg = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(text) => match toml::from_str::<Config>(&text) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("config parse error ({e}); using defaults");
                        Config::default()
                    }
                },
                Err(e) => {
                    log::warn!("config read error ({e}); using defaults");
                    Config::default()
                }
            }
        } else {
            Config::default()
        };

        cfg.clamp();

        if !path.exists() {
            if let Err(e) = cfg.save_to(&path) {
                log::warn!("could not write default config: {e}");
            }
        }
        cfg
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }

    /// Clamp all numeric settings into safe ranges so a hand-edited file can't
    /// produce a broken overlay.
    pub fn clamp(&mut self) {
        self.overlay.opacity = self.overlay.opacity.clamp(0.10, 1.0);
        self.overlay.refresh_hz = self.overlay.refresh_hz.clamp(1, 60);
        self.overlay.font_size = self.overlay.font_size.clamp(8, 72);
        self.overlay.margin = self.overlay.margin.clamp(0, 4000);
        self.telemetry.low_sample_window = self.telemetry.low_sample_window.clamp(60, 100_000);
    }
}
