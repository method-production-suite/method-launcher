//! Pictures of the window drawn offscreen with egui_kittest: no window, no focus and no input
//! on anyone's screen. Shows what this computer has installed. Written to
//! `LAUNCHER_SCREENS_OUT` (default `target/screens`); nothing is committed. Run with
//! `cargo test -p method-launcher --release screens -- --ignored`.

use crate::Launcher;
use egui_kittest::Harness;
use launcher_core::catalog::APPS;
use std::path::{Path, PathBuf};

const SIZE: egui::Vec2 = egui::vec2(960.0, 640.0);

#[test]
#[ignore = "pictures, run by hand"]
fn screens() {
    let out = std::env::var_os("LAUNCHER_SCREENS_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screens"));
    std::fs::create_dir_all(&out).unwrap();
    for (theme, name) in [(egui::Theme::Light, "apps-light.png"), (egui::Theme::Dark, "apps-dark.png")] {
        let mut harness = Harness::builder()
            .with_size(SIZE)
            .with_pixels_per_point(2.0)
            .with_theme(theme)
            .with_max_steps(64)
            .build_eframe(|cc| Launcher::new(&cc.egui_ctx));
        // Wait for the search for installed apps, then draw.
        for _ in 0..500 {
            harness.run_steps(1);
            if !harness.state().scanning && harness.state().latest.iter().zip(APPS).all(|(l, a)| !a.released || *l != crate::state::Latest::Unknown) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        harness.run_steps(4);
        let image = harness.render().unwrap();
        let path = out.join(name);
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), image.width(), image.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(image.as_raw()).unwrap();
        println!("{}", path.display());
    }
}
