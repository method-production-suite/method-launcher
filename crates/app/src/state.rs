//! What each app's card says and offers (docs/spec.md §3.1), decided from what is installed,
//! what the latest release is, and what is happening now. Kept apart from drawing so every
//! state is tested.

use launcher_core::catalog::App;
use launcher_core::platform::{Installed, Target};
use launcher_core::release::{Release, short_date};
use launcher_core::version::is_newer;
use launcher_sys::{InstallError, Stage};

/// What the launcher knows about one app on this computer.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Found {
    #[default]
    Looking,
    Installed(Installed),
    Missing,
}

/// The latest release, as far as the launcher knows, asked of the app's repository at run
/// time (spec §5.1).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Latest {
    #[default]
    Unknown,
    /// A published release with an installer for this system.
    Release(Release),
    /// A published release with no installer for this system.
    NoInstaller(Release),
    /// No published release yet.
    NoRelease,
    Failed(String),
}

impl Latest {
    fn release(&self) -> Option<&Release> {
        match self {
            Latest::Release(r) | Latest::NoInstaller(r) => Some(r),
            _ => None,
        }
    }
}

/// Whether the app has a published release: what its repository said, or the catalog's
/// fallback when it couldn't be asked.
pub fn released(app: &App, latest: &Latest) -> bool {
    match latest {
        Latest::Release(_) | Latest::NoInstaller(_) => true,
        Latest::NoRelease => false,
        Latest::Unknown | Latest::Failed(_) => app.released,
    }
}

/// Whether this system can install the app: the suite builds for it, and the latest
/// release (when known) lists an installer for it.
fn installable_here(latest: &Latest) -> bool {
    buildable_here() && !matches!(latest, Latest::NoInstaller(_))
}

/// An install or update in progress, or how the last one ended.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Job {
    #[default]
    Idle,
    Waiting,
    Running(Stage),
    Failed(InstallError),
}

/// The one main button on a card. The last three are drawn as quiet links: an app that can't
/// be installed here never gets a filled button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Primary {
    Open,
    /// Install the latest version over the installed one.
    Update(String),
    /// Install the latest version; `replacing` names an earlier-named copy it replaces.
    Install {
        version: Option<String>,
        replacing: Option<&'static str>,
    },
    DownloadPage,
    VisitSite,
    BuildFromSource,
    /// Busy: no button.
    None,
}

impl Primary {
    /// Whether it is drawn as the card's filled button.
    pub fn filled(&self) -> bool {
        matches!(self, Primary::Open | Primary::Update(_) | Primary::Install { .. } | Primary::DownloadPage)
    }
}

/// A small tag beside the status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Badge {
    /// A newer version is out: drawn in the app's accent.
    Behind(String),
    UpToDate,
    InDevelopment,
}

/// What a card says under the app's name: one line, an optional badge, and an optional
/// quieter second line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub line: String,
    pub badge: Option<Badge>,
    pub detail: Option<String>,
    /// Something is happening: draw a spinner.
    pub busy: bool,
}

/// What the card says while the app is put in place, per system.
fn installing_line() -> &'static str {
    match Target::current() {
        Some(Target::WindowsX64) => "The installer is open",
        Some(Target::MacosArm64) => "Copying to Applications",
        _ => "Installing",
    }
}

/// Whether this system installs from inside the launcher: every system the suite builds apps
/// for (macOS, Windows x64, Linux x86_64), not yet Linux on ARM.
pub fn installs_here() -> bool {
    buildable_here()
}

fn buildable_here() -> bool {
    Target::current().is_some_and(|t| !t.kinds().is_empty())
}

fn released_on(release: &Release) -> String {
    match release.date {
        Some(date) => format!("Released {}.", short_date(date)),
        None => String::new(),
    }
}

fn joined(parts: &[&str]) -> Option<String> {
    let text = parts.iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

pub fn status(app: &App, found: &Found, latest: &Latest, job: &Job) -> Status {
    let plain = |line: String| Status { line, badge: None, detail: None, busy: false };
    match job {
        Job::Running(stage) => {
            let line = match stage {
                Stage::Finding => "Finding the latest release".to_string(),
                Stage::Downloading(bytes) => format!("Downloading, {:.1} MB", *bytes as f64 / 1_000_000.0),
                Stage::Checking => "Checking the download".to_string(),
                Stage::Copying => installing_line().to_string(),
                Stage::Quitting => format!("Waiting for {} to quit", app.name),
            };
            return Status { busy: true, ..plain(line) };
        }
        Job::Waiting => return Status { busy: true, ..plain("Waiting for the other install to finish".to_string()) },
        _ => {}
    }
    let release = latest.release();
    match found {
        Found::Looking => Status { busy: true, ..plain("Looking for it on this computer".to_string()) },
        Found::Installed(installed) => {
            let version = if installed.version.is_empty() { "Installed".to_string() } else { format!("Version {}", installed.version) };
            let earlier = match (installed.legacy, app.legacy) {
                (true, Some(legacy)) => format!("Installed as {}.", legacy.name),
                _ => String::new(),
            };
            match release {
                Some(r) if is_newer(&r.version, &installed.version) => Status {
                    line: version,
                    badge: Some(Badge::Behind(r.version.clone())),
                    detail: joined(&[&earlier, &released_on(r), r.summary.as_deref().unwrap_or("")]),
                    busy: false,
                },
                Some(_) => Status { line: version, badge: Some(Badge::UpToDate), detail: joined(&[&earlier]), busy: false },
                None if matches!(latest, Latest::Failed(_)) => {
                    Status { line: version, badge: None, detail: joined(&[&earlier, "Couldn't check for updates."]), busy: false }
                }
                None => Status { line: version, badge: None, detail: joined(&[&earlier]), busy: false },
            }
        }
        Found::Missing if !released(app, latest) => {
            Status { line: "No download yet".to_string(), badge: Some(Badge::InDevelopment), detail: None, busy: false }
        }
        // An Intel Mac and a release with no Intel code (from before the app went universal).
        Found::Missing if matches!(latest, Latest::NoInstaller(_)) && Target::current() == Some(Target::MacosX64) => Status {
            line: format!("This version of {} needs an Apple silicon Mac", app.name),
            badge: None,
            detail: Some("Its next release runs on Intel Macs too.".to_string()),
            busy: false,
        },
        Found::Missing if !installable_here(latest) => Status {
            line: "Not built for this system yet".to_string(),
            badge: None,
            detail: Some("You can build it from source.".to_string()),
            busy: false,
        },
        Found::Missing => match release {
            Some(r) => Status {
                line: "Not installed".to_string(),
                badge: None,
                detail: joined(&[&format!("Version {}.", r.version), &released_on(r), r.summary.as_deref().unwrap_or("")]),
                busy: false,
            },
            None => plain("Not installed".to_string()),
        },
    }
}

/// The card's one main button.
pub fn primary(app: &App, found: &Found, latest: &Latest, job: &Job) -> Primary {
    if matches!(job, Job::Running(_) | Job::Waiting) {
        return Primary::None;
    }
    let version = latest.release().map(|r| r.version.clone());
    match found {
        Found::Looking => Primary::None,
        Found::Installed(installed) => match &version {
            Some(v) if installs_here() && installable_here(latest) && installed.legacy => {
                Primary::Install { version: Some(v.clone()), replacing: app.legacy.map(|l| l.name) }
            }
            Some(v) if installs_here() && installable_here(latest) && is_newer(v, &installed.version) => Primary::Update(v.clone()),
            _ => Primary::Open,
        },
        Found::Missing if !released(app, latest) => Primary::VisitSite,
        // Nothing to install on an Intel Mac until the app's universal release: its site only.
        Found::Missing if matches!(latest, Latest::NoInstaller(_)) && Target::current() == Some(Target::MacosX64) => Primary::VisitSite,
        Found::Missing if !installable_here(latest) => Primary::BuildFromSource,
        Found::Missing if installs_here() => Primary::Install { version, replacing: None },
        Found::Missing => Primary::DownloadPage,
    }
}

/// The main button's words.
pub fn primary_label(app: &App, primary: &Primary) -> String {
    match primary {
        Primary::Open => "Open".to_string(),
        Primary::Update(v) => format!("Update to {v}"),
        Primary::Install { version: Some(v), .. } => format!("Install {v}"),
        Primary::Install { version: None, .. } => "Install".to_string(),
        Primary::VisitSite => "Visit the site".to_string(),
        Primary::BuildFromSource => "Build from source".to_string(),
        Primary::DownloadPage => format!("Download {}", app.name),
        Primary::None => String::new(),
    }
}

/// Whether a card's install can still be cancelled: waiting its turn, or finding, downloading
/// or checking. Once it is quitting the app or copying, it finishes.
pub fn cancellable(job: &Job) -> bool {
    matches!(job, Job::Waiting | Job::Running(Stage::Finding | Stage::Downloading(_) | Stage::Checking))
}

#[cfg(test)]
mod tests {
    use super::*;
    use launcher_core::catalog::by_slug;
    use std::path::PathBuf;

    fn installed(version: &str, legacy: bool) -> Found {
        Found::Installed(Installed { path: PathBuf::from("/Applications/x.app"), version: version.into(), legacy })
    }

    fn release(version: &str) -> Latest {
        Latest::Release(Release { version: version.into(), date: Some((2026, 10, 19)), summary: Some("A sample budget comes with it.".into()) })
    }

    #[test]
    fn an_older_version_shows_a_badge_and_whats_new() {
        let budget = by_slug("budget").unwrap();
        let s = status(budget, &installed("0.2.0", false), &release("0.5.0"), &Job::Idle);
        assert_eq!(s.line, "Version 0.2.0");
        assert_eq!(s.badge, Some(Badge::Behind("0.5.0".into())));
        assert_eq!(s.detail.as_deref(), Some("Released 19 Oct 2026. A sample budget comes with it."));
        let expected = if installs_here() { Primary::Update("0.5.0".into()) } else { Primary::Open };
        assert_eq!(primary(budget, &installed("0.2.0", false), &release("0.5.0"), &Job::Idle), expected);
        let current = status(budget, &installed("0.5.0", false), &release("0.5.0"), &Job::Idle);
        assert_eq!((current.badge, current.detail), (Some(Badge::UpToDate), None));
        assert_eq!(primary(budget, &installed("0.5.0", false), &release("0.5.0"), &Job::Idle), Primary::Open);
    }

    #[test]
    fn an_earlier_name_is_said_and_replaced() {
        let reader = by_slug("reader").unwrap();
        let s = status(reader, &installed("0.12.1", true), &release("0.13.0"), &Job::Idle);
        assert!(s.detail.unwrap_or_default().starts_with("Installed as Screepub Desktop. Released 19 Oct 2026."));
        if installs_here() {
            assert_eq!(
                primary(reader, &installed("0.12.1", true), &release("0.13.0"), &Job::Idle),
                Primary::Install { version: Some("0.13.0".into()), replacing: Some("Screepub Desktop") }
            );
        }
    }

    #[test]
    fn in_development_is_a_tag_and_a_quiet_link() {
        // An app whose repository has no release yet.
        let storyteller = by_slug("storyteller").unwrap();
        let s = status(storyteller, &Found::Missing, &Latest::NoRelease, &Job::Idle);
        assert_eq!((s.line.as_str(), s.badge), ("No download yet", Some(Badge::InDevelopment)));
        let main = primary(storyteller, &Found::Missing, &Latest::NoRelease, &Job::Idle);
        assert_eq!(main, Primary::VisitSite);
        assert!(!main.filled());
    }

    #[test]
    fn not_installed_says_what_is_available() {
        // Only where the suite publishes app builds (not yet Linux on ARM).
        if !buildable_here() {
            return;
        }
        let schedule = by_slug("schedule").unwrap();
        let s = status(schedule, &Found::Missing, &release("0.1.0"), &Job::Idle);
        assert_eq!(s.line, "Not installed");
        assert_eq!(s.detail.as_deref(), Some("Version 0.1.0. Released 19 Oct 2026. A sample budget comes with it."));
    }

    #[test]
    fn busy_cards_show_progress_and_no_button() {
        let schedule = by_slug("schedule").unwrap();
        let job = Job::Running(Stage::Downloading(12_345_678));
        let s = status(schedule, &Found::Missing, &release("0.1.0"), &job);
        assert_eq!((s.line.as_str(), s.busy), ("Downloading, 12.3 MB", true));
        assert_eq!(primary(schedule, &Found::Missing, &release("0.1.0"), &job), Primary::None);
        assert_eq!(primary(schedule, &Found::Missing, &release("0.1.0"), &Job::Waiting), Primary::None);
    }

    #[test]
    fn a_first_release_shows_up_at_run_time() {
        // Built in as unreleased, but its repository now has a release: installable.
        let new_app = launcher_core::catalog::App { released: false, ..*by_slug("storyteller").unwrap() };
        assert!(!released(&new_app, &Latest::Unknown));
        assert!(released(&new_app, &release("0.2.0")));
        if installs_here() {
            assert_eq!(
                primary(&new_app, &Found::Missing, &release("0.2.0"), &Job::Idle),
                Primary::Install { version: Some("0.2.0".into()), replacing: None }
            );
        }
        // Built in as released, but the repository says it has none: the site, not Install.
        let budget = by_slug("budget").unwrap();
        assert_eq!(primary(budget, &Found::Missing, &Latest::NoRelease, &Job::Idle), Primary::VisitSite);
    }

    #[test]
    fn a_release_without_this_systems_installer_offers_no_install() {
        let schedule = by_slug("schedule").unwrap();
        let latest = Latest::NoInstaller(Release { version: "0.2.0".into(), date: None, summary: None });
        // On an Intel Mac that release needs Apple silicon: only the site is offered.
        let expected = if Target::current() == Some(Target::MacosX64) { Primary::VisitSite } else { Primary::BuildFromSource };
        assert_eq!(primary(schedule, &Found::Missing, &latest, &Job::Idle), expected);
        if Target::current() == Some(Target::MacosX64) {
            assert_eq!(status(schedule, &Found::Missing, &latest, &Job::Idle).line, "This version of Method Schedule needs an Apple silicon Mac");
        }
        assert_eq!(primary(schedule, &installed("0.1.0", false), &latest, &Job::Idle), Primary::Open);
    }

    #[test]
    fn a_failed_check_is_quiet() {
        let budget = by_slug("budget").unwrap();
        let s = status(budget, &installed("0.5.0", false), &Latest::Failed("offline".into()), &Job::Idle);
        assert_eq!(s.detail.as_deref(), Some("Couldn't check for updates."));
    }

    #[test]
    fn an_install_can_be_cancelled_until_it_copies() {
        assert!(cancellable(&Job::Waiting));
        assert!(cancellable(&Job::Running(Stage::Finding)));
        assert!(cancellable(&Job::Running(Stage::Downloading(1))));
        assert!(cancellable(&Job::Running(Stage::Checking)));
        assert!(!cancellable(&Job::Running(Stage::Copying)));
        assert!(!cancellable(&Job::Running(Stage::Quitting)));
        assert!(!cancellable(&Job::Idle));
    }
}
