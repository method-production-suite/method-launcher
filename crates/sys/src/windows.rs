//! Windows: apps are found through their uninstall entries in the registry, matched by
//! display name `Method <Noun>` (docs/spec.md §4.6), and opened by running their executable
//! (§6).

use crate::parse::{exe_from_icon, reg_uninstall};
use crate::{env_path, output, run, start};
use launcher_core::catalog::App;
use launcher_core::platform::{Installed, Opened};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Per-user installs first, then per-machine (64-bit, then 32-bit).
const UNINSTALL_KEYS: [&str; 3] = [
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
];

fn system32(tool: &str) -> PathBuf {
    env_path("SystemRoot").unwrap_or_else(|| PathBuf::from(r"C:\Windows")).join("System32").join(tool)
}

pub fn locate(app: &App) -> Option<Installed> {
    let reg = system32("reg.exe");
    UNINSTALL_KEYS.iter().find_map(|key| {
        let listing = output(&reg.to_string_lossy(), &["query", key, "/s"])?;
        let entry = reg_uninstall(&listing, app.name)?;
        let from_icon = exe_from_icon(&entry.display_icon).map(PathBuf::from).filter(|p| p.is_file());
        let location = PathBuf::from(entry.install_location.trim_matches('"'));
        let path = from_icon.or_else(|| {
            [format!("{}.exe", app.binary()), format!("{}.exe", app.name)].into_iter().map(|exe| location.join(exe)).find(|p| p.is_file())
        })?;
        Some(Installed { path, version: entry.version, legacy: false })
    })
}

pub fn is_running(_app: &App, installed: &Installed) -> bool {
    let Some(exe) = installed.path.file_name().map(|n| n.to_string_lossy().into_owned()) else { return false };
    let filter = format!("IMAGENAME eq {exe}");
    output(&system32("tasklist.exe").to_string_lossy(), &["/FI", &filter, "/NH"])
        .is_some_and(|out| out.to_ascii_lowercase().contains(&exe.to_ascii_lowercase()))
}

pub fn open_app(app: &App, installed: &Installed, args: &[&str]) -> Result<Opened, String> {
    if is_running(app, installed) {
        return Ok(Opened::AlreadyRunning);
    }
    start(Command::new(&installed.path).args(args))?;
    Ok(Opened::Started)
}

pub fn open_file(_app: &App, installed: &Installed, file: &Path) -> Result<(), String> {
    start(Command::new(&installed.path).arg(file))
}

pub fn show_location(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    // Explorer reads `/select,"path"` as one argument, quoted its own way.
    let explorer = env_path("SystemRoot").unwrap_or_else(|| PathBuf::from(r"C:\Windows")).join("explorer.exe");
    // Explorer returns 1 even when it worked, so its status is not checked.
    let _ = Command::new(explorer).raw_arg(format!("/select,\"{}\"", path.display())).status();
    Ok(())
}

pub fn open_url(url: &str) -> Result<(), String> {
    run(Command::new(system32("rundll32.exe")).arg("url.dll,FileProtocolHandler").arg(url))
}

pub fn cache_root() -> Option<PathBuf> {
    env_path("LOCALAPPDATA")
}

/// `%APPDATA%` first, then `%LOCALAPPDATA%` (SUITE-STANDARDS.md §2).
pub fn data_roots() -> Vec<PathBuf> {
    ["APPDATA", "LOCALAPPDATA"].into_iter().filter_map(env_path).collect()
}
