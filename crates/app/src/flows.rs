//! What the window does when the person acts (docs/spec.md §3, §4.1, §7): the install queue,
//! the question before quitting open apps, Update all, the recent files, and the notes after
//! Open, Open Sample and Demo. Decided here, apart from drawing and from the system, so every
//! case is tested; main.rs only runs them.

use crate::state::{Found, Job, Latest, Primary, primary};
use launcher_core::catalog::App;
use launcher_core::platform::{Installed, Opened};
use launcher_sys::{InstallError, Recent};
use std::collections::VecDeque;

/// Installs waiting for the one running to finish (spec §3.1: one at a time), each with
/// whether the person agreed to quit the app first.
pub type Queue = VecDeque<(usize, bool)>;

/// Whether an install is running.
pub fn busy(jobs: &[Job]) -> bool {
    jobs.iter().any(|j| matches!(j, Job::Running(_)))
}

fn set(jobs: &mut [Job], index: usize, job: Job) {
    if let Some(slot) = jobs.get_mut(index) {
        *slot = job;
    }
}

/// Asks for an install of the app at `index`. It starts now when nothing is running or
/// waiting (true); otherwise it waits its turn behind them (false).
pub fn enqueue(jobs: &mut [Job], queue: &mut Queue, index: usize, quit: bool) -> bool {
    if busy(jobs) || !queue.is_empty() {
        set(jobs, index, Job::Waiting);
        queue.push_back((index, quit));
        false
    } else {
        true
    }
}

/// Cancel on a waiting card: it leaves the queue (true). A running one isn't waiting, so it
/// is the install's to stop (false).
pub fn cancel_waiting(jobs: &mut [Job], queue: &mut Queue, index: usize) -> bool {
    if !matches!(jobs.get(index), Some(Job::Waiting)) {
        return false;
    }
    queue.retain(|(i, _)| *i != index);
    set(jobs, index, Job::Idle);
    true
}

/// How an install ended: the card's job, and the note to show.
pub fn finished(app: &str, result: Result<Installed, InstallError>) -> (Job, Option<String>) {
    match result {
        Ok(installed) => (Job::Idle, Some(format!("{app} {} is installed.", installed.version))),
        Err(error) if error.cancelled => (Job::Idle, Some(format!("Cancelled. {app} wasn't changed."))),
        Err(error) => (Job::Failed(error), None),
    }
}

/// The question before quitting open apps to update them (spec §5.1): its title and text.
pub fn quit_question(names: &[&str]) -> (String, String) {
    match names {
        [one] => (
            format!("Quit {one} and update it?"),
            format!("{one} is open. Method Launcher will ask it to quit, so it can ask you about anything unsaved, then update it."),
        ),
        _ => (
            "Quit the open apps and update them?".to_string(),
            format!(
                "{} are open. Method Launcher will ask each one to quit, so it can ask you about anything unsaved, then update it.",
                names.join(" and ")
            ),
        ),
    }
}

/// The apps in `batch` that are open now, so the person is asked before they are quit.
pub fn open_now(batch: &[usize], apps: &[App], found: &[Found], is_running: impl Fn(&App, &Installed) -> bool) -> Vec<usize> {
    batch
        .iter()
        .copied()
        .filter(|&i| match (apps.get(i), found.get(i)) {
            (Some(app), Some(Found::Installed(installed))) => is_running(app, installed),
            _ => false,
        })
        .collect()
}

/// The apps whose main button is Update (or Install over an earlier name): Update all.
pub fn updatable(apps: &[App], found: &[Found], latest: &[Latest], jobs: &[Job]) -> Vec<usize> {
    (0..apps.len())
        .filter(|&i| {
            let (Some(app), Some(found), Some(latest), Some(job)) = (apps.get(i), found.get(i), latest.get(i), jobs.get(i)) else {
                return false;
            };
            matches!(found, Found::Installed(_)) && matches!(primary(app, found, latest, job), Primary::Update(_) | Primary::Install { replacing: Some(_), .. })
        })
        .collect()
}

/// The suite's recent files from every app's list, each with its app's index: newest first,
/// at most `shown`.
pub fn newest_files(per_app: &[Vec<Recent>], shown: usize) -> Vec<(usize, Recent)> {
    let mut recent: Vec<(usize, Recent)> =
        per_app.iter().enumerate().flat_map(|(index, files)| files.iter().cloned().map(move |r| (index, r))).collect();
    recent.sort_by_key(|(_, r)| std::cmp::Reverse(r.opened));
    recent.truncate(shown);
    recent
}

/// The note after Open, and whether it went wrong.
pub fn opened_note(app: &str, result: Result<Opened, String>) -> (String, bool) {
    match result {
        Ok(Opened::Started) => (format!("Opening {app}."), false),
        Ok(Opened::BroughtForward) => (format!("{app} is already open."), false),
        Ok(Opened::AlreadyRunning) => (format!("{app} is already open. Switch to it from the taskbar."), false),
        Err(e) => (format!("{app} couldn't be opened. {e}"), true),
    }
}

/// What Open Sample passes the app (spec §7): `--open-sample` only works as an app starts, so
/// one already running is only brought forward.
pub fn sample_arguments(running: bool) -> &'static [&'static str] {
    if running { &[] } else { &["--open-sample"] }
}

/// The note after Open Sample, and whether it went wrong.
pub fn sample_note<T>(app: &str, running: bool, result: Result<T, String>) -> (String, bool) {
    match result {
        Ok(_) if running => (format!("{app} is open. Choose Help > Open Sample."), false),
        Ok(_) => (format!("Opening the Field Station sample in {app}."), false),
        Err(e) => (format!("{app} couldn't be opened. {e}"), true),
    }
}

/// Demo the suite (the owner, 2026-10-19): every installed app opens on its Field Station
/// sample, with `--open-sample` for each that isn't running (SUITE-STANDARDS.md §2). A running
/// app is brought forward and the note says how to open the sample there; apps that aren't
/// installed are named as skipped. No app's settings change. Returns the note and whether
/// anything went wrong.
pub fn demo<T>(
    apps: &[App],
    found: &[Found],
    is_running: impl Fn(&App, &Installed) -> bool,
    mut open: impl FnMut(&App, &Installed, &[&str]) -> Result<T, String>,
) -> (String, bool) {
    let (mut opened, mut open_already, mut skipped, mut failed) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (app, found) in apps.iter().zip(found) {
        let Found::Installed(installed) = found else {
            skipped.push(short_name(app));
            continue;
        };
        if is_running(app, installed) {
            let _ = open(app, installed, &[]);
            open_already.push(short_name(app));
            continue;
        }
        match open(app, installed, &["--open-sample"]) {
            Ok(_) => opened.push(short_name(app)),
            Err(_) => failed.push(short_name(app)),
        }
    }
    (demo_note(&opened, &open_already, &skipped, &failed), !failed.is_empty())
}

/// "Method Budget" as "Budget", for notes that name several apps.
fn short_name(app: &App) -> &'static str {
    app.name.strip_prefix("Method ").unwrap_or(app.name)
}

/// "Reader and Budget" or "Reader, Schedule and Budget".
fn and_list(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The one-line note after Demo the suite.
fn demo_note(opened: &[&str], open_already: &[&str], skipped: &[&str], failed: &[&str]) -> String {
    let mut parts = Vec::new();
    if !opened.is_empty() {
        parts.push(format!("Opening the Field Station sample in {}.", and_list(opened)));
    }
    for name in open_already {
        parts.push(format!("{name} is open: use Help > Open Sample."));
    }
    if !failed.is_empty() {
        parts.push(format!("{} couldn't be opened.", and_list(failed)));
    }
    if !skipped.is_empty() {
        let verb = if skipped.len() == 1 { "isn't" } else { "aren't" };
        parts.push(format!("Skipped {}, which {verb} installed.", and_list(skipped)));
    }
    parts.join(" ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use launcher_core::catalog::BUILT_IN;
    use launcher_core::release::Release;
    use launcher_sys::Stage;
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    fn installed(version: &str) -> Installed {
        Installed { path: PathBuf::from(format!("/Applications/{version}.app")), version: version.into(), legacy: false }
    }

    #[test]
    fn installs_run_one_at_a_time_in_the_order_asked() {
        let mut jobs = vec![Job::Idle; 4];
        let mut queue = Queue::new();
        assert!(enqueue(&mut jobs, &mut queue, 0, false), "nothing running: starts now");
        jobs[0] = Job::Running(Stage::Finding);
        assert!(!enqueue(&mut jobs, &mut queue, 2, true));
        assert!(!enqueue(&mut jobs, &mut queue, 1, false));
        assert_eq!(queue, [(2, true), (1, false)]);
        assert_eq!((&jobs[1], &jobs[2]), (&Job::Waiting, &Job::Waiting));
        // Once the first is done, a new ask still waits behind the two already waiting.
        jobs[0] = Job::Idle;
        assert!(!enqueue(&mut jobs, &mut queue, 3, false));
        assert_eq!(queue.pop_front(), Some((2, true)));
    }

    #[test]
    fn cancel_takes_a_waiting_install_out_and_leaves_a_running_one() {
        let mut jobs = vec![Job::Running(Stage::Finding), Job::Waiting, Job::Waiting];
        let mut queue: Queue = [(1, false), (2, false)].into();
        assert!(cancel_waiting(&mut jobs, &mut queue, 1));
        assert_eq!((queue.clone(), jobs[1].clone()), ([(2, false)].into(), Job::Idle));
        assert!(!cancel_waiting(&mut jobs, &mut queue, 0), "running: the install stops it");
        assert!(!cancel_waiting(&mut jobs, &mut queue, 9));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn an_install_ends_with_a_note_or_its_failure() {
        assert_eq!(finished("Method Budget", Ok(installed("0.9.0"))), (Job::Idle, Some("Method Budget 0.9.0 is installed.".into())));
        let cancelled = InstallError { cancelled: true, ..InstallError::new("x", "y") };
        assert_eq!(finished("Method Budget", Err(cancelled)), (Job::Idle, Some("Cancelled. Method Budget wasn't changed.".into())));
        let failed = InstallError::new("The download stopped. Try again.", "slow");
        assert_eq!(finished("Method Budget", Err(failed.clone())), (Job::Failed(failed), None));
    }

    #[test]
    fn the_quit_question_names_the_open_apps() {
        let (title, body) = quit_question(&["Method Reader"]);
        assert_eq!(title, "Quit Method Reader and update it?");
        assert!(body.starts_with("Method Reader is open. "), "{body}");
        let (title, body) = quit_question(&["Method Reader", "Method Budget"]);
        assert_eq!(title, "Quit the open apps and update them?");
        assert!(body.starts_with("Method Reader and Method Budget are open. "), "{body}");
    }

    #[test]
    fn only_installed_apps_that_are_open_are_asked_about() {
        let found = [Found::Installed(installed("1")), Found::Missing, Found::Installed(installed("2")), Found::Looking];
        let open = open_now(&[0, 1, 2, 3, 7], &BUILT_IN, &found, |_, i| i.version == "2");
        assert_eq!(open, [2]);
    }

    #[test]
    fn update_all_takes_the_installed_apps_that_are_behind() {
        let release = |v: &str| Latest::Release(Release { version: v.into(), date: None, summary: None });
        let found = [Found::Installed(installed("0.6.0")), Found::Installed(installed("1.2.0")), Found::Missing, Found::Installed(installed("0.8.0"))];
        let latest = [release("0.7.0"), release("1.2.0"), release("0.8.0"), release("0.9.0")];
        let jobs = [Job::Idle, Job::Idle, Job::Idle, Job::Running(Stage::Finding)];
        let expected: &[usize] = if crate::state::installs_here() { &[0] } else { &[] };
        // Up to date, not installed, or already updating: left out.
        assert_eq!(updatable(&BUILT_IN, &found, &latest, &jobs), expected);
    }

    #[test]
    fn recent_files_are_the_newest_across_the_suite() {
        let at = |secs: u64| UNIX_EPOCH + Duration::from_secs(secs);
        let file = |name: &str, secs| Recent { path: PathBuf::from(name), opened: at(secs) };
        let per_app = vec![vec![file("a", 30), file("b", 10)], vec![], vec![file("c", 20), file("d", 40)]];
        let names: Vec<(usize, String)> = newest_files(&per_app, 3).into_iter().map(|(i, r)| (i, r.path.display().to_string())).collect();
        assert_eq!(names, [(2, "d".to_string()), (0, "a".to_string()), (2, "c".to_string())]);
        assert!(newest_files(&[], 10).is_empty());
    }

    #[test]
    fn open_and_open_sample_say_what_happened() {
        assert_eq!(opened_note("Method Budget", Ok(Opened::Started)), ("Opening Method Budget.".into(), false));
        assert_eq!(opened_note("Method Budget", Ok(Opened::BroughtForward)), ("Method Budget is already open.".into(), false));
        assert!(opened_note("Method Budget", Ok(Opened::AlreadyRunning)).0.ends_with("Switch to it from the taskbar."));
        assert_eq!(opened_note("Method Budget", Err("No such file.".into())), ("Method Budget couldn't be opened. No such file.".into(), true));
        assert_eq!(sample_arguments(false), ["--open-sample"]);
        assert!(sample_arguments(true).is_empty(), "a running app only comes forward");
        assert_eq!(sample_note("Method Budget", true, Ok(())), ("Method Budget is open. Choose Help > Open Sample.".into(), false));
        assert_eq!(sample_note("Method Budget", false, Ok(())), ("Opening the Field Station sample in Method Budget.".into(), false));
        assert!(sample_note::<()>("Method Budget", false, Err("x".into())).1);
    }

    #[test]
    fn demo_opens_each_installed_app_on_its_sample() {
        // Storyteller missing, Reader running, Schedule opens, Budget fails to open.
        let found = [Found::Missing, Found::Installed(installed("reader")), Found::Installed(installed("schedule")), Found::Installed(installed("budget"))];
        let mut asked = Vec::new();
        let (note, failed) = demo(
            &BUILT_IN,
            &found,
            |_, i| i.version == "reader",
            |app, _, args: &[&str]| {
                asked.push((app.slug, args.iter().map(|a| a.to_string()).collect::<Vec<_>>()));
                if app.slug == "budget" { Err("no".to_string()) } else { Ok(()) }
            },
        );
        assert_eq!(
            note,
            "Opening the Field Station sample in Schedule. Reader is open: use Help > Open Sample. Budget couldn't be opened. Skipped Storyteller, which isn't installed."
        );
        assert!(failed);
        assert_eq!(asked, [("reader", vec![]), ("schedule", vec!["--open-sample".to_string()]), ("budget", vec!["--open-sample".to_string()])]);
    }

    #[test]
    fn the_demo_note_names_every_app() {
        assert_eq!(
            demo_note(&["Reader", "Schedule"], &["Budget"], &["Storyteller"], &[]),
            "Opening the Field Station sample in Reader and Schedule. Budget is open: use Help > Open Sample. Skipped Storyteller, which isn't installed."
        );
        assert_eq!(
            demo_note(&["Storyteller", "Reader", "Schedule", "Budget"], &[], &[], &[]),
            "Opening the Field Station sample in Storyteller, Reader, Schedule and Budget."
        );
        assert_eq!(
            demo_note(&[], &[], &["Storyteller", "Schedule"], &["Reader"]),
            "Reader couldn't be opened. Skipped Storyteller and Schedule, which aren't installed."
        );
    }
}
