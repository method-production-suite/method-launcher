//! scripts/check-crate-ages.py's exceptions, executed (SUITE-STANDARDS.md §6). The script is
//! the suite's shared one, copied verbatim from method-libs; these tests pin the rules this
//! repository's release depends on: `--release` honours the RUSTSEC lines and an
//! owner-approved override only when it names the release being tagged (`--tag`), and
//! release.yml passes that tag. They drive the script's own `check()` with a registry that
//! calls every listed version young, so nothing touches the network. Skipped, saying so,
//! where no Python 3.11 or later is on PATH.
//!
//! A unit test, not one in `tests/`, for the reason given in `release_workflow.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A Python that has tomllib (3.11+), or None.
fn python() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|p| Command::new(p).args(["-I", "-c", "import tomllib"]).output().is_ok_and(|o| o.status.success()))
}

fn scratch(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("launcher-crate-ages-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(name);
    std::fs::write(&file, text).unwrap();
    file
}

/// Loads the script, locks every listed version as young, and prints the ones `check()`
/// lets through, sorted.
const HONOURED: &str = r#"
import sys, datetime as dt, importlib.util
spec = importlib.util.spec_from_file_location("check_crate_ages", sys.argv[1])
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
exceptions = m.read_exceptions(sys.argv[2])
release = sys.argv[3] == "release"
tag = None if sys.argv[4] == "-" else sys.argv[4]
now = dt.datetime(2026, 11, 1, tzinfo=dt.timezone.utc)
class Young:
    def published(self, name, version, cutoff):
        return now, False
    def index(self, name):
        return {}
pkgs = {(n, v, m.CRATES_IO) for n, v in exceptions}
bad, _, _ = m.check(pkgs, Young(), now=now, exceptions=exceptions, release=release, tag=tag, out=lambda line: None)
for n, v in sorted(exceptions):
    if f"{n} {v}" not in bad:
        print(n, v)
"#;

fn honoured(py: &str, exceptions: &Path, release: bool, tag: Option<&str>) -> Vec<String> {
    // -B: loading the script as a module must not leave a __pycache__ in the repository.
    let out = Command::new(py)
        .arg("-I")
        .arg("-B")
        .arg("-c")
        .arg(HONOURED)
        .arg(root().join("scripts/check-crate-ages.py"))
        .arg(exceptions)
        .arg(if release { "release" } else { "main" })
        .arg(tag.unwrap_or("-"))
        .output()
        .expect("python runs");
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
    assert_eq!(honoured(py, &f, false, None), ["foo 1.2.3", "parley 0.7.0", "rustls 0.23.46"]);
    // A release honours the override only when it is the release named; never the age-out.
    assert_eq!(honoured(py, &f, true, Some("v0.2.2")), ["foo 1.2.3", "rustls 0.23.46"]);
    for other in [Some("v0.2.3"), Some("v1.0.0"), Some(""), None] {
        assert_eq!(honoured(py, &f, true, other), ["rustls 0.23.46"], "{other:?}");
    }
}

#[test]
fn an_override_for_a_prerelease_is_not_the_release() {
    let Some(py) = python() else {
        eprintln!("skipped: no Python 3.11 or later on PATH");
        return;
    };
    let f = scratch("prerelease.txt", &format!("{BASE}foo 1.2.3   # owner-approved override, the owner, 2026-10-27, release v0.2.2-rc1\n"));
    assert_eq!(honoured(py, &f, true, Some("v0.2.2")), ["rustls 0.23.46"]);
    assert_eq!(honoured(py, &f, true, Some("v0.2.2-rc1")), ["foo 1.2.3", "rustls 0.23.46"]);
}

#[test]
fn an_override_that_names_no_release_never_ships() {
    let Some(py) = python() else {
        eprintln!("skipped: no Python 3.11 or later on PATH");
        return;
    };
    // No release at all, or a four-part version that only starts like v0.2.2: an age-out
    // line, which main honours and no release does.
    for (file, named) in [("no-release.txt", ""), ("four-part.txt", ", release v0.2.2.1")] {
        let f = scratch(file, &format!("{BASE}foo 1.2.3   # owner-approved override, the owner, 2026-10-27{named}\n"));
        assert_eq!(honoured(py, &f, false, None), ["foo 1.2.3", "parley 0.7.0", "rustls 0.23.46"], "{file}");
        assert_eq!(honoured(py, &f, true, Some("v0.2.2")), ["rustls 0.23.46"], "{file}");
    }
}

#[test]
fn the_committed_exceptions_parse() {
    let Some(py) = python() else {
        eprintln!("skipped: no Python 3.11 or later on PATH");
        return;
    };
    let f = root().join("scripts/crate-age-exceptions.txt");
    honoured(py, &f, false, None);
    honoured(py, &f, true, None);
}

#[test]
fn the_git_allowlist_names_only_method_libs() {
    let list = std::fs::read_to_string(root().join("scripts/crate-age-git-allow.txt")).unwrap();
    let urls: Vec<&str> = list.lines().filter_map(|l| l.split('#').next()).map(str::trim).filter(|l| !l.is_empty()).collect();
    assert_eq!(urls, ["https://github.com/method-production-suite/method-libs"]);
}

#[test]
fn the_release_workflow_names_its_tag_to_the_crate_age_check() {
    let wf = std::fs::read_to_string(root().join(".github/workflows/release.yml")).unwrap();
    assert!(
        wf.contains("TAG: ${{ steps.tag.outputs.tag }}") && wf.contains("check-crate-ages.py --release --tag \"$TAG\""),
        "release.yml's --release run must pass the checked tag, or no override can hold"
    );
}
