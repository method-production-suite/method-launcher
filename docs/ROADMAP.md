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

Built on recommendation (reversible; say if you want it otherwise):

- **The launcher's page stays on the suite's site** at methodproduction.com/launcher/, not
  a domain of its own: the launcher is the suite's front door. Its `site/` holds that page's
  fragment and pictures, and MOrg syncs them with each release.

Otherwise none open.

Decided by the owner, 2026-10-18:

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

**Install and update on a real Windows and a real Linux machine.** Built and tested
automatically; no one has run them on those systems yet (0.1.2 and 0.2.0 shipped before this
test, on the owner's yes). Download the launcher from the latest release
(`Method-Launcher-windows-x64-setup.exe`, `Method-Launcher-linux-x86_64.AppImage` or the
`.deb`), then on each machine:

1. Windows x64: install Method Schedule from its card. The setup opens; finish it. Expect
   Schedule in the Start menu, its version on the card, and Open working.
2. Windows, update: with an older Schedule installed and open, click Update to X. Expect the
   question first, Schedule asked to close, then the new version.
3. Linux x86_64: install Method Schedule. Expect `~/Applications/Method-Schedule.AppImage`,
   "Method Schedule" in the desktop's app menu with its icon, and the version on its card.
4. Linux, update: the same as step 2, over the AppImage.
5. Linux with apt: Settings > Install apps as > System package, then install an app. Expect
   the password prompt, then the app listed by `dpkg-query -W method-<noun>`. Dismiss the
   prompt once: expect "Not installed. You can install the AppImage instead."
6. On both: Cancel during a download leaves the card as it was; a release without a
   checksum file (or a tampered download) is refused with "This release can't be checked
   yet" or "doesn't match the release's checksum".
7. Linux on ARM (a Raspberry Pi 5 or an ARM laptop): the launcher's arm64 AppImage opens
   and shows the apps, each linking how to build it.

**Still to try on the Mac:**

1. The launcher updating itself: with 0.2.0 installed, Help > Check for Updates once 0.2.1
   is out. Expect "Update ready", Restart to Update, and 0.2.1 in About.
2. Update over an app that is open (the quit question).
3. An install that needs the administrator prompt.
4. Settings > "Open the launcher when I log in": turn it on, log out and back in, and the
   launcher opens (the same on Windows and Linux with the tests above).

### Release go-aheads

- None waiting. 0.2.1 published 2026-10-21 (the updater's TLS security fix and open at
  login); 0.2.0 and 0.1.2 published 2026-10-20.

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
- **[have] S** Mach-O reader: every executable has this Mac's slice (apps), and the launcher's own build is exactly universal.
- **[have] M** Download with the redirect allow-list, limits and progress, through the
  system's `curl` (spec §4.1), and Cancel: a waiting install leaves the queue, and a running
  one stops while it is finding, downloading or checking.
- **[have] M** macOS: mount, verify (bundle id, Developer ID team XSRB3D643J,
  notarisation, arm64, links) and copy with staging and rename (spec §4.2); updates over
  the installed copy, and replacing an earlier-named copy.
- **[partial] S** macOS: administrator prompt (built, not yet tried: waiting on the owner's
  screen) and the `~/Applications` choice in the new Settings window (have). Settings also
  has "Open the launcher when I log in" on every system (have; tested by unit tests, not yet
  turned on by a person).
- **[partial] S** Windows: the setup checked against `SHA256SUMS`, run per user with its own
  window, then confirmed by looking again (spec §4.3). Built and cross-checked, not yet run
  on a Windows machine. Missing: Authenticode, once the suite signs Windows builds.
- **[have] M** Linux: the AppImage checked against `SHA256SUMS`, placed in
  `~/Applications` through a staging name, with a menu entry and icon, and its version
  recorded (spec §4.4); the ARM message; the FUSE help link; and the `.deb` as a Settings
  choice where apt is, installed by `pkexec apt-get install` and confirmed with `dpkg-query`
  (an update goes the way the app is installed). Built and unit-tested, not yet tested by a
  person: waiting on the owner (Windows/Linux).
- **[have] S** Every failure case in spec §8, with a test each: offline told apart from
  GitHub not answering, no download for this system, a full disk (checked before copying on
  macOS and Linux), FUSE missing on Linux with its help link, and the release page for a
  release that can't be checked. Leftovers are cleaned up at launch on every system.
- **[have] S** `method-launcher verify <app> <path>` (macOS), `install <app>` (every system;
  `--to <folder>` on macOS), `latest` and `catalog`.

## M3: Updates, recent files, samples

- **[have] M** Updates run by the launcher (spec §5.1): "Update available", Update to X,
  ask before quitting an open app, Update all, and the switch. Have: all of it, on every
  system (Windows and Linux built, not yet run on those machines), checked at launch and on
  Refresh, and every 6 hours while it stays open.
- **[have] S** Recent files across the apps (spec §6).
- **[have] S** Open Sample through `--open-sample`, with the fallback (Demo the suite).

## M4: Help, identity, its own updates

- **[have] S** Help menu (SUITE-STANDARDS.md §4): About, User Guide (docs/user-guide.md),
  Keyboard Shortcuts, What's New, Check for Updates…, Report a Bug prefilled with the version
  and system, Licenses and Notices, methodproduction.com.
- **[have] S** The mark, wordmark and app icon: the suite's four pages, as the app icon
  (scripts/make-icons.py), and "method launcher" in the family's strokes
  (scripts/make-brand.py, from the website's glyphs).
- **[have] M** Its own updates (spec §5.2): `method-update` on Macs (Apple silicon and
  Intel), with the Help menu's switch and Restart to Update; on Windows and Linux, a link to
  the download. Every release since 0.1.2 carries the signed updater archive, `latest.json`
  and `SHA256SUMS.minisig`. The first update a person sees is 0.2.0 to 0.2.1 (a Mac test
  above).

## M5: Release

- **[have] M** Release workflow (everything below, with a main-only rehearsal; the Linux
  arm64 build from 0.2.0, on GitHub's arm runner): build before secrets; macOS signed,
  notarised, stapled with the enforced universal check; Windows setup (signtool step off until a certificate
  exists); Linux AppImage and `.deb` (x86_64, and arm64 per decision 5); version-free names
  (`Method-Launcher-macOS.dmg`, `Method-Launcher-windows-x64-setup.exe`,
  `Method-Launcher-linux-x86_64.AppImage`, ...); `SHA256SUMS`; a draft release
  (SUITE-STANDARDS.md §3, RELEASE-CHECKLIST.md). A crate-age job checks the whole lock, and
  every build and signing job needs it: no age-out exceptions, but a young crate that fixes
  an advisory passes when marked with its RUSTSEC id (SUITE-STANDARDS.md §6; the
  method-storyteller release workflow is the model, or the shared method-libs script once it
  lands).
- **[have] S** Pictures: the four shots, light and dark at 1280 by 800, from an invented
  state, drawn offscreen (crates/app/src/screens.rs, SUITE-STANDARDS.md §7), into
  `assets/screens/` and `site/img/`.
- **[have] S** `site/`: the launcher's page on the suite's site (`site/launcher.html`, the
  hub's fragment for `/launcher/`) and its pictures, which MOrg syncs at each release tag
  (live from 0.2.0). The mark, wordmark and favicon are there too, unused by the hub page.
  Built on recommendation (see Waiting on the owner).

## Next

- **[have] S** A clearer password prompt for the Linux `.deb`: the launcher's own `.deb`
  ships the install step as `/usr/libexec/method-launcher/install-deb` with a polkit action
  that says what the password is for. The AppImage launcher runs the same step through
  `sh -c`, with the plain prompt. (MReview's idea, 2026-10-20.)
- **[missing] S** Windows code signing: the release workflow's signtool step is ready and
  off. It needs a code-signing certificate, the owner's call (cost and who holds it), after
  which the launcher can also check Authenticode on each app's setup.
- **[missing] S** Cross-app follow-through: as each app adds `SHA256SUMS`, `--open-sample`
  and `recent.json` (above), drop the launcher's fallbacks for it and say so in the notes.
