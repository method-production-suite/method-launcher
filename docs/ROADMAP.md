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

1. **The product name.** Options: Method Launcher; Method Home; Method Hub. The suite's
   pattern is "Method <Noun>" with the noun saying what the app does (SUITE-STANDARDS.md §2),
   and the website already uses "hub" for its front page. **Recommend: Method Launcher**:
   plain, and says what it does. Sets the repo (`method-launcher`), CLI
   (`method-launcher`), window binary (`method-launcher-app`), bundle id
   (`com.methodlauncher.desktop`) and download names (`Method-Launcher-macOS.dmg` and the
   Windows and Linux equivalents).
   **Built on the recommendation**; renaming before the first release is a search and
   replace.
2. **The accent colour.** The four apps use rose, violet, blue and teal. **Recommend:
   ochre, `#9A5B00` light and `#F0B54A` dark** (5.4:1 on white and 10:1 on the dark
   background, the same contrast range as the others), a hue none of the four uses.
   Alternative: the suite's own neutral, to say "the suite" rather than "a fifth app".
   **Built on the recommendation.**
3. **The GitHub repository.** Create `method-production-suite/method-launcher`.
   **Recommend: public from the start**, like Method Storyteller while in development:
   on the org's free plan a private repository gets no branch rulesets (so `main` can't be
   protected for MReview's flow) and its macOS CI runs count against a small monthly
   allowance (the new three-system matrix, macOS included, runs on every change). Nothing
   private is tracked (KICKOFF.md and HANDOFF.md are ignored). The kickoff says private until
   the owner says otherwise, so this is the owner's call. Needs the owner's yes, relayed by
   MOrg; then the first push and the ruleset are MOrg's.
4. **Linux: AppImage or .deb by default** (spec §4.4). **Recommend: AppImage by default**,
   in `~/Applications` with a desktop entry: no administrator password, the same on every
   distribution, and replaceable in place for updates. The `.deb` stays a Settings choice on
   systems with apt. **Built on the recommendation.**
5. **Build the launcher for Linux arm64.** The suite has no arm64 app builds yet, so on ARM
   Linux the launcher can only show the apps and link their build-from-source steps.
   **Recommend: yes, once the repository is public**: GitHub's arm64 Linux runners are free
   for public repositories, the launcher is small, and it is where those people will hear
   when arm64 builds arrive. Skip it if the repository stays private. **Built on the
   recommendation** (in the CI matrix; released with the first release).

### Tests that need the owner's hands

None yet. Before the first release: a real install of each released app from a clean
account on macOS, Windows and Linux (the list will be here, ready to run).

### Release go-aheads

None yet.

---

## Cross-app items (through MOrg)

- **`SHA256SUMS` on every release**, one name, listing every asset with the installers
  (spec §9.1). Today: Method Reader has the installers in `SHA256SUMS-app`; Method Schedule
  v0.1.0 and Method Budget v0.4.0 have none, so the launcher can't install them on Windows
  or Linux. Later: sign `SHA256SUMS` with each app's update key, so a checksum also proves
  who made the release.
- **Each app's Linux package name and Windows uninstall display name**, and its data folder
  on Windows and Linux (spec §4.6, §6).
- **Launch arguments** every app accepts: "check for updates now" and "open the sample"
  (spec §5.1, §7). Recommend `--check-for-updates` and `--open-sample`, passed with
  `open -b <id> --args`. Until an app has them, the launcher falls back as the spec says.
- **The shared `recent.json` shape** (spec §6): a list of `{ "path": "/abs/path",
  "opened": "RFC 3339" }`, newest first, in each app's data folder.
- **A catalog file on methodproduction.com** (spec §9) and a "New in the apps" feed:
  recommend later, not in the first version.

---

## M1: The window and the catalog

- **[have] S** Workspace, CI on three systems, cargo deny, crate-age check, licences,
  README (this scaffold).
- **[partial] S** The built-in catalog with its checks (spec §9). Have: the four entries and
  their tests.
- **[partial] S** Platform traits (spec §10) and the target, with installer names per
  target. Have: the traits, `Target`, and the names with their tests.
- **[missing] S** The apps view (spec §3.1).
- **[missing] M** `Locate` on each system: bundles by id on macOS, uninstall entries on
  Windows, `installed.json` and `dpkg-query` on Linux.
- **[missing] S** `Launch` on each system: open an app, show its location; Release notes
  link.
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
