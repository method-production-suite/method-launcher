# Roadmap

What to build, in order, to ship the first version described in [spec.md](spec.md). One
small pull request per item once the repository exists.

Status: **[have]** built, **[partial]** some of it built, **[missing]** not started.
Size: **S** a day or two, **M** about a week, **L** more than a week.

---

## Waiting on the owner

Read every Monday for the owner's digest. Each decision shows the recommendation; where the
choice is reversible, it has been **built on the recommendation** and stays unless the owner
says otherwise.

### Decisions

None open. Decided by the owner, 2026-10-18:

1. **Name: Method Launcher.** Repo `method-launcher`, CLI `method-launcher`, window binary
   `method-launcher-app`, bundle id `com.methodlauncher.desktop`, downloads
   `Method-Launcher-macOS.dmg` and the Windows and Linux equivalents.
2. **Accent: ochre**, `#9A5B00` light and `#F0B54A` dark.
3. **Repository: public**, `method-production-suite/method-launcher`, `main` protected,
   required checks the four CI gates.
4. **Linux: the AppImage by default** (spec §4.4), the `.deb` a Settings choice on systems
   with apt.
5. **Linux arm64: build the launcher too** (the repository is public, so the runners are
   free).
6. **Cross-app rules adopted** into SUITE-STANDARDS.md §2 and §3: one `SHA256SUMS` per
   release listing every file; `--check-for-updates` and `--open-sample` in every app; the
   `recent.json` shape and data folders; the installed names. No catalog file or news feed in
   the first version.

### Tests that need the owner's hands

None yet. Before the first release: a real install of each released app from a clean
account on macOS, Windows and Linux (the list will be here, ready to run).

### Release go-aheads

None yet.

---

## Cross-app items

Adopted into SUITE-STANDARDS.md §2 and §3 (2026-10-18); MOrg is asking each app. Until an
app has done its part, the launcher falls back as spec §4.1, §5.1 and §7 say.

- **`SHA256SUMS` on every release**, one name, every file (spec §9.1). Today Method Reader
  has its installers in `SHA256SUMS-app`, and Method Schedule v0.1.0 and Method Budget v0.4.0
  have none, so the launcher can't install those two on Windows or Linux yet. Later, signed
  as `SHA256SUMS.minisig`.
- **`--check-for-updates` and `--open-sample`** in every app's window binary.
- **`recent.json`** as `[{"path": ..., "opened": ...}]`, newest first, in each app's data
  folder.
- **Installed names**: Linux package `method-<noun>` and desktop entry
  `com.method<noun>.desktop.desktop`; Windows uninstall display name `Method <Noun>`.

## M1: The window and the catalog

- **[have] S** Workspace, CI on three systems, cargo deny, crate-age check, licences,
  README (this scaffold).
- **[partial] S** The built-in catalog with its checks (spec §9). Have: the four entries and
  their tests.
- **[partial] S** Platform traits (spec §10) and the target, with installer names per
  target. Have: the traits, `Target`, and the names with their tests.
- **[missing] S** The apps view (spec §3.1).
- **[missing] M** `Locate` on each system: bundles by id on macOS, uninstall entries on
  Windows, `installed.json`, `dpkg-query` and `method-<noun>-app` on the PATH on Linux.
- **[missing] S** `Launch` on each system: whether an app is running, open it (launch
  arguments only when it isn't), show its location; Release notes link.
- **[missing] S** Command line: `method-launcher list` and `status`.

## M2: Install

- **[partial] S** Release pinning, checksums and hashing (spec §4.1). Have: `SHA256SUMS`
  parsing and the SHA-256 check.
- **[partial] S** Mach-O reader: every executable is arm64 only (spec §4.4 step 4). Have:
  the header check for thin and fat files.
- **[missing] M** Download with the redirect allow-list, limits, cancel and progress (spec
  §4.2).
- **[missing] M** macOS: mount, verify (bundle id, Developer ID team XSRB3D643J,
  notarisation, arm64, links) and copy with staging and rename (spec §4.2).
- **[missing] S** macOS: administrator prompt and the `~/Applications` choice.
- **[missing] S** Windows: run the setup per user and confirm (spec §4.3); Authenticode once
  the suite signs Windows builds.
- **[missing] M** Linux: AppImage with desktop entry; `.deb` through pkexec and apt; the ARM
  message (spec §4.4).
- **[missing] S** Every failure case in spec §8, with a test each; leftover cleanup at
  launch.
- **[missing] S** `method-launcher verify <app>` and `install <slug> --to <folder>`.

## M3: Updates, recent files, samples

- **[missing] S** Read each release's `latest.json` and show "Version X is ready" (spec
  §5.1), on the standard schedule with the switch; Get version X on Windows and Linux.
- **[missing] S** Recent files across the apps (spec §6).
- **[missing] S** Open Sample and Open to update through the launch arguments, with the
  fallbacks.

## M4: Help, identity, its own updates

- **[missing] S** Help menu (SUITE-STANDARDS.md §4), About, Licenses and Notices, Report a
  Bug.
- **[missing] S** The mark, wordmark and app icon in the chosen accent (BRAND.md rules).
- **[missing] M** Its own updates: `method-update` on macOS, with a key in 1Password; on
  Windows and Linux, "Version X is ready" with a link to the download (spec §5.2).

## M5: Release

- **[missing] M** Release workflow: build before secrets; macOS signed, notarised, stapled
  with the enforced arm64 check; Windows setup (signtool step off until a certificate
  exists); Linux AppImage and `.deb` (x86_64, and arm64 per decision 5); version-free names
  (`Method-Launcher-macOS.dmg`, `Method-Launcher-windows-x64-setup.exe`,
  `Method-Launcher-linux-x86_64.AppImage`, ...); `SHA256SUMS`; a draft release
  (SUITE-STANDARDS.md §3, RELEASE-CHECKLIST.md).
- **[missing] S** Pictures: the four shots from an offscreen render (SUITE-STANDARDS.md §7).
- **[missing] S** `site/` for the website, and MOrg adds the launcher to the catalog.
