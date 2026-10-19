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
- **[have] S** The built-in catalog with its checks (spec §9), with each app's binary,
  package and earlier name.
- **[have] S** Platform traits (spec §10) and the target, with installer names per target.
- **[have] S** The apps view (spec §3.1): installed or not, version, Open, Show in Finder,
  Release notes; looks again when the window comes forward or on Refresh (⌘R).
- **[partial] M** `Locate` on each system: bundles by id on macOS (including an app's earlier
  name), uninstall entries on Windows, `dpkg-query` and `method-<noun>-app` on the PATH on
  Linux. Tested on macOS; Windows and Linux by unit tests and cross-builds, not yet on a
  machine. Linux's `installed.json` comes with installs (M2).
- **[partial] S** `Launch` on each system: whether an app is running, open it (launch
  arguments only when it isn't), show its location; Release notes link. Built for all
  three; not yet tried on a screen.
- **[have] S** Command line: `method-launcher list` and `status`.

## M2: Install

- **[have] S** Release pinning, checksums and hashing (spec §4.1).
- **[have] S** Mach-O reader: every executable is arm64 only.
- **[partial] M** Download with the redirect allow-list, limits and progress, through the
  system's `curl` (spec §4.1). Cancel is missing.
- **[have] M** macOS: mount, verify (bundle id, Developer ID team XSRB3D643J,
  notarisation, arm64, links) and copy with staging and rename (spec §4.2); updates over
  the installed copy, and replacing an earlier-named copy.
- **[partial] S** macOS: administrator prompt (built, not yet tried) and the
  `~/Applications` choice (missing).
- **[missing] S** Windows: run the setup per user and confirm (spec §4.3); Authenticode once
  the suite signs Windows builds.
- **[missing] M** Linux: AppImage with desktop entry; `.deb` through pkexec and apt; the ARM
  message (spec §4.4).
- **[missing] S** Every failure case in spec §8, with a test each; leftover cleanup at
  launch.
- **[have] S** `method-launcher verify <app> <path>`, `install <app> --to <folder>` and
  `latest` (macOS).

## M3: Updates, recent files, samples

- **[partial] M** Updates run by the launcher (spec §5.1): "Update available", Update to X,
  ask before quitting an open app, Update all. Have: all of it on macOS, checked at launch
  and on Refresh. Missing: Windows and Linux (with their installs, M2), the 6-hour schedule
  and the switch.
- **[missing] S** Recent files across the apps (spec §6).
- **[missing] S** Open Sample through `--open-sample`, with the fallback.

## M4: Help, identity, its own updates

- **[missing] S** Help menu (SUITE-STANDARDS.md §4), About, Licenses and Notices, Report a
  Bug.
- **[partial] S** The mark, wordmark and app icon in the chosen accent. Have: the app icon
  (scripts/make-icons.py). Missing: the wordmark.
- **[missing] M** Its own updates: `method-update` on macOS, with a key in 1Password; on
  Windows and Linux, "Version X is ready" with a link to the download (spec §5.2).

## M5: Release

- **[missing] M** Release workflow: build before secrets; macOS signed, notarised, stapled
  with the enforced arm64 check; Windows setup (signtool step off until a certificate
  exists); Linux AppImage and `.deb` (x86_64, and arm64 per decision 5); version-free names
  (`Method-Launcher-macOS.dmg`, `Method-Launcher-windows-x64-setup.exe`,
  `Method-Launcher-linux-x86_64.AppImage`, ...); `SHA256SUMS`; a draft release
  (SUITE-STANDARDS.md §3, RELEASE-CHECKLIST.md). A crate-age job checks the whole lock, and
  every build and signing job needs it: no age-out exceptions, but a young crate that fixes
  an advisory passes when marked with its RUSTSEC id (SUITE-STANDARDS.md §6; the
  method-storyteller release workflow is the model, or the shared method-libs script once it
  lands).
- **[missing] S** Pictures: the four shots from an offscreen render (SUITE-STANDARDS.md §7).
- **[missing] S** `site/` for the website, and MOrg adds the launcher to the catalog.
