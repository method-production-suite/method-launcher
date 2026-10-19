# Method Launcher: specification

Method Launcher (working name) is one small Mac app that is the front door to the Method
Production Suite. It shows the suite's apps, installs the ones you want, tells you when an
update is ready, opens them, and opens your recent files in the right app. One download
instead of four.

Status: draft for the first version. The product name and accent are open
([ROADMAP.md](ROADMAP.md), "Waiting on the owner"); everything here says "Method Launcher"
until then.

## 1. Scope

**In the first version**

- macOS on Apple silicon only (SUITE-STANDARDS.md §3).
- The four apps, in the order of a production: Method Storyteller, Method Reader, Method
  Schedule, Method Budget.
- Install, open, update notice, recent files, samples, Help.
- Its own updates, through the suite's updater (`method-update` from `method-libs`).

**Not in the first version**

- Windows and Linux.
- Uninstalling apps. The launcher can show an app in the Finder; moving it to the Bin is
  left to the person.
- Installing a specific older version, or a pre-release.
- News from the apps, unless the suite publishes a simple public feed (open question, §10).

**Never**

- No server of our own, no accounts, no sign-in, no telemetry, no analytics, no crash
  reporting that sends anything on its own.
- Never runs, opens or copies anything it downloaded before that download has passed every
  check in §4.
- Never updates an app itself: each app updates itself (§5), so there is one update path per
  app.
- Never reads, moves or changes a person's documents. It reads only the apps' own recent-file
  lists (§6) and hands a file to its app to open.
- Never asks for an administrator password unless /Applications is not writable, and then
  only for the one copy (§4.5).
- No generative AI.

## 2. Words used here

- **Catalog**: the list of apps the launcher knows about: name, slug, one-line description,
  status, repository, bundle id, download name, accent. Built into the launcher (§9).
- **Installed**: an app bundle with the catalog's bundle id found in `/Applications` or
  `~/Applications`.
- **Release**: the app's latest published GitHub release. Draft releases are never seen.
- **Manifest**: the release's `latest.json` (SUITE-STANDARDS.md §1), read with
  `method-update`'s manifest parser.

## 3. Screens and states

One window, 960 by 640 logical points by default, resizable down to 720 by 520. Light and
dark follow the system.

### 3.1 The apps

The main view. One card per catalog app, in catalog order, in a single column at narrow
widths and two columns at wider ones. Each card shows the app's mark in its accent, its
name, its one line, its status from the catalog (for example "Alpha, ready for testing"),
and one main button whose label follows the app's state:

| State | What the card says | Main button | Other actions |
|---|---|---|---|
| No release yet | "In development. No download yet." | Visit the site | |
| Not installed | the release's version, "Not installed" | Install | Release notes |
| Downloading | progress bar, bytes of total | Cancel | |
| Checking | "Checking the download" | (none) | |
| Copying | "Copying to Applications" | (none) | |
| Installed, current | "Version 0.4.0" | Open | Show in Finder, Release notes, Open Sample |
| Installed, update ready | "Version 0.4.0. Version 0.5.0 is ready." | Open to update | Open, Release notes, Show in Finder |
| Installed, check failed | "Version 0.4.0" and a quiet "Couldn't check for updates" | Open | Try again |
| Install failed | one sentence saying what went wrong (§8) | Try again | Details |
| Running | a small "Open" dot beside the version | Open (brings it forward) | as above |

Only one install runs at a time; other Install buttons wait in a queue and say "Waiting".

### 3.2 Recent files

Below the cards, or as a second tab at narrow widths: the most recent files across the
suite, newest first, up to 20. Each row: the file's name, the app's mark, the folder it is
in (shortened from the home folder, `~/Films/Field Station`), and when it was last opened.
A file whose app is not installed shows "Install Method Schedule to open this" with the
Install button. A file that no longer exists is not shown. Double-click or Return opens it
in its app (§6). Right-click: Show in Finder, Open with its app, Remove from this list
(hides it in the launcher only; the app's own list is never changed).

Empty state: "Files you open in the Method apps show up here."

### 3.3 First launch

The first time, the window opens on the apps with one quiet line above them: "Install the
apps you want. Each one updates itself." Nothing installs on its own. A person who installs
nothing can still close the launcher and come back.

### 3.4 Settings

A small window (Method Launcher > Settings): "Check for updates automatically" (on by
default, SUITE-STANDARDS.md §1); "Open the launcher at login" (off by default); "Install
apps in: /Applications or my Applications folder" (the second is `~/Applications`, for a
person without administrator rights).

### 3.5 Menus

- **Method Launcher**: About Method Launcher, Settings, Check for Updates..., Quit.
- **File**: Open Recent (the same list as §3.2), Close Window.
- **Help** (SUITE-STANDARDS.md §4): User Guide, Keyboard Shortcuts, What's New, Check for
  Updates..., Report a Bug, Licenses and Notices, methodproduction.com.

## 4. Install and verify

Every step below runs off the interface thread; the card shows where it is. Any failure
stops the install, removes everything it made, and shows one plain sentence (§8).

### 4.1 Find the download

The download URL is fixed by the catalog, never read from a manifest or a page:

```
https://github.com/method-production-suite/<repo>/releases/latest/download/<download>
```

For example `.../method-budget/releases/latest/download/Method-Budget-macOS.dmg`
(SUITE-STANDARDS.md §3, version-free names). The release's version for the card comes from
the same release's `latest.json`.

### 4.2 Download

- https only. The request starts at `github.com`; redirects are followed only to GitHub's
  release-asset hosts (an allow-list in code, the same list `method-update` uses), and only
  over https. Any other redirect fails the install.
- Written to `~/Library/Caches/<launcher bundle id>/downloads/<slug>-<random>.dmg.partial`,
  owner-only, renamed without `.partial` when complete.
- Limits: 1 GiB at most; the download fails if the length header or the bytes go over it.
  Stalls longer than 60 seconds fail with "The download stopped". Cancel removes the file.
- Nothing downloaded is ever opened by Finder, LaunchServices or Gatekeeper's quarantine
  flow before the checks below. The disk image is mounted only to be read.

### 4.3 Mount read-only

```
hdiutil attach -readonly -nobrowse -noautoopen -mountrandom <cache dir> <file.dmg>
```

The launcher reads the mount point from `hdiutil`'s plist output, never guesses it. Always
detached afterwards (`hdiutil detach`), on success and on failure. The image must hold
exactly one `.app` bundle at its top level, named as the catalog says
(`Method Budget.app`); anything else fails ("The download isn't what we expected").

### 4.4 Verify the app, before anything is copied

All of these must pass. Each uses tools that ship with macOS (`/usr/bin/codesign`,
`/usr/sbin/spctl`), called by full path, or Rust code in the launcher.

1. **The right app.** The bundle's `Info.plist` has the catalog's bundle id
   (`com.methodbudget.desktop`) and a version that parses as semver. The version is shown on
   the card after install.
2. **Signed by the suite's Developer ID.** `codesign --verify --deep --strict` passes, and
   the signature satisfies this requirement:
   ```
   anchor apple generic
     and certificate leaf[subject.OU] = "XSRB3D643J"
     and certificate 1[field.1.2.840.113635.100.6.2.6]
     and certificate leaf[field.1.2.840.113635.100.6.1.13]
   ```
   That is: an Apple-rooted Developer ID Application certificate for team XSRB3D643J, and no
   other. The team id is a constant in the launcher's code with a test that pins it.
3. **Notarised.** `spctl --assess --type execute -vv` accepts the bundle and reports the
   source as a notarised Developer ID. The ticket is stapled by every suite release
   (RELEASE-CHECKLIST.md), so this works offline too.
4. **Apple silicon.** Every Mach-O file in the bundle (the main executable, helpers,
   frameworks and dylibs) is a thin arm64 file, or a fat file whose only slice is arm64.
   The launcher reads the Mach-O headers itself, in Rust, the same rule the release workflows
   enforce (SUITE-STANDARDS.md §3), so it doesn't depend on developer tools being installed.
5. **No surprises.** No symbolic link in the bundle points outside it.

If any check fails the image is detached, the download deleted, and nothing is copied.

### 4.5 Copy into Applications

1. Destination: `/Applications/<Name>.app`, or `~/Applications/<Name>.app` per Settings.
2. If an app with the same bundle id is already there, the launcher does not install over it
   (that is an update, §5). A different bundle at that path (another app with the same name)
   stops the install: "Something else called Method Budget.app is already in Applications."
3. Copy with `/usr/bin/ditto` from the mounted image to a hidden staging name beside the
   destination (`.Method Budget.app.installing-<random>`), check the copy's signature again
   (§4.4 step 2, on the copy), then rename it into place. A rename on the same volume is
   atomic, so there is never a half-copied app at the real name.
4. If the destination folder isn't writable, the launcher asks once for an administrator,
   through the system's own password prompt, for that one copy and rename (the same approach
   as `method-update`'s admin swap), and says why first: "Applications is protected on this
   Mac. macOS will ask for an administrator's password to copy Method Budget there." If the
   person cancels, the launcher offers `~/Applications` instead.
5. Detach the image and delete the download. The card turns to "Installed, current" and the
   main button to Open.

The launcher does not add a quarantine flag to the installed copy: it has made the same
notarisation check Gatekeeper would make on first launch (§4.4 step 3).

## 5. Updates

### 5.1 The suite's apps: detect only

- For each installed app, read `CFBundleShortVersionString` from its bundle and its
  release's manifest (`releases/latest/download/latest.json`), parsed with `method-update`.
  When the manifest's version is newer (semver), the card says "Version X is ready."
- **Open to update** opens the app. Each app's own updater (method-update, or the Tauri
  updater for Budget and Schedule) then shows its own "Update ready, restart to update". The
  launcher never downloads an update archive, never replaces an installed app, never touches
  another app's settings.
- To make that immediate, the launcher asks each app to accept one launch argument meaning
  "check for updates now" (a cross-app item, ROADMAP.md). Until an app supports it, the
  launcher opens the app and its card says "Choose Help > Check for Updates in Method
  Budget."
- Timing follows SUITE-STANDARDS.md §1: 2 minutes after launch, then every 6 hours while
  open, and when the window comes forward if the last check is more than an hour old. The
  Settings switch turns all of it off; Check for Updates... still works by hand.
- An app with no `latest.json` (no signed updater yet) shows the installed version only.

### 5.2 The launcher itself

The launcher updates itself the way Method Storyteller does: `method-update` pinned by full
commit, its own minisign key (1Password, "Method Launcher - Update signing key"), the
standard schedule and switch, Help > Check for Updates..., and What's New once after an
update.

## 6. Open and recent files

- **Open an app**: `/usr/bin/open -b <bundle id>`. Nothing else is passed.
- **Recent files**: each app keeps `recent.json` in its data folder
  (`~/Library/Application Support/<bundle id>/recent.json`, SUITE-STANDARDS.md §2). The
  launcher reads those files and nothing else. It reads them only; a missing, unreadable or
  unknown-shape file is skipped for that app, silently. The shared shape is a cross-app item
  (ROADMAP.md); the launcher accepts a list of entries each with an absolute `path` and an
  optional RFC 3339 `opened` time.
- **Open a recent file**: `/usr/bin/open -b <bundle id> <path>`, so the file opens in the app
  that listed it, whatever the system's default for that extension is. A path that no longer
  exists is dropped from the view, not opened.
- The launcher watches nothing in the background: it reads the lists when its window comes
  forward.

## 7. Samples

Each installed app's card has Open Sample: it opens the app with its Field Station sample
(SUITE-STANDARDS.md §8), as a fresh untitled copy, exactly as the app's own Help > Open
Sample does. The launcher ships no sample files of its own; it asks the app, by the same
kind of launch argument as §5.1 ("open the sample"). Until an app supports it, the button is
hidden for that app.

## 8. Failure cases

Every message is one plain sentence, says what happened and what to do, and has Details
(the technical reason, selectable) and Try Again where retrying could help.

| What happened | What the person sees |
|---|---|
| No network | "You're offline. Connect to the internet and try again." |
| GitHub unreachable, 5xx, timeout | "GitHub didn't answer. Try again in a few minutes." |
| 404 for the download | "Method Budget has no download yet." (catalog and release disagree) |
| Redirect to a host not on the list | "The download came from somewhere unexpected, so it was stopped." |
| Too big, or the download stalls | "The download stopped. Try again." |
| Not enough disk space | "There isn't enough free space to install Method Budget (it needs 210 MB)." |
| The image won't mount | "The download is damaged. Try again." |
| Not exactly one app, wrong name or bundle id | "The download isn't what we expected, so nothing was installed." |
| Signature, team, notarisation or architecture check fails | "This copy of Method Budget couldn't be verified, so nothing was installed." Details names the check. |
| Something else already has the name | "Something else called Method Budget.app is already in Applications." |
| Administrator prompt cancelled | "Not installed. You can install it in your own Applications folder instead." |
| Copy fails partway | "Method Budget couldn't be copied. Nothing was changed." (the staging copy is removed) |
| The manifest can't be read | the card keeps the installed version and shows "Couldn't check for updates" |
| An app is installed on an Intel Mac copy, or is older than the launcher understands | shown as installed, with the version it reports; never changed |
| The launcher quits mid-install | at next launch, leftover `.partial` downloads, staging copies and mounts it made are cleaned up |

## 9. Where things come from

- **The catalog** is built into the launcher, from a file in this repository
  (`crates/core/src/catalog.rs`), kept in step with the website's `apps.toml` by hand for
  now. A test checks every entry: version-free download name matching
  `Method-<Noun>-macOS.dmg`, bundle id `com.method<noun>.desktop`, https GitHub repository
  under `method-production-suite`. If the suite publishes the catalog as a file on
  methodproduction.com, the launcher can read it later, with the built-in copy as the
  fallback (open question, §10).
- **Network**: only `github.com` and GitHub's release-asset hosts, for manifests and
  downloads, and the suite's own domains for links a person clicks (which open in their
  browser). https only. No other host, ever, and the list is a constant in code with a test.

## 10. Open questions

For the owner, with recommendations, in ROADMAP.md "Waiting on the owner": the name, the
accent, the repository.

For MOrg and the other app sessions (cross-app), also in ROADMAP.md:

1. A public feed for "New in the apps", if the website wants one. Recommend: not in the
   first version.
2. Publishing the catalog as a file on methodproduction.com (for example `/apps.json` built
   from `apps.toml`). Recommend: later; built-in catalog first.
3. A shared launch argument in every app for "check for updates now" and "open the
   sample", and the shared `recent.json` shape.

## 11. Testing

Headless by default (the suite's rule): the core crate's logic (catalog, versions, Mach-O
reading, recent-file parsing, the redirect allow-list, the verify requirement string) by unit
tests; the install flow against local fixture images built in tests, with the signature
checks behind a trait so tests can use a fixed answer; and a `method-launcher` command line
(`list`, `status`, `verify <path.app>`, `install <slug> --to <folder>`) that drives the same
code without a window. Pictures for the README and site are drawn offscreen (SUITE-STANDARDS.md
§7). A real install of a real release into a scratch folder is a manual check before each
release, run by the owner from the ready-to-run list in ROADMAP.md.
