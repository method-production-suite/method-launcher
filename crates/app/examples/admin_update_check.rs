//! The hand test of the launcher's own update through an administrator (docs/ROADMAP.md,
//! "Tests that need the owner's hands"). It runs the same method-update calls Restart to
//! Update runs, from a Terminal, on a copy of the launcher in a folder only root can write, so
//! the system's password prompt is the only thing on screen. macOS only; nothing here ships.
//!
//!   install <installed.app> <update.app.tar.gz>
//!       The whole path, as Restart to Update takes it: unpack, check the new copy's code
//!       signature against the installed copy's team, then the administrator step (root takes
//!       its own copy, checks it again, and swaps it in).
//!   root-only <installed.app> <unpacked new.app>
//!       The administrator step alone, so root's own check can be shown refusing a copy
//!       whose signature is broken (the full path would refuse it earlier, before any prompt).
//!
//! Prints what happened and exits 0 when the update went in, 1 when it was refused.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let names = method_update::BundleNames::new("Method Launcher.app");
    let result = match args.as_slice() {
        [mode, installed, archive] if mode == "install" => match std::fs::read(archive) {
            Ok(bytes) => method_update::install::install_app(&bytes, Path::new(installed), &names, &|bundle: &Path, new: &Path| {
                method_update::install::admin_swap(bundle, new, &names)
            })
            .map(|done| done.bundle.display().to_string()),
            Err(e) => {
                eprintln!("admin-update-check: {archive} couldn't be read: {e}");
                return ExitCode::from(2);
            }
        },
        [mode, installed, new] if mode == "root-only" => {
            method_update::install::admin_swap(Path::new(installed), Path::new(new), &names).map(|()| installed.clone())
        }
        _ => {
            eprintln!("usage: admin_update_check install <installed.app> <update.app.tar.gz>");
            eprintln!("       admin_update_check root-only <installed.app> <unpacked new.app>");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(at) => {
            println!("UPDATED: {at}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("REFUSED: {e}");
            println!("({e:?})");
            ExitCode::FAILURE
        }
    }
}
