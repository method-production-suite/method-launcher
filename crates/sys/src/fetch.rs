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
use std::time::Duration;

pub(crate) fn fail(message: impl Into<String>, details: impl Into<String>) -> InstallError {
    InstallError::new(message, details)
}

pub(crate) fn net_fail(app: &App, e: &NetError) -> InstallError {
    match e {
        NetError::NotFound => fail(format!("{} has no download for this computer yet.", app.name), e.to_string()),
        NetError::Redirected(_) => fail("The download came from somewhere unexpected, so it was stopped.", e.to_string()),
        NetError::Stopped(_) => fail("The download stopped. Try again.", e.to_string()),
        _ => fail("GitHub didn't answer. Check the connection and try again.", e.to_string()),
    }
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

    stage(Stage::Finding);
    let location = net::redirect_of(&app.latest_url(), &scratch(app, "redirect")?).map_err(|e| net_fail(app, &e))?;
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
    if require_checksum && expected.is_none() {
        return Err(fail("This release can't be checked yet, so it can't be installed from here.", format!("{tag} lists no checksum for {file}")));
    }

    let fetched = Fetched { file: scratch(app, &file)?, tag: tag.clone() };
    let url = app.asset_url(&tag, &file);
    let downloaded = {
        let target = fetched.file.clone();
        let worker = std::thread::spawn(move || net::download(&url, &target, LARGE));
        while !worker.is_finished() {
            stage(Stage::Downloading(std::fs::metadata(&fetched.file).map(|m| m.len()).unwrap_or(0)));
            std::thread::sleep(Duration::from_millis(250));
        }
        worker.join().unwrap_or_else(|_| Err(NetError::Local("the download stopped unexpectedly".into())))
    };
    downloaded.map_err(|e| net_fail(app, &e))?;

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
