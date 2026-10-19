//! Installing a suite app on Linux as its AppImage (docs/spec.md §4.4): downloaded from the
//! latest release and checked against the release's SHA-256 checksums, then placed in
//! `~/Applications/Method-<Noun>.AppImage` through a staging name (made executable, then
//! renamed into place, so there is never a half-written app at the real name), with a menu
//! entry and an icon. The launcher records the version, since an AppImage can't say it
//! without running. Nothing downloaded runs during the install. An update is the same, over
//! the installed file.

use crate::fetch::{fail, fetch_verified};
use crate::linux::appimage_path;
use crate::parse::{desktop_entry, with_installed};
use crate::{InstallError, Stage, System, env_path};
use launcher_core::catalog::App;
use launcher_core::platform::{Folders, Installed, Kind, Launch};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Where the launcher keeps its record of installs.
pub fn record_path() -> Option<PathBuf> {
    System.data().map(|d| d.join("installed.json"))
}

pub fn install(
    app: &App,
    current: Option<&Installed>,
    quit: bool,
    icon_png: Option<&[u8]>,
    stage: &dyn Fn(Stage),
) -> Result<Installed, InstallError> {
    if let Some(current) = current
        && System.is_running(app, current)
    {
        if !quit {
            return Err(fail(format!("Quit {} first, then try again.", app.name), "it is running"));
        }
        stage(Stage::Quitting);
        quit_app(app, current)?;
    }
    let fetched = fetch_verified(app, Kind::AppImage, true, stage)?;
    stage(Stage::Copying);
    let dest = appimage_path(app).ok_or_else(|| fail("Your home folder couldn't be found.", "HOME is not set"))?;
    let dir = dest.parent().map(PathBuf::from).unwrap_or_default();
    std::fs::create_dir_all(&dir).map_err(|e| fail(format!("{} couldn't be installed. Nothing was changed.", app.name), e.to_string()))?;
    let staging = dir.join(format!(".Method-{}.AppImage.installing-{}", app.noun(), std::process::id()));
    let placed = std::fs::copy(&fetched.file, &staging)
        .and_then(|_| std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755)))
        .and_then(|()| std::fs::rename(&staging, &dest));
    if let Err(e) = placed {
        let _ = std::fs::remove_file(&staging);
        return Err(fail(format!("{} couldn't be installed. Nothing was changed.", app.name), e.to_string()));
    }
    menu_entry(app, &dest, icon_png);
    let version = fetched.tag.trim_start_matches('v').to_string();
    if let Some(record) = record_path() {
        let json = std::fs::read_to_string(&record).unwrap_or_default();
        if let Some(parent) = record.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&record, with_installed(&json, app.slug, &version, &dest.to_string_lossy()));
    }
    Ok(Installed { path: dest, version, legacy: false })
}

/// The app's menu entry and icon, so it opens like any other app. Best effort: the AppImage
/// is installed and opens from the launcher even if these can't be written.
fn menu_entry(app: &App, appimage: &std::path::Path, icon_png: Option<&[u8]>) {
    let Some(data) = env_path("XDG_DATA_HOME").or_else(|| env_path("HOME").map(|h| h.join(".local/share"))) else { return };
    let icon = format!("method-{}", app.slug);
    if let Some(png) = icon_png {
        let dir = data.join("icons/hicolor/256x256/apps");
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::fs::write(dir.join(format!("{icon}.png")), png);
        }
    }
    let apps = data.join("applications");
    if std::fs::create_dir_all(&apps).is_ok() {
        let entry = desktop_entry(app.name, app.tagline, &appimage.to_string_lossy(), &icon, &app.binary());
        let _ = std::fs::write(apps.join(format!("{}.desktop", app.bundle_id)), entry);
        let _ = Command::new("update-desktop-database").arg(&apps).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
}

/// Asks the running app to end (SIGTERM, as closing its window would), and waits up to two
/// minutes.
fn quit_app(app: &App, current: &Installed) -> Result<(), InstallError> {
    let path = current.path.to_string_lossy().into_owned();
    let _ = Command::new("pkill").args(["-TERM", "-f", "--", &path]).stdin(Stdio::null()).status();
    for _ in 0..240 {
        if !System.is_running(app, current) {
            std::thread::sleep(Duration::from_millis(500));
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(fail(format!("{} is still open, so it wasn't updated.", app.name), "it didn't quit within two minutes"))
}
