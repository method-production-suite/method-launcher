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
    manifest(json).map(|r| r.version)
}

/// What a release manifest tells a person: its version, the day it was published, and one
/// line of what's new.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    /// Year, month, day, from `pub_date`.
    pub date: Option<(u32, u32, u32)>,
    /// The first plain sentence of `notes` that says something (spec §3.1).
    pub summary: Option<String>,
}

/// A release manifest (`latest.json`), if its version reads.
pub fn manifest(json: &str) -> Option<Release> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let version = value.get("version")?.as_str()?.trim();
    let version = version.strip_prefix('v').unwrap_or(version);
    Version::parse(version)?;
    let date = value.get("pub_date").and_then(|d| d.as_str()).and_then(parse_date);
    let summary = value.get("notes").and_then(|n| n.as_str()).and_then(summary);
    Some(Release { version: version.to_string(), date, summary })
}

/// `2026-10-19T04:18:39Z` as (2026, 10, 19).
fn parse_date(text: &str) -> Option<(u32, u32, u32)> {
    let day = text.get(..10)?;
    let mut parts = day.split('-').map(|p| p.parse::<u32>().ok());
    let (y, m, d) = (parts.next()??, parts.next()??, parts.next()??);
    ((1..=12).contains(&m) && (1..=31).contains(&d)).then_some((y, m, d))
}

/// A date as a person reads it: `19 Oct 2026`.
pub fn short_date((y, m, d): (u32, u32, u32)) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let month = MONTHS.get(m.saturating_sub(1) as usize).copied().unwrap_or("");
    format!("{d} {month} {y}")
}

/// The first sentence of release notes that says something: not a heading, not a bare name
/// and version, not a link. At most 140 characters.
fn summary(notes: &str) -> Option<String> {
    let says_something = |s: &str| {
        let words = s.split_whitespace().count();
        words >= 4 && !s.contains("http") && !s.starts_with('#')
    };
    let sentence = notes
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('-') && !l.starts_with('*'))
        .flat_map(|line| line.split_inclusive(". "))
        .map(str::trim)
        .find(|s| says_something(s))?;
    let mut text: String = sentence.chars().take(140).collect();
    if sentence.chars().count() > 140 {
        text = text.trim_end().to_string() + "…";
    }
    Some(text)
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
    fn reads_what_a_release_says() {
        let budget = manifest(r#"{"version":"0.5.0","pub_date":"2026-10-19T04:18:39Z","notes":"A finished sample budget comes with the app: Field Station, ready to explore.\n\nA sample budget\n- Field Station open"}"#).unwrap();
        assert_eq!(budget.date, Some((2026, 10, 19)));
        assert_eq!(budget.summary.as_deref(), Some("A finished sample budget comes with the app: Field Station, ready to explore."));
        assert_eq!(short_date((2026, 10, 19)), "19 Oct 2026");
        let schedule = manifest(
            r##"{"version":"0.1.0","notes":"# Method Schedule 0.1.0\n\nThe first release: a free, open scheduling app. It is an early alpha."}"##,
        )
        .unwrap();
        assert_eq!(schedule.summary.as_deref(), Some("The first release: a free, open scheduling app."));
        assert_eq!(schedule.date, None);
        let reader =
            manifest(r#"{"version":"0.13.0","pub_date":"bad","notes":"Method Reader 0.13.0. Release notes: https://example.org/x"}"#).unwrap();
        assert_eq!(reader.summary, None);
        assert_eq!(reader.date, None);
    }

    #[test]
    fn reads_the_manifest_version() {
        assert_eq!(manifest_version(r#"{"version":"0.5.0","platforms":{}}"#).as_deref(), Some("0.5.0"));
        assert_eq!(manifest_version(r#"{"version":"v0.13.0"}"#).as_deref(), Some("0.13.0"));
        assert_eq!(manifest_version(r#"{"version":"soon"}"#), None);
        assert_eq!(manifest_version("not json"), None);
    }
}
