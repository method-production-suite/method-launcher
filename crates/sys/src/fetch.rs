//! The first half of every install (docs/spec.md §4.1), on every system: pin the release from
//! where `releases/latest` redirects, so the checksums and the installer come from the same
//! tag; read the release's checksums; download the installer over GitHub-only https,
//! reporting progress; and check its SHA-256. On Windows and Linux a checksum is required (it
//! is the check); on macOS it is an extra one, the signature checks being what count.

use crate::net::{self, LARGE, NetError};
use crate::{InstallError, Stage, System};
use launcher_core::catalog::App;
use launcher_core::checksums::{self, Hasher};
use launcher_core::platform::Kind;
use launcher_core::release::tag_from_location;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// The app whose install can be cancelled now (one runs at a time), and whether it was.
static CANCELLABLE: Mutex<Option<String>> = Mutex::new(None);
static CANCELLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn cancel(slug: &str) {
    if CANCELLABLE.lock().is_ok_and(|now| now.as_deref() == Some(slug)) {
        CANCELLED.store(true, Ordering::SeqCst);
    }
}

fn cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

/// While alive, `slug`'s install can be cancelled; after, it can't.
struct Cancellable;

impl Cancellable {
    fn new(slug: &str) -> Self {
        if let Ok(mut now) = CANCELLABLE.lock() {
            *now = Some(slug.to_string());
        }
        CANCELLED.store(false, Ordering::SeqCst);
        Cancellable
    }

    fn check(&self) -> Result<(), InstallError> {
        if cancelled() { Err(InstallError::cancelled()) } else { Ok(()) }
    }
}

impl Drop for Cancellable {
    fn drop(&mut self) {
        if let Ok(mut now) = CANCELLABLE.lock() {
            *now = None;
        }
        CANCELLED.store(false, Ordering::SeqCst);
    }
}

pub(crate) fn fail(message: impl Into<String>, details: impl Into<String>) -> InstallError {
    InstallError::new(message, details)
}

/// The sentence for a failed request (spec §8). A missing file is the release having no
/// download for this system.
pub(crate) fn net_fail(app: &App, e: &NetError) -> InstallError {
    let details = e.to_string();
    match e {
        NetError::Cancelled => InstallError::cancelled(),
        NetError::Offline(_) => fail("You're offline. Connect to the internet and try again.", details),
        NetError::NotFound => fail(format!("This release of {} has no download for {}.", app.name, crate::system_name()), details),
        NetError::Redirected(_) => fail("The download came from somewhere unexpected, so it was stopped.", details),
        NetError::Stopped(_) => fail("The download stopped. Try again.", details),
        NetError::NoSpace(_) => crate::no_space(app, None, details),
        NetError::Local(_) => fail(format!("{} couldn't be downloaded.", app.name), details),
        NetError::Unreachable(_) | NetError::Status(_) => fail("GitHub didn't answer. Try again in a few minutes.", details),
    }
}

/// The release's page, for a message that sends the person there.
pub(crate) fn release_page(app: &App, tag: &str) -> String {
    app.asset_url(tag, "").replace("/releases/download/", "/releases/tag/").trim_end_matches('/').to_string()
}

fn scratch(app: &App, name: &str) -> Result<PathBuf, InstallError> {
    System.scratch(&format!("{}-{name}", app.slug)).ok_or_else(|| fail("The launcher has no cache folder.", ""))
}

/// A downloaded installer that passed its checksum (when there was one), with the release it
/// came from. The file is deleted when this is dropped.
pub struct Fetched {
    pub file: PathBuf,
    pub tag: String,
}

impl Drop for Fetched {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.file);
    }
}

/// Downloads `app`'s installer of `kind` from its latest release and checks it. With
/// `require_checksum`, a release that doesn't list the installer in its checksums can't be
/// installed from here (spec §4.1 step 2).
pub fn fetch_verified(app: &App, kind: Kind, require_checksum: bool, stage: &dyn Fn(Stage)) -> Result<Fetched, InstallError> {
    let file = app.installer(kind).ok_or_else(|| fail(format!("{} has no download yet.", app.name), "no release"))?;
    let cancellable = Cancellable::new(app.slug);

    stage(Stage::Finding);
    let location = net::redirect_of(&app.latest_url(), &scratch(app, "redirect")?).map_err(|e| match e {
        NetError::NotFound => fail(format!("{} has no release yet.", app.name), e.to_string()),
        e => net_fail(app, &e),
    })?;
    let tag = tag_from_location(&location).ok_or_else(|| fail("The latest release isn't what we expected.", location.clone()))?;

    let mut expected = None;
    for name in checksums::FILES {
        match net::fetch_text(&app.asset_url(&tag, name), &scratch(app, name)?) {
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
    cancellable.check()?;
    if require_checksum && expected.is_none() {
        return Err(fail("This release can't be checked yet, so it can't be installed from here.", format!("{tag} lists no checksum for {file}"))
            .with_link("Open the release page", release_page(app, &tag)));
    }

    let fetched = Fetched { file: scratch(app, &file)?, tag: tag.clone() };
    let url = app.asset_url(&tag, &file);
    let downloaded = {
        let target = fetched.file.clone();
        let worker = std::thread::spawn(move || net::download_until(&url, &target, LARGE, &cancelled));
        while !worker.is_finished() {
            stage(Stage::Downloading(std::fs::metadata(&fetched.file).map(|m| m.len()).unwrap_or(0)));
            std::thread::sleep(Duration::from_millis(250));
        }
        worker.join().unwrap_or_else(|_| Err(NetError::Local("the download stopped unexpectedly".into())))
    };
    downloaded.map_err(|e| net_fail(app, &e))?;

    cancellable.check()?;
    stage(Stage::Checking);
    if let Some(expected) = &expected {
        let actual = hash_file(&fetched.file).map_err(|e| fail("The download couldn't be read.", e))?;
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(fail(
                "The download doesn't match the release's checksum, so it was deleted. Try again.",
                format!("expected {expected}, got {actual}"),
            ));
        }
    }
    cancellable.check()?;
    Ok(fetched)
}

pub(crate) fn hash_file(path: &Path) -> Result<String, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> &'static App {
        launcher_core::catalog::apps().iter().find(|a| a.slug == "budget").expect("budget")
    }

    #[test]
    fn every_request_failure_has_spec_8_s_sentence() {
        let says = |e: NetError| net_fail(budget(), &e).message;
        assert_eq!(says(NetError::Offline("x".into())), "You're offline. Connect to the internet and try again.");
        assert_eq!(says(NetError::Unreachable("x".into())), "GitHub didn't answer. Try again in a few minutes.");
        assert_eq!(says(NetError::Status(502)), "GitHub didn't answer. Try again in a few minutes.");
        assert_eq!(says(NetError::NotFound), format!("This release of Method Budget has no download for {}.", crate::system_name()));
        assert_eq!(says(NetError::Redirected("https://example.com".into())), "The download came from somewhere unexpected, so it was stopped.");
        assert_eq!(says(NetError::Stopped("slow".into())), "The download stopped. Try again.");
        assert_eq!(says(NetError::NoSpace("full".into())), "There isn't enough free space to install Method Budget.");
    }

    #[test]
    fn only_the_install_running_can_be_cancelled() {
        let running = Cancellable::new("budget");
        cancel("schedule");
        assert!(running.check().is_ok());
        cancel("budget");
        assert!(running.check().is_err_and(|e| e.cancelled));
        drop(running);
        cancel("budget");
        assert!(!cancelled(), "nothing to cancel once it is over");
        assert!(net_fail(budget(), &NetError::Cancelled).cancelled);
    }

    #[test]
    fn the_details_keep_the_technical_reason() {
        let e = net_fail(budget(), &NetError::Status(503));
        assert!(e.details.contains("503"), "{}", e.details);
    }

    #[test]
    fn the_release_page_is_the_tag_s_page() {
        assert_eq!(release_page(budget(), "v0.5.0"), "https://github.com/method-production-suite/method-budget/releases/tag/v0.5.0");
    }
}
