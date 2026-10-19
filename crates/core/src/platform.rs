//! What differs per system, behind one small trait per concern (docs/spec.md §10). The window
//! and the install flow are written once against these; each system has one implementation,
//! and tests use a fake one.

use crate::catalog::App;
use std::path::{Path, PathBuf};

/// The systems the launcher knows (docs/spec.md §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    MacosArm64,
    WindowsX64,
    LinuxX86_64,
    LinuxArm64,
}

impl Target {
    /// The system this build runs on; `None` on one the suite doesn't build for.
    pub const fn current() -> Option<Target> {
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Some(Target::MacosArm64)
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            Some(Target::WindowsX64)
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            Some(Target::LinuxX86_64)
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            Some(Target::LinuxArm64)
        } else {
            None
        }
    }

    /// The short name used in logs and on the command line.
    pub const fn name(self) -> &'static str {
        match self {
            Target::MacosArm64 => "macos-arm64",
            Target::WindowsX64 => "windows-x64",
            Target::LinuxX86_64 => "linux-x86_64",
            Target::LinuxArm64 => "linux-arm64",
        }
    }

    /// The installer kinds the suite publishes for this system, the default first.
    pub const fn kinds(self) -> &'static [Kind] {
        match self {
            Target::MacosArm64 => &[Kind::Dmg],
            Target::WindowsX64 => &[Kind::SetupExe],
            Target::LinuxX86_64 => &[Kind::AppImage, Kind::Deb],
            Target::LinuxArm64 => &[],
        }
    }

    /// Whether an install here must find a checksum line first (docs/spec.md §4.1). On macOS
    /// the signature checks are what count; a checksum is an extra check when there is one.
    pub const fn needs_checksum(self) -> bool {
        !matches!(self, Target::MacosArm64)
    }
}

/// An installer format (docs/spec.md §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Dmg,
    SetupExe,
    AppImage,
    Deb,
}

impl Kind {
    /// The version-free release asset name for an app's noun (SUITE-STANDARDS.md §3).
    pub fn file_name(self, noun: &str) -> String {
        match self {
            Kind::Dmg => format!("Method-{noun}-macOS.dmg"),
            Kind::SetupExe => format!("Method-{noun}-windows-x64-setup.exe"),
            Kind::AppImage => format!("Method-{noun}-linux-x86_64.AppImage"),
            Kind::Deb => format!("Method-{noun}-linux-amd64.deb"),
        }
    }
}

/// An app found on this computer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    /// The bundle, executable or AppImage that opens it.
    pub path: PathBuf,
    /// The version it reports, as written there.
    pub version: String,
}

/// Which catalog apps are installed, where, and at which version (docs/spec.md §4.6).
pub trait Locate {
    fn locate(&self, app: &App) -> Option<Installed>;
}

/// Opening apps and files, and showing a location (docs/spec.md §6).
pub trait Launch {
    fn open_app(&self, app: &App, installed: &Installed) -> Result<(), String>;
    fn open_file(&self, app: &App, installed: &Installed, file: &Path) -> Result<(), String>;
    fn show_location(&self, path: &Path) -> Result<(), String>;
}

/// Turns a downloaded, checksum-checked installer into an installed app, with the system's own
/// verification first (docs/spec.md §4.2 to §4.4). Never runs the file before that.
pub trait Install {
    fn install(&self, app: &App, kind: Kind, downloaded: &Path) -> Result<Installed, String>;
}

/// Where the launcher keeps its cache and data, and where an app keeps `recent.json`.
pub trait Folders {
    fn cache(&self) -> PathBuf;
    fn data(&self) -> PathBuf;
    fn app_data(&self, app: &App) -> PathBuf;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_build_has_a_target() {
        // CI runs on the four supported systems.
        assert!(Target::current().is_some());
    }

    #[test]
    fn names_follow_the_standard() {
        assert_eq!(Kind::Dmg.file_name("Budget"), "Method-Budget-macOS.dmg");
        assert_eq!(Kind::SetupExe.file_name("Budget"), "Method-Budget-windows-x64-setup.exe");
        assert_eq!(Kind::AppImage.file_name("Budget"), "Method-Budget-linux-x86_64.AppImage");
        assert_eq!(Kind::Deb.file_name("Budget"), "Method-Budget-linux-amd64.deb");
    }

    #[test]
    fn arm_linux_has_nothing_to_install_and_only_macos_skips_checksums() {
        assert!(Target::LinuxArm64.kinds().is_empty());
        assert_eq!(Target::LinuxX86_64.kinds().first(), Some(&Kind::AppImage));
        assert!(!Target::MacosArm64.needs_checksum());
        assert!(Target::WindowsX64.needs_checksum() && Target::LinuxX86_64.needs_checksum());
    }
}
