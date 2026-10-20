//! The `.deb` route on Linux (docs/spec.md §4.4): when to take it, and what `pkexec apt-get`
//! answering means. Kept apart from `install_linux` so it is tested on every system.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use crate::InstallError;
use launcher_core::catalog::App;
use std::path::Path;

/// Whether an install goes through the system package: an update goes the way the app is
/// installed now (the package, unless it is the launcher's AppImage); a new install follows
/// Settings, when this system has apt.
pub fn use_deb(chosen: bool, apt: bool, current: Option<&Path>, appimage: Option<&Path>) -> bool {
    match current {
        Some(path) => apt && appimage.is_none_or(|a| a != path),
        None => chosen && apt,
    }
}

/// The privileged step, run by `pkexec /bin/sh -c INSTALL sh <download> <sha256>`. Everything
/// root installs is checked by root: the download is copied into a fresh root-owned folder
/// (mktemp makes it 0700), that copy is checked against the release's SHA-256, and apt
/// installs that copy. So a process of the person's own can't swap the file between the
/// launcher's check and the install. The paths and the hash arrive as arguments, never in the
/// script's text. Its own exit codes: 3, the copy doesn't match; 4, it couldn't be copied.
pub const INSTALL: &str = r#"set -eu
dir=$(/bin/mktemp -d /tmp/method-launcher-deb.XXXXXXXX)
trap '/bin/rm -rf "$dir"' EXIT
/bin/cp -- "$1" "$dir/package.deb" || exit 4
printf '%s  %s
' "$2" "$dir/package.deb" | /usr/bin/sha256sum --check --strict --quiet - || exit 3
/usr/bin/apt-get install -y -- "$dir/package.deb"
"#;

/// The same step installed by the launcher's own `.deb` (packaging/linux/install-deb), root's
/// file, with a polkit action that names it, so the password prompt says what it is for.
pub const HELPER: &str = "/usr/libexec/method-launcher/install-deb";

/// The arguments for `pkexec` that run [`INSTALL`] on `download`, which must match `sha256`:
/// through [`HELPER`] when the launcher was installed as a package (`helper` says so), else
/// through `/bin/sh -c`. `None` when the hash isn't one (64 hex digits), so nothing unchecked
/// ever reaches root.
pub fn pkexec_args(download: &Path, sha256: &str, helper: bool) -> Option<Vec<std::ffi::OsString>> {
    if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let (file, sha) = (download.as_os_str().to_owned(), sha256.to_ascii_lowercase().into());
    Some(if helper { vec![HELPER.into(), file, sha] } else { vec!["/bin/sh".into(), "-c".into(), INSTALL.into(), "sh".into(), file, sha] })
}

/// What the privileged step exiting with `code` means (spec §8): 126 and 127 are pkexec's own,
/// for a password prompt dismissed or refused; 3 and 4 are [`INSTALL`]'s.
pub fn result(app: &App, code: Option<i32>, stderr: &str) -> Result<(), InstallError> {
    match code {
        Some(0) => Ok(()),
        Some(3) => Err(InstallError::new(
            "The download doesn't match the release's checksum, so it was deleted. Try again.",
            "the copy the administrator step made didn't match the release's SHA-256",
        )),
        Some(4) => Err(InstallError::new(
            format!("{}'s system package couldn't be installed. Nothing was changed.", app.name),
            format!("the download couldn't be copied for installing: {}", stderr.trim()),
        )),
        Some(126 | 127) => Err(InstallError::new("Not installed. You can install the AppImage instead.", "the administrator prompt was cancelled")
            .with_link(
                "How to choose the AppImage",
                "https://github.com/method-production-suite/method-launcher/blob/main/docs/user-guide.md#settings",
            )),
        _ if crate::net::is_disk_full(stderr) => Err(crate::no_space(app, None, stderr.trim().to_string())),
        code => Err(InstallError::new(
            format!("{}'s system package couldn't be installed. Nothing was changed.", app.name),
            format!("apt-get exited with {code:?}: {}", stderr.trim()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> &'static App {
        launcher_core::catalog::apps().iter().find(|a| a.slug == "budget").expect("budget")
    }

    #[test]
    fn a_new_install_follows_settings_where_apt_is() {
        assert!(use_deb(true, true, None, None));
        assert!(!use_deb(true, false, None, None), "no apt, no package");
        assert!(!use_deb(false, true, None, None));
    }

    #[test]
    fn an_update_goes_the_way_the_app_is_installed() {
        let appimage = Path::new("/home/you/Applications/Method-Budget.AppImage");
        let package = Path::new("/usr/bin/method-budget-app");
        assert!(!use_deb(true, true, Some(appimage), Some(appimage)), "the AppImage stays an AppImage");
        assert!(use_deb(false, true, Some(package), Some(appimage)), "the package stays a package");
        assert!(!use_deb(true, false, Some(package), Some(appimage)));
    }

    #[test]
    fn root_checks_its_own_copy_before_apt_installs_it() {
        let sha = "a".repeat(64);
        let args = pkexec_args(Path::new("/home/you/.cache/x/budget.deb"), &sha, false).unwrap();
        assert_eq!(args[0], "/bin/sh");
        assert_eq!(args[1], "-c");
        assert_eq!(args[2], INSTALL);
        assert_eq!(args[4], "/home/you/.cache/x/budget.deb");
        assert_eq!(args[5], sha.as_str());
        // In this order: copy into root's folder, check that copy, install that copy.
        let copy = INSTALL.find("/bin/cp").unwrap();
        let check = INSTALL.find("sha256sum --check").unwrap();
        let install = INSTALL.find("apt-get install").unwrap();
        assert!(copy < check && check < install);
        assert!(INSTALL.contains(r#"apt-get install -y -- "$dir/package.deb""#), "apt installs the checked copy");
        assert!(!INSTALL.contains(r#"apt-get install -y -- "$1""#), "never the person's own file");
        // Nothing reaches root without a real hash.
        assert!(pkexec_args(Path::new("/x.deb"), "abc", false).is_none());
        assert!(pkexec_args(Path::new("/x.deb"), &format!("{}; rm -rf /", "a".repeat(52)), true).is_none());
    }

    #[test]
    fn the_packaged_helper_is_the_same_step() {
        let sha = "b".repeat(64);
        let args = pkexec_args(Path::new("/tmp/budget.deb"), &sha, true).unwrap();
        assert_eq!(args, vec![std::ffi::OsString::from(HELPER), "/tmp/budget.deb".into(), sha.into()]);
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/linux");
        let helper = std::fs::read_to_string(root.join("install-deb")).unwrap();
        assert!(helper.starts_with("#!/bin/sh\n"));
        assert!(helper.ends_with(INSTALL), "packaging/linux/install-deb must end with INSTALL, unchanged");
        let policy = std::fs::read_to_string(root.join("com.methodlauncher.desktop.install-deb.policy")).unwrap();
        assert!(policy.contains(&format!(r#"<annotate key="org.freedesktop.policykit.exec.path">{HELPER}</annotate>"#)));
        assert!(policy.contains("<allow_active>auth_admin</allow_active>"), "always a password, never kept");
    }

    /// The script itself, run unprivileged where `sh`, `mktemp`, `cp` and `sha256sum` are, with
    /// apt swapped for `true`: a matching copy passes, a swapped one stops with 3.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_script_refuses_a_swapped_file() {
        let dir = std::env::temp_dir().join(format!("launcher-deb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("budget.deb");
        std::fs::write(&file, b"package").unwrap();
        let good = crate::fetch::hash_file(&file).unwrap();
        let script = INSTALL.replace("/usr/bin/apt-get install -y --", "true");
        let run = |sha: &str| std::process::Command::new("/bin/sh").args(["-c", &script, "sh"]).arg(&file).arg(sha).status().unwrap().code();
        assert_eq!(run(&good), Some(0));
        std::fs::write(&file, b"swapped").unwrap();
        assert_eq!(run(&good), Some(3));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_what_pkexec_and_apt_say() {
        assert!(result(budget(), Some(0), "").is_ok());
        let cancelled = result(budget(), Some(126), "").unwrap_err();
        assert_eq!(cancelled.message, "Not installed. You can install the AppImage instead.");
        assert!(result(budget(), Some(127), "").is_err_and(|e| e.message.starts_with("Not installed.")));
        let failed = result(budget(), Some(100), "E: Unable to locate package").unwrap_err();
        assert_eq!(failed.message, "Method Budget's system package couldn't be installed. Nothing was changed.");
        assert!(failed.details.contains("Unable to locate"));
        let full =
            result(budget(), Some(100), "E: You don't have enough free space in /var/cache/apt/archives/. No space left on device").unwrap_err();
        assert!(full.message.starts_with("There isn't enough free space"), "{}", full.message);
    }
}
