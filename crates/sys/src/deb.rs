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

/// What `pkexec apt-get install` exiting with `code` means (spec §8): 126 and 127 are pkexec's
/// own, for a password prompt dismissed or refused.
pub fn result(app: &App, code: Option<i32>, stderr: &str) -> Result<(), InstallError> {
    match code {
        Some(0) => Ok(()),
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
