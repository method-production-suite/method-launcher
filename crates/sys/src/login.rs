//! "Open the launcher when I log in" (docs/spec.md §3.4), the way each system does it for one
//! person, with no administrator: a LaunchAgent on macOS, an autostart entry on Linux, and a
//! value under the per-user Run key on Windows. The system's own record is the setting, so
//! Settings shows what will really happen. The files' contents are made here, tested on every
//! system.
#![cfg_attr(not(any(target_os = "macos", target_os = "linux", target_os = "windows")), allow(dead_code))]

use launcher_core::catalog::LAUNCHER_ID;
use std::path::{Path, PathBuf};

/// The LaunchAgent's label and file name.
pub const AGENT: &str = "com.methodlauncher.desktop.login";
/// The Run key's value name on Windows.
pub const RUN_NAME: &str = "Method Launcher";
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

fn xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// macOS: opens the launcher's bundle at login, as Finder would (`open -a`).
pub fn launch_agent(bundle: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n\
         \t<key>Label</key>\n\t<string>{AGENT}</string>\n\
         \t<key>ProgramArguments</key>\n\t<array>\n\t\t<string>/usr/bin/open</string>\n\t\t<string>-a</string>\n\t\t<string>{}</string>\n\t</array>\n\
         \t<key>RunAtLoad</key>\n\t<true/>\n\
         </dict>\n</plist>\n",
        xml(bundle)
    )
}

/// Linux: the autostart entry (the freedesktop autostart folder) for the AppImage or binary.
pub fn autostart_entry(exec: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Method Launcher\nComment=Opens Method Launcher when you log in\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\nX-Method-Launcher=login\n",
        crate::parse::desktop_exec_quote(exec)
    )
}

/// Windows: the Run value, the program quoted so a path with spaces stays one.
pub fn run_value(exe: &str) -> String {
    format!("\"{}\"", exe.replace('"', ""))
}

/// The bundle a macOS executable is in (`…/Method Launcher.app/Contents/MacOS/x` gives the
/// `.app`), or the executable itself when it isn't in one (a build being tried out).
pub fn mac_target(exe: &Path) -> PathBuf {
    exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).map_or_else(|| exe.to_path_buf(), Path::to_path_buf)
}

/// What opens at login on this system: the bundle, the AppImage (`$APPIMAGE` while one runs)
/// or the program.
fn target() -> Result<PathBuf, String> {
    if cfg!(target_os = "linux")
        && let Some(appimage) = crate::env_path("APPIMAGE")
    {
        return Ok(appimage);
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(if cfg!(target_os = "macos") { mac_target(&exe) } else { exe })
}

/// The file that turns it on, on macOS and Linux.
fn file() -> Option<PathBuf> {
    let home = crate::env_path("HOME")?;
    if cfg!(target_os = "macos") {
        Some(home.join("Library/LaunchAgents").join(format!("{AGENT}.plist")))
    } else {
        let config = crate::env_path("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config"));
        Some(config.join("autostart").join(format!("{LAUNCHER_ID}.desktop")))
    }
}

/// Whether the launcher opens at login now.
pub fn enabled() -> bool {
    if cfg!(target_os = "windows") {
        return crate::output(&reg(), &["query", RUN_KEY, "/v", RUN_NAME]).is_some();
    }
    file().is_some_and(|f| f.is_file())
}

/// While opening at login is on, points it at the launcher running now (it may have moved, or
/// a new AppImage may have a new name). Best effort, at launch.
pub fn refresh() {
    if enabled() {
        let _ = set(true);
    }
}

/// Turns opening at login on or off.
pub fn set(on: bool) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        let reg = reg();
        let mut command = std::process::Command::new(&reg);
        if on {
            let value = run_value(&target()?.to_string_lossy());
            command.args(["add", RUN_KEY, "/v", RUN_NAME, "/t", "REG_SZ", "/d", &value, "/f"]);
        } else if enabled() {
            command.args(["delete", RUN_KEY, "/v", RUN_NAME, "/f"]);
        } else {
            return Ok(());
        }
        return crate::run(&mut command);
    }
    let file = file().ok_or("your home folder couldn't be found")?;
    if !on {
        return match std::fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        };
    }
    let target = target()?.to_string_lossy().into_owned();
    let text = if cfg!(target_os = "macos") { launch_agent(&target) } else { autostart_entry(&target) };
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file, text).map_err(|e| e.to_string())
}

fn reg() -> String {
    crate::env_path("SystemRoot").unwrap_or_else(|| PathBuf::from(r"C:\Windows")).join(r"System32\reg.exe").to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_launch_agent_opens_the_bundle_at_login() {
        let plist = launch_agent("/Applications/Method Launcher.app");
        assert!(plist.contains(&format!("<string>{AGENT}</string>")));
        assert!(plist.contains("<string>/usr/bin/open</string>\n\t\t<string>-a</string>\n\t\t<string>/Applications/Method Launcher.app</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>\n\t<true/>"));
        assert!(launch_agent("/Users/a&b/<x>.app").contains("/Users/a&amp;b/&lt;x&gt;.app"));
    }

    /// The system's own check of the LaunchAgent, written to a scratch file (never to the
    /// person's LaunchAgents folder).
    #[cfg(target_os = "macos")]
    #[test]
    fn plutil_accepts_the_launch_agent() {
        let path = std::env::temp_dir().join(format!("launcher-agent-{}.plist", std::process::id()));
        std::fs::write(&path, launch_agent("/Applications/Method Launcher.app")).unwrap();
        let ok = std::process::Command::new("/usr/bin/plutil").arg("-lint").arg(&path).output().unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stdout));
    }

    #[test]
    fn finds_the_bundle_an_executable_is_in() {
        let exe = Path::new("/Applications/Method Launcher.app/Contents/MacOS/method-launcher-app");
        assert_eq!(mac_target(exe), PathBuf::from("/Applications/Method Launcher.app"));
        let loose = Path::new("/Users/you/method-launcher/target/release/method-launcher-app");
        assert_eq!(mac_target(loose), loose.to_path_buf());
    }

    #[test]
    fn the_autostart_entry_runs_the_launcher() {
        let entry = autostart_entry("/home/you/Applications/Method Launcher.AppImage");
        assert!(entry.starts_with("[Desktop Entry]\nType=Application\nName=Method Launcher\n"));
        assert!(entry.contains("Exec=\"/home/you/Applications/Method Launcher.AppImage\"\n"), "{entry}");
    }

    #[test]
    fn the_run_value_is_one_quoted_program() {
        assert_eq!(
            run_value(r"C:\Users\you\AppData\Local\Programs\Method Launcher\method-launcher-app.exe"),
            r#""C:\Users\you\AppData\Local\Programs\Method Launcher\method-launcher-app.exe""#
        );
        assert_eq!(run_value(r#"C:\a"b.exe"#), r#""C:\ab.exe""#);
    }
}
