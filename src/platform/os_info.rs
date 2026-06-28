//! Windows version detection and capability gating.
//!
//! We read the real OS build from the registry (`CurrentVersion`). This is plain
//! data and is *not* affected by the application manifest's "supportedOS"
//! shimming the way `GetVersionEx` is, so it gives us the true build number.

use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

/// Minimum supported build: Windows 10 version 2004 (build 19041).
pub const MIN_SUPPORTED_BUILD: u32 = 19041;
/// Builds >= this are Windows 11.
pub const WINDOWS_11_BUILD: u32 = 22000;
/// PDH "GPU Engine" counters require Windows 10 1709 (build 16299).
pub const GPU_PDH_MIN_BUILD: u32 = 16299;
/// Hardware-Accelerated GPU Scheduling shipped in Windows 10 2004 (build 19041).
pub const HAGS_MIN_BUILD: u32 = 19041;

#[derive(Debug, Clone)]
pub struct OsInfo {
    pub major: u32,
    pub build: u32,
    pub ubr: u32,
    /// e.g. "22H2"
    pub display_version: String,
    /// e.g. "Windows 11 Pro" (note: registry ProductName lags and may still say
    /// "Windows 10 Pro" on Win11; trust `is_windows_11` instead).
    pub product_name: String,
}

impl OsInfo {
    pub fn detect() -> anyhow::Result<OsInfo> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let cur = hklm.open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")?;

        // CurrentBuildNumber is stored as a string (REG_SZ).
        let build_str: String = cur.get_value("CurrentBuildNumber").unwrap_or_default();
        let build: u32 = build_str.trim().parse().unwrap_or(0);
        let major: u32 = cur.get_value("CurrentMajorVersionNumber").unwrap_or(0);
        let ubr: u32 = cur.get_value("UBR").unwrap_or(0);
        let display_version: String = cur
            .get_value("DisplayVersion")
            .or_else(|_| cur.get_value("ReleaseId"))
            .unwrap_or_default();
        let product_name: String = cur.get_value("ProductName").unwrap_or_default();

        Ok(OsInfo {
            major,
            build,
            ubr,
            display_version,
            product_name,
        })
    }

    pub fn is_windows_11(&self) -> bool {
        self.build >= WINDOWS_11_BUILD
    }

    pub fn is_supported(&self) -> bool {
        self.build >= MIN_SUPPORTED_BUILD
    }

    /// Human-readable one-liner for logs / the about screen.
    pub fn version_string(&self) -> String {
        let name = if self.is_windows_11() {
            "Windows 11"
        } else {
            "Windows 10"
        };
        let disp = if self.display_version.is_empty() {
            String::new()
        } else {
            format!(" {}", self.display_version)
        };
        format!("{name}{disp} (build {}.{})", self.build, self.ubr)
    }

    /// Compute which optional features are available on this build.
    pub fn features(&self) -> Features {
        Features {
            gpu_usage: self.build >= GPU_PDH_MIN_BUILD,
            hags: self.build >= HAGS_MIN_BUILD,
        }
    }
}

/// Capability flags derived from the running OS build. Anything `false` here is
/// hidden/disabled in the UI rather than producing an error.
#[derive(Debug, Clone, Copy)]
pub struct Features {
    /// GPU utilization via PDH "GPU Engine" counters.
    pub gpu_usage: bool,
    /// Hardware-Accelerated GPU Scheduling is a meaningful setting on this build.
    pub hags: bool,
}
