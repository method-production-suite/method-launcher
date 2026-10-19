//! An app's `recent.json` (docs/spec.md §6). The suite's shape is a list, newest first, of
//! `{"path": "/absolute/path", "opened": "<RFC 3339>"}` (SUITE-STANDARDS.md §2); apps that
//! wrote theirs before that rule keep `{"files": ["/absolute/path", ...]}`, newest first, and
//! the launcher reads both. Anything else reads as no files.

use std::path::PathBuf;

/// One file an app opened lately: its path, and when, as seconds since 1970 if the list says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub opened: Option<i64>,
}

/// The entries of a `recent.json`, in the order the app wrote them (newest first). Only
/// absolute paths are kept.
pub fn parse(json: &str) -> Vec<Entry> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    let items: Vec<serde_json::Value> = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(mut map) => match map.remove("files") {
            Some(serde_json::Value::Array(items)) => items,
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    items
        .into_iter()
        .filter_map(|item| match item {
            serde_json::Value::String(path) => Some(Entry { path: PathBuf::from(path), opened: None }),
            serde_json::Value::Object(map) => {
                let path = map.get("path")?.as_str()?;
                let opened = map.get("opened").and_then(|o| o.as_str()).and_then(rfc3339_seconds);
                Some(Entry { path: PathBuf::from(path), opened })
            }
            _ => None,
        })
        .filter(|e| e.path.is_absolute())
        .collect()
}

/// `2026-10-19T04:18:39Z` (or with an offset like `+02:00`, or fractional seconds) as seconds
/// since 1970.
pub fn rfc3339_seconds(text: &str) -> Option<i64> {
    let n = |s: &str| s.parse::<i64>().ok();
    let (y, mo, d) = (n(text.get(0..4)?)?, n(text.get(5..7)?)?, n(text.get(8..10)?)?);
    let (h, mi, s) = (n(text.get(11..13)?)?, n(text.get(14..16)?)?, n(text.get(17..19)?)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let rest = text.get(19..)?;
    let rest = rest.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = match rest {
        "Z" | "z" => 0,
        _ => {
            let sign = match rest.get(0..1)? {
                "+" => 1,
                "-" => -1,
                _ => return None,
            };
            sign * (n(rest.get(1..3)?)? * 3600 + n(rest.get(4..6)?)? * 60)
        }
    };
    // Days from 1970-01-01 to the date (Howard Hinnant's days_from_civil).
    let y = if mo <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + h * 3600 + mi * 60 + s - offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_suite_shape() {
        let json = r#"[{"path":"/Films/Field Station/field-station.osf","opened":"2026-10-19T04:18:39Z"},{"path":"relative.osf"},{"nope":1}]"#;
        assert_eq!(parse(json), vec![Entry { path: PathBuf::from("/Films/Field Station/field-station.osf"), opened: Some(1_792_383_519) }]);
    }

    #[test]
    fn reads_the_earlier_shape() {
        let json = r#"{"files":["/Films/Field Station/schedule.mtsc","/Films/b.mtsc"],"bookmarks":{}}"#;
        let entries = parse(json);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], Entry { path: PathBuf::from("/Films/Field Station/schedule.mtsc"), opened: None });
    }

    #[test]
    fn anything_else_is_no_files() {
        assert!(parse("not json").is_empty());
        assert!(parse(r#"{"other":[]}"#).is_empty());
        assert!(parse("42").is_empty());
    }

    #[test]
    fn reads_times() {
        assert_eq!(rfc3339_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(rfc3339_seconds("2026-10-19T04:18:39Z"), Some(1_792_383_519));
        assert_eq!(rfc3339_seconds("2026-10-19T06:18:39.263+02:00"), Some(1_792_383_519));
        assert_eq!(rfc3339_seconds("2026-13-19T04:18:39Z"), None);
        assert_eq!(rfc3339_seconds("yesterday"), None);
    }
}
