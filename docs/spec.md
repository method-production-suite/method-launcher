# Method Launcher: specification

Method Launcher is one small app, on macOS, Windows and Linux, that is the
front door to the Method Production Suite. It shows the suite's apps, installs the ones you
want, tells you when an update is ready, opens them, and opens your recent files in the right
app. One download instead of four, on whichever system you use.

Status: the first version's specification. Name, accent and the cross-app rules it relies on
were decided by the owner on 2026-10-18 (ROADMAP.md; SUITE-STANDARDS.md §2 and §3).

## 1. Scope

**Systems**

| System | Launcher builds | Apps it can install |
|---|---|---|
| macOS, Apple silicon and Intel (universal from 0.1.2) | yes | yes, the `.dmg`; on an Intel Mac only an app's universal releases |
| Windows x64 | yes | yes, the setup `.exe` |
| Linux x86_64 | yes | yes, the `.AppImage` (or the `.deb`, §4.4) |
| Linux arm64 | yes | not yet: the suite publishes no arm64 app builds, so each card says so and links the app's build-from-source steps |
| Intel Mac, Windows on ARM, 32-bit | no | no |

The screens are the same everywhere. What differs per system (where apps live, how they are
installed, verified and launched, where their recent-file lists are) sits behind one small
trait per concern (§10), so each system is one implementation and the window never asks which
system it is on.

**In the first version**: the four apps, in the order of a production (Method Storyteller,
Method Reader, Method Schedule, Method Budget); install, open, update notice, recent files,
samples, Help; the launcher's own updates (§5.2).

**Not in the first version**

- Uninstalling apps. The launcher can show an app's location; removing it is left to the
  person and the system's own tools.
- Installing an older version or a pre-release.
- News from the apps, unless the suite publishes a simple public feed (§11).

**Never**

- No server of our own, no accounts, no sign-in, no telemetry, no analytics, no crash
  reporting that sends anything on its own.
- Never runs, opens or copies anything it downloaded before that download has passed every
  check for its system (§4).
- Never updates an app without being asked: an update is a person pressing Update or
  Update all (§5.1), it goes through every check a new install does, and an open app is
  asked before it is quit.
- Never reads, moves or changes a person's documents. It reads only the apps' own recent-file
  lists (§6) and hands a file to its app to open.
- Never asks for administrator rights unless the chosen install needs them (§4), and says why
  first.
- No generative AI.

## 2. Words used here

- **Catalog**: the apps the launcher knows about: name, slug, one line, repository, bundle
  id, whether it has a release yet, accent. Built into the launcher (§9).
- **Target**: the system the launcher runs on: `macos-arm64`, `windows-x64`, `linux-x86_64`,
  `linux-arm64`.
- **Installer**: the release asset for a target, by its version-free name
  (SUITE-STANDARDS.md §3):

  | Target | Installer |
  |---|---|
  | macos-arm64 | `Method-<Noun>-macOS.dmg` |
  | windows-x64 | `Method-<Noun>-windows-x64-setup.exe` |
  | linux-x86_64 | `Method-<Noun>-linux-x86_64.AppImage`, or `Method-<Noun>-linux-amd64.deb` |
  | linux-arm64 | none yet |

- **Release**: the app's latest published GitHub release. Draft releases are never seen.
- **Checksums**: the release's `SHA256SUMS` file, in the usual `<hex>  <file name>` lines
  (§4.1).
- **Manifest**: the release's `latest.json` (SUITE-STANDARDS.md §1), read with
  `method-update`'s parser. The launcher uses its `version` on every system.

## 3. Screens and states

One window, 960 by 640 logical points by default, resizable down to 720 by 520. Light and
dark follow the system.

The launcher carries the Method Production Suite's branding (the owner, 2026-10-19): its
icon is the suite's mark on the family's white rounded square; the window opens with the
suite's wordmark, light or dark with the system, and "Install the apps you want. Each one
updates itself." under it; colours are the suite's tokens (the website's
`shared/tokens-base.css`) and the type is the system's own sans-serif, as the sites use.
Ochre (`#9A5B00` light, `#F0B54A` dark) is the launcher's own accent, for its own main
buttons (Update all, Quit and update) and focus only; each app's card uses that app's
accent for its main button, as the suite's website does.

The layout (a design pass, 2026-10-19): one 4-point grid with 32-point margins; the
wordmark and the promise top left, and "Checked N minutes ago", Refresh and Update all top
right, on the cards' edges; two columns of cards from 820 points wide; a footer that says
where downloads come from. Every card has the same rows: the app's icon (56 points, the
installed app's own icon trimmed to its shape, or the app's mark) with its name and one
line; a status line with a small tag ("0.5.0 available" in the app's accent, "Up to date",
"In development"); up to two quieter lines (the release date and the first sentence of its
notes, or what went wrong); and one action row with a single filled button and quiet links,
centred on one line. An app that can't be installed here gets a quiet link, never a filled
button.

### 3.1 The apps

The main view. One card per catalog app, in catalog order, one column at narrow widths and two
at wider ones. Each card shows the app's mark in its accent, its name, its one line, its
status from the catalog (for example "Alpha, ready for testing"), and one main button whose
label follows the app's state:

| State | What the card says | Main button | Other actions |
|---|---|---|---|
| No release yet | "In development. No download yet." | Visit the site | |
| No build for this system | "There's no Method Budget for Linux on ARM yet. You can build it from source." | Build from source (opens the steps) | Visit the site |
| Not installed | the release's version, "Not installed" | Install | Release notes |
| Downloading | progress bar, bytes of total | Cancel | |
| Checking | "Checking the download" | (none) | |
| Installing | "Copying to Applications" (macOS), "The installer is open" (Windows), "Installing" (Linux) | (none) | |
| Installed, current | "Version 0.4.0" | Open | Show location, Release notes, Open Sample |
| Installed, update ready | "Version 0.4.0. Update available: 0.5.0" | Update to 0.5.0 | Open, Show location, Release notes |
| Installed, check failed | "Version 0.4.0" and a quiet "Couldn't check for updates" | Open | Try again |
| Install failed | one sentence saying what went wrong (§8) | Try again | Details |

"Show location" is Show in Finder on macOS, Show in Explorer on Windows, Show in Files on
Linux. Only one install runs at a time; other Install buttons wait and say "Waiting".

### 3.2 Recent files

Below the cards, or as a second tab at narrow widths: the most recent files across the suite,
newest first, up to 20. Each row: the file's name, the app's mark, the folder it is in
(shortened from the home folder, `~/Films/Field Station`), and when it was last opened. A
file whose app is not installed shows "Install Method Schedule to open this" with the Install
button. A file that no longer exists is not shown. Double-click or Enter opens it in its app
(§6). Context menu: Show location, Open with its app, Remove from this list (hides it in the
launcher only; the app's own list is never changed).

Empty state: "Files you open in the Method apps show up here."

### 3.3 First launch

The window opens on the apps with one quiet line above them: "Install the apps you want.
Each one updates itself." Nothing installs on its own.

### 3.4 Settings

Settings is a window, opened from the Settings button beside Help or with ⌘, (Ctrl+, on
Windows and Linux), saved in `settings.json` in the launcher's data folder.

- "Check for updates automatically" (on by default, SUITE-STANDARDS.md §1).
- "Open the launcher when I log in" (off by default): a LaunchAgent in
  `~/Library/LaunchAgents` on macOS, an entry in `~/.config/autostart` on Linux, and a value
  under the per-user Run key on Windows. The system's own record is the setting.
- macOS: "Install apps in: Applications / my Applications folder" (`/Applications` or
  `~/Applications`).
- Linux on a system with apt: "Install apps as: AppImage in my Applications folder / system
  package (.deb)" (AppImage by default, §4.4).

### 3.5 Menus

The macOS menu bar, or the window's own menu bar on Windows and Linux.

- **Method Launcher** (macOS only): About, Settings, Check for Updates..., Quit.
- **File**: Open Recent (the same list as §3.2); on Windows and Linux also Settings and
  Quit (Exit on Windows).
- **Help** (SUITE-STANDARDS.md §4): About, User Guide, Keyboard Shortcuts, What's New, Check
  for Updates..., Report a Bug, Licenses and Notices, methodproduction.com. The window draws
  its own Help menu on every system, so About is there on the Mac too.

## 4. Install and verify

Every step runs off the interface thread; the card shows where it is. Any failure stops the
install, removes everything it made, and shows one plain sentence (§8).

### 4.1 Steps common to every system

1. **Pin the release.** Request `https://github.com/method-production-suite/<repo>/releases/latest`
   without following the redirect; its `Location` names the tag (`.../releases/tag/v0.4.0`).
   Every later download for this install uses that tag
   (`.../releases/download/v0.4.0/<file>`), so the checksums and the installer always come
   from the same release, even if a new one is published mid-install. The tag must parse as a
   version.
2. **Fetch the checksums** (Windows and Linux; on macOS too where the release has them, as
   a second check): `SHA256SUMS` from that release. A release whose checksum file is
   missing, or has no line for the installer, cannot be installed on Windows or Linux: the
   card says "This release can't be checked yet, so it can't be installed from here" and
   offers the release page. Every release now publishes `SHA256SUMS` (SUITE-STANDARDS.md §3);
   for releases made before that rule, the launcher also reads `SHA256SUMS-app` (Method
   Reader's earlier name for its installers' checksums).
3. **Download** the installer.
   - https only. Requests start at `github.com`; redirects are followed only to GitHub's
     release-asset hosts (an allow-list in code, the same list `method-update` uses), and only
     over https. Any other redirect fails the install.
   - Into the launcher's cache folder (`~/Library/Caches/<bundle id>/downloads`,
     `%LOCALAPPDATA%\<bundle id>\cache\downloads`, `$XDG_CACHE_HOME/<bundle id>/downloads`),
     owner-only, as `<slug>-<random>.partial`, renamed when complete.
   - At most 1 GiB. A stall of 60 seconds fails with "The download stopped". Cancel removes
     the file.
4. **Hash** the file while it downloads (SHA-256) and compare with the checksum line when
   there is one. A mismatch deletes the file.
5. Then the system's own steps below.

What a checksum proves: that the file is the one the release lists, unchanged on the way.
Both files come from the same release, so it doesn't prove who made the release; a signature
does that (Developer ID on macOS today, Authenticode on Windows once the suite signs Windows
builds). `SHA256SUMS.minisig`, signed with each app's update key (SUITE-STANDARDS.md §3,
later), closes that gap; the launcher checks it when it is there.

### 4.2 macOS: the .dmg

1. **Mount read-only**:
   `hdiutil attach -readonly -nobrowse -noautoopen -mountrandom <cache dir> <file.dmg>`, with
   the mount point read from `hdiutil`'s plist output. Always detached afterwards, on success
   and on failure. The image must hold exactly one `.app` at its top level, named as the
   catalog says (`Method Budget.app`).
2. **Verify, before anything is copied.** All must pass, using tools that ship with macOS,
   called by full path, or Rust code in the launcher:
   - **The right app**: `Info.plist` has the catalog's bundle id and a version that parses.
   - **Signed by the suite**: `codesign --verify --deep --strict` with the requirement
     ```
     anchor apple generic
       and certificate leaf[subject.OU] = "XSRB3D643J"
       and certificate 1[field.1.2.840.113635.100.6.2.6]
       and certificate leaf[field.1.2.840.113635.100.6.1.13]
     ```
     that is, an Apple-rooted Developer ID Application certificate for team XSRB3D643J and
     no other. The team id is a constant in code with a test that pins it.
   - **Notarised**: `spctl --assess --type execute -vv` accepts it as a notarised Developer
     ID app (the ticket is stapled, so this works offline too).
   - **This Mac's chip** (from 0.1.2, when the suite's Mac builds went universal): every
     Mach-O file in the bundle has a slice for this Mac's own chip, so a universal app installs
     on any Mac and an Apple-silicon-only release installs only on Apple silicon. On an Intel
     Mac such a release says "This version of Method <Noun> needs an Apple silicon Mac" and
     offers nothing to install; the launcher knows it before downloading when the release's
     `latest.json` has no `darwin-x86_64` entry. (Until 0.1.2: every Mach-O file in the bundle is thin arm64 or fat with arm64
     slices alone, read by the launcher in Rust (the same rule the release workflows
     enforce).
   - **No escaping links**: no symbolic link in the bundle points outside it.
3. **Copy**: to `/Applications/<Name>.app` (or `~/Applications`, per Settings). An app with
   the same bundle id already there means it is installed (an update is §5); a different
   bundle at that path stops the install. `ditto` to a hidden staging name beside the
   destination, check the copy's signature again, then rename into place (atomic on one
   volume). If the folder isn't writable, the system's own administrator prompt for that copy
   and rename alone, after saying why: "Applications is protected on this Mac. macOS will ask
   for an administrator's password to copy Method Budget there." Cancel offers
   `~/Applications`.
4. No quarantine flag is added: the launcher has made Gatekeeper's notarisation check itself.

### 4.3 Windows: the setup .exe

1. **Verify**: the SHA-256 matches the checksum line (§4.1). Once the suite signs Windows
   builds, also the Authenticode signature (`WinVerifyTrust`) with the suite's publisher
   name, pinned in code; until then the card's Details says "Checked against the release's
   checksums. Windows builds are not signed yet."
2. **Run the installer** as the signed-in person, with its own window, and wait for it to
   finish. The launcher passes nothing that changes where or how it installs. Windows'
   SmartScreen may warn about an unsigned installer; the card says so beforehand: "Windows
   may say it doesn't recognise this app. Choose More info, then Run anyway." (until the
   builds are signed).
3. **Confirm**: re-read the installed apps (§4.6). If the app isn't there after the installer
   exits, the card says "The installer closed before Method Budget was installed."

### 4.4 Linux: the AppImage, or the .deb

**AppImage (the default, no administrator needed)**

1. **Verify**: the SHA-256 matches the checksum line (§4.1).
2. **Place it**: `~/Applications/Method-Budget.AppImage` (the folder made if needed), copied
   to a hidden staging name, made executable (0755), then renamed into place.
3. **Desktop entry**: `~/.local/share/applications/com.methodbudget.desktop.desktop`
   (SUITE-STANDARDS.md §2) with the app's
   name, one line, `Exec="<path>" %F`, and the app's mark as its icon (the launcher ships the
   marks, so nothing in the AppImage is run to get one), then `update-desktop-database` on
   that folder if it exists. The launcher records what it installed (app, version, path) in
   its own `installed.json`.
4. AppImages need FUSE on some systems. If the AppImage fails to start for that reason, the
   card links the suite's help page on it rather than trying to install system packages.

**.deb (a Settings choice on systems with apt)**

1. **Verify**: the SHA-256 matches.
2. **Install**: `pkexec apt-get install -y <file.deb>` (apt resolves its dependencies), after
   saying why: "Installing a system package needs an administrator's password." Cancel leaves
   nothing installed.
3. **Confirm**: `dpkg-query` shows the package and its version (§4.6).

**Linux on ARM**: no install button. "There's no Method Budget for Linux on ARM yet. You can
build it from source." linking the app's README build steps.

### 4.5 After any install

The cache file is deleted, the card turns to "Installed, current" with Open as its main
button. A failure cleans up the same way; at next launch the launcher removes any leftover
partial downloads, staging copies and mounts of its own.

### 4.6 What counts as installed

| System | Where the launcher looks | Version from |
|---|---|---|
| macOS | `/Applications` and `~/Applications`, bundles with the catalog's bundle id | `CFBundleShortVersionString` |
| Windows | the per-user and per-machine uninstall entries in the registry, matched by display name `Method <Noun>` | `DisplayVersion`; the executable from `InstallLocation` |
| Linux, AppImage | the launcher's `installed.json`, then the files it names | the version recorded at install |
| Linux, .deb | `dpkg-query -W method-<noun>`, and `method-<noun>-app` on the PATH to open it (never the `.desktop` file's name, which varies) | the package version |

An app installed by hand (dragged in, or installed from the website) is found the same way
and shown as installed.

## 5. Updates

### 5.1 The suite's apps: the launcher installs the update

The owner's call (2026-10-19): "we should be able to directly download the apps and update
the apps from the launcher." So the launcher updates the apps itself, on every system,
macOS first; each app's own updater stays a second way.

- For each installed app, compare its installed version (§4.6, read from the bundle,
  registry entry or package) with the release manifest's `version`
  (`releases/latest/download/latest.json`). Newer (semver) shows "Update available: X" and
  the card's main button becomes **Update to X**.
- **Update** downloads the latest release's installer and checks it exactly as an install
  does (§4.1, and §4.2 to §4.4 for each system: on macOS the Developer ID of team
  XSRB3D643J, notarisation, code for this Mac's chip and `SHA256SUMS` where the release has it), then
  replaces the installed copy. On macOS the old bundle is moved aside, the new one renamed
  into place, and the old one removed only once that worked. On Windows the setup upgrades
  in place; on Linux the AppImage is replaced through its staging name, the `.deb` through
  apt.
- **An open app is asked first.** If the app is running, the launcher says "Method Budget is
  open. Method Launcher will ask it to quit, so it can ask you about anything unsaved, then
  update it." with **Quit and update** and **Cancel**. On macOS it asks the app to quit the
  way its Quit menu does (macOS asks once whether Method Launcher may control it) and waits
  up to two minutes; if the app is still open, nothing is replaced ("Method Budget is still
  open, so it wasn't updated"). On Windows it asks the app to close its windows (`taskkill`
  without `/F`), and on Linux it sends the app SIGTERM, each with the same two minutes.
- **An earlier name is replaced.** An app installed under its earlier name or file name
  (Method Reader as Screepub Desktop) is installed under its current name, and the earlier
  copy removed once the new one is in place.
- **Update all** (beside Refresh, in the launcher's ochre) runs every pending update, one at a
  time, with one question for all the apps that are open.
- The launcher never downloads an app's update archive (its `.app.tar.gz`); it uses the same
  installer a new install does.
- Timing: the launcher checks at launch, on Refresh (⌘R), and again whenever its last check
  is 6 hours old while it stays open. Help's "Check for Updates Automatically" switch
  (SUITE-STANDARDS.md §1) turns off the automatic checks; Refresh always checks.

### 5.2 The launcher itself

The owner's call (2026-10-19): "method launcher should have the same auto update
functionality that the individual apps have".

- **On Macs** (Apple silicon and Intel) the launcher updates itself through the suite's updater,
  `method-update`, pinned by full commit (SUITE-STANDARDS.md §1, Method Budget's way): a
  quiet check 2 minutes after launch and every 6 hours; when a newer release is out it
  downloads it, checks its minisign signature against the launcher's own key (`PUBKEY`,
  key ID E115E2CD9AEE0DF9), and shows "Update ready: Method Launcher X. Restart to update"
  with **Restart to Update**, which swaps the bundle and starts the new one. An update
  interrupted part-way is put right at the next launch.
- **Help** holds Check for Updates…, the "Check for Updates Automatically" switch (on by
  default; it covers both the launcher's own updates and the apps' checks), What's New,
  Report a Bug and methodproduction.com.
- **On Windows and Linux** the updater doesn't run yet: the launcher reads where its own
  `releases/latest` redirects and, when that tag is newer, says "Method Launcher X is
  available" with **Download it**. They move to the updater when it covers them.

## 6. Open and recent files

- **Open an app**: macOS `open -b <bundle id>`; Windows, the app's executable from §4.6;
  Linux, the AppImage path, or `method-<noun>-app` on the PATH for the `.deb`. Nothing else
  is passed.
- **Launch arguments** (`--check-for-updates`, `--open-sample`, SUITE-STANDARDS.md §2) work
  only when the app starts: an app without single-instance handling drops them on a second
  launch. So the launcher passes them only when the app isn't running. When it is, the
  launcher brings it forward with no arguments, and the card says what to choose instead:
  "Method Budget is open. Choose Help > Check for Updates" (or Help > Open Sample).
- **Recent files**: each app keeps `recent.json` in its data folder named by its bundle id
  (SUITE-STANDARDS.md §2): `~/Library/Application Support/<id>/` on macOS,
  `%APPDATA%\<id>\` on Windows, then `%LOCALAPPDATA%\<id>\` (where Method Budget, like
  other apps on its framework, keeps it), `$XDG_DATA_HOME/<id>/` (default `~/.local/share/<id>/`) on
  Linux. The launcher reads those files only; a missing, unreadable or unknown-shape file is
  skipped for that app. The shape (SUITE-STANDARDS.md §2) is a list, newest first, of
  entries each with an absolute `path` and an RFC 3339 `opened` time.
- **Open a recent file**: the same launch as above with the file's path as the one argument
  (macOS `open -b <id> <path>`), so it opens in the app that listed it, whatever the system's
  default for that extension is.
- The launcher watches nothing in the background: it reads the lists when its window comes
  forward.

## 7. Samples

Each installed app's card has Open Sample: it opens the app with its Field Station sample
(SUITE-STANDARDS.md §8), as a fresh untitled copy, exactly as the app's own Help > Open
Sample does, by opening the app with `--open-sample` (SUITE-STANDARDS.md §2) when it isn't
already running (§6). For an installed version older than that rule, the button is hidden. The launcher ships no sample
files.

**Demo the suite** (the owner, 2026-10-19): a quiet button beside Refresh opens every
installed app on its Field Station sample at once: Storyteller on the script, Reader on the
PDF, Schedule on the schedule, Budget on the budget. Each app that isn't running is opened
with `--open-sample`; one that is running is brought forward with nothing passed, and the
note says "Budget is open: use Help > Open Sample". Apps that aren't installed are named as
skipped. It changes no app's settings.

## 8. Failure cases

Every message is one plain sentence, says what happened and what to do, and has Details (the
technical reason, selectable) and Try Again where retrying could help.

| What happened | What the person sees |
|---|---|
| No network | "You're offline. Connect to the internet and try again." |
| GitHub unreachable, 5xx, timeout | "GitHub didn't answer. Try again in a few minutes." |
| No installer for this system on the release | "This release of Method Budget has no download for Windows." |
| No checksum file, or no line for the installer (Windows, Linux) | "This release can't be checked yet, so it can't be installed from here." with the release page |
| Checksum mismatch | "The download doesn't match the release's checksum, so it was deleted. Try again." |
| Redirect to a host not on the list | "The download came from somewhere unexpected, so it was stopped." |
| Too big, or the download stalls | "The download stopped. Try again." |
| Not enough disk space | "There isn't enough free space to install Method Budget (it needs 210 MB)." |
| macOS: the image won't mount | "The download is damaged. Try again." |
| macOS: not exactly one app, wrong name or bundle id | "The download isn't what we expected, so nothing was installed." |
| macOS: signature, team, notarisation or architecture check fails | "This copy of Method Budget couldn't be verified, so nothing was installed." Details names the check. |
| macOS: something else has the name | "Something else called Method Budget.app is already in Applications." |
| Administrator prompt cancelled | macOS: "Not installed. You can install it in your own Applications folder instead." Linux .deb: "Not installed. You can install the AppImage instead." |
| Windows: installer closed without installing | "The installer closed before Method Budget was installed." |
| Windows: installer reported an error | "The installer stopped with an error." with its exit code in Details |
| Linux: AppImage won't start (FUSE missing) | "Method Budget needs FUSE to run on this system." with the help link |
| Copy or placement fails partway | "Method Budget couldn't be installed. Nothing was changed." |
| The manifest can't be read | the card keeps the installed version and shows "Couldn't check for updates" |
| The launcher quits mid-install | at next launch its leftovers are cleaned up (§4.5) |

## 9. Where things come from

- **The catalog** (from 0.1.1) is read from the suite's website,
  `https://methodproduction.com/apps.json`, made from the website's `apps.toml`: a list, in
  the order to show, of `{"slug", "name", "tagline", "repo", "bundle_id", "site", "accent":
  ["#light", "#dark"]}` (extra fields are ignored). Every entry must follow the suite's rules
  ("Method <Noun>", slug, `com.method<noun>.desktop`, a `method-` repository under the
  organisation, the app's own site `https://method<slug>.com` or the suite's, two `#RRGGBB`
  accents), or the whole list is refused. The list can't move what is downloaded: an app the
  launcher knows keeps its built-in repository and bundle id, and a new app's repository
  must be `method-<slug>`.
  The last good list is kept for the next launch; the built-in list
  (`crates/core/src/catalog.rs`, tested by the same rules) is the fallback. A new list is
  shown only when no install is running. An app the launcher has no mark for draws its
  accent.
- **Releases are read at run time** (from 0.1.1). An app has a release when its
  `releases/latest` redirects to a version tag; it has an installer for this system when the
  release's `SHA256SUMS` (or, for older releases, `SHA256SUMS-app`) lists one. The catalog's
  `released` flag is only the fallback when the repository can't be asked. So a first
  release shows up on the next check, with no new launcher.
- **Network**: only `github.com` and GitHub's release-asset hosts, for manifests, checksums
  and downloads, and `methodproduction.com`, for the list of apps; links a person clicks
  open in their browser. https only. The list is a
  constant in code with a test.

### 9.1 What the current releases carry (2026-10-18)

| App | Release | macOS .dmg | Windows setup .exe | Linux AppImage / .deb | Checksums | Manifest platforms |
|---|---|---|---|---|---|---|
| Method Storyteller | none yet | | | | | |
| Method Reader | v0.13.0 | yes | yes | yes / yes | `SHA256SUMS-app` covers the installers (`SHA256SUMS` covers only the command-line archives) | darwin-aarch64 |
| Method Schedule | v0.1.0 | yes | yes | yes / yes | **none** | darwin-aarch64 |
| Method Budget | v0.4.0 | yes | yes | yes / yes | **none** | darwin-aarch64 |

So today the launcher can install Method Reader on every system, and Method Schedule and
Method Budget on macOS only. SUITE-STANDARDS.md §3 now requires one `SHA256SUMS` listing
every file on every release, so their next releases fix this.

## 10. Platform traits

The window and the install flow are written once, against these traits (in
`launcher-core::platform`); each system has one implementation, and tests use a fake one.

| Trait | What it answers |
|---|---|
| `Locate` | Which catalog apps are installed, where, and at which version (§4.6). |
| `Launch` | Whether an app is running; open it (with launch arguments only when it isn't); open a file in its app; show a location (§6). |
| `Install` | Turn a downloaded, checksum-checked installer into an installed app (§4.2 to §4.4), including its own verification. |
| `Folders` | The launcher's cache and data folders, and an app's data folder for `recent.json`. |

## 11. Decided

The owner's decisions and the cross-app rules (2026-10-18) are recorded in ROADMAP.md and
SUITE-STANDARDS.md §2 and §3. Not in the first version: a catalog file on
methodproduction.com and a "New in the apps" feed.

## 12. Testing

Headless by default (the suite's rule), on all three systems in CI: the core crate's logic
(catalog and installer names per target, versions, Mach-O reading, checksum parsing and
hashing, the redirect allow-list, the signing requirement) by unit tests; the install flow
against a fake `Install` and local fixture files; and a `method-launcher` command line
(`list`, `status`, `verify <file>`, `install <slug> --to <folder>`) that drives the same code
without a window. Pictures for the README and site are drawn offscreen (SUITE-STANDARDS.md
§7). A real install of each released app on each system is a manual check before each
release, from the ready-to-run list in ROADMAP.md.
