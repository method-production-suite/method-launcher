//! scripts/check-crate-ages.py's exceptions, executed (SUITE-STANDARDS.md §6): `--release`
//! honours the RUSTSEC lines and an owner-approved override only when it names the release
//! being tagged (`--tag`), and release.yml passes that tag. `--print-exceptions` answers
//! without the network, so these run the script against scratch exception files. Skipped,
//! saying so, where no Python 3.11 or later is on PATH.
//!
//! A unit test, not one in `tests/`, for the reason given in `release_workflow.rs`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A Python that has tomllib (3.11+), or None.
fn python() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|p| Command::new(p).args(["-c", "import tomllib"]).output().is_ok_and(|o| o.status.success()))
}

fn scratch(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("launcher-crate-ages-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(name);
    std::fs::write(&file, text).unwrap();
    file
}

fn run(py: &str, exceptions: &Path, release: bool, tag: Option<&str>) -> Output {
    let mut c = Command::new(py);
    c.arg(root().join("scripts/check-crate-ages.py")).arg("--exceptions").arg(exceptions).arg("--print-exceptions");
    if release {
        c.arg("--release");
    }
    if let Some(tag) = tag {
        c.args(["--tag", tag]);
    }
    c.output().expect("python runs")
}

fn honoured(out: &Output) -> Vec<String> {
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect()
}

const BASE: &str =
    "# header\n\nrustls 0.23.46   # security fix for RUSTSEC-2026-0346\nparley 0.7.0     # cannot be pinned back; 14 days old on 2026-10-23\n";

#[test]
fn an_owner_approved_override_holds_for_its_named_release_only() {
    let Some(py) = python() else {
        eprintln!("skipped: no Python 3.11 or later on PATH");
        return;
    };
    let f = scratch("override.txt", &format!("{BASE}foo 1.2.3   # owner-approved override, the owner, 2026-10-27, release v0.2.2: \"Yes\"\n"));
    // Main and a rehearsal honour every line.
    assert_eq!(honoured(&run(py, &f, false, None)), ["foo 1.2.3", "parley 0.7.0", "rustls 0.23.46"]);
    // A release honours the override only when it is the release named; never the age-out.
    assert_eq!(honoured(&run(py, &f, true, Some("v0.2.2"))), ["foo 1.2.3", "rustls 0.23.46"]);
    for other in [Some("v0.2.3"), Some("v1.0.0"), Some(""), None] {
        assert_eq!(honoured(&run(py, &f, true, other)), ["rustls 0.23.46"], "{other:?}");
    }
}

#[test]
fn an_override_that_names_no_release_is_refused() {
    let Some(py) = python() else {
        eprintln!("skipped: no Python 3.11 or later on PATH");
        return;
    };
    let f = scratch("no-release.txt", &format!("{BASE}foo 1.2.3   # owner-approved override, the owner, 2026-10-27\n"));
    for release in [false, true] {
        let out = run(py, &f, release, Some("v0.2.2"));
        assert!(!out.status.success(), "passed with release={release}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("must name its release"));
    }
}

#[test]
fn the_committed_exceptions_parse() {
    let Some(py) = python() else {
        eprintln!("skipped: no Python 3.11 or later on PATH");
        return;
    };
    let f = root().join("scripts/crate-age-exceptions.txt");
    honoured(&run(py, &f, false, None));
    honoured(&run(py, &f, true, None));
}

#[test]
fn the_release_workflow_names_its_tag_to_the_crate_age_check() {
    let wf = std::fs::read_to_string(root().join(".github/workflows/release.yml")).unwrap();
    assert!(
        wf.contains("TAG: ${{ steps.tag.outputs.tag }}") && wf.contains("check-crate-ages.py --release --tag \"$TAG\""),
        "release.yml's --release run must pass the checked tag, or no override can hold"
    );
}
