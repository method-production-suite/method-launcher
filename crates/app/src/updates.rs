//! The launcher's own updates (docs/spec.md §5.2): Help > Check for Updates, the automatic
//! checks (2 minutes after launch, then every 6 hours, with the switch) and "Restart to
//! Update", through the suite's updater (`method-update`, SUITE-STANDARDS.md §1), the way the
//! apps update themselves. Macs only (Apple silicon and Intel), as the updater is; elsewhere the launcher
//! offers its new versions as a download.
//!
//! Releases are signed with the launcher's update key ([`PUBKEY`], key ID E115E2CD9AEE0DF9).
//! This is the one documented place its public half lives (docs/RELEASING.md points here); an
//! update whose signature does not check against it is never installed.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use method_update::policy::TICK;
use method_update::{AppConfig, Said, Status, Updater, baked};

/// Where this app looks for a new version.
const ENDPOINT: &str = "https://github.com/method-production-suite/method-launcher/releases/latest/download/latest.json";
/// The minisign public key the releases are signed with (key ID E115E2CD9AEE0DF9), as the
/// base64 of its key box. The private half lives with the release workflow's secrets and in
/// 1Password ("Method Launcher - Update signing key"); it is never in the repository.
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEUxMTVFMkNEOUFFRTBERjkKUldUNURlNmF6ZUlWNFYrRFNDaHNVOG5Wd203ZlV3bU0xeUF4dzdML0lRRUxEQjJ4QlJkTytETlMK";

/// The bundle as the installer names it.
pub const BUNDLE_NAME: &str = "Method Launcher.app";

/// The updater serves Macs (`darwin-aarch64` and, since method-libs#7, `darwin-x86_64`): any
/// other build never touches the network.
const MAC: bool = cfg!(all(target_os = "macos", any(target_arch = "aarch64", target_arch = "x86_64")));

/// What the menu says when this build cannot update itself (a version that is not semver, say).
pub const UPDATES_LATER: &str = "This build cannot update itself; download new versions from methodproduction.com";
/// Anywhere but a Mac.
pub const MAC_ONLY: &str = "Updates come through the launcher on Macs; elsewhere, download new versions from methodproduction.com";

pub fn update_config() -> AppConfig {
    AppConfig::new(
        crate::APP_NAME,
        BUNDLE_NAME,
        concat!("method-launcher-update/", env!("CARGO_PKG_VERSION")),
        // A rehearsal build may bake a local endpoint, a throwaway key and plain HTTP from
        // METHOD_LAUNCHER_UPDATE_ENDPOINT, _PUBKEY and _INSECURE; build.rs refuses them in any
        // other build.
        baked!(prefix = "METHOD_LAUNCHER_UPDATE", rehearsal = cfg!(feature = "rehearsal"), endpoint = ENDPOINT, pubkey = PUBKEY,),
    )
}

/// Puts back a bundle an interrupted update left hidden, before the window opens.
pub fn sweep_at_startup() {
    if MAC && let Ok(exe) = std::env::current_exe() {
        let swept = method_update::install::sweep_at_startup(&exe, &update_config().bundle);
        if swept.restored.is_some() || !swept.removed.is_empty() {
            eprintln!("update sweep: restored {:?}, removed {:?}", swept.restored, swept.removed);
        }
    }
}

/// The running app's updater and the answer to the last check the writer asked for.
pub struct Updates {
    updater: Arc<Updater>,
    said: Arc<Mutex<Option<Said>>>,
    asking: Arc<AtomicBool>,
}

impl Updates {
    /// `settings`: the `updates.json` file (`None` keeps the switch in memory, for tests).
    /// `initial_automatic`: the switch from older settings, used while nothing is stored yet.
    pub fn start(ctx: &egui::Context, settings: Option<PathBuf>, initial_automatic: bool) -> Self {
        let mut updater =
            Updater::new(update_config(), env!("CARGO_PKG_VERSION"), settings, Instant::now()).with_initial_automatic(initial_automatic);
        if let Some(bundle) = std::env::current_exe().ok().as_deref().and_then(method_update::install::bundle_of_exe) {
            updater = updater.with_bundle(bundle);
        }
        let updater = Arc::new(updater);
        // The schedule: a look every TICK, a check when one is due. A build that cannot
        // update itself starts no timer at all.
        if MAC && updater.unavailable().is_none() {
            let (u, ctx) = (updater.clone(), ctx.clone());
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(TICK);
                    if u.due(Instant::now()) {
                        u.check_now(false);
                        ctx.request_repaint();
                    }
                }
            });
        }
        Updates { updater, said: Arc::default(), asking: Arc::default() }
    }

    /// Why Check for Updates cannot run here, for the menu's tooltip; `None` when it can.
    pub fn unavailable(&self) -> Option<&'static str> {
        if !MAC {
            Some(MAC_ONLY)
        } else if self.updater.unavailable().is_some() {
            Some(UPDATES_LATER)
        } else {
            None
        }
    }

    pub fn automatic(&self) -> bool {
        self.updater.automatic()
    }

    pub fn set_automatic(&self, on: bool) -> std::io::Result<()> {
        self.updater.set_automatic(on)
    }

    /// Help > Check for Updates…: checks off the UI thread; the answer shows on the next frame.
    pub fn check(&self, ctx: &egui::Context) {
        if self.unavailable().is_some() || self.asking.swap(true, Ordering::SeqCst) {
            return;
        }
        let (u, said, asking, ctx) = (self.updater.clone(), self.said.clone(), self.asking.clone(), ctx.clone());
        std::thread::spawn(move || {
            let answer = u.check_now(true);
            if let Ok(mut slot) = said.lock() {
                *slot = answer;
            }
            asking.store(false, Ordering::SeqCst);
            ctx.request_repaint();
        });
    }

    /// What to show now: the phase, plus a one-time answer, handed over once. A failure is
    /// shown once: then the updater goes back to idle.
    pub fn status(&self) -> Status {
        let said = self.said.lock().ok().and_then(|mut slot| slot.take());
        let status = self.updater.status(said);
        if status.phase == "failed" {
            self.updater.dismiss();
        }
        status
    }

    /// "Restart to Update": swaps the bundle, starts the new one, and the caller exits.
    pub fn install_and_relaunch(&self) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let bundle = method_update::install::bundle_of_exe(&exe).ok_or("Method Launcher is not running from an app bundle")?;
        let names = self.updater.app().bundle.clone();
        let installed = self
            .updater
            .install_ready(&bundle, &|b: &Path, new: &Path| method_update::install::admin_swap(b, new, &names))
            .map_err(|e| install_failed_note(&e))?;
        method_update::restart::relaunch(&installed.bundle)
            .map_err(|e| format!("The update installed, but Method Launcher couldn't restart ({e}). Open it again."))
    }
}

/// What the window says when Restart to Update didn't install. A code signature that isn't
/// the suite's team (method-update checks it before the swap) won't pass on a second try, so
/// it never says to try again.
pub fn install_failed_note(error: &method_update::InstallError) -> String {
    use method_update::codesign::SignatureError;
    match error {
        method_update::InstallError::Signature(SignatureError::RunningUnreadable { .. }) => {
            "This copy of Method Launcher couldn't read its own signature, so the update wasn't installed. Download the new version from methodproduction.com.".to_string()
        }
        method_update::InstallError::Signature(_) => {
            "This update isn't signed by the suite's developer, so it wasn't installed. Method Launcher is unchanged.".to_string()
        }
        e => format!("The update didn't install ({e}). It is still ready: Restart to Update tries again."),
    }
}

/// A check that failed, in words, with what to do.
fn update_check_failed(detail: Option<&str>) -> String {
    match detail.map(str::trim).filter(|d| !d.is_empty()) {
        Some(detail) => format!("Couldn't check for updates ({detail}). Help > Check for Updates tries again."),
        None => "Couldn't check for updates. Help > Check for Updates tries again.".to_string(),
    }
}

/// The status line for an update status, or `None` when there is nothing to say.
pub fn status_line(status: &Status) -> Option<String> {
    let version = status.version.as_deref().unwrap_or("");
    match (status.phase, status.said) {
        (_, Some(Said::Current)) => Some(format!("{} {} is up to date.", crate::APP_NAME, env!("CARGO_PKG_VERSION"))),
        (_, Some(Said::ReadOnly)) | ("read-only", _) if status.manual || status.said.is_some() => status.message.clone(),
        (_, Some(Said::Failed)) | ("failed", _) => Some(update_check_failed(status.message.as_deref())),
        (_, Some(Said::InstallFailed)) => Some("The update did not install. It is still ready: Restart to Update tries again.".to_string()),
        ("checking", _) if status.manual => Some("Checking for updates…".into()),
        ("downloading", _) if status.manual => Some(format!("Downloading {version}…")),
        ("ready", _) => Some(format!("Update ready: Method Launcher {version}. Restart to update.")),
        ("installing", _) => Some(format!("Installing {version}…")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(phase: &'static str, said: Option<Said>) -> Status {
        Status { phase, version: Some("0.3.0".into()), manual: true, message: Some("offline".into()), said, automatic: true, available: true }
    }

    #[test]
    fn this_build_can_update_itself() {
        let config = update_config();
        assert_eq!(config.unavailable_reason(), None, "the key decodes and the endpoint is https");
        assert!(config.endpoint.starts_with("https://github.com/method-production-suite/method-launcher/"));
    }

    #[test]
    fn a_refused_signature_says_so_plainly() {
        use method_update::InstallError;
        use method_update::codesign::SignatureError;
        let other_team = InstallError::Signature(SignatureError::TeamMismatch { expected: "A".into(), found: Some("B".into()) });
        assert_eq!(
            install_failed_note(&other_team),
            "This update isn't signed by the suite's developer, so it wasn't installed. Method Launcher is unchanged."
        );
        let invalid = InstallError::Signature(SignatureError::Invalid { team: "A".into(), detail: "x".into() });
        assert!(!install_failed_note(&invalid).contains("tries again"), "a bad signature won't pass on a second try");
        let unreadable = InstallError::Signature(SignatureError::RunningUnreadable { detail: "x".into() });
        assert!(install_failed_note(&unreadable).ends_with("Download the new version from methodproduction.com."));
        let busy = InstallError::Io("Could not move it".into());
        assert_eq!(install_failed_note(&busy), "The update didn't install (Could not move it). It is still ready: Restart to Update tries again.");
    }

    /// The launcher builds its updater with `Updater::new` and a settings file, so the
    /// high-water mark is kept beside it: once a version's download has passed its signature
    /// check, a later offer of an older one is refused before anything is downloaded.
    #[test]
    fn an_older_offer_than_one_accepted_is_refused() {
        let dir = std::env::temp_dir().join(format!("launcher-highwater-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let settings = dir.join("updates.json");
        let offer = |version: &str| method_update::Update {
            current_version: "0.2.3".into(),
            version: version.into(),
            body: None,
            date: None,
            target: "darwin-aarch64".into(),
            download_url: "https://github.com/method-production-suite/method-launcher/releases/download/x".into(),
            signature: "s".into(),
            raw_json: serde_json::Value::Null,
        };
        let updater = || Updater::new(update_config(), "0.2.3", Some(settings.clone()), Instant::now());
        let ready = |version: &str| {
            let u = updater();
            u.check_with(true, || Ok(Some(offer(version))), |_| Ok(vec![1]));
            u.status(None).phase
        };
        // 0.3.0 is offered and its download verifies (it is never installed).
        assert_eq!(ready("0.3.0"), "ready");
        assert!(dir.join("updates.high-water.json").is_file(), "kept beside updates.json");
        // A later run, still on 0.2.3, is offered 0.2.9: refused, and nothing is downloaded.
        let later = updater();
        assert_eq!(later.check_with(true, || Ok(Some(offer("0.2.9"))), |_| -> Result<Vec<u8>, String> { panic!("downloaded") }), Some(Said::Failed));
        assert!(later.status(None).message.unwrap_or_default().contains("0.3.0"));
        // The same version again, or a newer one, still passes.
        assert_eq!(ready("0.3.0"), "ready");
        assert_eq!(ready("0.3.1"), "ready");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn status_lines() {
        assert_eq!(status_line(&status("idle", Some(Said::Current))).as_deref().map(|s| s.ends_with("is up to date.")), Some(true));
        assert_eq!(
            status_line(&status("failed", Some(Said::Failed))).as_deref(),
            Some("Couldn't check for updates (offline). Help > Check for Updates tries again.")
        );
        assert_eq!(status_line(&status("checking", None)).as_deref(), Some("Checking for updates…"));
        assert_eq!(status_line(&status("ready", None)).as_deref(), Some("Update ready: Method Launcher 0.3.0. Restart to update."));
        assert_eq!(status_line(&Status { manual: false, ..status("checking", None) }), None, "automatic checks are quiet");
        assert_eq!(status_line(&status("idle", None)), None);
    }
}
