//! `method-launcher`: the launcher's command line, for scripts and headless tests
//! (docs/spec.md §12). `list` shows the catalog and what this system can install; `status`
//! shows which apps are installed here, where, and at which version.

use launcher_core::catalog::{App, apps};
use launcher_core::platform::{Locate, Target};
use launcher_sys::System;
use std::process::ExitCode;

const USAGE: &str = "usage: method-launcher <list|catalog|status|latest|--version|--help>
       method-launcher install <app> [--to <folder>]   (install or update from the latest release; --to is macOS only)
       method-launcher verify <app> <path.app>          (macOS: the checks an install makes)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            println!("method-launcher {} (Method Launcher)", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("catalog") => {
            // The website's list, as the window reads it (spec §9).
            match System.website_catalog() {
                Some((_, apps)) => {
                    for app in &apps {
                        println!("{:<12} {:<20} {:<32} {}", app.slug, app.name, app.bundle_id, app.repo);
                    }
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!("method-launcher: the website's list couldn't be read or didn't pass its checks; the built-in list is used");
                    ExitCode::FAILURE
                }
            }
        }
        Some("list") => {
            for app in apps() {
                println!("{:<12} {:<20} {:<32} {}", app.slug, app.name, app.bundle_id, downloads(app));
            }
            ExitCode::SUCCESS
        }
        Some("status") => {
            let system = System;
            for app in apps() {
                let state = match system.locate(app) {
                    Some(found) => {
                        let version = if found.version.is_empty() { "version unknown".to_string() } else { found.version };
                        let earlier = if found.legacy { ", under its earlier name" } else { "" };
                        format!("installed {version}{earlier} at {}", found.path.display())
                    }
                    None => "not installed".to_string(),
                };
                println!("{:<12} {state}", app.slug);
            }
            ExitCode::SUCCESS
        }
        Some("latest") => {
            for app in apps() {
                match System.latest_release(app) {
                    Ok(Some((release, has_installer))) => {
                        let date = release.date.map(launcher_core::release::short_date).unwrap_or_default();
                        let here = if has_installer { "" } else { " (no installer for this system)" };
                        println!("{:<12} {:<8} {date:<12} {}{here}", app.slug, release.version, release.summary.unwrap_or_default());
                    }
                    Ok(None) => println!("{:<12} no release yet", app.slug),
                    Err(e) => println!("{:<12} couldn't check: {e}", app.slug),
                }
            }
            ExitCode::SUCCESS
        }
        Some("install") => install(args.get(1..).unwrap_or_default()),
        Some("verify") => verify(args.get(1..).unwrap_or_default()),
        Some("--help" | "-h") | None => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("method-launcher: unknown command {other}\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// The installers this system can use, or why there are none.
fn downloads(app: &App) -> String {
    let installers: Vec<String> = Target::current().map(|t| t.kinds().iter().filter_map(|&k| app.installer(k)).collect()).unwrap_or_default();
    if !app.released {
        "no release yet".to_string()
    } else if installers.is_empty() {
        "no build for this system yet".to_string()
    } else {
        installers.join(", ")
    }
}

/// `install <app> [--to <folder>]`: the window's Install and Update, without a window. The
/// folder is for macOS (a scratch folder for testing); elsewhere the app goes where its
/// installer puts it.
fn install(args: &[String]) -> ExitCode {
    use launcher_sys::Stage;
    let Some(app) = args.first().and_then(|slug| launcher_core::catalog::by_slug(slug)) else {
        eprintln!("method-launcher install: name an app: {}", apps().iter().map(|a| a.slug).collect::<Vec<_>>().join(", "));
        return ExitCode::FAILURE;
    };
    let folder = match args.get(1..) {
        Some([flag, folder]) if flag == "--to" => Some(std::path::PathBuf::from(folder)),
        Some([]) => None,
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let last = std::cell::Cell::new(0u64);
    let report = |stage: Stage| match stage {
        Stage::Downloading(bytes) if bytes / 10_000_000 != last.get() / 10_000_000 => {
            last.set(bytes);
            eprintln!("  downloading: {} MB", bytes / 1_000_000);
        }
        Stage::Downloading(_) => {}
        other => eprintln!("  {other:?}"),
    };
    let found = System.locate(app);
    let result = match folder {
        #[cfg(target_os = "macos")]
        Some(dest) => {
            // Replace the installed copy only when installing into the folder it is in.
            let current = found.filter(|f| f.path.parent() == Some(dest.as_path()));
            launcher_sys::install_macos::install(app, &dest, current.as_ref(), false, &report)
        }
        #[cfg(not(target_os = "macos"))]
        Some(_) => {
            eprintln!("method-launcher install: --to is for macOS only");
            return ExitCode::FAILURE;
        }
        None => System.install(app, found.as_ref(), false, None, &report),
    };
    match result {
        Ok(found) => {
            println!("installed {} {} at {}", app.name, found.version, found.path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("method-launcher: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `verify <app> <path.app>`: the checks an install makes on a bundle (spec §4.2 step 2).
#[cfg(target_os = "macos")]
fn verify(args: &[String]) -> ExitCode {
    let (Some(app), Some(path)) = (args.first().and_then(|slug| launcher_core::catalog::by_slug(slug)), args.get(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    match launcher_sys::install_macos::verify_bundle(app, std::path::Path::new(path)) {
        Ok(()) => {
            println!("verified: {path} is signed by the suite, notarised and built for Apple silicon only");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("method-launcher: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn verify(_args: &[String]) -> ExitCode {
    eprintln!("method-launcher verify: macOS only for now");
    ExitCode::FAILURE
}
