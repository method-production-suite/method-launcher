//! Method Launcher's window (docs/spec.md §3). Today it shows the catalog; installing,
//! opening and the rest arrive one pull request at a time (docs/ROADMAP.md).

use launcher_core::catalog::{APPS, App};
use launcher_core::platform::Target;

const APP_NAME: &str = "Method Launcher";

fn main() -> eframe::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("method-launcher-app {} ({APP_NAME})", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let viewport = egui::ViewportBuilder::default().with_title(APP_NAME).with_inner_size([960.0, 640.0]).with_min_inner_size([720.0, 520.0]);
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(APP_NAME, options, Box::new(|_cc| Ok(Box::new(Launcher))))
}

struct Launcher;

/// `#RRGGBB` to a colour; grey for anything else (the catalog's tests keep that from happening).
fn colour(hex: &str) -> egui::Color32 {
    let channel = |at: usize| hex.get(at..at + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
    match (channel(1), channel(3), channel(5)) {
        (Some(r), Some(g), Some(b)) => egui::Color32::from_rgb(r, g, b),
        _ => egui::Color32::GRAY,
    }
}

fn card(ui: &mut egui::Ui, app: &App) {
    let accent = colour(if ui.visuals().dark_mode { app.accent.1 } else { app.accent.0 });
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 6.0, accent);
            ui.vertical(|ui| {
                ui.heading(app.name);
                ui.label(app.tagline);
                let buildable = Target::current().is_some_and(|t| !t.kinds().is_empty());
                ui.weak(match (app.released, buildable) {
                    (false, _) => "In development. No download yet.".to_string(),
                    (true, false) => format!("There's no {} for this system yet. You can build it from source.", app.name),
                    (true, true) => "Not installed".to_string(),
                });
            });
        });
    });
}

impl eframe::App for Launcher {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(8.0);
            ui.label("Install the apps you want. Each one updates itself.");
            ui.add_space(8.0);
            for app in &APPS {
                card(ui, app);
                ui.add_space(6.0);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_parse() {
        assert_eq!(colour("#0E7468"), egui::Color32::from_rgb(0x0E, 0x74, 0x68));
        assert_eq!(colour("nope"), egui::Color32::GRAY);
    }
}
