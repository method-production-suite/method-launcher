//! Pictures: the launcher's own icon, each app's mark, and PNG decoding.

use launcher_core::catalog::App;

/// The launcher's icon, the suite's mark, drawn by scripts/make-icons.py.
pub const WINDOW_ICON: &[u8] = include_bytes!("../../../packaging/icons/icon-256.png");

/// The suite's wordmark for the header, light and dark, rendered by scripts/make-icons.py
/// from assets/brand/wordmark.svg and wordmark-dark.svg.
pub const WORDMARK: &[u8] = include_bytes!("../../../assets/brand/wordmark.png");
pub const WORDMARK_DARK: &[u8] = include_bytes!("../../../assets/brand/wordmark-dark.png");

/// Each app's mark (its site's touch icon), drawn when its own icon can't be read.
pub fn mark_png(app: &App) -> &'static [u8] {
    match app.slug {
        "storyteller" => include_bytes!("../../../assets/apps/storyteller.png"),
        "reader" => include_bytes!("../../../assets/apps/reader.png"),
        "schedule" => include_bytes!("../../../assets/apps/schedule.png"),
        _ => include_bytes!("../../../assets/apps/budget.png"),
    }
}

/// A PNG as an image egui can draw; `None` if it isn't a PNG it can read.
pub fn decode_png(bytes: &[u8]) -> Option<egui::ColorImage> {
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

/// An installed app's icon cut to its visible shape, so it draws the same size as the
/// full-bleed marks: macOS icons leave a transparent margin (about a tenth on each side) and
/// a soft shadow, and this keeps only what is at least half opaque, squared about its centre.
pub fn trim(image: egui::ColorImage) -> egui::ColorImage {
    let [w, h] = image.size;
    let opaque = |x: usize, y: usize| image.pixels.get(y * w + x).is_some_and(|p| p.a() >= 128);
    let rows: Vec<usize> = (0..h).filter(|&y| (0..w).any(|x| opaque(x, y))).collect();
    let cols: Vec<usize> = (0..w).filter(|&x| (0..h).any(|y| opaque(x, y))).collect();
    let (Some(&top), Some(&bottom), Some(&left), Some(&right)) = (rows.first(), rows.last(), cols.first(), cols.last()) else {
        return image;
    };
    let (width, height) = (right - left + 1, bottom - top + 1);
    let side = width.max(height);
    // The shorter side is widened evenly on both ends.
    let x0 = left.saturating_sub((side - width) / 2);
    let y0 = top.saturating_sub((side - height) / 2);
    let mut pixels = Vec::with_capacity(side * side);
    for y in y0..y0 + side {
        for x in x0..x0 + side {
            pixels.push(if x < w && y < h { image.pixels.get(y * w + x).copied().unwrap_or_default() } else { egui::Color32::TRANSPARENT });
        }
    }
    egui::ColorImage::new([side, side], pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_the_transparent_margin() {
        // A 10 by 10 picture with an opaque 4 by 4 square at (3, 2) and a faint shadow below.
        let mut pixels = vec![egui::Color32::TRANSPARENT; 100];
        for y in 2..6 {
            for x in 3..7 {
                pixels[y * 10 + x] = egui::Color32::RED;
            }
        }
        pixels[7 * 10 + 4] = egui::Color32::from_black_alpha(40);
        let image = egui::ColorImage::new([10, 10], pixels);
        let trimmed = trim(image);
        assert_eq!(trimmed.size, [4, 4]);
        assert!(trimmed.pixels.iter().all(|p| *p == egui::Color32::RED));
    }

    #[test]
    fn leaves_a_fully_clear_picture_alone() {
        let image = egui::ColorImage::new([3, 3], vec![egui::Color32::TRANSPARENT; 9]);
        assert_eq!(trim(image).size, [3, 3]);
    }

    #[test]
    fn decodes_the_bundled_pictures() {
        for app in &launcher_core::catalog::APPS {
            assert!(decode_png(mark_png(app)).is_some(), "{}", app.slug);
        }
        for png in [WINDOW_ICON, WORDMARK, WORDMARK_DARK] {
            assert!(decode_png(png).is_some());
        }
    }
}
