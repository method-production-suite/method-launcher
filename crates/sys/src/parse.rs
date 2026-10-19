//! Reading what the systems' own tools print, kept apart from running them so every system's
//! parsing is tested on every system.

use std::path::{Path, PathBuf};

/// One uninstall entry from `reg query <Uninstall key> /s` (Windows).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegEntry {
    pub display_name: String,
    pub version: String,
    pub install_location: String,
    pub display_icon: String,
}

/// The uninstall entry whose `DisplayName` is exactly `display_name`, from the output of
/// `reg query <key> /s`: blocks that start with a `HKEY_...` line, then `    Name    REG_SZ    Value`
/// lines.
pub fn reg_uninstall(output: &str, display_name: &str) -> Option<RegEntry> {
    let mut current = RegEntry::default();
    let mut found = None;
    let mut finish = |entry: &mut RegEntry| {
        let done = std::mem::take(entry);
        if found.is_none() && done.display_name == display_name {
            found = Some(done);
        }
    };
    for line in output.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with("HKEY_") {
            finish(&mut current);
            continue;
        }
        let Some((name, value)) = reg_value(line) else { continue };
        match name {
            "DisplayName" => current.display_name = value.to_string(),
            "DisplayVersion" => current.version = value.to_string(),
            "InstallLocation" => current.install_location = value.to_string(),
            "DisplayIcon" => current.display_icon = value.to_string(),
            _ => {}
        }
    }
    finish(&mut current);
    found
}

/// `    Name    REG_SZ    Value` as `(Name, Value)`; `None` for anything else.
fn reg_value(line: &str) -> Option<(&str, &str)> {
    let line = line.strip_prefix("    ")?;
    let (name, rest) = line.split_once("    ")?;
    let rest = rest.trim_start();
    let (kind, value) = rest.split_once("    ").unwrap_or((rest, ""));
    kind.starts_with("REG_").then_some((name.trim(), value.trim()))
}

/// The executable a `DisplayIcon` value names: `"C:\path\app.exe",0` becomes `C:\path\app.exe`.
/// `None` unless it names an `.exe`.
pub fn exe_from_icon(icon: &str) -> Option<String> {
    let icon = icon.trim();
    let path = match icon.strip_prefix('"') {
        Some(rest) => rest.split('"').next()?,
        None => icon.rsplit_once(',').filter(|(_, n)| n.trim().parse::<i64>().is_ok()).map_or(icon, |(p, _)| p),
    };
    path.to_ascii_lowercase().ends_with(".exe").then(|| path.to_string())
}

/// The version from `dpkg-query -W -f '${Status}\t${Version}' <package>`, when the package is
/// installed (Linux).
pub fn dpkg_version(output: &str) -> Option<String> {
    let line = output.lines().next()?;
    let (status, version) = line.split_once('\t')?;
    (status.trim().ends_with(" installed") && !version.trim().is_empty()).then(|| version.trim().to_string())
}

/// The first file called `name` (or `name` plus one of `extensions`) in the folders of a
/// `PATH`-style list.
pub fn find_on_path(name: &str, path_var: &str, separator: char, extensions: &[&str]) -> Option<PathBuf> {
    path_var.split(separator).filter(|dir| !dir.is_empty()).find_map(|dir| {
        let dir = Path::new(dir);
        std::iter::once(String::new())
            .chain(extensions.iter().map(|e| (*e).to_string()))
            .map(|ext| dir.join(format!("{name}{ext}")))
            .find(|candidate| candidate.is_file())
    })
}

/// Whether a bundle identifier read from disk belongs to this catalog app, under its current
/// identifier or its earlier one. Returns `Some(legacy)` on a match.
pub fn bundle_matches(id: &str, current: &str, legacy_suffix: Option<&str>) -> Option<bool> {
    if id == current {
        Some(false)
    } else if legacy_suffix.is_some_and(|suffix| id.ends_with(suffix)) {
        Some(true)
    } else {
        None
    }
}

/// The mount point in `hdiutil attach -plist` output: the `<string>` after the one
/// `<key>mount-point</key>` (a disk image with one volume has exactly one).
pub fn hdiutil_mount_point(plist: &str) -> Option<String> {
    let mut points = plist.match_indices("<key>mount-point</key>").filter_map(|(at, key)| {
        let rest = plist.get(at + key.len()..)?.trim_start();
        let value = rest.strip_prefix("<string>")?;
        let end = value.find("</string>")?;
        value.get(..end).map(|v| v.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">"))
    });
    let first = points.next()?;
    points.next().is_none().then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REG: &str = "\r
HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Other\r
    DisplayName    REG_SZ    Some Other App\r
    DisplayVersion    REG_SZ    9.9\r
\r
HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Method Budget\r
    DisplayName    REG_SZ    Method Budget\r
    DisplayVersion    REG_SZ    0.4.0\r
    InstallLocation    REG_SZ    C:\\Users\\a\\AppData\\Local\\Method Budget\r
    DisplayIcon    REG_SZ    \"C:\\Users\\a\\AppData\\Local\\Method Budget\\method-budget-app.exe\",0\r
    NoModify    REG_DWORD    0x1\r
\r
HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Method Budget Helper\r
    DisplayName    REG_SZ    Method Budget Helper\r
";

    #[test]
    fn finds_the_exact_uninstall_entry() {
        let entry = reg_uninstall(REG, "Method Budget").unwrap();
        assert_eq!(entry.version, "0.4.0");
        assert_eq!(entry.install_location, "C:\\Users\\a\\AppData\\Local\\Method Budget");
        assert_eq!(exe_from_icon(&entry.display_icon).as_deref(), Some("C:\\Users\\a\\AppData\\Local\\Method Budget\\method-budget-app.exe"));
        assert_eq!(reg_uninstall(REG, "Method Schedule"), None);
        assert_eq!(reg_uninstall(REG, "Method Budget Helper").map(|e| e.version), Some(String::new()));
    }

    #[test]
    fn reads_icon_values() {
        assert_eq!(exe_from_icon("C:\\a\\b.exe,0").as_deref(), Some("C:\\a\\b.exe"));
        assert_eq!(exe_from_icon("C:\\a\\b.exe").as_deref(), Some("C:\\a\\b.exe"));
        assert_eq!(exe_from_icon("\"C:\\a b\\c.EXE\"").as_deref(), Some("C:\\a b\\c.EXE"));
        assert_eq!(exe_from_icon("C:\\a\\icon.ico"), None);
        assert_eq!(exe_from_icon(""), None);
    }

    #[test]
    fn reads_dpkg_status() {
        assert_eq!(dpkg_version("install ok installed\t0.4.0\n").as_deref(), Some("0.4.0"));
        assert_eq!(dpkg_version("deinstall ok config-files\t0.3.0"), None);
        assert_eq!(dpkg_version("dpkg-query: no packages found matching method-budget"), None);
        assert_eq!(dpkg_version(""), None);
    }

    #[test]
    fn finds_programs_on_the_path() {
        let dir = std::env::temp_dir().join(format!("launcher-path-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("method-budget-app.exe"), b"").unwrap();
        // PATH's own separator: Windows paths have a colon after the drive letter.
        let sep = if cfg!(windows) { ';' } else { ':' };
        let path_var = format!("{}{sep}{}", std::env::temp_dir().join("launcher-no-such-folder").display(), dir.display());
        assert_eq!(find_on_path("method-budget-app", &path_var, sep, &[".exe"]), Some(dir.join("method-budget-app.exe")));
        assert_eq!(find_on_path("method-budget-app", &path_var, sep, &[]), None);
        assert_eq!(find_on_path("method-schedule-app", &path_var, sep, &[".exe"]), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_the_mount_point() {
        let one = "<dict><key>content-hint</key><string>Apple_HFS</string>\n<key>mount-point</key>\n\t<string>/private/tmp/dmg.Ab12/Method Schedule</string></dict>";
        assert_eq!(hdiutil_mount_point(one).as_deref(), Some("/private/tmp/dmg.Ab12/Method Schedule"));
        assert_eq!(hdiutil_mount_point("<key>mount-point</key><string>/a &amp; b</string>").as_deref(), Some("/a & b"));
        assert_eq!(hdiutil_mount_point("<dict></dict>"), None);
        let two = format!("{one}{one}");
        assert_eq!(hdiutil_mount_point(&two), None);
    }

    #[test]
    fn matches_current_and_earlier_identifiers() {
        assert_eq!(bundle_matches("com.methodreader.desktop", "com.methodreader.desktop", Some(".screepub")), Some(false));
        assert_eq!(bundle_matches("org.example.screepub", "com.methodreader.desktop", Some(".screepub")), Some(true));
        assert_eq!(bundle_matches("org.example.other", "com.methodreader.desktop", Some(".screepub")), None);
        assert_eq!(bundle_matches("org.example.screepub", "com.methodbudget.desktop", None), None);
    }
}
