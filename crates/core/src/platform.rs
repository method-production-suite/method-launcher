//! What differs per system, behind one small trait per concern (docs/spec.md §10). The window
//! and the install flow are written once against these; each system has one implementation,
//! and tests use a fake one.

use crate::catalog::App;
use std::path::{Path, PathBuf};

/// The systems the launcher knows (docs/spec.md §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    MacosArm64,
    /// Intel Macs (owner's call, 2026-10-19: universal Mac builds from each app's next release).
    MacosX64,
    WindowsX64,
    LinuxX86_64,
    LinuxArm64,
}

impl Target {
    /// The system this build runs on; `None` on one the suite doesn't build for.
    pub const fn current() -> Option<Target> {
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Some(Target::MacosArm64)
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            Some(Target::MacosX64)
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
            Target::MacosX64 => "macos-x64",
            Target::WindowsX64 => "windows-x64",
            Target::LinuxX86_64 => "linux-x86_64",
            Target::LinuxArm64 => "linux-arm64",
        }
    }

    /// The installer kinds the suite publishes for this system, the default first.
    pub const fn kinds(self) -> &'static [Kind] {
        match self {
            Target::MacosArm64 | Target::MacosX64 => &[Kind::Dmg],
            Target::WindowsX64 => &[Kind::SetupExe],
            Target::LinuxX86_64 => &[Kind::AppImage, Kind::Deb],
            Target::LinuxArm64 => &[],
        }
    }

    /// Whether an install here must find a checksum line first (docs/spec.md §4.1). On macOS
    /// the signature checks are what count; a checksum is an extra check when there is one.
    pub const fn needs_checksum(self) -> bool {
        !matches!(self, Target::MacosArm64 | Target::MacosX64)
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
    /// The version it reports, as written there; empty when it reports none.
    pub version: String,
    /// Installed under the app's earlier name ([`crate::catalog::Legacy`]).
    pub legacy: bool,
}

/// What opening an app did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opened {
    /// It started, with any launch arguments.
    Started,
    /// It was already running and was brought forward; no arguments were passed.
    BroughtForward,
    /// It was already running and this system can't bring it forward; nothing was done.
    AlreadyRunning,
}

/// Which catalog apps are installed, where, and at which version (docs/spec.md §4.6).
pub trait Locate {
    fn locate(&self, app: &App) -> Option<Installed>;
}

/// Opening apps and files, and showing a location (docs/spec.md §6).
pub trait Launch {
    /// Whether the app is running now. Launch arguments only work when it isn't.
    fn is_running(&self, app: &App, installed: &Installed) -> bool;
    /// Opens the app. `args` are passed only when it starts (SUITE-STANDARDS.md §2); a
    /// running app is brought forward where the system can, and left alone otherwise.
    fn open_app(&self, app: &App, installed: &Installed, args: &[&str]) -> Result<Opened, String>;
    fn open_file(&self, app: &App, installed: &Installed, file: &Path) -> Result<(), String>;
    fn show_location(&self, path: &Path) -> Result<(), String>;
    /// Opens an https link in the person's browser.
    fn open_url(&self, url: &str) -> Result<(), String>;
}

/// Turns a downloaded, checksum-checked installer into an installed app, with the system's own
/// verification first (docs/spec.md §4.2 to §4.4). Never runs the file before that.
pub trait Install {
    fn install(&self, app: &App, kind: Kind, downloaded: &Path) -> Result<Installed, String>;
}

/// Where the launcher keeps its cache and data, and where an app keeps `recent.json`.
pub trait Folders {
    fn cache(&self) -> Option<PathBuf>;
    fn data(&self) -> Option<PathBuf>;
    /// The folders an app's `recent.json` may be in, in the order to look.
    fn app_data(&self, app: &App) -> Vec<PathBuf>;
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
        assert!(!Target::MacosArm64.needs_checksum() && !Target::MacosX64.needs_checksum());
        assert_eq!(Target::MacosX64.kinds(), &[Kind::Dmg]);
        assert!(Target::WindowsX64.needs_checksum() && Target::LinuxX86_64.needs_checksum());
    }
}
