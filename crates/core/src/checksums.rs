//! A release's `SHA256SUMS` (docs/spec.md §4.1): `<64 hex>  <file name>` lines, as
//! `sha256sum` writes them (a `*` before the name marks binary mode and is ignored).

use sha2::{Digest, Sha256};

/// The checksum files the launcher reads, in order. `SHA256SUMS-app` is Method Reader's
/// current name for its installers' checksums, read until every app publishes the one name.
pub const FILES: [&str; 2] = ["SHA256SUMS", "SHA256SUMS-app"];

/// The lowercase hex SHA-256 listed for `file`, if exactly one well-formed line names it.
pub fn lookup(sums: &str, file: &str) -> Option<String> {
    let mut found = sums.lines().filter_map(|line| {
        let (hash, name) = line.trim_end_matches('\r').split_once(' ')?;
        let name = name.trim_start_matches(' ').trim_start_matches('*');
        (name == file && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then(|| hash.to_ascii_lowercase())
    });
    let first = found.next()?;
    // Two lines for one file is a malformed list; trust neither.
    if found.next().is_some() { None } else { Some(first) }
}

/// SHA-256 computed as the bytes arrive, so a download is hashed while it is written.
#[derive(Default)]
pub struct Hasher(Sha256);

impl Hasher {
    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    /// The digest as lowercase hex.
    pub fn hex(self) -> String {
        self.0.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Whether `bytes` hash to `expected` (hex, any case).
pub fn matches(bytes: &[u8], expected: &str) -> bool {
    let mut hasher = Hasher::default();
    hasher.update(bytes);
    hasher.hex().eq_ignore_ascii_case(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn finds_the_installer_line() {
        let sums = format!("{EMPTY}  Method-Reader-linux-amd64.deb\n{}  Method-Reader-windows-x64-setup.exe\r\n", "AB".repeat(32));
        assert_eq!(lookup(&sums, "Method-Reader-linux-amd64.deb").as_deref(), Some(EMPTY));
        assert_eq!(lookup(&sums, "Method-Reader-windows-x64-setup.exe"), Some("ab".repeat(32)));
        assert_eq!(lookup(&sums, "Method-Reader-macOS.dmg"), None);
        assert_eq!(lookup(&format!("{EMPTY} *x.dmg"), "x.dmg").as_deref(), Some(EMPTY));
    }

    #[test]
    fn rejects_malformed_or_ambiguous_lines() {
        assert_eq!(lookup("abc  x.dmg", "x.dmg"), None);
        assert_eq!(lookup(&format!("{}  x.dmg", "g".repeat(64)), "x.dmg"), None);
        assert_eq!(lookup(&format!("{EMPTY}  x.dmg\n{EMPTY}  x.dmg"), "x.dmg"), None);
        assert_eq!(lookup(&format!("{EMPTY}  x.dmg.partial"), "x.dmg"), None);
    }

    #[test]
    fn hashes_like_sha256sum() {
        assert!(matches(b"", EMPTY));
        assert!(matches(b"abc", "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"));
        assert!(!matches(b"abd", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));
    }
}
