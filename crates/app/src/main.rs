//! Method Launcher's window (docs/spec.md §3): one card per suite app, with its icon, whether
//! it is installed here and at which version, whether a newer release is out, and one main
//! button: Open, Update, or Install. Installing and updating run the checked install of
//! launcher-sys (spec §4) off the interface thread, one at a time.

mod images;
mod state;
mod theme;

#[cfg(test)]
mod screens;

use images::{WINDOW_ICON, WORDMARK, WORDMARK_DARK, decode_png, mark_png, trim};
use launcher_core::catalog::{APPS, App};
use launcher_core::platform::Folders;
use launcher_core::platform::{Installed, Launch, Locate, Opened};
use launcher_sys::{InstallError, Recent, Stage, System};
use state::{Badge, Found, Job, Latest, Primary, installs_here, primary, primary_label, status};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

const APP_NAME: &str = "Method Launcher";

/// Wide enough for two cards side by side.
const TWO_COLUMNS_FROM: f32 = 820.0;

// The window's grid, in points: everything sits on multiples of 4.
const PAD: f32 = 32.0;
const CARD_GAP: f32 = 16.0;
const CARD_PAD: f32 = 24.0;
const CARD_RADIUS: f32 = 14.0;
const ICON: f32 = 56.0;
const ACTION_HEIGHT: f32 = 36.0;
const WORDMARK_HEIGHT: f32 = 28.0;

/// Text on an accent button in dark mode (the sites' `--accent-ink`).
const DARK_INK: egui::Color32 = egui::Color32::from_rgb(0x11, 0x14, 0x17);

fn main() -> eframe::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("method-launcher-app {} ({APP_NAME})", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let mut viewport = egui::ViewportBuilder::default().with_title(APP_NAME).with_inner_size([1000.0, 820.0]).with_min_inner_size([760.0, 560.0]);
    if let Some(icon) = decode_png(WINDOW_ICON) {
        viewport =
            viewport.with_icon(Arc::new(egui::IconData { width: icon.width() as u32, height: icon.height() as u32, rgba: icon.as_raw().to_vec() }));
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(APP_NAME, options, Box::new(|cc| Ok(Box::new(Launcher::new(&cc.egui_ctx)))))
}

/// One app as a search found it: where it is, its own icon if it could be read, and the files
/// it opened lately.
type Scanned = (Option<Installed>, Option<egui::ColorImage>, Vec<Recent>);

/// How many recent files the window lists.
const RECENT_SHOWN: usize = 10;

/// News from the worker threads.
enum Event {
    Scanned(Vec<Scanned>),
    Latest(usize, Latest),
    /// The launcher's own latest release, when it is newer than this one.
    LauncherNewer(String),
    // Installs run from the launcher on macOS only, so far.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Stage(usize, Stage),
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Done(usize, Result<Installed, InstallError>),
}

struct Launcher {
    found: Vec<Found>,
    latest: Vec<Latest>,
    jobs: Vec<Job>,
    /// Installs waiting for the one running to finish (spec §3.1: one at a time), each with
    /// whether the person agreed to quit the app first.
    queue: VecDeque<(usize, bool)>,
    /// Installs waiting for the person to agree to quit the apps that are open: the batch,
    /// and which of them are running.
    confirm: Option<(Vec<usize>, Vec<usize>)>,
    scanning: bool,
    /// Version checks still running, and when the last round finished.
    checking: usize,
    checked: Option<std::time::SystemTime>,
    /// A newer release of the launcher itself (spec §5.2).
    launcher_newer: Option<String>,
    /// The suite's recent files, newest first, each with its app's index.
    recent: Vec<(usize, Recent)>,
    settings: Settings,
    send: mpsc::Sender<Event>,
    receive: mpsc::Receiver<Event>,
    /// Each app's mark, and its own icon once found installed, ready to draw.
    marks: Vec<Option<egui::TextureHandle>>,
    icons: Vec<Option<egui::TextureHandle>>,
    /// The suite's wordmark, light and dark.
    wordmark: [Option<egui::TextureHandle>; 2],
    /// Cards whose failure details are open.
    details: HashSet<usize>,
    /// One line about the last thing the person did, and whether it went wrong.
    notice: Option<(String, bool)>,
    focused: bool,
}

impl Launcher {
    fn new(ctx: &egui::Context) -> Self {
        theme::apply(ctx);
        theme::use_system_font(ctx);
        let texture = |name: String, png: &[u8]| decode_png(png).map(|image| ctx.load_texture(name, image, egui::TextureOptions::LINEAR));
        let marks = APPS.iter().map(|app| texture(format!("mark-{}", app.slug), mark_png(app))).collect();
        let (send, receive) = mpsc::channel();
        let mut launcher = Launcher {
            found: vec![Found::Looking; APPS.len()],
            latest: vec![Latest::Unknown; APPS.len()],
            jobs: vec![Job::Idle; APPS.len()],
            queue: VecDeque::new(),
            confirm: None,
            scanning: false,
            checking: 0,
            checked: None,
            launcher_newer: None,
            recent: Vec::new(),
            settings: Settings::load(),
            send,
            receive,
            marks,
            icons: vec![None; APPS.len()],
            wordmark: [texture("wordmark".into(), WORDMARK), texture("wordmark-dark".into(), WORDMARK_DARK)],
            details: HashSet::new(),
            notice: None,
            focused: true,
        };
        #[cfg(target_os = "macos")]
        {
            // Leftovers from an install the launcher was quit in the middle of (spec §4.5).
            let dirs = vec![PathBuf::from("/Applications"), home().join("Applications")];
            std::thread::spawn(move || launcher_sys::install_macos::clean_up(&dirs));
        }
        launcher.rescan(ctx);
        if launcher.settings.check_for_updates {
            launcher.check_latest(ctx);
        }
        launcher
    }

    /// Looks for the apps again, off the interface thread.
    fn rescan(&mut self, ctx: &egui::Context) {
        if self.scanning {
            return;
        }
        self.scanning = true;
        let (send, ctx) = (self.send.clone(), ctx.clone());
        std::thread::spawn(move || {
            let found = APPS
                .iter()
                .map(|app| {
                    let installed = System.locate(app);
                    let icon = installed.as_ref().and_then(|i| System.app_icon(app, i)).and_then(|png| decode_png(&png)).map(trim);
                    (installed, icon, System.recent_files(app))
                })
                .collect();
            let _ = send.send(Event::Scanned(found));
            ctx.request_repaint();
        });
    }

    /// Reads every released app's latest version (spec §5.1), each on its own thread.
    fn check_latest(&mut self, ctx: &egui::Context) {
        if self.checking > 0 {
            return;
        }
        self.checking = APPS.iter().filter(|a| a.released).count();
        {
            let (send, ctx) = (self.send.clone(), ctx.clone());
            std::thread::spawn(move || {
                if let Ok(latest) = System.launcher_latest()
                    && launcher_core::version::is_newer(&latest, env!("CARGO_PKG_VERSION"))
                {
                    let _ = send.send(Event::LauncherNewer(latest));
                    ctx.request_repaint();
                }
            });
        }
        for (index, app) in APPS.iter().enumerate().filter(|(_, a)| a.released) {
            let (send, ctx) = (self.send.clone(), ctx.clone());
            std::thread::spawn(move || {
                let latest = System.latest_release(app).map_or_else(Latest::Failed, Latest::Release);
                let _ = send.send(Event::Latest(index, latest));
                ctx.request_repaint();
            });
        }
    }

    fn take_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.receive.try_recv() {
            match event {
                Event::Scanned(found) => {
                    self.icons = APPS
                        .iter()
                        .zip(&found)
                        .map(|(app, (_, icon, _))| {
                            icon.clone().map(|image| ctx.load_texture(format!("icon-{}", app.slug), image, egui::TextureOptions::LINEAR))
                        })
                        .collect();
                    let mut recent: Vec<(usize, Recent)> =
                        found.iter().enumerate().flat_map(|(index, (_, _, files))| files.iter().cloned().map(move |r| (index, r))).collect();
                    recent.sort_by_key(|(_, r)| std::cmp::Reverse(r.opened));
                    recent.truncate(RECENT_SHOWN);
                    self.recent = recent;
                    self.found = found.into_iter().map(|(f, _, _)| f.map_or(Found::Missing, Found::Installed)).collect();
                    self.scanning = false;
                }
                Event::Latest(index, latest) => {
                    set(&mut self.latest, index, latest);
                    self.checking = self.checking.saturating_sub(1);
                    if self.checking == 0 {
                        self.checked = Some(std::time::SystemTime::now());
                    }
                }
                Event::LauncherNewer(version) => self.launcher_newer = Some(version),
                Event::Stage(index, stage) => set(&mut self.jobs, index, Job::Running(stage)),
                Event::Done(index, result) => {
                    let app = APPS.get(index).map_or("The app", |a| a.name);
                    match result {
                        Ok(installed) => {
                            set(&mut self.jobs, index, Job::Idle);
                            self.notice = Some((format!("{app} {} is installed.", installed.version), false));
                        }
                        Err(error) => set(&mut self.jobs, index, Job::Failed(error)),
                    }
                    self.rescan(ctx);
                    if let Some((next, quit)) = self.queue.pop_front() {
                        self.start_install(ctx, next, quit);
                    }
                }
            }
        }
    }

    fn busy(&self) -> bool {
        self.jobs.iter().any(|j| matches!(j, Job::Running(_)))
    }

    /// Installs or updates these apps, first asking to quit any that are open.
    fn request_install(&mut self, ctx: &egui::Context, batch: Vec<usize>) {
        let running: Vec<usize> = batch
            .iter()
            .copied()
            .filter(|&i| match (APPS.get(i), self.found.get(i)) {
                (Some(app), Some(Found::Installed(installed))) => System.is_running(app, installed),
                _ => false,
            })
            .collect();
        if running.is_empty() {
            for index in batch {
                self.install(ctx, index, false);
            }
        } else {
            self.confirm = Some((batch, running));
        }
    }

    /// Installs or updates the app at `index`, or queues it behind the one running.
    fn install(&mut self, ctx: &egui::Context, index: usize, quit: bool) {
        self.details.remove(&index);
        if self.busy() || !self.queue.is_empty() {
            set(&mut self.jobs, index, Job::Waiting);
            self.queue.push_back((index, quit));
        } else {
            self.start_install(ctx, index, quit);
        }
    }

    /// The apps whose main button is Update (or Install over an earlier name): Update all.
    fn updatable(&self) -> Vec<usize> {
        (0..APPS.len())
            .filter(|&i| {
                let (Some(app), Some(found), Some(latest), Some(job)) = (APPS.get(i), self.found.get(i), self.latest.get(i), self.jobs.get(i)) else {
                    return false;
                };
                matches!(found, Found::Installed(_))
                    && matches!(primary(app, found, latest, job), Primary::Update(_) | Primary::Install { replacing: Some(_), .. })
            })
            .collect()
    }

    /// The question before quitting open apps (spec §5.1).
    fn confirm_quit(&mut self, ctx: &egui::Context) {
        let Some((batch, running)) = self.confirm.clone() else { return };
        let names: Vec<&str> = running.iter().filter_map(|&i| APPS.get(i).map(|a| a.name)).collect();
        let (title, body) = match names.as_slice() {
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
        };
        let mut answer = None;
        egui::Modal::new(egui::Id::new("confirm-quit")).show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.label(egui::RichText::new(title).size(18.0).strong());
            ui.add_space(8.0);
            ui.label(egui::RichText::new(body).size(14.0));
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.add(accent_button(ui, "Quit and update", LAUNCHER_ACCENT)).clicked() {
                    answer = Some(true);
                }
                if ui.add(egui::Button::new(egui::RichText::new("Cancel").size(15.0)).corner_radius(8.0)).clicked() {
                    answer = Some(false);
                }
            });
        });
        match answer {
            Some(true) => {
                self.confirm = None;
                for index in batch {
                    self.install(ctx, index, running.contains(&index));
                }
            }
            Some(false) => self.confirm = None,
            None => {}
        }
    }

    #[cfg(target_os = "macos")]
    fn start_install(&mut self, ctx: &egui::Context, index: usize, quit: bool) {
        let Some(app) = APPS.get(index) else { return };
        let current = match self.found.get(index) {
            Some(Found::Installed(installed)) => Some(installed.clone()),
            _ => None,
        };
        // An update goes where the installed copy is; a new install into /Applications.
        let dest = current.as_ref().and_then(|c| c.path.parent().map(PathBuf::from)).unwrap_or_else(|| PathBuf::from("/Applications"));
        set(&mut self.jobs, index, Job::Running(Stage::Finding));
        let (send, ctx) = (self.send.clone(), ctx.clone());
        std::thread::spawn(move || {
            let report = |stage: Stage| {
                let _ = send.send(Event::Stage(index, stage));
                ctx.request_repaint();
            };
            let result = launcher_sys::install_macos::install(app, &dest, current.as_ref(), quit, &report);
            let _ = send.send(Event::Done(index, result));
            ctx.request_repaint();
        });
    }

    #[cfg(not(target_os = "macos"))]
    fn start_install(&mut self, _ctx: &egui::Context, index: usize, _quit: bool) {
        if let Some(app) = APPS.get(index) {
            self.link(&app.releases_url());
        }
    }

    fn open(&mut self, app: &App, installed: &Installed) {
        let result = System.open_app(app, installed, &[]).map(|opened| match opened {
            Opened::Started => format!("Opening {}.", app.name),
            Opened::BroughtForward => format!("{} is already open.", app.name),
            Opened::AlreadyRunning => format!("{} is already open. Switch to it from the taskbar.", app.name),
        });
        self.notice = Some(match result {
            Ok(text) => (text, false),
            Err(e) => (format!("{} couldn't be opened. {e}", app.name), true),
        });
    }

    /// Opens every installed app on its Field Station sample (the owner, 2026-10-19), with
    /// `--open-sample` for each app that isn't running (SUITE-STANDARDS.md §2). A running app
    /// is brought forward and the note says how to open the sample there; apps that aren't
    /// installed are named as skipped. No app's settings change.
    fn demo(&mut self) {
        let (mut opened, mut open_already, mut skipped, mut failed) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (app, found) in APPS.iter().zip(&self.found) {
            let Found::Installed(installed) = found else {
                skipped.push(short_name(app));
                continue;
            };
            if System.is_running(app, installed) {
                let _ = System.open_app(app, installed, &[]);
                open_already.push(short_name(app));
                continue;
            }
            match System.open_app(app, installed, &["--open-sample"]) {
                Ok(_) => opened.push(short_name(app)),
                Err(_) => failed.push(short_name(app)),
            }
        }
        self.notice = Some((demo_note(&opened, &open_already, &skipped, &failed), !failed.is_empty()));
    }

    fn link(&mut self, url: &str) {
        if let Err(e) = System.open_url(url) {
            self.notice = Some((format!("The link couldn't be opened. {e}"), true));
        }
    }

    /// Whether the app at `index` has a second line to show (detail or a failure).
    fn has_detail(&self, index: usize) -> bool {
        let (Some(app), Some(found), Some(latest), Some(job)) =
            (APPS.get(index), self.found.get(index), self.latest.get(index), self.jobs.get(index))
        else {
            return false;
        };
        matches!(job, Job::Failed(_)) || status(app, found, latest, job).detail.is_some()
    }

    /// One app's card. `detail` holds two lines for detail, so the cards in a row line up.
    fn card(&mut self, ui: &mut egui::Ui, index: usize, app: &App, detail: bool) {
        let dark = ui.visuals().dark_mode;
        let accent = colour(if dark { app.accent.1 } else { app.accent.0 });
        let found = self.found.get(index).cloned().unwrap_or_default();
        let latest = self.latest.get(index).cloned().unwrap_or_default();
        let job = self.jobs.get(index).cloned().unwrap_or_default();
        let picture = self.icons.get(index).cloned().flatten().or_else(|| self.marks.get(index).cloned().flatten());
        let status = status(app, &found, &latest, &job);
        let main = primary(app, &found, &latest, &job);
        let muted = ui.visuals().weak_text_color();

        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(CARD_RADIUS)
            .inner_margin(CARD_PAD)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                // Name row: icon, name and one line, centred on the icon.
                ui.horizontal(|ui| {
                    ui.set_height(ICON);
                    let size = egui::vec2(ICON, ICON);
                    match &picture {
                        Some(texture) => {
                            ui.add(egui::Image::new((texture.id(), size)).corner_radius(ICON * 0.2237));
                        }
                        None => {
                            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                            ui.painter().rect_filled(rect, ICON * 0.2237, accent);
                        }
                    }
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.add_space(7.0);
                        ui.label(egui::RichText::new(app.name).size(19.0).strong());
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(app.tagline).size(14.0).color(muted));
                    });
                });
                ui.add_space(18.0);
                // Status row: where it stands, and a tag.
                ui.horizontal(|ui| {
                    ui.set_height(22.0);
                    if status.busy {
                        ui.add(egui::Spinner::new().size(14.0));
                        ui.add_space(8.0);
                    }
                    ui.label(egui::RichText::new(&status.line).size(15.0));
                    if let Some(tag) = &status.badge {
                        ui.add_space(10.0);
                        badge(ui, tag, app);
                    }
                });
                // Two lines held for detail when any card in the row has some.
                let detail_height = if detail { 38.0 } else { 0.0 };
                ui.add_space(if detail { 6.0 } else { 0.0 });
                ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), detail_height), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_min_height(detail_height);
                    match &job {
                        Job::Failed(error) => {
                            let text = if self.details.contains(&index) && !error.details.is_empty() { &error.details } else { &error.message };
                            two_lines(ui, text, 13.5, ui.visuals().error_fg_color);
                        }
                        _ => {
                            if let Some(detail) = &status.detail {
                                two_lines(ui, detail, 13.5, muted);
                            }
                        }
                    }
                });
                ui.add_space(16.0);
                // Action row: the main button, then quieter links, on one centre line.
                ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), ACTION_HEIGHT), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_min_height(ACTION_HEIGHT);
                    ui.spacing_mut().item_spacing = egui::vec2(18.0, 0.0);
                    if main.filled() {
                        let label = match (&job, &main) {
                            (Job::Failed(_), Primary::Install { .. } | Primary::Update(_)) => "Try again".to_string(),
                            _ => primary_label(app, &main),
                        };
                        if ui.add(accent_button(ui, &label, app.accent)).clicked() {
                            self.act(ui.ctx(), index, app, &found, &main);
                        }
                        ui.add_space(4.0);
                    } else if main != Primary::None && link(ui, &primary_label(app, &main)) {
                        self.act(ui.ctx(), index, app, &found, &main);
                    }
                    if let Job::Failed(error) = &job
                        && !error.details.is_empty()
                    {
                        let open = self.details.contains(&index);
                        if link(ui, if open { "Hide details" } else { "Details" }) {
                            if open {
                                self.details.remove(&index);
                            } else {
                                self.details.insert(index);
                            }
                        }
                    }
                    if let Found::Installed(installed) = &found {
                        if main != Primary::Open && !matches!(job, Job::Running(_)) && link(ui, "Open") {
                            self.open(app, installed);
                        }
                        if link(ui, SHOW_LOCATION)
                            && let Err(e) = System.show_location(&installed.path)
                        {
                            self.notice = Some((format!("{} couldn't be shown. {e}", app.name), true));
                        }
                    }
                    if app.released && link(ui, "Release notes") {
                        self.link(&app.releases_url());
                    }
                    if !matches!(found, Found::Installed(_)) && main != Primary::VisitSite && link(ui, "Website") {
                        self.link(app.site);
                    }
                });
            });
    }

    /// Recent files across the suite (spec §3.2): newest first, each opening in its app.
    fn recent_files(&mut self, ui: &mut egui::Ui) {
        let muted = ui.visuals().weak_text_color();
        ui.label(egui::RichText::new("Recent files").size(17.0).strong());
        ui.add_space(12.0);
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(CARD_RADIUS)
            .inner_margin(egui::Margin::symmetric(24, 12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if self.recent.is_empty() {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Files you open in the Method apps show up here.").size(14.0).color(muted));
                    ui.add_space(6.0);
                    return;
                }
                let rows = self.recent.clone();
                for (n, (index, file)) in rows.iter().enumerate() {
                    if n > 0 {
                        ui.separator();
                    }
                    let Some(app) = APPS.get(*index) else { continue };
                    let installed = match self.found.get(*index) {
                        Some(Found::Installed(installed)) => Some(installed.clone()),
                        _ => None,
                    };
                    let mark = self.icons.get(*index).cloned().flatten().or_else(|| self.marks.get(*index).cloned().flatten());
                    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), 36.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        if let Some(mark) = &mark {
                            ui.add(egui::Image::new((mark.id(), egui::vec2(22.0, 22.0))).corner_radius(5.0));
                        }
                        let name = file.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        let opened = ui
                            .add(egui::Label::new(egui::RichText::new(&name).size(14.5)).sense(egui::Sense::click()).truncate())
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(format!("Open in {}", app.name));
                        if opened.clicked() {
                            match &installed {
                                Some(installed) => {
                                    if let Err(e) = System.open_file(app, installed, &file.path) {
                                        self.notice = Some((format!("{name} couldn't be opened. {e}"), true));
                                    }
                                }
                                None => self.notice = Some((format!("Install {} to open {name}.", app.name), true)),
                            }
                        }
                        let folder = file.path.parent().map(shorten_home).unwrap_or_default();
                        ui.add(egui::Label::new(egui::RichText::new(folder).size(13.0).color(muted)).truncate());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 18.0;
                            if link(ui, SHOW_LOCATION)
                                && let Err(e) = System.show_location(&file.path)
                            {
                                self.notice = Some((format!("{name} couldn't be shown. {e}"), true));
                            }
                            ui.label(egui::RichText::new(day(file.opened)).size(13.0).color(muted));
                            if installed.is_none() {
                                ui.label(egui::RichText::new(format!("{} isn't installed", app.name)).size(13.0).color(muted));
                            }
                        });
                    });
                }
            });
    }

    fn act(&mut self, ctx: &egui::Context, index: usize, app: &App, found: &Found, main: &Primary) {
        match main {
            Primary::Open => {
                if let Found::Installed(installed) = found {
                    self.open(app, installed);
                }
            }
            Primary::Update(_) | Primary::Install { .. } => self.request_install(ctx, vec![index]),
            Primary::VisitSite => self.link(app.site),
            Primary::BuildFromSource => self.link(&app.source_url()),
            Primary::DownloadPage => self.link(&app.releases_url()),
            Primary::None => {}
        }
    }
}

/// The launcher's own accent, ochre, light and dark: its own main buttons only.
const LAUNCHER_ACCENT: (&str, &str) = ("#9A5B00", "#F0B54A");

/// A main button filled with an accent, as the sites draw theirs.
fn accent_button<'a>(ui: &egui::Ui, label: &str, accent: (&str, &str)) -> egui::Button<'a> {
    let dark = ui.visuals().dark_mode;
    let fill = colour(if dark { accent.1 } else { accent.0 });
    let ink = if dark { DARK_INK } else { egui::Color32::WHITE };
    egui::Button::new(egui::RichText::new(label).size(15.0).strong().color(ink)).fill(fill).corner_radius(8.0).min_size(egui::vec2(96.0, 0.0))
}

fn set<T>(list: &mut [T], index: usize, value: T) {
    if let Some(slot) = list.get_mut(index) {
        *slot = value;
    }
}

#[cfg(target_os = "macos")]
fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

#[cfg(target_os = "macos")]
const SHOW_LOCATION: &str = "Show in Finder";
#[cfg(target_os = "windows")]
const SHOW_LOCATION: &str = "Show in Explorer";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const SHOW_LOCATION: &str = "Show in Files";

/// `#RRGGBB` to a colour; grey for anything else (the catalog's tests keep that from happening).
fn colour(hex: &str) -> egui::Color32 {
    let channel = |at: usize| hex.get(at..at + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
    match (channel(1), channel(3), channel(5)) {
        (Some(r), Some(g), Some(b)) => egui::Color32::from_rgb(r, g, b),
        _ => egui::Color32::GRAY,
    }
}

impl eframe::App for Launcher {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.take_events(&ctx);
        // Look again when the window comes forward: an app may have been installed meanwhile.
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        if focused && !self.focused {
            self.rescan(&ctx);
        }
        self.focused = focused;
        if ctx.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::R))) {
            self.rescan(&ctx);
            self.check_latest(&ctx);
        }
        // "Checked 3 minutes ago" stays true.
        ctx.request_repaint_after(std::time::Duration::from_secs(30));

        self.confirm_quit(&ctx);
        egui::CentralPanel::default().frame(egui::Frame::central_panel(ui.style()).inner_margin(PAD)).show(ui, |ui| {
            let muted = ui.visuals().weak_text_color();
            // Header: the suite's wordmark and the promise on the left, the launcher's own
            // buttons on the right, both on the cards' edges.
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let dark = ui.visuals().dark_mode;
                    if let Some(wordmark) = self.wordmark.get(usize::from(dark)).cloned().flatten() {
                        let size = wordmark.size_vec2();
                        ui.add(egui::Image::new((wordmark.id(), egui::vec2(WORDMARK_HEIGHT * size.x / size.y, WORDMARK_HEIGHT))));
                    } else {
                        ui.label(egui::RichText::new("Method Production Suite").size(24.0).strong());
                    }
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Install the apps you want. Each one updates itself.").size(15.0).color(muted));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let updatable = self.updatable();
                    if installs_here() && !updatable.is_empty() {
                        let label = format!("Update all ({})", updatable.len());
                        if ui.add(accent_button(ui, &label, LAUNCHER_ACCENT)).clicked() {
                            self.request_install(&ctx, updatable);
                        }
                    }
                    let refresh = egui::Button::new(egui::RichText::new("Refresh").size(15.0)).corner_radius(8.0);
                    let installed_any = self.found.iter().any(|f| matches!(f, Found::Installed(_)));
                    if ui.add_enabled(!self.scanning && self.checking == 0, refresh).on_hover_text("Look again (⌘R)").clicked() {
                        self.rescan(&ctx);
                        self.check_latest(&ctx);
                    }
                    let demo = egui::Button::new(egui::RichText::new("Demo the suite").size(15.0)).corner_radius(8.0);
                    if ui.add_enabled(installed_any, demo).on_hover_text("Open every installed app on the Field Station sample").clicked() {
                        self.demo();
                    }
                    let checked = match (self.checking, self.checked) {
                        (0, Some(at)) => format!("Checked {}", ago(at)),
                        (0, None) if !self.settings.check_for_updates => "Update checks are off".to_string(),
                        (0, None) => String::new(),
                        _ => "Checking for updates".to_string(),
                    };
                    ui.label(egui::RichText::new(checked).size(13.0).color(muted));
                });
            });
            if let Some(version) = self.launcher_newer.clone() {
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    ui.label(egui::RichText::new(format!("Method Launcher {version} is available.")).size(14.0));
                    if link(ui, "Download it") {
                        self.link(launcher_core::catalog::LAUNCHER_RELEASES);
                    }
                });
            }
            if let Some((text, error)) = &self.notice {
                ui.add_space(14.0);
                let colour = if *error { ui.visuals().error_fg_color } else { ui.visuals().text_color() };
                ui.label(egui::RichText::new(text).size(14.0).color(colour));
            }
            ui.add_space(24.0);
            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                let columns = if ui.available_width() >= TWO_COLUMNS_FROM { 2 } else { 1 };
                for (row, chunk) in APPS.chunks(columns).enumerate() {
                    if row > 0 {
                        ui.add_space(CARD_GAP);
                    }
                    let detail = (0..chunk.len()).any(|col| self.has_detail(row * columns + col));
                    ui.columns(columns, |cols| {
                        for (col, (ui, app)) in cols.iter_mut().zip(chunk).enumerate() {
                            self.card(ui, row * columns + col, app, detail);
                        }
                    });
                }
                ui.add_space(32.0);
                self.recent_files(ui);
                // Footer: where downloads come from, the update-check switch, and the suite's site.
                ui.add_space(24.0);
                ui.separator();
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Every download comes from the app's own release on GitHub and is checked before anything is installed.")
                            .size(13.0)
                            .color(muted),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if link(ui, "methodproduction.com") {
                            self.link("https://methodproduction.com");
                        }
                    });
                });
                ui.add_space(8.0);
                let mut on = self.settings.check_for_updates;
                if ui.checkbox(&mut on, egui::RichText::new("Check for updates automatically").size(13.0).color(muted)).changed() {
                    self.settings.check_for_updates = on;
                    if let Err(e) = self.settings.save() {
                        self.notice = Some((format!("The setting couldn't be saved. {e}"), true));
                    }
                    if on && self.checked.is_none() {
                        self.check_latest(&ctx);
                    }
                }
            });
        });
    }
}

/// A small rounded tag (spec §3.1): in the app's accent when there is news, quiet otherwise.
fn badge(ui: &mut egui::Ui, badge: &Badge, app: &App) {
    let dark = ui.visuals().dark_mode;
    let (text, fg, bg) = match badge {
        Badge::Behind(version) => {
            let accent = colour(if dark { app.accent.1 } else { app.accent.0 });
            (format!("{version} available"), accent, accent.gamma_multiply(if dark { 0.22 } else { 0.12 }))
        }
        Badge::UpToDate => ("Up to date".to_string(), ui.visuals().weak_text_color(), ui.visuals().extreme_bg_color),
        Badge::InDevelopment => ("In development".to_string(), ui.visuals().weak_text_color(), ui.visuals().extreme_bg_color),
    };
    egui::Frame::new().fill(bg).corner_radius(10.0).inner_margin(egui::Margin::symmetric(9, 3)).show(ui, |ui| {
        ui.label(egui::RichText::new(text).size(12.5).strong().color(fg));
    });
}

/// Text that wraps to at most two lines, ending in "…" if it runs on.
fn two_lines(ui: &mut egui::Ui, text: &str, size: f32, colour: egui::Color32) {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_string(),
        egui::TextFormat { font_id: egui::FontId::proportional(size), color: colour, ..Default::default() },
    );
    job.wrap = egui::text::TextWrapping { max_width: ui.available_width(), max_rows: 2, break_anywhere: false, overflow_character: Some('…') };
    ui.label(job);
}

/// A quiet text link: muted, darker on hover, with the pointing hand.
fn link(ui: &mut egui::Ui, text: &str) -> bool {
    let muted = ui.visuals().weak_text_color();
    let response = ui.add(egui::Label::new(egui::RichText::new(text).size(14.0).color(muted)).sense(egui::Sense::click()));
    let hovered = response.hovered();
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if hovered {
        let rect = response.rect;
        let y = rect.bottom() - 1.0;
        ui.painter().line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(1.0, muted));
    }
    response.clicked()
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

/// A folder with the home folder shown as `~`.
fn shorten_home(path: &std::path::Path) -> String {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    match home.as_deref().and_then(|h| path.strip_prefix(h).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display()),
        None => path.display().to_string(),
    }
}

/// When a file was opened, as a day: "Today", "Yesterday", or a date.
fn day(at: std::time::SystemTime) -> String {
    let secs = |t: std::time::SystemTime| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (then, now) = (secs(at).div_euclid(86_400), secs(std::time::SystemTime::now()).div_euclid(86_400));
    match now - then {
        0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        _ => {
            // Days since 1970 back to a date (Howard Hinnant's civil_from_days).
            let z = then + 719_468;
            let era = z.div_euclid(146_097);
            let doe = z - era * 146_097;
            let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let d = doy - (153 * mp + 2) / 5 + 1;
            let m = if mp < 10 { mp + 3 } else { mp - 9 };
            let y = yoe + era * 400 + i64::from(m <= 2);
            launcher_core::release::short_date((y as u32, m as u32, d as u32))
        }
    }
}

/// The launcher's own settings, in `settings.json` in its data folder.
#[derive(Clone, Debug, PartialEq)]
struct Settings {
    /// SUITE-STANDARDS.md §1: on by default; Refresh still checks by hand.
    check_for_updates: bool,
}

impl Settings {
    fn path() -> Option<PathBuf> {
        System.data().map(|d| d.join("settings.json"))
    }

    fn load() -> Settings {
        let json = Settings::path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        Settings { check_for_updates: value.get("check_for_updates").and_then(|v| v.as_bool()).unwrap_or(true) }
    }

    fn save(&self) -> Result<(), String> {
        let path = Settings::path().ok_or("no data folder")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::json!({ "check_for_updates": self.check_for_updates });
        std::fs::write(&path, format!("{json:#}\n")).map_err(|e| e.to_string())
    }
}

/// How long ago, in words.
fn ago(at: std::time::SystemTime) -> String {
    let minutes = at.elapsed().map(|d| d.as_secs() / 60).unwrap_or(0);
    match minutes {
        0 => "just now".to_string(),
        1 => "a minute ago".to_string(),
        2..=59 => format!("{minutes} minutes ago"),
        60..=119 => "an hour ago".to_string(),
        _ => format!("{} hours ago", minutes / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn days_read_as_dates() {
        assert_eq!(day(std::time::SystemTime::now()), "Today");
        assert_eq!(day(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_772_712_000)), "5 Mar 2026");
    }

    #[test]
    fn home_is_shortened() {
        if let Some(home) = std::env::var_os("HOME") {
            let path = PathBuf::from(home).join("Films");
            assert_eq!(shorten_home(&path), format!("~{}Films", std::path::MAIN_SEPARATOR));
        }
        assert_eq!(shorten_home(std::path::Path::new("/Volumes/Films")), "/Volumes/Films");
    }

    #[test]
    fn colours_parse() {
        assert_eq!(colour("#0E7468"), egui::Color32::from_rgb(0x0E, 0x74, 0x68));
        assert_eq!(colour("nope"), egui::Color32::GRAY);
    }
}
