//! Packaging: `cargo xtask version`, `bundle-macos`, `dmg-macos`, `package-linux` and
//! `package-windows`. The release workflow (`.github/workflows/release.yml`) runs these; they
//! also run on a developer's machine. Everything lands in `bundle/<platform>/` in the build
//! folder (cargo's `target_directory`: the shared cache on a developer's machine, `./target` in
//! the release), under the suite's version-free names. Releasing: docs/RELEASING.md.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::root;

const APP_NAME: &str = "Method Launcher";
const APP_BIN: &str = "method-launcher-app";
const APP_PACKAGE: &str = "method-launcher";
/// The Linux package and icon name.
const LINUX_NAME: &str = "method-launcher";
/// The desktop entry, named by the bundle id (SUITE-STANDARDS.md §2).
const DESKTOP_FILE: &str = "com.methodlauncher.desktop.desktop";

pub const DMG: &str = "Method-Launcher-macOS.dmg";
/// The Linux packages' names for one chip: Debian's name for it, and the AppImage's.
pub fn linux_names(deb_arch: &str, appimage_arch: &str) -> (String, String) {
    (format!("Method-Launcher-linux-{deb_arch}.deb"), format!("Method-Launcher-linux-{appimage_arch}.AppImage"))
}

/// This machine's chip as Debian and AppImage name it (the Linux packages build for the host).
fn linux_arch() -> (&'static str, &'static str) {
    if cfg!(target_arch = "aarch64") { ("arm64", "aarch64") } else { ("amd64", "x86_64") }
}
pub const SETUP_EXE: &str = "Method-Launcher-windows-x64-setup.exe";

/// Options shared by the packaging tasks: `--target <triple>`, `--sign <identity>`,
/// `--no-build`, `--appimagetool <path>`, `--runtime <path>`, `--iscc <path>`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Options {
    target: Option<String>,
    sign: Option<String>,
    no_build: bool,
    appimagetool: Option<PathBuf>,
    runtime: Option<PathBuf>,
    iscc: Option<PathBuf>,
}

pub fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().cloned().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--target" => options.target = Some(value()?),
            "--sign" => options.sign = Some(value()?),
            "--no-build" => options.no_build = true,
            "--appimagetool" => options.appimagetool = Some(PathBuf::from(value()?)),
            "--runtime" => options.runtime = Some(PathBuf::from(value()?)),
            "--iscc" => options.iscc = Some(PathBuf::from(value()?)),
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(options)
}

/// The `version` under `[workspace.package]` in the root Cargo.toml.
pub fn workspace_version() -> Result<String, String> {
    let manifest = std::fs::read_to_string(root().join("Cargo.toml")).map_err(|e| format!("Cargo.toml: {e}"))?;
    version_from_manifest(&manifest).ok_or_else(|| "no version under [workspace.package] in Cargo.toml".into())
}

fn version_from_manifest(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
            continue;
        }
        if in_package && let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"');
            if key.trim() == "version" && !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// `0.2.0-beta.1` sorts before `0.2.0` in Debian only with a tilde: `0.2.0~beta.1`.
fn debian_version(version: &str) -> String {
    version.replacen('-', "~", 1)
}

/// The `X.Y.Z` part, for places that take numbers only (the Windows file version).
fn numeric_version(version: &str) -> &str {
    version.split(['-', '+']).next().unwrap_or(version)
}

/// Where cargo builds: `CARGO_TARGET_DIR` when set (the suite's shared cache, and every
/// release job), else what `cargo metadata` says, so nothing here assumes `./target`. On a
/// CI runner it never runs cargo: the release runs this tool beside the signing keys
/// (SUITE-STANDARDS.md §3), so there a missing variable means `./target`.
fn target_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    if std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some() {
        return root().join("target");
    }
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(root())
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| metadata_target_dir(&out.stdout))
        .unwrap_or_else(|| root().join("target"))
}

/// `target_directory` from `cargo metadata`'s JSON.
fn metadata_target_dir(json: &[u8]) -> Option<PathBuf> {
    let value: serde_json::Value = serde_json::from_slice(json).ok()?;
    value.get("target_directory")?.as_str().map(PathBuf::from)
}

fn bundle_dir(platform: &str) -> PathBuf {
    target_dir().join("bundle").join(platform)
}

/// The two Mac targets a universal build joins (SUITE-STANDARDS.md §3, from 2026-10-19).
pub const MAC_TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

/// `--target universal-apple-darwin`: the app built for both Mac targets (unless `--no-build`,
/// when both must already be built) and joined into one binary with `lipo -create`.
fn build_universal(options: &Options) -> Result<PathBuf, String> {
    let mut slices = Vec::new();
    for target in MAC_TARGETS {
        let one = Options { target: Some(target.to_string()), ..options.clone() };
        slices.push(build_app(&one)?);
    }
    let out = target_dir().join("universal-apple-darwin").join("release");
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let binary = out.join(APP_BIN);
    let mut args = vec![os("-create"), os("-output"), binary.as_os_str()];
    args.extend(slices.iter().map(|p| p.as_os_str()));
    run("lipo", &args)?;
    Ok(binary)
}

/// Builds the app in release mode (unless `--no-build`) and returns its binary.
fn build_app(options: &Options) -> Result<PathBuf, String> {
    if options.target.as_deref() == Some("universal-apple-darwin") {
        return build_universal(options);
    }
    if !options.no_build {
        let mut args = vec!["build", "--release", "--locked", "--package", APP_PACKAGE, "--bin", APP_BIN];
        if let Some(target) = &options.target {
            args.extend(["--target", target.as_str()]);
        }
        crate::cargo(&args)?;
    }
    let mut dir = target_dir();
    if let Some(target) = &options.target {
        dir.push(target);
    }
    let binary = dir.join("release").join(format!("{APP_BIN}{}", std::env::consts::EXE_SUFFIX));
    if binary.is_file() { Ok(binary) } else { Err(format!("{} not found; build it first", binary.display())) }
}

fn run(program: impl AsRef<std::ffi::OsStr>, args: &[&std::ffi::OsStr]) -> Result<(), String> {
    let program = program.as_ref();
    let shown: Vec<String> = args.iter().map(|a| a.to_string_lossy().into_owned()).collect();
    println!("$ {} {}", program.to_string_lossy(), shown.join(" "));
    let status = Command::new(program).args(args).current_dir(root()).status().map_err(|e| format!("{}: {e}", program.to_string_lossy()))?;
    if status.success() { Ok(()) } else { Err(format!("{} failed", program.to_string_lossy())) }
}

fn os(s: &str) -> &std::ffi::OsStr {
    std::ffi::OsStr::new(s)
}

fn fresh_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_dir_all(path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    std::fs::create_dir_all(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::copy(from, to).map(|_| ()).map_err(|e| format!("{} -> {}: {e}", from.display(), to.display()))
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<(), String> {
    Ok(())
}

fn packaging(relative: &str) -> PathBuf {
    root().join("packaging").join(relative)
}

/// `cargo xtask bundle-macos [--target <triple>] [--sign <identity>] [--no-build]`: builds
/// `<build folder>/bundle/macos/Method Launcher.app` and signs it, with the Developer ID identity
/// (hardened runtime and a secure timestamp, ready to notarize) or ad hoc without one.
pub fn bundle_macos(options: &Options) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("bundle-macos runs on macOS only".into());
    }
    let version = workspace_version()?;
    let binary = build_app(options)?;
    let out = bundle_dir("macos");
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let app = out.join(format!("{APP_NAME}.app"));
    fresh_dir(&app)?;
    let contents = app.join("Contents");
    copy(&binary, &contents.join("MacOS").join(APP_BIN))?;
    set_mode(&contents.join("MacOS").join(APP_BIN), 0o755)?;
    copy(&packaging("icons/icon.icns"), &contents.join("Resources").join("AppIcon.icns"))?;
    let plist = read(&packaging("macos/Info.plist"))?;
    if !plist.contains("{{VERSION}}") {
        return Err("packaging/macos/Info.plist has no {{VERSION}} placeholder".into());
    }
    let info = contents.join("Info.plist");
    write(&info, &plist.replace("{{VERSION}}", &version))?;
    write(&contents.join("PkgInfo"), "APPL????")?;
    run("plutil", &[os("-lint"), info.as_os_str()])?;
    match &options.sign {
        Some(identity) => {
            run("codesign", &[os("--force"), os("--options"), os("runtime"), os("--timestamp"), os("--sign"), os(identity), app.as_os_str()])?
        }
        None => run("codesign", &[os("--force"), os("--sign"), os("-"), app.as_os_str()])?,
    }
    run("codesign", &[os("--verify"), os("--deep"), os("--strict"), os("--verbose=2"), app.as_os_str()])?;
    println!("bundle-macos: {} ({version}, {})", app.display(), if options.sign.is_some() { "Developer ID" } else { "ad hoc" });
    Ok(())
}

/// `cargo xtask dmg-macos [--sign <identity>]`: puts the bundle made by `bundle-macos` (and a
/// link to /Applications) into `<build folder>/bundle/macos/Method-Launcher-macOS.dmg`, signed
/// with the identity when one is given.
pub fn dmg_macos(options: &Options) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("dmg-macos runs on macOS only".into());
    }
    let out = bundle_dir("macos");
    let app = out.join(format!("{APP_NAME}.app"));
    if !app.is_dir() {
        return Err(format!("{} not found; run cargo xtask bundle-macos first", app.display()));
    }
    let staging = out.join("dmg");
    fresh_dir(&staging)?;
    // ditto keeps the signature, the stapled ticket and the extended attributes.
    run("ditto", &[app.as_os_str(), staging.join(format!("{APP_NAME}.app")).as_os_str()])?;
    run("ln", &[os("-s"), os("/Applications"), staging.join("Applications").as_os_str()])?;
    let dmg = out.join(DMG);
    run(
        "hdiutil",
        &[os("create"), os("-volname"), os(APP_NAME), os("-srcfolder"), staging.as_os_str(), os("-ov"), os("-format"), os("UDZO"), dmg.as_os_str()],
    )?;
    std::fs::remove_dir_all(&staging).map_err(|e| format!("{}: {e}", staging.display()))?;
    if let Some(identity) = &options.sign {
        run("codesign", &[os("--force"), os("--timestamp"), os("--sign"), os(identity), dmg.as_os_str()])?;
    }
    println!("dmg-macos: {}", dmg.display());
    Ok(())
}

/// The Debian package's control file. The libraries are the ones eframe loads at run time
/// (OpenGL, xkbcommon, X11 and Wayland); the binary itself links only libc and libgcc. The
/// launcher fetches releases with the system's curl.
fn deb_control(version: &str, arch: &str, installed_kib: u64) -> String {
    format!(
        "Package: {LINUX_NAME}\n\
         Version: {}\n\
         Architecture: {arch}\n\
         Maintainer: Method Production Suite <hello@methodproduction.com>\n\
         Installed-Size: {installed_kib}\n\
         Depends: libc6 (>= 2.35), libgcc-s1, libgl1, libegl1, libxkbcommon0, libxkbcommon-x11-0, libx11-6, libxcursor1, libxi6, libxrandr2, libwayland-client0, libwayland-cursor0, libwayland-egl1, curl, xdg-utils\n\
         Section: utils\n\
         Priority: optional\n\
         Homepage: https://methodproduction.com\n\
         Description: Install, update and open the Method Production Suite's apps\n \
         Method Launcher is the front door to the Method Production Suite: it shows the\n \
         suite's apps, installs the ones you want from their own releases, checked\n \
         before anything runs, tells you when an update is ready, and opens them.\n",
        debian_version(version)
    )
}

/// Lays out the files both Linux packages share under `dir` (`usr/bin`, the desktop file,
/// the icons and the licences).
fn linux_tree(dir: &Path, binary: &Path) -> Result<(), String> {
    let bin = dir.join("usr/bin").join(APP_BIN);
    copy(binary, &bin)?;
    set_mode(&bin, 0o755)?;
    copy(&packaging(&format!("linux/{DESKTOP_FILE}")), &dir.join("usr/share/applications").join(DESKTOP_FILE))?;
    for size in ["256", "512"] {
        copy(&packaging(&format!("icons/icon-{size}.png")), &dir.join(format!("usr/share/icons/hicolor/{size}x{size}/apps/{LINUX_NAME}.png")))?;
    }
    let mut copyright = read(&root().join("NOTICE"))?;
    for licence in ["LICENSE-MIT", "LICENSE-APACHE"] {
        copyright.push_str(&format!("\n\n---- {licence} ----\n\n"));
        copyright.push_str(&read(&root().join(licence))?);
    }
    write(&dir.join("usr/share/doc").join(LINUX_NAME).join("copyright"), &copyright)
}

fn tree_size(dir: &Path) -> Result<u64, String> {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten() {
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total = total.saturating_add(meta.len());
            }
        }
    }
    Ok(total)
}

/// `cargo xtask package-linux [--no-build] [--appimagetool <path> --runtime <path>]`: the
/// `.deb` (with `dpkg-deb`) and, given appimagetool and its runtime file, the AppImage.
pub fn package_linux(options: &Options) -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err("package-linux runs on Linux only".into());
    }
    let version = workspace_version()?;
    let binary = build_app(options)?;
    let (deb_arch, appimage_arch) = linux_arch();
    let (deb_name, appimage_name) = linux_names(deb_arch, appimage_arch);
    let out = bundle_dir("linux");
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;

    let deb_root = out.join("deb");
    fresh_dir(&deb_root)?;
    linux_tree(&deb_root, &binary)?;
    // The package alone carries the install step as root's file, with the polkit action that
    // names it (an AppImage launcher runs the same step through `sh -c`).
    let helper = deb_root.join("usr/libexec/method-launcher/install-deb");
    copy(&packaging("linux/install-deb"), &helper)?;
    set_mode(&helper, 0o755)?;
    let policy = "com.methodlauncher.desktop.install-deb.policy";
    copy(&packaging(&format!("linux/{policy}")), &deb_root.join("usr/share/polkit-1/actions").join(policy))?;
    let installed_kib = tree_size(&deb_root)?.div_ceil(1024);
    write(&deb_root.join("DEBIAN/control"), &deb_control(&version, deb_arch, installed_kib))?;
    let deb = out.join(deb_name);
    run("dpkg-deb", &[os("--root-owner-group"), os("--build"), deb_root.as_os_str(), deb.as_os_str()])?;
    println!("package-linux: {}", deb.display());

    let (Some(tool), Some(runtime)) = (&options.appimagetool, &options.runtime) else {
        println!("package-linux: no AppImage (pass --appimagetool <path> --runtime <path>)");
        return Ok(());
    };
    let app_dir = out.join("AppDir");
    fresh_dir(&app_dir)?;
    linux_tree(&app_dir, &binary)?;
    let app_run = app_dir.join("AppRun");
    write(&app_run, &format!("#!/bin/sh\nHERE=\"$(dirname \"$(readlink -f \"$0\")\")\"\nexec \"$HERE/usr/bin/{APP_BIN}\" \"$@\"\n"))?;
    set_mode(&app_run, 0o755)?;
    copy(&packaging(&format!("linux/{DESKTOP_FILE}")), &app_dir.join(DESKTOP_FILE))?;
    copy(&packaging("icons/icon-256.png"), &app_dir.join(format!("{LINUX_NAME}.png")))?;
    copy(&packaging("icons/icon-256.png"), &app_dir.join(".DirIcon"))?;
    let appimage = out.join(appimage_name);
    println!(
        "$ ARCH={appimage_arch} {} --no-appstream --runtime-file {} {} {}",
        tool.display(),
        runtime.display(),
        app_dir.display(),
        appimage.display()
    );
    let status = Command::new(tool)
        .env("ARCH", appimage_arch)
        // Runs without FUSE (build machines rarely have it).
        .env("APPIMAGE_EXTRACT_AND_RUN", "1")
        .args([os("--no-appstream"), os("--runtime-file"), runtime.as_os_str(), app_dir.as_os_str(), appimage.as_os_str()])
        .current_dir(root())
        .status()
        .map_err(|e| format!("{}: {e}", tool.display()))?;
    if !status.success() {
        return Err("appimagetool failed".into());
    }
    println!("package-linux: {}", appimage.display());
    Ok(())
}

/// `cargo xtask package-windows [--no-build] [--iscc <path>]`: the Inno Setup installer.
pub fn package_windows(options: &Options) -> Result<(), String> {
    if !cfg!(target_os = "windows") {
        return Err("package-windows runs on Windows only".into());
    }
    if options.target.is_some() {
        return Err("package-windows builds for this machine only, with no --target".into());
    }
    let version = workspace_version()?;
    let binary = build_app(options)?;
    let out = bundle_dir("windows");
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let iscc = options.iscc.clone().unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe"));
    let defines = [
        format!("/DAppVersion={version}"),
        format!("/DAppNumericVersion={}", numeric_version(&version)),
        format!("/DSourceDir={}", root().display()),
        format!("/DOutputDir={}", out.display()),
        // The program from the build folder, wherever that is (the shared cache, or ./target).
        format!("/DAppBinary={}", binary.display()),
    ];
    let script = packaging("windows/method-launcher.iss");
    let mut args: Vec<&std::ffi::OsStr> = defines.iter().map(|d| os(d)).collect();
    args.push(script.as_os_str());
    run(&iscc, &args)?;
    let setup = out.join(SETUP_EXE);
    if !setup.is_file() {
        return Err(format!("{} was not written", setup.display()));
    }
    println!("package-windows: {}", setup.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_workspace_version_only() {
        let manifest =
            "[package]\nversion = \"9.9.9\"\n[workspace.package]\nedition = \"2024\"\nversion = \"0.2.0\"\n[dependencies]\nversion = \"1\"\n";
        assert_eq!(version_from_manifest(manifest).as_deref(), Some("0.2.0"));
        assert_eq!(version_from_manifest("[workspace]\nmembers = []\n"), None);
    }

    #[test]
    fn the_real_manifest_has_a_version() {
        assert!(workspace_version().is_ok_and(|v| !v.is_empty()));
    }

    #[test]
    fn prereleases_sort_first_in_debian_and_drop_out_on_windows() {
        assert_eq!(debian_version("0.2.0"), "0.2.0");
        assert_eq!(debian_version("0.2.0-beta.1"), "0.2.0~beta.1");
        assert_eq!(numeric_version("0.2.0-beta.1"), "0.2.0");
        assert_eq!(numeric_version("1.0.0+build.5"), "1.0.0");
    }

    #[test]
    fn options() {
        let args: Vec<String> =
            ["--target", "aarch64-apple-darwin", "--no-build", "--sign", "Developer ID Application: X"].map(String::from).to_vec();
        let parsed = parse_options(&args);
        assert!(parsed.as_ref().is_ok_and(|o| o.no_build && o.target.as_deref() == Some("aarch64-apple-darwin") && o.sign.is_some()));
        assert!(parse_options(&["--sign".to_string()]).is_err());
        assert!(parse_options(&["--bogus".to_string()]).is_err());
    }

    #[test]
    fn deb_control_is_well_formed() {
        let control = deb_control("0.2.0-beta.1", "arm64", 12345);
        assert!(control.contains("Version: 0.2.0~beta.1\n"));
        assert!(control.contains("Architecture: arm64\n"));
        assert!(control.contains("Installed-Size: 12345\n"));
        assert!(control.ends_with('\n'));
        // Every continuation line of the description starts with one space.
        let description = control.split("Description: ").nth(1).unwrap_or_default();
        assert!(description.lines().skip(1).all(|l| l.starts_with(' ') && !l.starts_with("  ")));
    }

    #[test]
    fn reads_the_target_directory_from_cargo_metadata() {
        let json = br#"{"packages":[],"target_directory":"/Users/you/.cache/method-target/method-launcher","version":1}"#;
        assert_eq!(metadata_target_dir(json), Some(PathBuf::from("/Users/you/.cache/method-target/method-launcher")));
        assert_eq!(metadata_target_dir(b"{}"), None);
    }

    #[test]
    fn linux_packages_are_named_for_their_chip() {
        assert_eq!(
            linux_names("amd64", "x86_64"),
            ("Method-Launcher-linux-amd64.deb".to_string(), "Method-Launcher-linux-x86_64.AppImage".to_string())
        );
        assert_eq!(
            linux_names("arm64", "aarch64"),
            ("Method-Launcher-linux-arm64.deb".to_string(), "Method-Launcher-linux-aarch64.AppImage".to_string())
        );
    }

    #[test]
    fn the_plist_names_the_bundle_id_and_the_binary() {
        let plist = read(&packaging("macos/Info.plist")).unwrap_or_default();
        assert!(plist.contains("<string>com.methodlauncher.desktop</string>"));
        assert!(plist.contains(&format!("<string>{APP_BIN}</string>")));
        assert!(plist.contains("{{VERSION}}"));
        assert!(plist.contains("<key>NSAppleEventsUsageDescription</key>"));
    }

    #[test]
    fn the_packaging_files_are_there() {
        for file in [
            "icons/icon.icns",
            "icons/icon.ico",
            "icons/icon-256.png",
            "icons/icon-512.png",
            "windows/method-launcher.iss",
            "linux/com.methodlauncher.desktop.desktop",
        ] {
            assert!(packaging(file).is_file(), "packaging/{file}");
        }
        let iss = read(&packaging("windows/method-launcher.iss")).unwrap_or_default();
        assert!(iss.contains("OutputBaseFilename=Method-Launcher-windows-x64-setup"));
        assert!(iss.contains("UninstallDisplayName=Method Launcher"));
    }
}
