//! The suite's apps (docs/spec.md §9). The launcher reads the list from the suite's website
//! (`apps.json`, made from the website's `apps.toml`), checks every entry with the same rules
//! the built-in list is tested against, and falls back to the built-in list ([`BUILT_IN`])
//! when the website can't be reached or says something unexpected. Installer names come from
//! each app's noun per system (`platform::Kind`). Whether an app has a release is decided at
//! run time from its repository (docs/spec.md §5.1); `released` is only the offline fallback.

use crate::platform::Kind;
use std::sync::RwLock;

/// Where the suite publishes its list of apps.
pub const CATALOG_URL: &str = "https://methodproduction.com/apps.json";

/// GitHub organisation every app's repository lives in.
pub const ORG: &str = "method-production-suite";

/// The launcher's own bundle identifier, which also names its data folders.
pub const LAUNCHER_ID: &str = "com.methodlauncher.desktop";

/// The launcher's own releases (docs/spec.md §5.2).
pub const LAUNCHER_RELEASES: &str = "https://github.com/method-production-suite/method-launcher/releases/latest";

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
    /// Whether the app had a published release when this list was made: the fallback when
    /// its repository can't be asked.
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

    /// The installer's file name for this kind. Whether the latest release has it is asked of
    /// the release itself (its `SHA256SUMS`), at run time.
    pub fn installer(&self, kind: Kind) -> Option<String> {
        Some(kind.file_name(self.noun()))
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
/// The apps as built into this launcher, in the order of a production.
pub const BUILT_IN: [App; 4] = [
    App {
        slug: "storyteller",
        name: "Method Storyteller",
        tagline: "Screenwriting that formats itself.",
        repo: "method-storyteller",
        bundle_id: "com.methodstoryteller.desktop",
        // First release 0.2.0, 2026-10-19.
        released: true,
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
    apps().iter().find(|app| app.slug == slug)
}

/// The list in use: the built-in one until [`set_apps`] replaces it.
static CURRENT: RwLock<&'static [App]> = RwLock::new(&BUILT_IN);

/// The apps the launcher shows now.
pub fn apps() -> &'static [App] {
    match CURRENT.read() {
        Ok(apps) => *apps,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

/// Puts a checked list in use. The list lives for the rest of the run (a few hundred bytes,
/// replaced at most when the website's list changes).
pub fn set_apps(apps: Vec<App>) {
    let apps: &'static [App] = Box::leak(apps.into_boxed_slice());
    match CURRENT.write() {
        Ok(mut current) => *current = apps,
        Err(poisoned) => *poisoned.into_inner() = apps,
    }
}

fn leak(text: &str) -> &'static str {
    Box::leak(text.to_owned().into_boxed_str())
}

/// Whether an entry follows the suite's rules (SUITE-STANDARDS.md §2): "Method <Noun>", slug
/// the noun in lower case, bundle id `com.method<noun>.desktop`, a `method-` repository under
/// [`ORG`], the app's own site (`https://method<slug>.com`) or the suite's, and two `#RRGGBB`
/// accents. The website's list must pass the same
/// checks as the built-in one, since it names what the launcher downloads.
pub fn valid(app: &App) -> bool {
    let Some(noun) = app.name.strip_prefix("Method ") else { return false };
    let colour = |c: &str| c.len() == 7 && c.starts_with('#') && c.bytes().skip(1).all(|b| b.is_ascii_hexdigit());
    !noun.is_empty()
        && noun.chars().all(|c| c.is_ascii_alphabetic())
        && noun.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && app.slug == noun.to_lowercase()
        && app.bundle_id == format!("com.method{}.desktop", app.slug)
        && app.repo.strip_prefix("method-").is_some_and(|r| !r.is_empty() && r.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'))
        && (app.site == format!("https://method{}.com", app.slug) || app.site == "https://methodproduction.com")
        && !app.tagline.is_empty()
        && app.tagline.len() <= 120
        && colour(app.accent.0)
        && colour(app.accent.1)
}

/// The website's `apps.json`: a list of `{"slug", "name", "tagline", "repo", "bundle_id",
/// "site", "accent": ["#light", "#dark"]}` in the order to show (extra fields are ignored).
/// Every entry must pass [`valid`], slugs must be unique, and there must be at least one app;
/// otherwise the whole list is refused. An app the launcher also knows keeps its built-in
/// earlier name and release fallback.
pub fn parse_remote(json: &str) -> Option<Vec<App>> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let items = match &value {
        serde_json::Value::Array(items) => items.as_slice(),
        serde_json::Value::Object(map) => map.get("apps")?.as_array()?.as_slice(),
        _ => return None,
    };
    if items.is_empty() || items.len() > 32 {
        return None;
    }
    let mut apps = Vec::new();
    for item in items {
        let text = |key: &str| item.get(key).and_then(|v| v.as_str()).map(str::trim);
        let accent = item.get("accent")?.as_array()?;
        let (light, dark) = (accent.first()?.as_str()?, accent.get(1)?.as_str()?);
        let slug = text("slug")?;
        let built_in = BUILT_IN.iter().find(|a| a.slug == slug);
        // What the launcher downloads can't be moved by the list: an app it knows keeps its
        // repository and bundle id, and a new app's repository is `method-<slug>`.
        let (repo, bundle_id) = (text("repo")?, text("bundle_id")?);
        let fixed = match built_in {
            Some(known) => repo == known.repo && bundle_id == known.bundle_id,
            None => repo == format!("method-{slug}"),
        };
        if !fixed {
            return None;
        }
        let app = App {
            slug: leak(slug),
            name: leak(text("name")?),
            tagline: leak(text("tagline")?),
            repo: leak(text("repo")?),
            bundle_id: leak(text("bundle_id")?),
            released: built_in.is_none_or(|a| a.released),
            site: leak(text("site")?),
            accent: (leak(light), leak(dark)),
            legacy: built_in.and_then(|a| a.legacy),
        };
        if !valid(&app) || apps.iter().any(|a: &App| a.slug == app.slug) {
            return None;
        }
        apps.push(app);
    }
    Some(apps)
}

/// The app with this bundle identifier.
pub fn by_bundle_id(id: &str) -> Option<&'static App> {
    apps().iter().find(|app| app.bundle_id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_list_passes_the_same_checks_as_the_websites() {
        for app in &BUILT_IN {
            assert!(valid(app), "{}", app.slug);
        }
    }

    const REMOTE: &str = r##"{"apps":[
        {"slug":"storyteller","name":"Method Storyteller","tagline":"Screenwriting that formats itself.","repo":"method-storyteller","bundle_id":"com.methodstoryteller.desktop","site":"https://methodstoryteller.com","accent":["#AE3D56","#EE8DA2"],"status":"Beta"},
        {"slug":"reader","name":"Method Reader","tagline":"Screenplay PDFs into e-books.","repo":"method-reader","bundle_id":"com.methodreader.desktop","site":"https://methodreader.com","accent":["#7052BA","#B49CF0"]},
        {"slug":"office","name":"Method Office","tagline":"A new app.","repo":"method-office","bundle_id":"com.methodoffice.desktop","site":"https://methodoffice.com","accent":["#123456","#ABCDEF"]}
    ]}"##;

    #[test]
    fn reads_the_websites_list() {
        let apps = parse_remote(REMOTE).unwrap();
        assert_eq!(apps.iter().map(|a| a.slug).collect::<Vec<_>>(), ["storyteller", "reader", "office"]);
        // A known app keeps its earlier name; a new one starts with no fallback release.
        assert!(apps[1].legacy.is_some());
        assert!(apps[2].released && apps[2].legacy.is_none());
        assert_eq!(apps[2].installer(Kind::Dmg).as_deref(), Some("Method-Office-macOS.dmg"));
    }

    #[test]
    fn the_suites_own_site_is_allowed() {
        assert!(parse_remote(&REMOTE.replace("https://methodoffice.com", "https://methodproduction.com")).is_some());
    }

    #[test]
    fn refuses_a_list_with_any_bad_entry() {
        for (from, to) in [
            ("\"repo\":\"method-office\"", "\"repo\":\"someone-else/x\""),
            ("com.methodoffice.desktop", "com.example.desktop"),
            ("https://methodoffice.com", "http://methodoffice.com"),
            ("https://methodoffice.com", "https://evil.example/methodoffice.com"),
            ("\"name\":\"Method Office\"", "\"name\":\"Office\""),
            ("#123456", "red"),
            ("\"slug\":\"reader\"", "\"slug\":\"storyteller\""),
            // A known app can't be pointed at another app's releases or bundle.
            ("\"repo\":\"method-reader\"", "\"repo\":\"method-budget\""),
            ("com.methodreader.desktop", "com.methodbudget.desktop"),
            // A new app's repository is method-<slug>.
            ("\"repo\":\"method-office\"", "\"repo\":\"method-officesuite\""),
            // The site is the app's own or the suite's, never a look-alike.
            ("https://methodoffice.com", "https://methodoffice-download.com"),
            ("https://methodstoryteller.com", "https://methodstory.com"),
        ] {
            assert_eq!(parse_remote(&REMOTE.replace(from, to)), None, "{to}");
        }
        assert_eq!(parse_remote("[]"), None);
        assert_eq!(parse_remote("not json"), None);
    }

    #[test]
    fn names_follow_the_suite_pattern() {
        for app in &BUILT_IN {
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
    fn installers_are_named_whatever_the_fallback_says() {
        let budget = by_slug("budget").unwrap();
        assert_eq!(budget.installer(Kind::SetupExe).as_deref(), Some("Method-Budget-windows-x64-setup.exe"));
        // An app built in as unreleased can still be installed once its release is found.
        let unreleased = App { released: false, ..*budget };
        assert_eq!(unreleased.installer(Kind::Dmg).as_deref(), Some("Method-Budget-macOS.dmg"));
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
        for app in &BUILT_IN {
            assert!(crate::net::is_allowed(&app.manifest_url()) && crate::net::is_allowed(&app.latest_url()));
        }
    }

    #[test]
    fn slugs_and_ids_are_unique_and_found() {
        for app in &BUILT_IN {
            assert_eq!(by_slug(app.slug), Some(app));
            assert_eq!(by_bundle_id(app.bundle_id), Some(app));
        }
        assert_eq!(by_slug("nope"), None);
    }

    #[test]
    fn accents_are_colours() {
        for app in &BUILT_IN {
            for c in [app.accent.0, app.accent.1] {
                assert!(c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit()), "{c}");
            }
        }
    }
}
