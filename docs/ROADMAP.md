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
   (`com.methodlauncher.desktop`) and download name (`Method-Launcher-macOS.dmg`).
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
   allowance. Nothing private is tracked (KICKOFF.md and HANDOFF.md are ignored). Needs the
   owner's yes, relayed by MOrg; then the first push and the ruleset are MOrg's.

### Tests that need the owner's hands

None yet. Before the first release: a real install of each released app from a clean
account (the list will be here, ready to run).

### Release go-aheads

None yet.

---

## Cross-app items (through MOrg)

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
- **[missing] S** The apps view (spec §3.1) with installed detection: find bundles by id in
  `/Applications` and `~/Applications`, read their versions.
- **[missing] S** Open an app; Show in Finder; Release notes link.
- **[missing] S** Command line: `method-launcher list` and `status`.

## M2: Install

- **[partial] S** Mach-O reader: every executable is arm64 only (spec §4.4 step 4). Have:
  the header check for thin and fat files.
- **[missing] M** Download with the redirect allow-list, limits, cancel and progress (spec
  §4.2).
- **[missing] M** Mount, verify (bundle id, Developer ID team XSRB3D643J, notarisation,
  arm64, links) and copy with staging and rename (spec §4.3 to §4.5).
- **[missing] S** Administrator prompt and the `~/Applications` choice.
- **[missing] S** Every failure case in spec §8, with a test each; leftover cleanup at
  launch.
- **[missing] S** `method-launcher verify <app>` and `install <slug> --to <folder>`.

## M3: Updates, recent files, samples

- **[missing] S** Read each release's `latest.json` with `method-update` and show "Version X
  is ready" (spec §5.1), on the standard schedule with the switch.
- **[missing] S** Recent files across the apps (spec §6).
- **[missing] S** Open Sample and Open to update through the launch arguments, with the
  fallbacks.

## M4: Help, identity, its own updates

- **[missing] S** Help menu (SUITE-STANDARDS.md §4), About, Licenses and Notices, Report a
  Bug.
- **[missing] S** The mark, wordmark and app icon in the chosen accent (BRAND.md rules).
- **[missing] M** Its own updater through `method-update`, with a key in 1Password.

## M5: Release

- **[missing] M** Release workflow: build before secrets, sign, notarise, staple, the
  enforced arm64 check, the version-free `Method-Launcher-macOS.dmg`, a draft release
  (SUITE-STANDARDS.md §3, RELEASE-CHECKLIST.md).
- **[missing] S** Pictures: the four shots from an offscreen render (SUITE-STANDARDS.md §7).
- **[missing] S** `site/` for the website, and MOrg adds the launcher to the catalog.
