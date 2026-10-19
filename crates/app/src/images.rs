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
