# Method Launcher user guide

Method Launcher shows the apps of the Method Production Suite, installs the ones you want,
keeps them up to date and opens them. Each app also updates itself, so you can use the
launcher once and forget it, or keep it as your starting point.

## The window

Each app has a card with its name, what it is for, and whether it is on this computer.

- **Not installed:** the card offers **Install**. When the app has no download for this
  computer yet, the card offers its site or download page instead.
- **Installed:** the card shows the version you have, with **Open**, **Show in Finder**
  (Show in Explorer on Windows, Show in Files on Linux) and **Release notes**.
- **A newer version is out:** the card offers **Update to** that version. **Update all**, at
  the top of the window, updates every app that has one.

The launcher looks again when its window comes forward and when you press **Refresh**
(⌘R on a Mac, Ctrl+R on Windows and Linux).

## Installing and updating

The launcher downloads each app from that app's own releases and checks it before anything
is copied:

- **On a Mac:** the app must be signed by the suite, notarised, and have code for your Mac's
  chip. If a version of an app only runs on Apple silicon and your Mac has an Intel chip, the
  card says so and offers nothing to install.
- **On Windows and Linux:** the download must match the checksum the release publishes. A
  release without one can't be checked yet, and the launcher won't install it.

If any check fails, nothing is installed and the card says why in one sentence. If an app
is open when you update it, the launcher asks before quitting it.

## Recent files and the sample

**Recent files** lists what you opened last in any of the apps; choose one to open it in its
app. **Demo the suite** opens every installed app on Field Station, the suite's invented
sample production. An app that is already open is left as it is, with a note to use its own
Help > Open Sample.

## Settings

Open Settings from its button beside Help, or with ⌘, (Ctrl+, on Windows and Linux).

- **Check for updates automatically:** the same switch as in the Help menu. With it on, the
  launcher looks for new versions at launch and every 6 hours while it is open.
- **Install apps in** (Mac): Applications, or your own Applications folder in your home
  folder, which needs no administrator password. It applies to new installs; an update goes
  where the app already is.
- **Install apps as** (Linux with apt): the AppImage in your Applications folder, which needs
  no password, or the system package (.deb), which asks for an administrator's password and
  lets apt add what the app needs. An update goes the way the app is installed now.

## The Help menu

- **About Method Launcher:** the version and the licence.
- **User Guide:** this page.
- **Keyboard Shortcuts:** the keys the launcher knows.
- **What's New:** the notes for this version.
- **Check for Updates…** and **Check for Updates Automatically:** on by default. The switch
  covers both the launcher's own updates and its checks for new versions of the apps.
- **Report a Bug:** opens a new bug report in your browser with the launcher's version and
  your system filled in. Nothing is sent until you submit it.
- **Licenses and Notices:** the launcher's licence and notices.

## The launcher's own updates

On a Mac, the launcher downloads its new versions in the background, checks their
signature, and offers **Restart to Update**. On Windows and Linux it says when a new version
is out, with a link to the download.

## Privacy

The launcher reads the suite's list of apps from methodproduction.com and each app's
releases from GitHub. It sends nothing about you or your files.
