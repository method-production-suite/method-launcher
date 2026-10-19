//! Method Launcher's window (docs/spec.md §3): one card per suite app, showing whether it is
//! installed here and at which version, with Open and Show in Finder for the ones that are.
//! Installing arrives with milestone M2 (docs/ROADMAP.md); until then a card for an app that
//! isn't installed links to its download page.

use launcher_core::catalog::{APPS, App};
use launcher_core::platform::{Installed, Launch, Locate, Opened, Target};
use launcher_sys::System;
use std::sync::{Arc, mpsc};

#[cfg(test)]
mod screens;

const APP_NAME: &str = "Method Launcher";

/// Wide enough for two cards side by side.
const TWO_COLUMNS_FROM: f32 = 860.0;

fn main() -> eframe::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("method-launcher-app {} ({APP_NAME})", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let mut viewport = egui::ViewportBuilder::default().with_title(APP_NAME).with_inner_size([960.0, 640.0]).with_min_inner_size([720.0, 520.0]);
    if let Some(icon) = decode_png(WINDOW_ICON) {
        viewport =
            viewport.with_icon(Arc::new(egui::IconData { width: icon.width() as u32, height: icon.height() as u32, rgba: icon.as_raw().to_vec() }));
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(APP_NAME, options, Box::new(|cc| Ok(Box::new(Launcher::new(&cc.egui_ctx)))))
}

/// The launcher's own icon, drawn by scripts/make-icons.py.
const WINDOW_ICON: &[u8] = include_bytes!("../../../packaging/icons/icon-256.png");

/// Each app's mark (its site's touch icon), drawn when its own icon can't be read.
fn mark_png(app: &App) -> &'static [u8] {
    match app.slug {
        "storyteller" => include_bytes!("../../../assets/apps/storyteller.png"),
        "reader" => include_bytes!("../../../assets/apps/reader.png"),
        "schedule" => include_bytes!("../../../assets/apps/schedule.png"),
        _ => include_bytes!("../../../assets/apps/budget.png"),
    }
}

/// A PNG as an image egui can draw; `None` if it isn't a PNG it can read.
fn decode_png(bytes: &[u8]) -> Option<egui::ColorImage> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut buffer).ok()?;
    let pixels = buffer.get(..frame.buffer_size())?;
    let rgba: Vec<u8> = match frame.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels
            .chunks_exact(3)
            .flat_map(|p| match *p {
                [r, g, b] => [r, g, b, 255],
                _ => [0; 4],
            })
            .collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .chunks_exact(2)
            .flat_map(|p| match *p {
                [g, a] => [g, g, g, a],
                _ => [0; 4],
            })
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    let size = [frame.width as usize, frame.height as usize];
    (rgba.len() == size[0] * size[1] * 4).then(|| egui::ColorImage::from_rgba_unmultiplied(size, &rgba))
}

/// What the launcher knows about one app on this computer.
#[derive(Clone, Debug, PartialEq)]
enum Found {
    Looking,
    Installed(Installed),
    Missing,
}

/// One app as a search found it: where it is, and its own icon if it could be read.
type Scanned = (Option<Installed>, Option<egui::ColorImage>);

struct Launcher {
    found: Vec<Found>,
    scan: Option<mpsc::Receiver<Vec<Scanned>>>,
    /// Each app's mark, and its own icon once found installed, ready to draw.
    marks: Vec<Option<egui::TextureHandle>>,
    icons: Vec<Option<egui::TextureHandle>>,
    /// One line about the last thing the person did, and whether it went wrong.
    notice: Option<(String, bool)>,
    focused: bool,
}

impl Launcher {
    fn new(ctx: &egui::Context) -> Self {
        let marks = APPS
            .iter()
            .map(|app| decode_png(mark_png(app)).map(|image| ctx.load_texture(format!("mark-{}", app.slug), image, egui::TextureOptions::LINEAR)))
            .collect();
        let mut launcher =
            Launcher { found: vec![Found::Looking; APPS.len()], scan: None, marks, icons: vec![None; APPS.len()], notice: None, focused: true };
        launcher.rescan(ctx);
        launcher
    }

    /// Looks for the apps again, off the interface thread.
    fn rescan(&mut self, ctx: &egui::Context) {
        if self.scan.is_some() {
            return;
        }
        let (send, receive) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let found: Vec<Scanned> = APPS
                .iter()
                .map(|app| {
                    let installed = System.locate(app);
                    let icon = installed.as_ref().and_then(|i| System.app_icon(app, i)).and_then(|png| decode_png(&png));
                    (installed, icon)
                })
                .collect();
            let _ = send.send(found);
            ctx.request_repaint();
        });
        self.scan = Some(receive);
    }

    fn take_scan(&mut self, ctx: &egui::Context) {
        let Some(receive) = &self.scan else { return };
        if let Ok(found) = receive.try_recv() {
            self.icons = APPS
                .iter()
                .zip(&found)
                .map(|(app, (_, icon))| icon.clone().map(|image| ctx.load_texture(format!("icon-{}", app.slug), image, egui::TextureOptions::LINEAR)))
                .collect();
            self.found = found.into_iter().map(|(f, _)| f.map_or(Found::Missing, Found::Installed)).collect();
            self.scan = None;
        }
    }

    fn report(&mut self, result: Result<String, String>) {
        self.notice = Some(match result {
            Ok(text) => (text, false),
            Err(text) => (text, true),
        });
    }

    fn open(&mut self, app: &App, installed: &Installed) {
        let result = System.open_app(app, installed, &[]).map(|opened| match opened {
            Opened::Started => format!("Opening {}.", app.name),
            Opened::BroughtForward => format!("{} is already open.", app.name),
            Opened::AlreadyRunning => format!("{} is already open. Switch to it from the taskbar.", app.name),
        });
        self.report(result.map_err(|e| format!("{} couldn't be opened. {e}", app.name)));
    }

    fn link(&mut self, url: &str) {
        if let Err(e) = System.open_url(url) {
            self.report(Err(format!("The link couldn't be opened. {e}")));
        }
    }

    fn card(&mut self, ui: &mut egui::Ui, index: usize, app: &App) {
        let accent = colour(if ui.visuals().dark_mode { app.accent.1 } else { app.accent.0 });
        let found = self.found.get(index).cloned().unwrap_or(Found::Looking);
        let picture = self.icons.get(index).cloned().flatten().or_else(|| self.marks.get(index).cloned().flatten());
        egui::Frame::group(ui.style()).inner_margin(12.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let size = egui::vec2(48.0, 48.0);
                match &picture {
                    Some(texture) => {
                        ui.add(egui::Image::new((texture.id(), size)).corner_radius(10.0));
                    }
                    None => {
                        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                        ui.painter().rect_filled(rect, 10.0, accent);
                    }
                }
                ui.vertical(|ui| {
                    ui.heading(app.name);
                    ui.label(app.tagline);
                });
            });
            ui.add_space(6.0);
            ui.weak(status_line(app, &found));
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| match &found {
                Found::Looking => {
                    ui.spinner();
                }
                Found::Installed(installed) => {
                    if ui.button("Open").clicked() {
                        self.open(app, installed);
                    }
                    if ui.button(SHOW_LOCATION).clicked()
                        && let Err(e) = System.show_location(&installed.path)
                    {
                        self.report(Err(format!("{} couldn't be shown. {e}", app.name)));
                    }
                    if app.released && ui.link("Release notes").clicked() {
                        self.link(&app.releases_url());
                    }
                }
                Found::Missing if !app.released => {
                    if ui.button("Visit the site").clicked() {
                        self.link(app.site);
                    }
                }
                Found::Missing if !buildable_here() => {
                    if ui.button("Build from source").clicked() {
                        self.link(&app.source_url());
                    }
                    if ui.link("Visit the site").clicked() {
                        self.link(app.site);
                    }
                }
                Found::Missing => {
                    if ui.button("Download page").clicked() {
                        self.link(&app.releases_url());
                    }
                    if ui.link("Visit the site").clicked() {
                        self.link(app.site);
                    }
                }
            });
        });
    }
}

#[cfg(target_os = "macos")]
const SHOW_LOCATION: &str = "Show in Finder";
#[cfg(target_os = "windows")]
const SHOW_LOCATION: &str = "Show in Explorer";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const SHOW_LOCATION: &str = "Show in Files";

/// Whether the suite publishes app builds for this system (not yet for Linux on ARM).
fn buildable_here() -> bool {
    Target::current().is_some_and(|t| !t.kinds().is_empty())
}

/// The quiet line under an app's name.
fn status_line(app: &App, found: &Found) -> String {
    match found {
        Found::Looking => "Looking for it on this computer".to_string(),
        Found::Installed(installed) => {
            let version = if installed.version.is_empty() { "Installed".to_string() } else { format!("Version {}", installed.version) };
            match (installed.legacy, app.legacy) {
                (true, Some(legacy)) => format!("{version}, installed as {}", legacy.name),
                _ => version,
            }
        }
        Found::Missing if !app.released => "In development. No download yet.".to_string(),
        Found::Missing if !buildable_here() => format!("There's no {} for this system yet. You can build it from source.", app.name),
        Found::Missing => "Not installed".to_string(),
    }
}

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
        self.take_scan(&ctx);
        // Look again when the window comes forward: an app may have been installed meanwhile.
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        if focused && !self.focused {
            self.rescan(&ctx);
        }
        self.focused = focused;
        if ctx.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::R))) {
            self.rescan(&ctx);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Install the apps you want. Each one updates itself.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_enabled(self.scan.is_none(), egui::Button::new("Refresh")).clicked() {
                        self.rescan(&ctx);
                    }
                });
            });
            if let Some((text, error)) = &self.notice {
                ui.add_space(4.0);
                if *error {
                    ui.colored_label(ui.visuals().error_fg_color, text);
                } else {
                    ui.label(text);
                }
            }
            ui.add_space(8.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                let columns = if ui.available_width() >= TWO_COLUMNS_FROM { 2 } else { 1 };
                for (row, chunk) in APPS.chunks(columns).enumerate() {
                    ui.columns(columns, |cols| {
                        for (col, (ui, app)) in cols.iter_mut().zip(chunk).enumerate() {
                            self.card(ui, row * columns + col, app);
                        }
                    });
                    ui.add_space(8.0);
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use launcher_core::catalog::by_slug;
    use std::path::PathBuf;

    #[test]
    fn colours_parse() {
        assert_eq!(colour("#0E7468"), egui::Color32::from_rgb(0x0E, 0x74, 0x68));
        assert_eq!(colour("nope"), egui::Color32::GRAY);
    }

    #[test]
    fn status_lines_say_what_is_installed() {
        let reader = by_slug("reader").unwrap();
        let legacy =
            Found::Installed(Installed { path: PathBuf::from("/Applications/Screepub Desktop.app"), version: "0.12.1".into(), legacy: true });
        assert_eq!(status_line(reader, &legacy), "Version 0.12.1, installed as Screepub Desktop");
        let budget = by_slug("budget").unwrap();
        let unknown = Found::Installed(Installed { path: PathBuf::from("x"), version: String::new(), legacy: false });
        assert_eq!(status_line(budget, &unknown), "Installed");
        assert_eq!(status_line(by_slug("storyteller").unwrap(), &Found::Missing), "In development. No download yet.");
    }
}
