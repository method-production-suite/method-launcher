//! Installing a suite app on macOS from its latest release's `.dmg` (docs/spec.md §4.1,
//! §4.2): pin the release, download, check the checksum where the release has one, mount
//! read-only, verify the app (bundle id, the suite's Developer ID, notarisation, code for this
//! Mac's chip in every executable, no links out of the bundle), then copy it beside its destination and swap it
//! into place. Nothing downloaded runs before every check has passed; every failure removes
//! what it made.
//!
//! An update is the same install over the installed copy (spec §5.1, as the owner asked on
//! 2026-10-19): the old copy is moved aside, the new one renamed into place, and the old one
//! removed only after that worked. An app installed under its earlier name is replaced by the
//! new one under its current name.

use crate::fetch::{fail, fetch_verified};
use crate::macos::info;
use crate::parse::hdiutil_mount_point;
use crate::{InstallError, Stage, System};
use launcher_core::catalog::App;
use launcher_core::macho;
use launcher_core::platform::{Folders, Installed, Kind, Launch};
use launcher_core::verify::{self, CODESIGN, DITTO, HDIUTIL, SPCTL};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const OSASCRIPT: &str = "/usr/bin/osascript";

/// Runs a tool, returning its status and what it wrote to stdout and stderr together.
fn tool(program: &str, args: &[&str]) -> Result<(bool, String), String> {
    let out = Command::new(program).args(args).stdin(Stdio::null()).output().map_err(|e| format!("{program}: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok((out.status.success(), text))
}

/// Installs (or updates) `app` into `dest_dir`, reporting each stage. `current` is the copy
/// already installed, if any, which this replaces. With `quit`, a running copy is asked to
/// quit first (the person agreed to that on the card); without it, a running copy stops the
/// update.
pub fn install(app: &App, dest_dir: &Path, current: Option<&Installed>, quit: bool, stage: &dyn Fn(Stage)) -> Result<Installed, InstallError> {
    let system = System;
    if let Some(current) = current
        && system.is_running(app, current)
    {
        if !quit {
            return Err(fail(format!("Quit {} first, then try again.", app.name), "it is running"));
        }
        stage(Stage::Quitting);
        quit_app(app, current)?;
    }
    let fetched = fetch_verified(app, Kind::Dmg, false, stage)?;
    install_from_image(app, &fetched.file, dest_dir, current, stage)
}

/// Asks a running app to quit the way the Quit menu does, so it can ask about unsaved work,
/// and waits up to two minutes for it. (macOS asks once whether Method Launcher may control
/// the app.)
fn quit_app(app: &App, current: &Installed) -> Result<(), InstallError> {
    let id = info(&current.path, "CFBundleIdentifier").unwrap_or_else(|| app.bundle_id.to_string());
    let script = format!("tell application id \"{}\" to quit", applescript_escape(&id));
    let (ok, text) = tool(OSASCRIPT, &["-e", &script]).map_err(|e| fail(format!("{} couldn't be asked to quit.", app.name), e))?;
    if !ok {
        return Err(fail(format!("{} couldn't be asked to quit, so it wasn't updated. Quit it and try again.", app.name), text.trim().to_string()));
    }
    for _ in 0..240 {
        if !System.is_running(app, current) {
            // A moment for the app to finish writing its files.
            std::thread::sleep(Duration::from_millis(500));
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(fail(format!("{} is still open, so it wasn't updated.", app.name), "it didn't quit within two minutes"))
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
    let cpu = macho::host_cpu().ok_or_else(|| unverified("this Mac's chip isn't one the suite builds for".into()))?;
    check_files(bundle, bundle, cpu).map_err(|problem| match problem {
        // The release has no code for this Mac's chip: on an Intel Mac, a release from before
        // the app went universal.
        Problem::NotForThisMac(details) if cpu == macho::CPU_X86_64 => {
            fail(format!("This version of {} needs an Apple silicon Mac.", app.name), details)
        }
        Problem::NotForThisMac(details) | Problem::Other(details) => unverified(details),
    })
}

enum Problem {
    /// A Mach-O with no slice for this Mac's chip.
    NotForThisMac(String),
    Other(String),
}

impl From<String> for Problem {
    fn from(text: String) -> Self {
        Problem::Other(text)
    }
}

/// Every Mach-O file has code for this Mac's chip (universal, or this chip's alone), and no
/// link points outside the bundle.
fn check_files(root: &Path, dir: &Path, cpu: u32) -> Result<(), Problem> {
    for entry in std::fs::read_dir(dir).map_err(|e| Problem::Other(e.to_string()))?.flatten() {
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| Problem::Other(e.to_string()))?;
        if kind.is_symlink() {
            let target = std::fs::read_link(&path).map_err(|e| Problem::Other(e.to_string()))?;
            let resolved = normalise(&path.parent().unwrap_or(root).join(target));
            if !resolved.starts_with(root) {
                return Err(Problem::Other(format!("{} links outside the app", path.display())));
            }
        } else if kind.is_dir() {
            check_files(root, &path, cpu)?;
        } else if kind.is_file() {
            let mut head = vec![0; macho::HEADER_BYTES];
            let n = std::fs::File::open(&path).and_then(|mut f| f.read(&mut head)).map_err(|e| Problem::Other(e.to_string()))?;
            let head = head.get(..n).unwrap_or_default();
            if !macho::runs_on(head, cpu) {
                return Err(Problem::NotForThisMac(format!("{} has no code for this Mac's chip ({:?})", path.display(), macho::classify(head))));
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

    // Room for the new copy beside the old one until the swap (spec §8).
    let needed = folder_size(&source);
    if let Some(free) = crate::free_space(dest_dir)
        && free < needed + needed / 10
    {
        return Err(crate::no_space(app, Some(needed), format!("{} free in {}", crate::megabytes(free), dest_dir.display())));
    }
    let direct = std::fs::create_dir(&staging).and_then(|()| std::fs::remove_dir(&staging)).is_ok();
    if direct {
        let copied = tool(DITTO, &[&source.to_string_lossy(), &staging.to_string_lossy()]);
        if !matches!(copied, Ok((true, _))) {
            let _ = std::fs::remove_dir_all(&staging);
            let details = format!("{copied:?}");
            if crate::net::is_disk_full(&details) {
                return Err(crate::no_space(app, Some(needed), details));
            }
            return Err(fail(format!("{} couldn't be copied. Nothing was changed.", app.name), details));
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

/// The bytes of every file in `dir`, links not followed.
fn folder_size(dir: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            match entry.file_type() {
                Ok(t) if t.is_dir() => stack.push(entry.path()),
                Ok(t) if t.is_file() => total += entry.metadata().map(|m| m.len()).unwrap_or(0),
                _ => {}
            }
        }
    }
    total
}

/// macOS's leftovers (spec §4.5): staging and set-aside copies in the Applications folders,
/// and the launcher's own mounts, when more than an hour old.
pub fn clean_up(dirs: &[PathBuf]) {
    let stale = crate::stale;
    for dir in dirs {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let ours = launcher_core::catalog::apps().iter().any(|a| name.starts_with(&format!(".{}.", a.bundle_name())));
            if ours && (name.contains(".installing-") || name.contains(".replaced-")) && stale(&entry.path()) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    let Some(cache) = System.cache() else { return };
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
    fn counts_a_folder_s_bytes() {
        let dir = std::env::temp_dir().join(format!("launcher-size-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Contents/MacOS")).unwrap();
        std::fs::write(dir.join("Contents/Info.plist"), [0u8; 100]).unwrap();
        std::fs::write(dir.join("Contents/MacOS/app"), [0u8; 1000]).unwrap();
        assert_eq!(folder_size(&dir), 1100);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn normalises_link_targets() {
        assert_eq!(normalise(Path::new("/A.app/Contents/Frameworks/../MacOS/x")), PathBuf::from("/A.app/Contents/MacOS/x"));
        assert!(!normalise(Path::new("/A.app/Contents/../../etc/passwd")).starts_with("/A.app"));
    }
}
