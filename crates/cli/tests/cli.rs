//! The command line's answers that need no network and change nothing on this computer:
//! `--version`, `--help`, `list`, and the refusals for a wrong command or missing argument.
//! `catalog`, `latest` and `install` read the network and `status` this computer's apps, so
//! they are left to the weekly build's real runs and the release check.

#![allow(clippy::expect_used, reason = "a test that can't start the command line should stop there")]

use launcher_core::catalog::BUILT_IN;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_method-launcher")).args(args).output().expect("the command line runs")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn version_names_the_launcher_and_this_version() {
    for flag in ["--version", "-V"] {
        let out = run(&[flag]);
        assert!(out.status.success());
        assert_eq!(stdout(&out).trim(), format!("method-launcher {} (Method Launcher)", env!("CARGO_PKG_VERSION")));
    }
}

#[test]
fn help_and_no_command_print_the_usage() {
    for args in [&["--help"][..], &["-h"], &[]] {
        let out = run(args);
        assert!(out.status.success(), "{args:?}");
        let text = stdout(&out);
        assert!(text.starts_with("usage: method-launcher "), "{args:?}: {text}");
        for command in ["list", "status", "latest", "install <app>", "verify <app>"] {
            assert!(text.contains(command), "{args:?} lacks {command}");
        }
    }
}

#[test]
fn an_unknown_command_is_refused_with_the_usage() {
    let out = run(&["frobnicate"]);
    assert!(!out.status.success());
    assert!(stdout(&out).is_empty());
    let text = stderr(&out);
    assert!(text.contains("unknown command frobnicate") && text.contains("usage: method-launcher"), "{text}");
}

#[test]
fn list_shows_every_built_in_app_in_order() {
    let out = run(&["list"]);
    assert!(out.status.success());
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), BUILT_IN.len(), "{text}");
    for (line, app) in lines.iter().zip(&BUILT_IN) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(fields.first(), Some(&app.slug), "{line}");
        assert!(line.contains(app.name) && line.contains(app.bundle_id), "{line}");
    }
}

#[test]
fn install_refuses_without_an_app_it_knows() {
    for args in [&["install"][..], &["install", "nonesuch"]] {
        let out = run(args);
        assert!(!out.status.success(), "{args:?}");
        let text = stderr(&out);
        assert!(text.contains("name an app"), "{args:?}: {text}");
        for app in &BUILT_IN {
            assert!(text.contains(app.slug), "{args:?} lacks {}", app.slug);
        }
    }
}

#[test]
fn install_refuses_arguments_it_does_not_take() {
    for args in [&["install", "reader", "--to"][..], &["install", "reader", "--into", "/tmp"], &["install", "reader", "--to", "/tmp", "extra"]] {
        let out = run(args);
        assert!(!out.status.success(), "{args:?}");
        assert!(stderr(&out).contains("usage: method-launcher"), "{args:?}");
    }
}

#[test]
fn verify_refuses_without_an_app_and_a_path() {
    for args in [&["verify"][..], &["verify", "reader"], &["verify", "nonesuch", "/nowhere.app"]] {
        let out = run(args);
        assert!(!out.status.success(), "{args:?}");
        assert!(!stderr(&out).is_empty(), "{args:?}");
    }
}
