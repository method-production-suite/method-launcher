//! The suite's apps, built into the launcher (docs/spec.md §9). Installer names come from each
//! app's noun per system (`platform::Kind`). Kept in step with the
//! website's `apps.toml` by hand; the tests below check every entry's shape.

use crate::platform::Kind;

/// GitHub organisation every app's repository lives in.
pub const ORG: &str = "method-production-suite";

/// The launcher's own bundle identifier, which also names its data folders.
pub const LAUNCHER_ID: &str = "com.methodlauncher.desktop";

/// An app's name and identifier before the suite's naming standard, still found on
/// computers that installed an older release (docs/spec.md §4.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Legacy {
    /// The old display name, also the old bundle's file name without `.app`.
    pub name: &'static str,
    /// The end of the old bundle identifier; the full identifier is checked to end with it.
    pub id_suffix: &'static str,
}

/// One app the launcher can show, install and open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct App {
    /// Short name used in file names and on the command line: `budget`.
    pub slug: &'static str,
    /// Display name, also the bundle's file name without `.app`: `Method Budget`.
    pub name: &'static str,
    /// One line saying what the app does.
    pub tagline: &'static str,
    /// Repository name under [`ORG`] (not always `method-<slug>`).
    pub repo: &'static str,
    /// The app's macOS bundle identifier.
    pub bundle_id: &'static str,
    /// Whether the app has a published release to install from yet.
    pub released: bool,
    /// The app's site.
    pub site: &'static str,
    /// Accent for light and dark appearance, as `#RRGGBB`.
    pub accent: (&'static str, &'static str),
    /// The app as older releases installed it, if it was named differently.
    pub legacy: Option<Legacy>,
}

impl App {
    /// The noun in "Method <Noun>": `Budget`.
    pub fn noun(&self) -> &'static str {
        self.name.strip_prefix("Method ").unwrap_or(self.name)
    }

    /// The window binary: `method-budget-app` (SUITE-STANDARDS.md §2).
    pub fn binary(&self) -> String {
        format!("method-{}-app", self.slug)
    }

    /// The Linux package name: `method-budget` (SUITE-STANDARDS.md §2).
    pub fn package(&self) -> String {
        format!("method-{}", self.slug)
    }

    /// The latest release's page, for "Release notes" and downloads.
    pub fn releases_url(&self) -> String {
        self.latest_url()
    }

    /// Where the build-from-source steps are.
    pub fn source_url(&self) -> String {
        format!("https://github.com/{ORG}/{}#build-from-source", self.repo)
    }

    /// The installer's file name for this kind, or `None` while there is no release yet.
    pub fn installer(&self, kind: Kind) -> Option<String> {
        self.released.then(|| kind.file_name(self.noun()))
    }

    /// The URL of the latest release, which redirects to its tag (spec §4.1 step 1).
    pub fn latest_url(&self) -> String {
        format!("https://github.com/{ORG}/{}/releases/latest", self.repo)
    }

    /// A file on a pinned release: `.../releases/download/v0.4.0/SHA256SUMS`.
    pub fn asset_url(&self, tag: &str, file: &str) -> String {
        format!("https://github.com/{ORG}/{}/releases/download/{tag}/{file}", self.repo)
    }

    /// The latest release's update manifest (SUITE-STANDARDS.md §1).
    pub fn manifest_url(&self) -> String {
        format!("https://github.com/{ORG}/{}/releases/latest/download/latest.json", self.repo)
    }

    /// The bundle's file name in Applications: `Method Budget.app`.
    pub fn bundle_name(&self) -> String {
        format!("{}.app", self.name)
    }
}

/// The apps in the order of a production: write, read, schedule, budget.
pub const APPS: [App; 4] = [
    App {
        slug: "storyteller",
        name: "Method Storyteller",
        tagline: "Screenwriting that formats itself.",
        repo: "method-storyteller",
        bundle_id: "com.methodstoryteller.desktop",
        released: false,
        site: "https://methodstoryteller.com",
        accent: ("#AE3D56", "#EE8DA2"),
        legacy: None,
    },
    App {
        slug: "reader",
        name: "Method Reader",
        tagline: "Screenplay PDFs into e-books and editable scripts.",
        repo: "method-reader",
        bundle_id: "com.methodreader.desktop",
        released: true,
        site: "https://methodreader.com",
        accent: ("#7052BA", "#B49CF0"),
        // Releases before the rename installed the app under its earlier name.
        legacy: Some(Legacy { name: "Screepub Desktop", id_suffix: ".screepub" }),
    },
    App {
        slug: "schedule",
        name: "Method Schedule",
        tagline: "Film and TV scheduling, from script to stripboard.",
        repo: "method-scheduling",
        bundle_id: "com.methodschedule.desktop",
        released: true,
        site: "https://methodschedule.com",
        accent: ("#2F5FB3", "#7AA2F0"),
        legacy: None,
    },
    App {
        slug: "budget",
        name: "Method Budget",
        tagline: "Film and TV budgeting that shows its working.",
        repo: "method-budget",
        bundle_id: "com.methodbudget.desktop",
        released: true,
        site: "https://methodbudget.com",
        accent: ("#0E7468", "#2CC7B2"),
        legacy: None,
    },
];

/// The app with this slug.
pub fn by_slug(slug: &str) -> Option<&'static App> {
    APPS.iter().find(|app| app.slug == slug)
}

/// The app with this bundle identifier.
pub fn by_bundle_id(id: &str) -> Option<&'static App> {
    APPS.iter().find(|app| app.bundle_id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_suite_pattern() {
        for app in &APPS {
            let noun = app.noun();
            assert!(app.name.starts_with("Method ") && !noun.contains(' '), "{}: not \"Method <Noun>\"", app.name);
            assert_eq!(app.slug, noun.to_lowercase());
            assert_eq!(app.bundle_id, format!("com.method{}.desktop", noun.to_lowercase()));
            assert!(app.site.starts_with("https://method"), "{}: site is not https", app.slug);
        }
    }

    #[test]
    fn binaries_and_packages_follow_the_standard() {
        let budget = by_slug("budget").unwrap();
        assert_eq!(budget.binary(), "method-budget-app");
        assert_eq!(budget.package(), "method-budget");
        assert_eq!(budget.source_url(), "https://github.com/method-production-suite/method-budget#build-from-source");
    }

    #[test]
    fn installers_only_once_released() {
        let budget = by_slug("budget").unwrap();
        assert_eq!(budget.installer(Kind::SetupExe).as_deref(), Some("Method-Budget-windows-x64-setup.exe"));
        assert_eq!(by_slug("storyteller").unwrap().installer(Kind::Dmg), None);
    }

    #[test]
    fn urls_point_at_the_suite_on_github() {
        let budget = by_slug("budget").unwrap();
        assert_eq!(budget.latest_url(), "https://github.com/method-production-suite/method-budget/releases/latest");
        assert_eq!(
            budget.asset_url("v0.4.0", "SHA256SUMS"),
            "https://github.com/method-production-suite/method-budget/releases/download/v0.4.0/SHA256SUMS"
        );
        assert_eq!(budget.manifest_url(), "https://github.com/method-production-suite/method-budget/releases/latest/download/latest.json");
        for app in &APPS {
            assert!(crate::net::is_allowed(&app.manifest_url()) && crate::net::is_allowed(&app.latest_url()));
        }
    }

    #[test]
    fn slugs_and_ids_are_unique_and_found() {
        for app in &APPS {
            assert_eq!(by_slug(app.slug), Some(app));
            assert_eq!(by_bundle_id(app.bundle_id), Some(app));
        }
        assert_eq!(by_slug("nope"), None);
    }

    #[test]
    fn accents_are_colours() {
        for app in &APPS {
            for c in [app.accent.0, app.accent.1] {
                assert!(c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit()), "{c}");
            }
        }
    }
}
