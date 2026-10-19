//! The Method Production Suite's look (the owner, 2026-10-19: "the method launcher should
//! carry that branding"): the suite's neutrals, light and dark, value for value from the
//! website's `shared/tokens-base.css`, the system's own sans-serif as the sites use, and the
//! launcher's ochre for its own main buttons and focus only, so it reads as the suite first.

use egui::Color32;

/// The suite's tokens for one appearance.
pub struct Tokens {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_2: Color32,
    pub ink: Color32,
    pub ink_soft: Color32,
    pub muted: Color32,
    pub rule: Color32,
    pub rule_strong: Color32,
    /// The launcher's own accent.
    pub accent: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub const LIGHT: Tokens = Tokens {
    bg: rgb(0xF7F6F2),
    surface: rgb(0xFFFFFF),
    surface_2: rgb(0xEFEDE7),
    ink: rgb(0x1C2228),
    ink_soft: rgb(0x3A424C),
    muted: rgb(0x59626D),
    rule: rgb(0xDEDBD3),
    rule_strong: rgb(0x8F897C),
    accent: rgb(0x9A5B00),
};

pub const DARK: Tokens = Tokens {
    bg: rgb(0x111417),
    surface: rgb(0x1B1F24),
    surface_2: rgb(0x22272E),
    ink: rgb(0xE4E8EC),
    ink_soft: rgb(0xC3CAD2),
    muted: rgb(0x9AA3AD),
    rule: rgb(0x2B3138),
    rule_strong: rgb(0x67717D),
    accent: rgb(0xF0B54A),
};

/// The suite's colours on egui's light and dark visuals; egui follows the system's choice.
pub fn apply(ctx: &egui::Context) {
    for (theme, t) in [(egui::Theme::Light, &LIGHT), (egui::Theme::Dark, &DARK)] {
        ctx.style_mut_of(theme, |style| {
            // Buttons sized by their padding, so their words sit in the middle.
            style.spacing.button_padding = egui::vec2(16.0, 8.0);
            let v = &mut style.visuals;
            v.panel_fill = t.bg;
            v.window_fill = t.surface;
            v.faint_bg_color = t.surface;
            v.extreme_bg_color = t.surface_2;
            v.code_bg_color = t.surface_2;
            v.window_stroke.color = t.rule;
            v.override_text_color = Some(t.ink);
            v.weak_text_color = Some(t.muted);
            v.hyperlink_color = t.ink_soft;
            v.selection.bg_fill = t.accent.gamma_multiply(0.35);
            v.selection.stroke.color = t.accent;
            v.widgets.noninteractive.bg_stroke.color = t.rule;
            v.widgets.noninteractive.fg_stroke.color = t.ink;
            v.widgets.inactive.weak_bg_fill = t.surface_2;
            v.widgets.inactive.bg_fill = t.surface_2;
            v.widgets.inactive.fg_stroke.color = t.ink;
            v.widgets.hovered.weak_bg_fill = t.surface_2;
            v.widgets.hovered.bg_stroke.color = t.rule_strong;
            v.widgets.hovered.fg_stroke.color = t.ink;
            v.widgets.active.bg_stroke.color = t.accent;
            v.widgets.active.fg_stroke.color = t.ink;
            // Focus is drawn in the launcher's accent.
            v.widgets.open.bg_stroke.color = t.accent;
        });
    }
}

/// The system's own sans-serif, as the sites ask for it (`-apple-system`, "Segoe UI",
/// system-ui), ahead of egui's built-in fonts, which stay as the fallback for anything it
/// lacks. Nothing is bundled: the font is read from the system where it is.
pub fn use_system_font(ctx: &egui::Context) {
    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &["/System/Library/Fonts/SFNS.ttf", "/System/Library/Fonts/HelveticaNeue.ttc"]
    } else if cfg!(target_os = "windows") {
        &[r"C:\Windows\Fonts\segoeui.ttf"]
    } else {
        &[
            "/usr/share/fonts/truetype/ubuntu/Ubuntu[wdth,wght].ttf",
            "/usr/share/fonts/opentype/cantarell/Cantarell-VF.otf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
    };
    let Some(bytes) = candidates.iter().find_map(|path| std::fs::read(path).ok()) else { return };
    let mut fonts = egui::FontDefinitions::default();
    // The system fonts sit higher in their line than egui's own; this centres them in buttons.
    let tweak = egui::FontTweak { y_offset_factor: 0.12, ..Default::default() };
    fonts.font_data.insert("system".to_owned(), std::sync::Arc::new(egui::FontData::from_owned(bytes).tweak(tweak)));
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.insert(0, "system".to_owned());
    }
    ctx.set_fonts(fonts);
}
