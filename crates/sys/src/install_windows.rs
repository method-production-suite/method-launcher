//! Installing a suite app on Windows from its setup `.exe` (docs/spec.md §4.3): downloaded
//! from the latest release and checked against the release's SHA-256 checksums (Authenticode
//! once the suite signs Windows builds), then run as the signed-in person, with its own
//! window, and waited for. The launcher passes nothing that changes where or how it
//! installs. An update is the same: the setup upgrades in place. Afterwards the launcher looks
//! for the app again, so a setup closed early is reported, not assumed.

use crate::fetch::{fail, fetch_verified};
use crate::{InstallError, Stage, System, output};
use launcher_core::catalog::App;
use launcher_core::platform::{Installed, Kind, Launch, Locate};
use std::process::Command;
use std::time::Duration;

pub fn install(app: &App, current: Option<&Installed>, quit: bool, stage: &dyn Fn(Stage)) -> Result<Installed, InstallError> {
    if let Some(current) = current
        && System.is_running(app, current)
    {
        if !quit {
            return Err(fail(format!("Quit {} first, then try again.", app.name), "it is running"));
        }
        stage(Stage::Quitting);
        quit_app(app, current)?;
    }
    let fetched = fetch_verified(app, Kind::SetupExe, true, stage)?;
    // The setup needs its own name to run: keep the downloaded file's .exe ending.
    let setup = fetched.file.with_extension("exe");
    std::fs::rename(&fetched.file, &setup).map_err(|e| fail("The download couldn't be prepared.", e.to_string()))?;
    stage(Stage::Copying);
    let status = Command::new(&setup).status();
    let _ = std::fs::remove_file(&setup);
    let status = status.map_err(|e| fail(format!("The installer for {} couldn't be started.", app.name), e.to_string()))?;
    if !status.success() {
        return Err(fail("The installer stopped with an error.", format!("exit code {:?}", status.code())));
    }
    System.locate(app).ok_or_else(|| fail(format!("The installer closed before {} was installed.", app.name), "not found after the setup ended"))
}

/// Asks the running app to close its windows (taskkill without /F, as closing them would),
/// and waits up to two minutes.
fn quit_app(app: &App, current: &Installed) -> Result<(), InstallError> {
    let exe = current.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let taskkill = crate::env_path("SystemRoot").unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows")).join(r"System32\taskkill.exe");
    let _ = output(&taskkill.to_string_lossy(), &["/IM", &exe]);
    for _ in 0..240 {
        if !System.is_running(app, current) {
            std::thread::sleep(Duration::from_millis(500));
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(fail(format!("{} is still open, so it wasn't updated.", app.name), "it didn't close within two minutes"))
}
