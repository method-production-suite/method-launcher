//! `method-launcher`: the launcher's command line, for scripts and headless tests
//! (docs/spec.md §11). Today: `list` and `--version`; `status`, `verify` and `install` follow
//! (docs/ROADMAP.md).

use launcher_core::catalog::APPS;
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
            for app in &APPS {
                let download = app.download.unwrap_or("no download yet");
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
