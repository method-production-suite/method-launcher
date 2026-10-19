//! What each app's card says and offers (docs/spec.md §3.1), decided from what is installed,
//! what the latest release is, and what is happening now. Kept apart from drawing so every
//! state is tested.

use launcher_core::catalog::App;
use launcher_core::platform::{Installed, Target};
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

/// The latest release's version, as far as the launcher knows.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Latest {
    #[default]
    Unknown,
    Version(String),
    Failed(String),
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

/// The one main button on a card.
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
    VisitSite,
    BuildFromSource,
    DownloadPage,
    /// Busy: no button.
    None,
}

/// Whether this system installs from inside the launcher yet (macOS today).
pub fn installs_here() -> bool {
    Target::current() == Some(Target::MacosArm64)
}

fn buildable_here() -> bool {
    Target::current().is_some_and(|t| !t.kinds().is_empty())
}

/// The quiet line under the name, and whether it is news (an update) to draw in the accent.
pub fn status_line(app: &App, found: &Found, latest: &Latest, job: &Job) -> (String, bool) {
    if let Job::Running(stage) = job {
        let text = match stage {
            Stage::Finding => "Finding the latest release".to_string(),
            Stage::Downloading(bytes) => format!("Downloading: {:.1} MB", *bytes as f64 / 1_000_000.0),
            Stage::Checking => "Checking the download".to_string(),
            Stage::Copying => "Copying to Applications".to_string(),
            Stage::Quitting => format!("Waiting for {} to quit", app.name),
        };
        return (text, false);
    }
    if let Job::Waiting = job {
        return ("Waiting for the other install to finish".to_string(), false);
    }
    let newer = |installed: &str| match latest {
        Latest::Version(v) if is_newer(v, installed) => Some(v.clone()),
        _ => None,
    };
    match found {
        Found::Looking => ("Looking for it on this computer".to_string(), false),
        Found::Installed(installed) => {
            let version = if installed.version.is_empty() { "Installed".to_string() } else { format!("Version {}", installed.version) };
            let earlier = match (installed.legacy, app.legacy) {
                (true, Some(legacy)) => format!(", installed as {}", legacy.name),
                _ => String::new(),
            };
            match newer(&installed.version) {
                Some(v) => (format!("{version}{earlier}. Update available: {v}"), true),
                None if matches!(latest, Latest::Failed(_)) => (format!("{version}{earlier}. Couldn't check for updates."), false),
                None => (format!("{version}{earlier}"), false),
            }
        }
        Found::Missing if !app.released => ("In development. No download yet.".to_string(), false),
        Found::Missing if !buildable_here() => (format!("There's no {} for this system yet. You can build it from source.", app.name), false),
        Found::Missing => match latest {
            Latest::Version(v) => (format!("Not installed. Version {v} is available."), false),
            _ => ("Not installed".to_string(), false),
        },
    }
}

/// The card's one main button.
pub fn primary(app: &App, found: &Found, latest: &Latest, job: &Job) -> Primary {
    if matches!(job, Job::Running(_) | Job::Waiting) {
        return Primary::None;
    }
    let version = match latest {
        Latest::Version(v) => Some(v.clone()),
        _ => None,
    };
    match found {
        Found::Looking => Primary::None,
        Found::Installed(installed) => match &version {
            Some(v) if installs_here() && installed.legacy => Primary::Install { version: Some(v.clone()), replacing: app.legacy.map(|l| l.name) },
            Some(v) if installs_here() && is_newer(v, &installed.version) => Primary::Update(v.clone()),
            _ => Primary::Open,
        },
        Found::Missing if !app.released => Primary::VisitSite,
        Found::Missing if !buildable_here() => Primary::BuildFromSource,
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

#[cfg(test)]
mod tests {
    use super::*;
    use launcher_core::catalog::by_slug;
    use std::path::PathBuf;

    fn installed(version: &str, legacy: bool) -> Found {
        Found::Installed(Installed { path: PathBuf::from("/Applications/x.app"), version: version.into(), legacy })
    }

    fn v(version: &str) -> Latest {
        Latest::Version(version.into())
    }

    #[test]
    fn an_older_version_offers_the_update() {
        let budget = by_slug("budget").unwrap();
        let (line, news) = status_line(budget, &installed("0.2.0", false), &v("0.5.0"), &Job::Idle);
        assert_eq!(line, "Version 0.2.0. Update available: 0.5.0");
        assert!(news);
        let expected = if installs_here() { Primary::Update("0.5.0".into()) } else { Primary::Open };
        assert_eq!(primary(budget, &installed("0.2.0", false), &v("0.5.0"), &Job::Idle), expected);
        assert_eq!(primary(budget, &installed("0.5.0", false), &v("0.5.0"), &Job::Idle), Primary::Open);
        assert_eq!(primary_label(budget, &Primary::Update("0.5.0".into())), "Update to 0.5.0");
    }

    #[test]
    fn an_earlier_name_is_replaced_by_an_install() {
        let reader = by_slug("reader").unwrap();
        let (line, _) = status_line(reader, &installed("0.12.1", true), &v("0.13.0"), &Job::Idle);
        assert_eq!(line, "Version 0.12.1, installed as Screepub Desktop. Update available: 0.13.0");
        if installs_here() {
            assert_eq!(
                primary(reader, &installed("0.12.1", true), &v("0.13.0"), &Job::Idle),
                Primary::Install { version: Some("0.13.0".into()), replacing: Some("Screepub Desktop") }
            );
        }
    }

    #[test]
    fn not_installed_offers_install_or_the_site() {
        let schedule = by_slug("schedule").unwrap();
        assert_eq!(status_line(schedule, &Found::Missing, &v("0.1.0"), &Job::Idle).0, "Not installed. Version 0.1.0 is available.");
        let storyteller = by_slug("storyteller").unwrap();
        assert_eq!(primary(storyteller, &Found::Missing, &Latest::Unknown, &Job::Idle), Primary::VisitSite);
        assert_eq!(status_line(storyteller, &Found::Missing, &Latest::Unknown, &Job::Idle).0, "In development. No download yet.");
    }

    #[test]
    fn busy_cards_show_progress_and_no_button() {
        let schedule = by_slug("schedule").unwrap();
        let job = Job::Running(Stage::Downloading(12_345_678));
        assert_eq!(status_line(schedule, &Found::Missing, &v("0.1.0"), &job).0, "Downloading: 12.3 MB");
        assert_eq!(primary(schedule, &Found::Missing, &v("0.1.0"), &job), Primary::None);
        assert_eq!(primary(schedule, &Found::Missing, &v("0.1.0"), &Job::Waiting), Primary::None);
    }

    #[test]
    fn a_failed_check_is_quiet() {
        let budget = by_slug("budget").unwrap();
        assert_eq!(
            status_line(budget, &installed("0.5.0", false), &Latest::Failed("offline".into()), &Job::Idle).0,
            "Version 0.5.0. Couldn't check for updates."
        );
    }
}
