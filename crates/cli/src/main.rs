//! `method-launcher`: the launcher's command line, for scripts and headless tests
//! (docs/spec.md §12). `list` shows the catalog and what this system can install; `status`
//! shows which apps are installed here, where, and at which version.

use launcher_core::catalog::{APPS, App};
use launcher_core::platform::{Locate, Target};
use launcher_sys::System;
use std::process::ExitCode;

const USAGE: &str = "usage: method-launcher <list|status|--version|--help>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            println!("method-launcher {} (Method Launcher)", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("list") => {
            for app in &APPS {
                println!("{:<12} {:<20} {:<32} {}", app.slug, app.name, app.bundle_id, downloads(app));
            }
            ExitCode::SUCCESS
        }
        Some("status") => {
            let system = System;
            for app in &APPS {
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
