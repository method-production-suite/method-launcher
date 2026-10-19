//! Linux: an app is the AppImage the launcher puts in `~/Applications`, or the package
//! `method-<noun>` with its binary `method-<noun>-app` on the PATH (docs/spec.md §4.6). It is
//! opened by running that file (§6).

use crate::parse::{dpkg_version, find_on_path};
use crate::{env_path, output, run, start};
use launcher_core::catalog::App;
use launcher_core::platform::{Installed, Opened};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the launcher puts an app's AppImage: `~/Applications/Method-Budget.AppImage`.
pub fn appimage_path(app: &App) -> Option<PathBuf> {
    env_path("HOME").map(|home| home.join("Applications").join(format!("Method-{}.AppImage", app.noun())))
}

pub fn locate(app: &App) -> Option<Installed> {
    if let Some(path) = appimage_path(app).filter(|p| p.is_file()) {
        // The version an AppImage was installed at is recorded by the launcher's installs
        // (spec §4.4); until then it is unknown.
        return Some(Installed { path, version: String::new(), legacy: false });
    }
    let path_var = std::env::var("PATH").unwrap_or_default();
    let binary = find_on_path(&app.binary(), &path_var, ':', &[]);
    let version = output("dpkg-query", &["-W", "-f", "${Status}\t${Version}", &app.package()]).and_then(|out| dpkg_version(&out));
    match (binary, version) {
        (Some(path), version) => Some(Installed { path, version: version.unwrap_or_default(), legacy: false }),
        (None, Some(version)) => Some(Installed { path: PathBuf::from(app.binary()), version, legacy: false }),
        (None, None) => None,
    }
}

pub fn is_running(_app: &App, installed: &Installed) -> bool {
    // Process names are cut to 15 characters, so the full command line is matched.
    let path = installed.path.to_string_lossy();
    output("pgrep", &["-f", "--", &path]).is_some()
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
    let folder = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
    run(Command::new("xdg-open").arg(folder))
}

pub fn open_url(url: &str) -> Result<(), String> {
    run(Command::new("xdg-open").arg(url))
}

pub fn cache_root() -> Option<PathBuf> {
    env_path("XDG_CACHE_HOME").or_else(|| env_path("HOME").map(|h| h.join(".cache")))
}

pub fn data_roots() -> Vec<PathBuf> {
    env_path("XDG_DATA_HOME").or_else(|| env_path("HOME").map(|h| h.join(".local/share"))).into_iter().collect()
}
