//! Installing a suite app on macOS from its latest release's `.dmg` (docs/spec.md §4.1,
//! §4.2): pin the release, download, check the checksum where the release has one, mount
//! read-only, verify the app (bundle id, the suite's Developer ID, notarisation, Apple
//! silicon only, no links out of the bundle), then copy it beside its destination and swap it
//! into place. Nothing downloaded runs before every check has passed; every failure removes
//! what it made.
//!
//! An update is the same install over the installed copy (spec §5.1, as the owner asked on
//! 2026-10-19): the old copy is moved aside, the new one renamed into place, and the old one
//! removed only after that worked. An app installed under its earlier name is replaced by the
//! new one under its current name.

use crate::macos::info;
use crate::net::{self, LARGE, NetError};
use crate::parse::hdiutil_mount_point;
use crate::{InstallError, Stage, System};
use launcher_core::catalog::App;
use launcher_core::checksums::{self, Hasher};
use launcher_core::macho;
use launcher_core::platform::{Folders, Installed, Kind, Launch};
use launcher_core::release::tag_from_location;
use launcher_core::verify::{self, CODESIGN, DITTO, HDIUTIL, SPCTL};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const OSASCRIPT: &str = "/usr/bin/osascript";

fn fail(message: impl Into<String>, details: impl Into<String>) -> InstallError {
    InstallError::new(message, details)
}

fn net_fail(app: &App, e: &NetError) -> InstallError {
    match e {
        NetError::NotFound => fail(format!("{} has no download for this Mac yet.", app.name), e.to_string()),
        NetError::Redirected(_) => fail("The download came from somewhere unexpected, so it was stopped.", e.to_string()),
        NetError::Stopped(_) => fail("The download stopped. Try again.", e.to_string()),
        _ => fail("GitHub didn't answer. Check the connection and try again.", e.to_string()),
    }
}

/// Runs a tool, returning its status and what it wrote to stdout and stderr together.
fn tool(program: &str, args: &[&str]) -> Result<(bool, String), String> {
    let out = Command::new(program).args(args).stdin(Stdio::null()).output().map_err(|e| format!("{program}: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok((out.status.success(), text))
}

/// Installs (or updates) `app` into `dest_dir`, reporting each stage. `current` is the copy
/// already installed, if any, which this replaces.
pub fn install(app: &App, dest_dir: &Path, current: Option<&Installed>, stage: &dyn Fn(Stage)) -> Result<Installed, InstallError> {
    let system = System;
    if let Some(current) = current
        && system.is_running(app, current)
    {
        return Err(fail(format!("Quit {} first, then try again.", app.name), "it is running"));
    }
    let file = app.installer(Kind::Dmg).ok_or_else(|| fail(format!("{} has no download yet.", app.name), "no release"))?;

    // 1. Pin the release: the checksums and the image come from the same tag.
    stage(Stage::Finding);
    let scratch = system.scratch(&format!("{}-redirect", app.slug)).ok_or_else(|| fail("The launcher has no cache folder.", ""))?;
    let location = net::redirect_of(&app.latest_url(), &scratch).map_err(|e| net_fail(app, &e))?;
    let tag = tag_from_location(&location).ok_or_else(|| fail("The latest release isn't what we expected.", location.clone()))?;

    // 2. Checksums, where the release has them (an extra check on macOS, spec §4.1).
    let mut expected = None;
    for name in checksums::FILES {
        let scratch = system.scratch(&format!("{}-{name}", app.slug)).ok_or_else(|| fail("The launcher has no cache folder.", ""))?;
        match net::fetch_text(&app.asset_url(&tag, name), &scratch) {
            Ok(sums) => {
                expected = checksums::lookup(&sums, &file);
                if expected.is_some() {
                    break;
                }
            }
            Err(NetError::NotFound) => {}
            Err(e) => return Err(net_fail(app, &e)),
        }
    }

    // 3. Download, reporting the bytes so far.
    let dmg = system.scratch(&format!("{}.dmg", app.slug)).ok_or_else(|| fail("The launcher has no cache folder.", ""))?;
    let url = app.asset_url(&tag, &file);
    let downloaded = {
        let target = dmg.clone();
        let worker = std::thread::spawn(move || net::download(&url, &target, LARGE));
        while !worker.is_finished() {
            stage(Stage::Downloading(std::fs::metadata(&dmg).map(|m| m.len()).unwrap_or(0)));
            std::thread::sleep(Duration::from_millis(250));
        }
        worker.join().unwrap_or_else(|_| Err(NetError::Local("the download stopped unexpectedly".into())))
    };
    let result = downloaded.map_err(|e| net_fail(app, &e)).and_then(|()| {
        stage(Stage::Checking);
        if let Some(expected) = &expected {
            let actual = hash_file(&dmg).map_err(|e| fail("The download couldn't be read.", e))?;
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(fail(
                    "The download doesn't match the release's checksum, so it was deleted. Try again.",
                    format!("expected {expected}, got {actual}"),
                ));
            }
        }
        install_from_image(app, &dmg, dest_dir, current, stage)
    });
    let _ = std::fs::remove_file(&dmg);
    result
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Hasher::default();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(hasher.hex());
        }
        hasher.update(buffer.get(..n).unwrap_or_default());
    }
}

/// Mounts a downloaded image read-only, verifies and installs its app, and always detaches.
pub fn install_from_image(
    app: &App,
    dmg: &Path,
    dest_dir: &Path,
    current: Option<&Installed>,
    stage: &dyn Fn(Stage),
) -> Result<Installed, InstallError> {
    let mounts = System.cache().map(|c| c.join("mounts")).ok_or_else(|| fail("The launcher has no cache folder.", ""))?;
    std::fs::create_dir_all(&mounts).map_err(|e| fail("The launcher's cache folder couldn't be made.", e.to_string()))?;
    let (ok, plist) = tool(
        HDIUTIL,
        &["attach", "-readonly", "-nobrowse", "-noautoopen", "-plist", "-mountrandom", &mounts.to_string_lossy(), &dmg.to_string_lossy()],
    )
    .map_err(|e| fail("The download is damaged. Try again.", e))?;
    let mount = hdiutil_mount_point(&plist).filter(|_| ok).ok_or_else(|| fail("The download is damaged. Try again.", plist.trim().to_string()))?;
    let result = verify_and_copy(app, Path::new(&mount), dest_dir, current, stage);
    if !matches!(tool(HDIUTIL, &["detach", &mount, "-quiet"]), Ok((true, _))) {
        let _ = tool(HDIUTIL, &["detach", &mount, "-force", "-quiet"]);
    }
    result
}

/// The image's one app, checked (spec §4.2 step 2).
fn verified_app(app: &App, mount: &Path) -> Result<PathBuf, InstallError> {
    let unexpected = |details: String| fail("The download isn't what we expected, so nothing was installed.", details);
    let apps: Vec<PathBuf> = std::fs::read_dir(mount)
        .map_err(|e| unexpected(e.to_string()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "app") && !p.is_symlink())
        .collect();
    let [bundle] = apps.as_slice() else { return Err(unexpected(format!("{} apps in the image", apps.len()))) };
    if bundle.file_name().is_none_or(|n| n.to_string_lossy() != app.bundle_name()) {
        return Err(unexpected(format!("the image holds {}", bundle.display())));
    }
    let id = info(bundle, "CFBundleIdentifier").unwrap_or_default();
    if id != app.bundle_id {
        return Err(unexpected(format!("bundle id {id}")));
    }
    verify_bundle(app, bundle)?;
    Ok(bundle.clone())
}

/// Signature, notarisation, architecture and links.
pub fn verify_bundle(app: &App, bundle: &Path) -> Result<(), InstallError> {
    let unverified = |details: String| fail(format!("This copy of {} couldn't be verified, so nothing was installed.", app.name), details);
    let path = bundle.to_string_lossy().into_owned();
    let args = verify::codesign_args(&path);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match tool(CODESIGN, &args) {
        Ok((true, _)) => {}
        Ok((false, text)) => return Err(unverified(format!("signature: {}", text.trim()))),
        Err(e) => return Err(unverified(e)),
    }
    let args = verify::spctl_args(&path);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match tool(SPCTL, &args) {
        Ok((_, text)) if verify::spctl_says_notarised(&text) => {}
        Ok((_, text)) => return Err(unverified(format!("notarisation: {}", text.trim()))),
        Err(e) => return Err(unverified(e)),
    }
    check_files(bundle, bundle).map_err(unverified)
}

/// Every Mach-O file is arm64 only, and no link points outside the bundle.
fn check_files(root: &Path, dir: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            let target = std::fs::read_link(&path).map_err(|e| e.to_string())?;
            let resolved = normalise(&path.parent().unwrap_or(root).join(target));
            if !resolved.starts_with(root) {
                return Err(format!("{} links outside the app", path.display()));
            }
        } else if kind.is_dir() {
            check_files(root, &path)?;
        } else if kind.is_file() {
            let mut head = vec![0; macho::HEADER_BYTES];
            let n = std::fs::File::open(&path).and_then(|mut f| f.read(&mut head)).map_err(|e| e.to_string())?;
            if !macho::passes(head.get(..n).unwrap_or_default()) {
                return Err(format!(
                    "{} is not built for Apple silicon only ({:?})",
                    path.display(),
                    macho::classify(head.get(..n).unwrap_or_default())
                ));
            }
        }
    }
    Ok(())
}

/// `a/b/../c` as `a/c`, without touching the disk.
fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

fn verify_and_copy(app: &App, mount: &Path, dest_dir: &Path, current: Option<&Installed>, stage: &dyn Fn(Stage)) -> Result<Installed, InstallError> {
    let source = verified_app(app, mount)?;
    stage(Stage::Copying);
    let dest = dest_dir.join(app.bundle_name());
    // Something else under that name: never replaced.
    if dest.exists() && current.is_none_or(|c| c.path != dest) {
        let id = info(&dest, "CFBundleIdentifier").unwrap_or_default();
        if id != app.bundle_id {
            return Err(fail(format!("Something else called {} is already in {}.", app.bundle_name(), dest_dir.display()), id));
        }
    }
    let tag =
        format!("{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0));
    let staging = dest_dir.join(format!(".{}.installing-{tag}", app.bundle_name()));
    let aside = dest_dir.join(format!(".{}.replaced-{tag}", app.bundle_name()));
    // The copy to replace: one at the destination, then an earlier-named one elsewhere.
    let old = [Some(dest.clone()), current.map(|c| c.path.clone())].into_iter().flatten().find(|p| p.exists());
    let old_elsewhere = current.map(|c| c.path.clone()).filter(|p| *p != dest && p.exists());

    let direct = std::fs::create_dir(&staging).and_then(|()| std::fs::remove_dir(&staging)).is_ok();
    if direct {
        let copied = tool(DITTO, &[&source.to_string_lossy(), &staging.to_string_lossy()]);
        if !matches!(copied, Ok((true, _))) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(fail(format!("{} couldn't be copied. Nothing was changed.", app.name), format!("{copied:?}")));
        }
        if let Err(e) = verify_bundle(app, &staging) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(e);
        }
        swap(&staging, &dest, old.as_deref(), &aside).map_err(|e| {
            let _ = std::fs::remove_dir_all(&staging);
            fail(format!("{} couldn't be installed. Nothing was changed.", app.name), e)
        })?;
        if let Some(elsewhere) = &old_elsewhere {
            let _ = std::fs::remove_dir_all(elsewhere);
        }
    } else {
        admin_install(app, &source, &staging, &dest, old.as_deref(), old_elsewhere.as_deref(), &aside)?;
        if let Err(e) = verify_bundle(app, &dest) {
            return Err(fail(format!("{} was copied, but the copy couldn't be verified.", app.name), e.details));
        }
    }
    let version = info(&dest, "CFBundleShortVersionString").unwrap_or_default();
    Ok(Installed { path: dest, version, legacy: false })
}

/// Old copy aside, new copy in, old copy removed; the old copy comes back if the rename fails.
fn swap(staging: &Path, dest: &Path, old: Option<&Path>, aside: &Path) -> Result<(), String> {
    let moved_aside = match old.filter(|o| *o == dest) {
        Some(o) => {
            std::fs::rename(o, aside).map_err(|e| format!("moving the old copy aside: {e}"))?;
            true
        }
        None => false,
    };
    if let Err(e) = std::fs::rename(staging, dest) {
        if moved_aside {
            let _ = std::fs::rename(aside, dest);
        }
        return Err(format!("putting the new copy in place: {e}"));
    }
    if moved_aside {
        let _ = std::fs::remove_dir_all(aside);
    }
    Ok(())
}

/// The same copy and swap as one shell script run with an administrator's password, through
/// the system's own prompt (spec §4.2 step 3). The person is told why first, by the card.
fn admin_install(
    app: &App,
    source: &Path,
    staging: &Path,
    dest: &Path,
    old: Option<&Path>,
    elsewhere: Option<&Path>,
    aside: &Path,
) -> Result<(), InstallError> {
    let q = |p: &Path| shell_quote(&p.to_string_lossy());
    let mut script = format!("/usr/bin/ditto {} {} && ", q(source), q(staging));
    if old.is_some_and(|o| o == dest) {
        script.push_str(&format!("/bin/mv {} {} && /bin/mv {} {} && /bin/rm -rf {}", q(dest), q(aside), q(staging), q(dest), q(aside)));
    } else {
        script.push_str(&format!("/bin/mv {} {}", q(staging), q(dest)));
    }
    if let Some(elsewhere) = elsewhere {
        script.push_str(&format!(" && /bin/rm -rf {}", q(elsewhere)));
    }
    let apple = format!(
        "do shell script \"{}\" with prompt \"{}\" with administrator privileges",
        applescript_escape(&script),
        applescript_escape(&format!("Method Launcher wants to install {} in Applications.", app.name))
    );
    match tool(OSASCRIPT, &["-e", &apple]) {
        Ok((true, _)) => Ok(()),
        Ok((false, text)) if text.contains("-128") => {
            Err(fail("Not installed. You can install it in your own Applications folder instead.", "the administrator prompt was cancelled"))
        }
        Ok((false, text)) => Err(fail(format!("{} couldn't be installed. Nothing was changed.", app.name), text.trim().to_string())),
        Err(e) => Err(fail(format!("{} couldn't be installed. Nothing was changed.", app.name), e)),
    }
}

/// A shell word in single quotes.
fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// Text inside an AppleScript string literal.
fn applescript_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Leftovers from an install the launcher was quit in the middle of: staging and set-aside
/// copies in the Applications folders, partial downloads, and its own mounts. Only what is
/// more than an hour old, so a check or an install running now is never touched.
pub fn clean_up(dirs: &[PathBuf]) {
    let stale = |path: &Path| {
        std::fs::symlink_metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > Duration::from_secs(3600))
    };
    for dir in dirs {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let ours = launcher_core::catalog::APPS.iter().any(|a| name.starts_with(&format!(".{}.", a.bundle_name())));
            if ours && (name.contains(".installing-") || name.contains(".replaced-")) && stale(&entry.path()) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    let Some(cache) = System.cache() else { return };
    for entry in std::fs::read_dir(cache.join("downloads")).into_iter().flatten().flatten() {
        if stale(&entry.path()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    for entry in std::fs::read_dir(cache.join("mounts")).into_iter().flatten().flatten() {
        if stale(&entry.path()) {
            let _ = tool(HDIUTIL, &["detach", &entry.path().to_string_lossy(), "-force", "-quiet"]);
            let _ = std::fs::remove_dir(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_for_the_shell_and_applescript() {
        assert_eq!(shell_quote("/Applications/Method Budget.app"), "'/Applications/Method Budget.app'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(applescript_escape(r#"say "hi" \ bye"#), r#"say \"hi\" \\ bye"#);
    }

    #[test]
    fn normalises_link_targets() {
        assert_eq!(normalise(Path::new("/A.app/Contents/Frameworks/../MacOS/x")), PathBuf::from("/A.app/Contents/MacOS/x"));
        assert!(!normalise(Path::new("/A.app/Contents/../../etc/passwd")).starts_with("/A.app"));
    }
}
