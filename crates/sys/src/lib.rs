//! The launcher's per-system code (docs/spec.md §4.6, §6, §10): [`System`] implements
//! `launcher-core`'s `Locate`, `Launch` and `Folders` for the system this is built for, using
//! only tools that ship with it. What those tools print is read in [`parse`], tested
//! everywhere.

pub mod fetch;
pub mod net;
pub mod parse;

#[cfg(target_os = "linux")]
pub mod install_linux;
#[cfg(target_os = "macos")]
pub mod install_macos;
#[cfg(target_os = "windows")]
pub mod install_windows;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux as os;
#[cfg(target_os = "macos")]
use macos as os;
#[cfg(target_os = "windows")]
use windows as os;

use launcher_core::catalog::{App, LAUNCHER_ID};
use launcher_core::platform::{Folders, Installed, Launch, Locate, Opened};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// This computer.
#[derive(Clone, Copy, Debug, Default)]
pub struct System;

impl Locate for System {
    fn locate(&self, app: &App) -> Option<Installed> {
        os::locate(app)
    }
}

impl Launch for System {
    fn is_running(&self, app: &App, installed: &Installed) -> bool {
        os::is_running(app, installed)
    }

    fn open_app(&self, app: &App, installed: &Installed, args: &[&str]) -> Result<Opened, String> {
        os::open_app(app, installed, args)
    }

    fn open_file(&self, app: &App, installed: &Installed, file: &Path) -> Result<(), String> {
        os::open_file(app, installed, file)
    }

    fn show_location(&self, path: &Path) -> Result<(), String> {
        os::show_location(path)
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        // Only links the launcher itself makes: https, no spaces or quotes to misread.
        if !url.starts_with("https://") || url.contains(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
            return Err(format!("not a link the launcher opens: {url}"));
        }
        os::open_url(url)
    }
}

/// Where an install is: shown on the app's card (docs/spec.md §3.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Finding the latest release.
    Finding,
    /// Downloading, with the bytes so far.
    Downloading(u64),
    /// Checking the download.
    Checking,
    /// Copying into place.
    Copying,
    /// Asking the running app to quit before it is replaced.
    Quitting,
}

/// Why an install stopped: one plain sentence for the card, and the technical detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallError {
    pub message: String,
    pub details: String,
}

impl InstallError {
    pub fn new(message: impl Into<String>, details: impl Into<String>) -> Self {
        InstallError { message: message.into(), details: details.into() }
    }
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.details.is_empty() { write!(f, "{}", self.message) } else { write!(f, "{} ({})", self.message, self.details) }
    }
}

impl System {
    /// Installs or updates `app` from its latest release on this system (spec §4): over
    /// `current` when there is one, asking it to quit first when `quit` says the person agreed.
    /// `icon_png` is the app's mark, for the Linux menu entry.
    pub fn install(
        &self,
        app: &App,
        current: Option<&Installed>,
        quit: bool,
        icon_png: Option<&[u8]>,
        stage: &dyn Fn(Stage),
    ) -> Result<Installed, InstallError> {
        #[cfg(target_os = "macos")]
        {
            let _ = icon_png;
            // An update goes where the installed copy is; a new install into /Applications.
            let dest = current.and_then(|c| c.path.parent().map(PathBuf::from)).unwrap_or_else(|| PathBuf::from("/Applications"));
            install_macos::install(app, &dest, current, quit, stage)
        }
        #[cfg(target_os = "windows")]
        {
            let _ = icon_png;
            install_windows::install(app, current, quit, stage)
        }
        #[cfg(target_os = "linux")]
        {
            install_linux::install(app, current, quit, icon_png, stage)
        }
    }

    /// A fresh path in the launcher's cache for a scratch or downloaded file.
    pub fn scratch(&self, name: &str) -> Option<PathBuf> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = self.cache()?.join("downloads");
        std::fs::create_dir_all(&dir).ok()?;
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        Some(dir.join(format!("{name}-{}-{n}", std::process::id())))
    }

    /// The version of the launcher's own latest release, from where `releases/latest`
    /// redirects (spec §5.2). The launcher doesn't update itself yet; it offers the download.
    pub fn launcher_latest(&self) -> Result<String, String> {
        let scratch = self.scratch("launcher-redirect").ok_or("no cache folder")?;
        let location = net::redirect_of(launcher_core::catalog::LAUNCHER_RELEASES, &scratch).map_err(|e| e.to_string())?;
        let tag = launcher_core::release::tag_from_location(&location).ok_or("the latest release has no version tag")?;
        Ok(tag.trim_start_matches('v').to_string())
    }

    /// An app's latest published release (spec §5.1), decided at run time: `Ok(None)` when it
    /// has none yet (its `releases/latest` answers 404). The version is the tag's; the date and
    /// the line of what's new come from its `latest.json` when it has one. The flag says
    /// whether the release offers an installer for this system: false only when its
    /// `SHA256SUMS`, which lists every file, lacks one (or the suite builds none here).
    pub fn latest_release(&self, app: &App) -> Result<Option<(launcher_core::release::Release, bool)>, String> {
        use launcher_core::release::{Release, manifest, tag_from_location};
        let scratch = |name: &str| self.scratch(&format!("{}-{name}", app.slug)).ok_or("no cache folder");
        let location = match net::redirect_of(&app.latest_url(), &scratch("redirect")?) {
            Ok(location) => location,
            Err(net::NetError::NotFound) => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let tag = tag_from_location(&location).ok_or("the latest release has no version tag")?;
        let version = tag.trim_start_matches('v').to_string();
        let mut release = Release { version: version.clone(), date: None, summary: None };
        if let Ok(json) = net::fetch_text(&app.asset_url(&tag, "latest.json"), &scratch("latest.json")?)
            && let Some(found) = manifest(&json)
            && found.version == version
        {
            release = found;
        }
        let kinds = launcher_core::platform::Target::current().map(|t| t.kinds()).unwrap_or_default();
        // Every checksum file is read: an older release lists its installers in a second one.
        let (mut listed, mut any_list) = (false, false);
        for name in launcher_core::checksums::FILES {
            if let Ok(sums) = net::fetch_text(&app.asset_url(&tag, name), &scratch(name)?) {
                any_list = true;
                listed |= kinds.iter().filter_map(|&k| app.installer(k)).any(|file| launcher_core::checksums::lookup(&sums, &file).is_some());
            }
        }
        let has_installer = !kinds.is_empty() && (listed || !any_list);
        Ok(Some((release, has_installer)))
    }

    /// The suite's list of apps from its website (spec §9), checked; `None` when it can't be
    /// read or fails any check, and the launcher keeps the list it has.
    pub fn website_catalog(&self) -> Option<(String, Vec<App>)> {
        let json = net::fetch_text(launcher_core::catalog::CATALOG_URL, &self.scratch("apps.json")?).ok()?;
        let apps = launcher_core::catalog::parse_remote(&json)?;
        Some((json, apps))
    }

    /// An installed app's own icon as PNG bytes, about 256 pixels, if this system can read it
    /// (macOS today: the bundle's `.icns`). Cached under the launcher's cache folder.
    pub fn app_icon(&self, app: &App, installed: &Installed) -> Option<Vec<u8>> {
        let cache = self.cache()?.join("icons");
        os::app_icon(app, installed, &cache)
    }
}

/// A file an app opened lately, with when (the list's time, or else the file's own).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recent {
    pub path: PathBuf,
    pub opened: std::time::SystemTime,
}

impl System {
    /// The files `app` lists in its `recent.json` that still exist, newest first (spec §6).
    /// The list is only read; a missing or unreadable one is no files.
    pub fn recent_files(&self, app: &App) -> Vec<Recent> {
        let Some(json) = self.app_data(app).into_iter().find_map(|dir| std::fs::read_to_string(dir.join("recent.json")).ok()) else {
            return Vec::new();
        };
        launcher_core::recent::parse(&json)
            .into_iter()
            .filter(|e| e.path.is_file())
            .filter_map(|e| {
                let opened = match e.opened {
                    Some(secs) => std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(u64::try_from(secs).ok()?))?,
                    None => std::fs::metadata(&e.path).and_then(|m| m.modified()).ok()?,
                };
                Some(Recent { path: e.path, opened })
            })
            .collect()
    }
}

impl Folders for System {
    fn cache(&self) -> Option<PathBuf> {
        os::cache_root().map(|root| root.join(LAUNCHER_ID))
    }

    fn data(&self) -> Option<PathBuf> {
        os::data_roots().into_iter().next().map(|root| root.join(LAUNCHER_ID))
    }

    fn app_data(&self, app: &App) -> Vec<PathBuf> {
        os::data_roots().into_iter().map(|root| root.join(app.bundle_id)).collect()
    }
}

/// An environment variable that is set and not empty.
fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// Runs a tool and returns what it printed, or `None` if it couldn't run or failed.
fn output(program: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::null()).stderr(Stdio::null());
    quiet(&mut command);
    let out = command.output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Runs a tool to completion, for ones that return at once (`open`, `xdg-open`).
fn run(command: &mut Command) -> Result<(), String> {
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    quiet(command);
    let status = command.status().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err(format!("{:?} exited with {status}", command.get_program())) }
}

/// Starts a program that keeps running (an app), and reaps it on a thread when it exits.
/// (macOS opens apps through `open` instead.)
#[cfg(not(target_os = "macos"))]
fn start(command: &mut Command) -> Result<(), String> {
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    quiet(command);
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// No console window flashes up on Windows for the tools the launcher runs.
#[cfg(target_os = "windows")]
fn quiet(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn quiet(_command: &mut Command) {}
