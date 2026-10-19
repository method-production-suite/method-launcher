//! What a downloaded app must be before it is copied into Applications (docs/spec.md §4.4).
//! The checks that need the system (`codesign`, `spctl`) run in the app; this module holds
//! what they check against, so a test pins it.

/// The suite's Apple Developer team. Every suite release is signed by this team alone.
pub const TEAM_ID: &str = "XSRB3D643J";

/// The tools the checks use, by full path; all ship with macOS.
pub const CODESIGN: &str = "/usr/bin/codesign";
pub const SPCTL: &str = "/usr/sbin/spctl";
pub const HDIUTIL: &str = "/usr/bin/hdiutil";
pub const DITTO: &str = "/usr/bin/ditto";
pub const OPEN: &str = "/usr/bin/open";

/// The code requirement a suite app's signature must satisfy: an Apple-rooted Developer ID
/// Application certificate issued to [`TEAM_ID`].
pub fn requirement() -> String {
    format!(
        "anchor apple generic and certificate leaf[subject.OU] = \"{TEAM_ID}\" \
         and certificate 1[field.1.2.840.113635.100.6.2.6] \
         and certificate leaf[field.1.2.840.113635.100.6.1.13]"
    )
}

/// Arguments for `codesign` that verify a bundle against [`requirement`].
pub fn codesign_args(bundle: &str) -> Vec<String> {
    vec!["--verify".into(), "--deep".into(), "--strict".into(), format!("-R={}", requirement()), bundle.into()]
}

/// Arguments for `spctl` that assess a bundle as Gatekeeper would on first launch.
pub fn spctl_args(bundle: &str) -> Vec<String> {
    vec!["--assess".into(), "--type".into(), "execute".into(), "-vv".into(), bundle.into()]
}

/// Whether `spctl -vv` output (it writes to stderr) says the bundle was accepted as a
/// notarised Developer ID app.
pub fn spctl_says_notarised(output: &str) -> bool {
    output.lines().any(|l| l.trim_end().ends_with(": accepted")) && output.lines().any(|l| l.trim() == "source=Notarized Developer ID")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_team_is_pinned() {
        assert_eq!(TEAM_ID, "XSRB3D643J");
        assert!(requirement().contains("leaf[subject.OU] = \"XSRB3D643J\""));
        assert!(requirement().starts_with("anchor apple generic"));
    }

    #[test]
    fn codesign_is_strict_and_deep() {
        let args = codesign_args("/tmp/A.app");
        assert_eq!(&args[..3], ["--verify", "--deep", "--strict"]);
        assert_eq!(args.last().map(String::as_str), Some("/tmp/A.app"));
    }

    #[test]
    fn spctl_output_is_read_strictly() {
        let good = "/Volumes/x/Method Budget.app: accepted\nsource=Notarized Developer ID\norigin=Developer ID Application: Someone (XSRB3D643J)\n";
        assert!(spctl_says_notarised(good));
        assert!(!spctl_says_notarised("/Volumes/x/A.app: accepted\nsource=Developer ID\n"));
        assert!(!spctl_says_notarised("/Volumes/x/A.app: rejected\nsource=Notarized Developer ID\n"));
    }
}
