//! Repository tasks: `cargo xtask ci` runs every gate CI runs (format, lints, tests, crate
//! layers); `cargo xtask layers` checks only which crate may depend on which.

use std::path::Path;
use std::process::{Command, ExitCode};

/// Crates and the workspace crates each may depend on.
const LAYERS: [(&str, &[&str]); 4] = [
    ("launcher-core", &[]),
    ("launcher-sys", &["launcher-core"]),
    ("launcher-cli", &["launcher-core", "launcher-sys"]),
    ("method-launcher", &["launcher-core", "launcher-sys"]),
];
/// Dependencies only the desktop app may use.
const UI_ONLY: [&str; 3] = ["egui", "eframe", "winit"];

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let result = match task.as_str() {
        "ci" => ci(),
        "layers" => layers(),
        _ => Err("usage: cargo xtask <ci|layers>".to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap_or(Path::new("."))
}

/// Dependency names from a manifest's `[dependencies]` tables (a small TOML subset).
fn dependencies(manifest: &str) -> Vec<String> {
    let mut in_deps = false;
    let mut names = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]" || (line.starts_with("[target.") && line.ends_with(".dependencies]"));
            continue;
        }
        if in_deps && let Some((name, _)) = line.split_once('=') {
            names.push(name.trim().to_string());
        }
    }
    names
}

fn layers() -> Result<(), String> {
    let mut problems = Vec::new();
    for entry in std::fs::read_dir(root().join("crates")).map_err(|e| e.to_string())?.flatten() {
        let manifest = std::fs::read_to_string(entry.path().join("Cargo.toml")).map_err(|e| e.to_string())?;
        let name = manifest
            .lines()
            .find_map(|l| l.trim().strip_prefix("name = ").map(|n| n.trim_matches('"').to_string()))
            .ok_or("a crate manifest has no name")?;
        let Some((_, allowed)) = LAYERS.iter().find(|(crate_name, _)| *crate_name == name) else {
            problems.push(format!("{name} is not listed in xtask LAYERS"));
            continue;
        };
        for dep in dependencies(&manifest) {
            if dep.starts_with("launcher-") && !allowed.contains(&dep.as_str()) {
                problems.push(format!("{name} must not depend on {dep}"));
            }
            if UI_ONLY.contains(&dep.as_str()) && name != "method-launcher" {
                problems.push(format!("{name} must not depend on the UI crate {dep}"));
            }
        }
    }
    if problems.is_empty() {
        println!("layers: ok");
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

fn cargo(args: &[&str]) -> Result<(), String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo).args(args).current_dir(root()).status().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err(format!("cargo {} failed", args.join(" "))) }
}

fn ci() -> Result<(), String> {
    cargo(&["fmt", "--all", "--check"])?;
    cargo(&["clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"])?;
    cargo(&["test", "--locked", "--workspace"])?;
    layers()?;
    println!("ci: all gates passed");
    Ok(())
}
