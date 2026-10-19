//! `method-launcher`: the launcher's command line, for scripts and headless tests
//! (docs/spec.md §11). Today: `list` and `--version`; `status`, `verify` and `install` follow
//! (docs/ROADMAP.md).

use launcher_core::catalog::APPS;
use launcher_core::platform::Target;
use std::process::ExitCode;

const USAGE: &str = "usage: method-launcher <list|--version|--help>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            println!("method-launcher {} (Method Launcher)", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("list") => {
            let target = Target::current();
            for app in &APPS {
                let installers: Vec<String> = target.map(|t| t.kinds().iter().filter_map(|&k| app.installer(k)).collect()).unwrap_or_default();
                let download = if !app.released {
                    "no release yet".to_string()
                } else if installers.is_empty() {
                    "no build for this system yet".to_string()
                } else {
                    installers.join(", ")
                };
                println!("{:<12} {:<20} {:<32} {}", app.slug, app.name, app.bundle_id, download);
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
