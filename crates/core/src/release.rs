//! What a release says about itself (docs/spec.md §4.1, §5.1): the tag that
//! `releases/latest` redirects to, and the version in its `latest.json`.

use crate::version::Version;

/// The tag in a `releases/latest` redirect's `Location`
/// (`https://github.com/<org>/<repo>/releases/tag/v0.4.0`), if it is a version tag.
pub fn tag_from_location(location: &str) -> Option<String> {
    let tag = location.trim().rsplit_once("/releases/tag/")?.1;
    let tag = tag.split(['?', '#']).next()?;
    (tag.starts_with('v') && Version::parse(tag).is_some()).then(|| tag.to_string())
}

/// The `version` field of a release manifest (`latest.json`), if it is a version.
pub fn manifest_version(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let version = value.get("version")?.as_str()?.trim();
    let version = version.strip_prefix('v').unwrap_or(version);
    Version::parse(version).map(|_| version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_tag_from_the_redirect() {
        let base = "https://github.com/method-production-suite/method-budget/releases/tag/";
        assert_eq!(tag_from_location(&format!("{base}v0.5.0")).as_deref(), Some("v0.5.0"));
        assert_eq!(tag_from_location(&format!("{base}v1.2.3-beta.1\r\n")).as_deref(), Some("v1.2.3-beta.1"));
        assert_eq!(tag_from_location(&format!("{base}nightly")), None);
        assert_eq!(tag_from_location("https://github.com/method-production-suite/method-budget/releases"), None);
    }

    #[test]
    fn reads_the_manifest_version() {
        assert_eq!(manifest_version(r#"{"version":"0.5.0","platforms":{}}"#).as_deref(), Some("0.5.0"));
        assert_eq!(manifest_version(r#"{"version":"v0.13.0"}"#).as_deref(), Some("0.13.0"));
        assert_eq!(manifest_version(r#"{"version":"soon"}"#), None);
        assert_eq!(manifest_version("not json"), None);
    }
}
