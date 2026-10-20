//! The Help menu's windows (SUITE-STANDARDS.md §4): About, Keyboard Shortcuts, and Licenses
//! and Notices, and the links the menu opens. Nothing here touches the network: links open in
//! the browser, and a bug report is only a prefilled page the person submits, or not.

/// The suite's website.
pub const SUITE_WEBSITE: &str = "https://methodproduction.com";
/// The user guide, `docs/user-guide.md` in the repository.
pub const USER_GUIDE: &str = "https://github.com/method-production-suite/method-launcher/blob/main/docs/user-guide.md";
/// Where a bug report goes.
const NEW_ISSUE: &str = "https://github.com/method-production-suite/method-launcher/issues/new";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const LICENCE: &str = "MIT OR Apache-2.0";
const COPYRIGHT: &str = "Copyright (c) 2026 Method Production Suite Team and the Method Launcher contributors.";

/// Shown in Licenses and Notices, in this order.
const NOTICES: [(&str, &str); 3] = [
    ("Notice", include_str!("../../../NOTICE")),
    ("MIT", include_str!("../../../LICENSE-MIT")),
    ("Apache-2.0", include_str!("../../../LICENSE-APACHE")),
];

/// The release notes page for this version.
pub fn whats_new_url() -> String {
    format!("https://github.com/method-production-suite/method-launcher/releases/tag/v{VERSION}")
}

/// `application/x-www-form-urlencoded`, as browsers write it.
fn form_encode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => out.push(char::from(b)),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The system, as the bug report names it: `macOS (aarch64)`.
pub fn os_label() -> String {
    let os = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    };
    format!("{os} ({})", std::env::consts::ARCH)
}

/// A new-issue page with the version and system filled in. Nothing else goes in it.
pub fn new_issue_url(version: &str, os: &str) -> String {
    let body = format!(
        "<!-- Describe the problem. Name the app it was about, if any. -->\n\n\
         **What happened:**\n\n\n**What you expected:**\n\n\n**Steps to repeat it:**\n\n\n---\nMethod Launcher {version} · {os}\n"
    );
    format!("{NEW_ISSUE}?body={}", form_encode(&body))
}

/// The keyboard shortcuts, as (what it does, the keys on this system).
pub fn shortcuts() -> Vec<(&'static str, String)> {
    let mac = cfg!(target_os = "macos");
    let names = if mac { &egui::ModifierNames::SYMBOLS } else { &egui::ModifierNames::NAMES };
    let refresh = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::R).format(names, mac);
    let settings = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Comma).format(names, mac);
    vec![
        ("Look again for installed apps and new versions", refresh),
        ("Open Settings", settings),
        ("Close Settings or a Help window", "Esc".to_string()),
    ]
}

/// Which Help window is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    About,
    Shortcuts,
    /// With the notice shown.
    Licenses(usize),
}

/// Draws the open window; returns it, or `None` once it is closed (Close or Esc).
pub fn show(ctx: &egui::Context, window: Window) -> Option<Window> {
    let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let next = match window {
        Window::About => about(ctx).then_some(Window::About),
        Window::Shortcuts => shortcuts_window(ctx).then_some(Window::Shortcuts),
        Window::Licenses(tab) => licenses(ctx, tab).map(Window::Licenses),
    };
    next.filter(|_| !escape)
}

fn centered<'a>(window: egui::Window<'a>) -> egui::Window<'a> {
    window.collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
}

fn close_button(ui: &mut egui::Ui) -> bool {
    ui.add_space(10.0);
    ui.button("Close").clicked()
}

fn about(ctx: &egui::Context) -> bool {
    let mut open = true;
    centered(egui::Window::new("About Method Launcher").id(egui::Id::new("launcher.about"))).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.heading("Method Launcher");
        ui.label(format!("Version {VERSION}"));
        ui.add_space(8.0);
        ui.label("Installs, updates and opens the apps of the Method Production Suite.");
        ui.add_space(8.0);
        ui.hyperlink_to("methodproduction.com", SUITE_WEBSITE);
        ui.add_space(8.0);
        let muted = ui.visuals().weak_text_color();
        ui.label(egui::RichText::new(format!("Licensed under {LICENCE}.")).size(12.5).color(muted));
        ui.label(egui::RichText::new(COPYRIGHT).size(12.5).color(muted));
        if close_button(ui) {
            open = false;
        }
    });
    open
}

fn shortcuts_window(ctx: &egui::Context) -> bool {
    let mut open = true;
    let rows = shortcuts();
    centered(egui::Window::new("Keyboard Shortcuts").id(egui::Id::new("launcher.shortcuts"))).show(ctx, |ui| {
        egui::Grid::new("launcher.shortcut_grid").num_columns(2).spacing([24.0, 6.0]).striped(true).show(ui, |ui| {
            for (label, keys) in &rows {
                ui.label(*label);
                ui.monospace(keys);
                ui.end_row();
            }
        });
        if close_button(ui) {
            open = false;
        }
    });
    open
}

fn licenses(ctx: &egui::Context, tab: usize) -> Option<usize> {
    let mut chosen = tab;
    let mut open = true;
    centered(egui::Window::new("Licenses and Notices").id(egui::Id::new("launcher.licenses")).default_size([720.0, 480.0])).show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (n, (name, _)) in NOTICES.iter().enumerate() {
                if ui.selectable_label(n == chosen, *name).clicked() {
                    chosen = n;
                }
            }
        });
        ui.separator();
        let text = NOTICES.get(chosen).map_or("", |(_, text)| text);
        egui::ScrollArea::both().max_height(420.0).auto_shrink([false, true]).show(ui, |ui| {
            // The files are wrapped already; wrapping again would break their lines.
            ui.add(egui::Label::new(egui::RichText::new(text).monospace().size(12.0)).extend());
        });
        if close_button(ui) {
            open = false;
        }
    });
    open.then_some(chosen)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn form_encoding_matches_browsers() {
        assert_eq!(form_encode("C++ a/b"), "C%2B%2B+a%2Fb");
        assert_eq!(form_encode("·"), "%C2%B7");
        assert_eq!(form_encode("a-b_c.d*"), "a-b_c.d*");
    }

    #[test]
    fn a_bug_report_carries_the_version_and_system_only() {
        let url = new_issue_url("0.1.2", "macOS (aarch64)");
        assert!(url.starts_with("https://github.com/method-production-suite/method-launcher/issues/new?body="));
        assert!(url.ends_with("Method+Launcher+0.1.2+%C2%B7+macOS+%28aarch64%29%0A"), "{url}");
        assert!(!url.contains("title="));
    }

    #[test]
    fn the_system_label_names_the_os_and_the_chip() {
        let label = os_label();
        assert!(label.ends_with(&format!("({})", std::env::consts::ARCH)), "{label}");
        assert!(!label.starts_with('('));
    }

    #[test]
    fn the_notices_are_the_repository_files() {
        assert!(NOTICES[0].1.starts_with("Method Launcher"));
        assert!(NOTICES[1].1.contains("MIT License"));
        assert!(NOTICES[2].1.contains("Apache License"));
        assert!(COPYRIGHT.contains("Method Production Suite Team"));
    }

    #[test]
    fn the_links_stay_on_the_suite_s_pages() {
        for url in [SUITE_WEBSITE, USER_GUIDE, NEW_ISSUE, &whats_new_url()] {
            assert!(
                url.starts_with("https://methodproduction.com") || url.starts_with("https://github.com/method-production-suite/method-launcher/"),
                "{url}"
            );
        }
        assert!(whats_new_url().ends_with(&format!("/v{VERSION}")));
    }

    #[test]
    fn every_shortcut_names_its_keys() {
        let rows = shortcuts();
        assert!(rows.iter().all(|(label, keys)| !label.is_empty() && !keys.is_empty()));
        let refresh = if cfg!(target_os = "macos") { "⌘R" } else { "Ctrl+R" };
        assert_eq!(rows[0].1, refresh);
    }
}

/// Each window drawn offscreen with egui_kittest (no window on anyone's screen): it shows what
/// it says, and Close shuts it.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod drawn {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    fn harness(window: Window) -> Harness<'static, Option<Window>> {
        Harness::builder().with_size(egui::vec2(900.0, 700.0)).build_ui_state(
            |ui, open: &mut Option<Window>| {
                if let Some(w) = *open {
                    *open = show(ui.ctx(), w);
                }
            },
            Some(window),
        )
    }

    #[test]
    fn about_names_the_version_and_licence() {
        let mut h = harness(Window::About);
        h.run();
        h.get_by_label(&format!("Version {VERSION}"));
        h.get_by_label(&format!("Licensed under {LICENCE}."));
        h.get_by_label("Close").click();
        h.run();
        assert_eq!(*h.state(), None);
    }

    #[test]
    fn the_shortcuts_window_lists_refresh() {
        let mut h = harness(Window::Shortcuts);
        h.run();
        h.get_by_label("Look again for installed apps and new versions");
        h.get_by_label(&shortcuts()[0].1);
    }

    #[test]
    fn licenses_switch_between_the_files() {
        let mut h = harness(Window::Licenses(0));
        h.run();
        h.get_by_label("Apache-2.0").click();
        h.run();
        assert_eq!(*h.state(), Some(Window::Licenses(2)));
        h.key_press(egui::Key::Escape);
        h.run();
        assert_eq!(*h.state(), None);
    }
}
