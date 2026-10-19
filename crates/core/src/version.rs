//! Comparing an installed app's version with its release's (docs/spec.md §5.1). Versions are
//! `X.Y.Z` with an optional `-pre` part, as every suite release tag is; build metadata after
//! `+` is ignored, as semver says.

use std::cmp::Ordering;

/// A parsed `X.Y.Z[-pre]` version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// The pre-release part, empty for a release.
    pub pre: String,
}

impl Version {
    /// Parses `1.2.3`, `v1.2.3`, `1.2.3-beta.1` or `1.2.3+build`. `None` for anything else.
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) if !pre.is_empty() => (core, pre.to_string()),
            Some(_) => return None,
            None => (text, String::new()),
        };
        let mut parts = core.split('.');
        let mut number = || -> Option<u64> {
            let part = parts.next()?;
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) || (part.len() > 1 && part.starts_with('0')) {
                return None;
            }
            part.parse().ok()
        };
        let (major, minor, patch) = (number()?, number()?, number()?);
        if parts.next().is_some() {
            return None;
        }
        Some(Version { major, minor, patch, pre })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch)).then_with(|| compare_pre(&self.pre, &other.pre))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Semver pre-release order: a release sorts after its pre-releases; identifiers compare
/// numerically when both are numbers, otherwise as text, and numbers sort before text.
fn compare_pre(a: &str, b: &str) -> Ordering {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => {}
    }
    let mut left = a.split('.');
    let mut right = b.split('.');
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

/// Whether `available` is newer than `installed`. `false` when either doesn't parse, so an
/// unreadable version never shows an update.
pub fn is_newer(available: &str, installed: &str) -> bool {
    match (Version::parse(available), Version::parse(installed)) {
        (Some(a), Some(i)) => a > i,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_suite_versions() {
        assert_eq!(Version::parse("0.13.0"), Some(Version { major: 0, minor: 13, patch: 0, pre: String::new() }));
        assert_eq!(Version::parse("v1.2.3-beta.1").map(|v| v.pre), Some("beta.1".to_string()));
        assert_eq!(Version::parse("1.2.3+abc").map(|v| v.patch), Some(3));
        for bad in ["", "1", "1.2", "1.2.3.4", "01.2.3", "1.2.x", "1.2.3-", "-1.2.3", "1..3"] {
            assert_eq!(Version::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn orders_like_semver() {
        let order =
            ["0.9.0", "0.10.0", "1.0.0-alpha", "1.0.0-alpha.1", "1.0.0-alpha.beta", "1.0.0-beta.2", "1.0.0-beta.11", "1.0.0-rc.1", "1.0.0", "1.0.1"];
        for pair in order.windows(2) {
            assert!(is_newer(pair[1], pair[0]), "{} should be newer than {}", pair[1], pair[0]);
            assert!(!is_newer(pair[0], pair[1]));
        }
    }

    #[test]
    fn unreadable_versions_never_offer_an_update() {
        assert!(!is_newer("garbage", "0.1.0"));
        assert!(!is_newer("0.2.0", ""));
        assert!(!is_newer("0.4.0", "0.4.0"));
    }
}
