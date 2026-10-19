//! The only hosts the launcher reaches (docs/spec.md §4.2, §9): GitHub and the hosts GitHub
//! redirects release downloads to (the same two `method-update` allows), for releases; and
//! the suite's own site, for its list of apps. https only.

/// Hosts a request may start at or be redirected to: GitHub for releases, and the suite's
/// own site for its list of apps.
pub const ALLOWED_HOSTS: [&str; 4] = ["github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com", "methodproduction.com"];

/// The host of an `https://` URL, lowercased; `None` for anything else, or for a URL with
/// credentials or a port.
pub fn https_host(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.contains(['@', ':']) {
        return None;
    }
    Some(authority.to_ascii_lowercase())
}

/// Whether the launcher may fetch this URL (or follow a redirect to it).
pub fn is_allowed(url: &str) -> bool {
    https_host(url).is_some_and(|host| ALLOWED_HOSTS.contains(&host.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_over_https_only() {
        assert!(is_allowed("https://github.com/method-production-suite/method-budget/releases/latest/download/latest.json"));
        assert!(is_allowed("https://release-assets.githubusercontent.com/github-production-release-asset/1?sig=x"));
        assert!(is_allowed("https://GitHub.com/x"));
        assert!(!is_allowed("http://github.com/x"));
        assert!(!is_allowed("https://github.com.example.net/x"));
        assert!(!is_allowed("https://example.net/github.com"));
        assert!(!is_allowed("https://github.com@example.net/x"));
        assert!(!is_allowed("https://github.com:8443/x"));
        assert!(!is_allowed("file:///Applications"));
        assert!(!is_allowed("https:///x"));
        assert!(is_allowed("https://methodproduction.com/apps.json"));
        assert!(!is_allowed("https://methodproduction.com.example.net/apps.json"));
    }
}
