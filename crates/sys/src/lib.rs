//! The launcher's per-system code (docs/spec.md §4.6, §6, §10): [`System`] implements
//! `launcher-core`'s `Locate`, `Launch` and `Folders` for the system this is built for, using
//! only tools that ship with it. What those tools print is read in [`parse`], tested
//! everywhere.

pub mod net;
pub mod parse;

#[cfg(target_os = "macos")]
pub mod install_macos;

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
    /// A fresh path in the launcher's cache for a scratch or downloaded file.
    pub fn scratch(&self, name: &str) -> Option<PathBuf> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = self.cache()?.join("downloads");
        std::fs::create_dir_all(&dir).ok()?;
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        Some(dir.join(format!("{name}-{}-{n}", std::process::id())))
    }

    /// The version of an app's latest release, from its `latest.json` (spec §5.1).
    pub fn latest_version(&self, app: &App) -> Result<String, String> {
        let scratch = self.scratch(&format!("{}-latest.json", app.slug)).ok_or("no cache folder")?;
        let json = net::fetch_text(&app.manifest_url(), &scratch).map_err(|e| e.to_string())?;
        launcher_core::release::manifest_version(&json).ok_or_else(|| "the release's latest.json has no version".to_string())
    }

    /// An installed app's own icon as PNG bytes, about 256 pixels, if this system can read it
    /// (macOS today: the bundle's `.icns`). Cached under the launcher's cache folder.
    pub fn app_icon(&self, app: &App, installed: &Installed) -> Option<Vec<u8>> {
        let cache = self.cache()?.join("icons");
        os::app_icon(app, installed, &cache)
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
