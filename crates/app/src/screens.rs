//! The README and site pictures (SUITE-STANDARDS.md §7.1), drawn offscreen with egui_kittest:
//! no window, no focus and no input on anyone's screen. The four shots, light and dark, at
//! 1280 by 800, from an invented state (Field Station, never what this computer has), so
//! every run gives the same pictures:
//!
//! 1. `apps`: the suite's apps, three installed and one to install.
//! 2. `updates`: two apps with a new version, and Update all.
//! 3. `install`: an app downloading, checked before anything is copied.
//! 4. `recent`: the files opened lately across the suite, each opening in its own app (all
//!    from today and yesterday, so the dates read the same on any day).
//!
//! Written to `assets/screens/<name>-<theme>.png` and `site/img/` (or `LAUNCHER_SCREENS_OUT`
//! alone). Run with `cargo test -p method-launcher --release screens -- --ignored`.

use crate::Launcher;
use crate::state::{Found, Job, Latest};
use egui_kittest::Harness;
use launcher_core::platform::Installed;
use launcher_core::release::Release;
use launcher_sys::Stage;
use std::path::{Path, PathBuf};

const SIZE: egui::Vec2 = egui::vec2(1280.0, 800.0);

fn release(version: &str, day: u32, summary: &str) -> Latest {
    Latest::Release(Release { version: version.into(), date: Some((2026, 10, day)), summary: Some(summary.into()) })
}

fn installed(app: &launcher_core::catalog::App, version: &str) -> Found {
    Found::Installed(Installed { path: PathBuf::from(format!("/Applications/{}", app.bundle_name())), version: version.into(), legacy: false })
}

/// The invented state for one shot.
fn stage(launcher: &mut Launcher, shot: &str) {
    let apps = launcher.apps;
    let latest = [
        release("0.3.0", 20, "Television scripts laid out in acts."),
        release("0.13.0", 18, "Screenplay PDFs into e-books and editable scripts."),
        release("0.3.0", 20, "One app for every Mac."),
        release("0.6.0", 20, "Fringes and globals explained line by line."),
    ];
    let behind = shot == "updates";
    let versions = [if behind { "0.2.0" } else { "0.3.0" }, "0.13.0", if behind { "0.2.0" } else { "0.3.0" }];
    for (i, app) in apps.iter().enumerate() {
        let found = match (i, shot) {
            (3, "updates" | "recent") => installed(app, "0.6.0"),
            (3, _) => Found::Missing,
            (i, _) => installed(app, versions.get(i).copied().unwrap_or("0.1.0")),
        };
        if let Some(slot) = launcher.found.get_mut(i) {
            *slot = found;
        }
        if let (Some(slot), Some(l)) = (launcher.latest.get_mut(i), latest.get(i)) {
            *slot = l.clone();
        }
        if let Some(slot) = launcher.jobs.get_mut(i) {
            *slot = if shot == "install" && i == 3 { Job::Running(Stage::Downloading(84_200_000)) } else { Job::Idle };
        }
        // Each app's mark, never an icon read from this computer.
        if let Some(slot) = launcher.icons.get_mut(i) {
            *slot = None;
        }
    }
    launcher.checked = Some(std::time::SystemTime::now());
    launcher.notice = None;
    launcher.launcher_newer = None;
    // Invented recent files only: never the real ones on this computer.
    let day = std::time::Duration::from_secs(86_400);
    let now = std::time::SystemTime::now();
    let file = |app: usize, path: &str, ago: u32| (app, launcher_sys::Recent { path: path.into(), opened: now - day * ago });
    launcher.recent = vec![
        file(3, "/Users/you/Films/Field Station/Field Station budget.mthd", 0),
        file(2, "/Users/you/Films/Field Station/Field Station schedule.mtsc", 0),
        file(0, "/Users/you/Films/Field Station/Field Station.fountain", 1),
        file(1, "/Users/you/Films/Field Station/Field Station.pdf", 1),
    ];
}

#[test]
#[ignore = "pictures, run by hand"]
fn screens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let outs: Vec<PathBuf> = match std::env::var_os("LAUNCHER_SCREENS_OUT") {
        Some(dir) => vec![PathBuf::from(dir)],
        None => vec![root.join("assets/screens"), root.join("site/img")],
    };
    for shot in ["apps", "updates", "install", "recent"] {
        for (theme, name) in [(egui::Theme::Light, "light"), (egui::Theme::Dark, "dark")] {
            let mut harness = Harness::builder()
                .with_size(SIZE)
                .with_pixels_per_point(2.0)
                .with_theme(theme)
                .with_max_steps(64)
                .build_eframe(|cc| Launcher::new(&cc.egui_ctx));
            // Let the launcher's own looks finish, so nothing arrives after the state is set.
            for _ in 0..500 {
                harness.run_steps(1);
                if !harness.state().scanning && harness.state().checking == 0 {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            harness.run_steps(4);
            stage(harness.state_mut(), shot);
            harness.run_steps(4);
            if shot == "recent" {
                // Down to the recent files, as a person would scroll.
                harness.input_mut().events.push(egui::Event::PointerMoved(egui::pos2(640.0, 500.0)));
                harness.input_mut().events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -2000.0),
                    modifiers: egui::Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                });
                harness.run_steps(8);
                // No pointer in the picture.
                harness.input_mut().events.push(egui::Event::PointerGone);
                harness.run_steps(4);
            }
            let image = harness.render().unwrap();
            for out in &outs {
                std::fs::create_dir_all(out).unwrap();
                let path = out.join(format!("{shot}-{name}.png"));
                let file = std::fs::File::create(&path).unwrap();
                let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), image.width(), image.height());
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(image.as_raw()).unwrap();
                println!("{}", path.display());
            }
        }
    }
}
