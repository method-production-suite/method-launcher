//! The suite's apps, built into the launcher (docs/spec.md §9). Kept in step with the
//! website's `apps.toml` by hand; the tests below check every entry's shape.

/// GitHub organisation every app's repository lives in.
pub const ORG: &str = "method-production-suite";

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
    /// Version-free installer name on the latest release, or `None` while there is no
    /// download yet.
    pub download: Option<&'static str>,
    /// The app's site.
    pub site: &'static str,
    /// Accent for light and dark appearance, as `#RRGGBB`.
    pub accent: (&'static str, &'static str),
}

impl App {
    /// The installer's URL on the latest release, fixed by the catalog (spec §4.1).
    pub fn download_url(&self) -> Option<String> {
        self.download.map(|file| format!("https://github.com/{ORG}/{}/releases/latest/download/{file}", self.repo))
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
        download: None,
        site: "https://methodstoryteller.com",
        accent: ("#AE3D56", "#EE8DA2"),
    },
    App {
        slug: "reader",
        name: "Method Reader",
        tagline: "Screenplay PDFs into e-books and editable scripts.",
        repo: "method-reader",
        bundle_id: "com.methodreader.desktop",
        download: Some("Method-Reader-macOS.dmg"),
        site: "https://methodreader.com",
        accent: ("#7052BA", "#B49CF0"),
    },
    App {
        slug: "schedule",
        name: "Method Schedule",
        tagline: "Film and TV scheduling, from script to stripboard.",
        repo: "method-scheduling",
        bundle_id: "com.methodschedule.desktop",
        download: Some("Method-Schedule-macOS.dmg"),
        site: "https://methodschedule.com",
        accent: ("#2F5FB3", "#7AA2F0"),
    },
    App {
        slug: "budget",
        name: "Method Budget",
        tagline: "Film and TV budgeting that shows its working.",
        repo: "method-budget",
        bundle_id: "com.methodbudget.desktop",
        download: Some("Method-Budget-macOS.dmg"),
        site: "https://methodbudget.com",
        accent: ("#0E7468", "#2CC7B2"),
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

    fn noun(app: &App) -> &str {
        app.name.strip_prefix("Method ").unwrap_or("")
    }

    #[test]
    fn names_follow_the_suite_pattern() {
        for app in &APPS {
            let noun = noun(app);
            assert!(!noun.is_empty() && !noun.contains(' '), "{}: not \"Method <Noun>\"", app.name);
            assert_eq!(app.slug, noun.to_lowercase());
            assert_eq!(app.bundle_id, format!("com.method{}.desktop", noun.to_lowercase()));
            assert!(app.site.starts_with("https://method"), "{}: site is not https", app.slug);
        }
    }

    #[test]
    fn downloads_are_version_free() {
        for app in &APPS {
            if let Some(file) = app.download {
                assert_eq!(file, format!("Method-{}-macOS.dmg", noun(app)));
            }
        }
    }

    #[test]
    fn urls_point_at_the_suite_on_github() {
        let budget = by_slug("budget").unwrap();
        assert_eq!(
            budget.download_url().as_deref(),
            Some("https://github.com/method-production-suite/method-budget/releases/latest/download/Method-Budget-macOS.dmg")
        );
        assert_eq!(budget.manifest_url(), "https://github.com/method-production-suite/method-budget/releases/latest/download/latest.json");
        assert_eq!(by_slug("storyteller").unwrap().download_url(), None);
        for app in &APPS {
            assert!(crate::net::is_allowed(&app.manifest_url()));
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
