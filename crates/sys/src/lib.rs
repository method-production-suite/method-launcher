//! The launcher's per-system code (docs/spec.md §4.6, §6, §10): [`System`] implements
//! `launcher-core`'s `Locate`, `Launch` and `Folders` for the system this is built for, using
//! only tools that ship with it. What those tools print is read in [`parse`], tested
//! everywhere.

pub mod parse;

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

impl System {
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
