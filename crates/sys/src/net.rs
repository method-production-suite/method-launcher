//! Fetching from GitHub (docs/spec.md §4.1, §4.2) with the system's own `curl`, which ships
//! with macOS, Windows 10 and later, and Linux. The launcher follows redirects itself, one
//! at a time, so every hop is checked against `launcher_core::net`'s allow-list; `curl`
//! itself never follows one and accepts only https.

use launcher_core::net::is_allowed;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// At most this many redirects per request.
const MAX_REDIRECTS: usize = 6;

/// Small files (manifests, checksums): at most 1 MiB.
pub const SMALL: u64 = 1 << 20;

/// Installers: at most 1 GiB (spec §4.2).
pub const LARGE: u64 = 1 << 30;

/// Why a request failed, in words a person can be shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetError {
    /// curl couldn't connect, or timed out.
    Unreachable(String),
    /// The server answered 404.
    NotFound,
    /// Any other status.
    Status(u16),
    /// A redirect to a host not on the list, or not https.
    Redirected(String),
    /// Over the size limit, or the download stalled.
    Stopped(String),
    /// curl isn't there, or a local file couldn't be written.
    Local(String),
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::Unreachable(e) => write!(f, "GitHub didn't answer. Check the connection and try again. ({e})"),
            NetError::NotFound => write!(f, "The release has no such file."),
            NetError::Status(code) => write!(f, "GitHub answered {code}. Try again in a few minutes."),
            NetError::Redirected(url) => write!(f, "The download came from somewhere unexpected, so it was stopped. ({url})"),
            NetError::Stopped(e) => write!(f, "The download stopped. Try again. ({e})"),
            NetError::Local(e) => write!(f, "{e}"),
        }
    }
}

fn curl() -> PathBuf {
    #[cfg(target_os = "macos")]
    return PathBuf::from("/usr/bin/curl");
    #[cfg(target_os = "windows")]
    return crate::env_path("SystemRoot").unwrap_or_else(|| PathBuf::from(r"C:\Windows")).join(r"System32\curl.exe");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return PathBuf::from("curl");
}

/// One request, no redirects followed: the status and the redirect target, with the body
/// written to `dest` (nothing written for a redirect).
fn once(url: &str, dest: &Path, max: u64) -> Result<(u16, String), NetError> {
    let mut command = Command::new(curl());
    command
        .args(["--silent", "--show-error", "--proto", "=https", "--proto-redir", "=https"])
        .args(["--connect-timeout", "20", "--speed-limit", "1", "--speed-time", "60"])
        .args(["--max-filesize", &max.to_string()])
        .args(["--user-agent", concat!("method-launcher/", env!("CARGO_PKG_VERSION"))])
        .arg("--output")
        .arg(dest)
        .args(["--write-out", "%{http_code} %{redirect_url}", "--", url])
        .stdin(Stdio::null());
    crate::quiet(&mut command);
    let out = command.output().map_err(|e| NetError::Local(format!("curl couldn't run: {e}")))?;
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    match out.status.code() {
        Some(0) => {}
        // 63: over --max-filesize; 28: timed out or too slow; 23: couldn't write.
        Some(63) => return Err(NetError::Stopped("the file is larger than the launcher allows".into())),
        Some(28) => return Err(NetError::Stopped(stderr)),
        Some(23) => return Err(NetError::Local(format!("The download couldn't be saved. {stderr}"))),
        _ => return Err(NetError::Unreachable(stderr)),
    }
    let written = String::from_utf8_lossy(&out.stdout).into_owned();
    let (code, redirect) = written.split_once(' ').unwrap_or((written.as_str(), ""));
    let code = code.trim().parse().map_err(|_| NetError::Unreachable(format!("unexpected answer {written:?}")))?;
    Ok((code, redirect.trim().to_string()))
}

/// The redirect a URL answers with, without following it (`releases/latest` names its tag
/// this way).
pub fn redirect_of(url: &str, scratch: &Path) -> Result<String, NetError> {
    if !is_allowed(url) {
        return Err(NetError::Redirected(url.into()));
    }
    let (code, location) = once(url, scratch, SMALL)?;
    let _ = std::fs::remove_file(scratch);
    match code {
        300..=399 if !location.is_empty() => Ok(location),
        404 => Err(NetError::NotFound),
        _ => Err(NetError::Status(code)),
    }
}

/// Downloads `url` to `dest`, following allowed redirects, at most `max` bytes.
pub fn download(url: &str, dest: &Path, max: u64) -> Result<(), NetError> {
    let mut url = url.to_string();
    for _ in 0..=MAX_REDIRECTS {
        if !is_allowed(&url) {
            return Err(NetError::Redirected(url));
        }
        let (code, location) = once(&url, dest, max)?;
        match code {
            200 => return Ok(()),
            300..=399 if !location.is_empty() => {
                let _ = std::fs::remove_file(dest);
                url = location;
            }
            404 => return Err(NetError::NotFound),
            _ => return Err(NetError::Status(code)),
        }
    }
    Err(NetError::Redirected(url))
}

/// A small text file (a manifest, checksums), read into memory through a scratch file.
pub fn fetch_text(url: &str, scratch: &Path) -> Result<String, NetError> {
    let result = download(url, scratch, SMALL).and_then(|()| std::fs::read_to_string(scratch).map_err(|e| NetError::Local(e.to_string())));
    let _ = std::fs::remove_file(scratch);
    result
}
