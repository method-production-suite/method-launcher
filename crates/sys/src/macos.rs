//! macOS: apps are bundles in `/Applications` or `~/Applications`, found by bundle id
//! (docs/spec.md §4.6), and opened with `open` (§6).

use crate::parse::bundle_matches;
use crate::{env_path, output, run};
use launcher_core::catalog::App;
use launcher_core::platform::{Installed, Opened};
use launcher_core::verify::OPEN;
use std::path::{Path, PathBuf};
use std::process::Command;

const PLUTIL: &str = "/usr/bin/plutil";
const MDFIND: &str = "/usr/bin/mdfind";
const PGREP: &str = "/usr/bin/pgrep";
const SIPS: &str = "/usr/bin/sips";

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Some(home) = env_path("HOME") {
        dirs.push(home.join("Applications"));
    }
    dirs
}

/// One key of a bundle's `Info.plist`, XML or binary, read with `plutil`.
pub(crate) fn info(bundle: &Path, key: &str) -> Option<String> {
    let plist = bundle.join("Contents/Info.plist");
    output(PLUTIL, &["-extract", key, "raw", "-o", "-", &plist.to_string_lossy()]).map(|s| s.trim().to_string())
}

/// The bundle at `path`, if it is this app.
fn check(app: &App, path: &Path) -> Option<Installed> {
    if !path.is_dir() {
        return None;
    }
    let id = info(path, "CFBundleIdentifier")?;
    let legacy = bundle_matches(&id, app.bundle_id, app.legacy.map(|l| l.id_suffix))?;
    let version = info(path, "CFBundleShortVersionString").unwrap_or_default();
    Some(Installed { path: path.to_path_buf(), version, legacy })
}

pub fn locate(app: &App) -> Option<Installed> {
    let dirs = application_dirs();
    let mut names = vec![app.name];
    names.extend(app.legacy.map(|l| l.name));
    // The usual names first; current names before earlier ones.
    for name in &names {
        for dir in &dirs {
            if let Some(found) = check(app, &dir.join(format!("{name}.app"))) {
                return Some(found);
            }
        }
    }
    // Then anything Spotlight knows with this id, anywhere under the Applications folders.
    let query = format!("kMDItemCFBundleIdentifier == \"{}\"", app.bundle_id);
    let found = output(MDFIND, &[&query]).unwrap_or_default();
    found.lines().map(PathBuf::from).filter(|p| dirs.iter().any(|d| p.starts_with(d))).find_map(|p| check(app, &p))
}

pub fn is_running(_app: &App, installed: &Installed) -> bool {
    let pattern = format!("^{}/Contents/MacOS/", regex_escape(&installed.path.to_string_lossy()));
    output(PGREP, &["-f", &pattern]).is_some()
}

/// Escapes a path for `pgrep`'s extended regular expressions.
fn regex_escape(text: &str) -> String {
    text.chars().fold(String::new(), |mut out, c| {
        if "\\^$.|?*+()[]{}".contains(c) {
            out.push('\\');
        }
        out.push(c);
        out
    })
}

pub fn open_app(app: &App, installed: &Installed, args: &[&str]) -> Result<Opened, String> {
    let mut command = Command::new(OPEN);
    command.arg(&installed.path);
    let running = is_running(app, installed);
    if !running && !args.is_empty() {
        command.arg("--args").args(args);
    }
    run(&mut command)?;
    Ok(if running { Opened::BroughtForward } else { Opened::Started })
}

pub fn open_file(_app: &App, installed: &Installed, file: &Path) -> Result<(), String> {
    run(Command::new(OPEN).arg("-a").arg(&installed.path).arg(file))
}

pub fn show_location(path: &Path) -> Result<(), String> {
    run(Command::new(OPEN).arg("-R").arg(path))
}

pub fn open_url(url: &str) -> Result<(), String> {
    run(Command::new(OPEN).arg(url))
}

/// The bundle's `.icns` (named by `CFBundleIconFile`), converted to a 256-pixel PNG by `sips`
/// and kept in `cache`, named by app and version so an update draws its new icon.
pub fn app_icon(app: &App, installed: &Installed, cache: &Path) -> Option<Vec<u8>> {
    let mut name = info(&installed.path, "CFBundleIconFile")?;
    if Path::new(&name).extension().is_none() {
        name.push_str(".icns");
    }
    let icns = installed.path.join("Contents/Resources").join(&name);
    if !icns.is_file() {
        return None;
    }
    let version: String = installed.version.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.').collect();
    let png = cache.join(format!("{}-{version}{}.png", app.slug, if installed.legacy { "-earlier" } else { "" }));
    if !png.is_file() {
        std::fs::create_dir_all(cache).ok()?;
        let (icns, out) = (icns.to_string_lossy(), png.to_string_lossy());
        output(SIPS, &["-s", "format", "png", "-Z", "256", &icns, "--out", &out])?;
    }
    std::fs::read(&png).ok()
}

pub fn cache_root() -> Option<PathBuf> {
    env_path("HOME").map(|h| h.join("Library/Caches"))
}

pub fn data_roots() -> Vec<PathBuf> {
    env_path("HOME").map(|h| h.join("Library/Application Support")).into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_bundle_paths_for_pgrep() {
        assert_eq!(regex_escape("/Applications/Method Budget.app"), "/Applications/Method Budget\\.app");
        assert_eq!(regex_escape("/a/(b)+c"), "/a/\\(b\\)\\+c");
    }
}
