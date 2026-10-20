# Releasing Method Launcher

Every release follows the suite's checklist (`RELEASE-CHECKLIST.md` in the org's `.github`
repository); this page adds the launcher's own commands. A tag needs the owner's yes, every
time.

## What a release is

`.github/workflows/release.yml`, on a `vX.Y.Z` tag, builds and tests on every system before
anything is signed, then makes a DRAFT release with:

| File | What |
|---|---|
| `Method-Launcher-macOS.dmg` | macOS, universal (Intel and Apple silicon): Developer ID signed, notarised, stapled |
| `Method-Launcher-windows-x64-setup.exe` | Windows x64 installer (unsigned until the suite has a certificate) |
| `Method-Launcher-linux-amd64.deb` | Debian and Ubuntu |
| `Method-Launcher-linux-x86_64.AppImage` | any x86_64 Linux |
| `Method-Launcher-linux-arm64.deb` | Debian and Ubuntu on ARM (from 0.2.0) |
| `Method-Launcher-linux-aarch64.AppImage` | any Linux on ARM (from 0.2.0) |
| `SHA256SUMS` | every file above |
| `SHA256SUMS.minisig` | `SHA256SUMS`, signed with the update key |
| `Method-Launcher_<version>_universal.app.tar.gz`, `.sig`, `latest.json` | the self-update for installed launchers on every Mac (`darwin-aarch64` and `darwin-x86_64`) |

The names never change, so the website links `releases/latest/download/<file>`. A person
checks the draft and publishes it.

The launcher updates itself on Macs (Apple silicon and Intel) through the suite's updater,
`method-update` (docs/spec.md §5.2), the way the apps do. On Windows and Linux it shows
"Method Launcher X is available" with a link to the download, until the updater covers them.

## The update key

The launcher's own minisign key, key ID E115E2CD9AEE0DF9.

- **Public half:** `PUBKEY` in `crates/app/src/updates.rs`, the one place it lives in the
  repository. Installed launchers refuse any update not signed by it, so a new key means a
  release with the new `PUBKEY` first.
- **Private half and its password:** 1Password, "Method Launcher updater key", and the
  repository secrets `TAURI_SIGNING_PRIVATE_KEY` (the base64 of the secret key box) and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (the suite's secret names). Never in the repository,
  an issue, a log or a chat.
- **Made with:** `cargo xtask new-update-key --out-dir <folder outside the repository>`. It
  writes the secret key box and a random password owner-only, and prints only the public
  half.

With the key and a notarised Mac build, a release also carries
`Method-Launcher_<version>_universal.app.tar.gz`, its `.sig`, `latest.json` (listing it under
`darwin-aarch64` and `darwin-x86_64`, checked against
`PUBKEY` before it is written) and `SHA256SUMS.minisig` (the checksums, signed with the same
key). Without the key the release is the same minus those files, and installed launchers
don't see it.

## Before the tag

1. **Version and notes.** The version in `Cargo.toml` (`[workspace.package]`), and
   `docs/releases/<version>.md`, written from [release-notes-template.md](release-notes-template.md).
   The release workflow refuses to build without the notes.
2. **The full local check** on a Mac: `cargo xtask ci`.
3. **The weekly full build, by hand**, on `main`: Actions, **weekly**, Run workflow. Wait
   for green.
4. **A rehearsal** of the release itself, on `main`:

   ```bash
   gh workflow run release.yml --ref main -f rehearse_version=0.1.0
   ```

   It runs the crate-age gate, every build, signature, notarisation and staple, and every
   bundle check, and skips the draft. It runs only from `main`, since it uses the real
   signing keys, and it honours the crate-age list's age-out entries, as `main` does (a real
   release does not). Delete its artifacts once it is green (`RUN` is its run id):

   ```bash
   for id in $(gh api repos/method-production-suite/method-launcher/actions/runs/RUN/artifacts --jq '.artifacts[].id'); do gh api -X DELETE repos/method-production-suite/method-launcher/actions/artifacts/$id; done
   ```

Tag only when all of these are green, and only with the owner's yes.

## The tag and the draft

```bash
git tag -a v0.1.0 -m "Method Launcher 0.1.0"
git push origin v0.1.0
```

Then check the draft: the four installers and `SHA256SUMS`, the notes, and the macOS build
opening without a warning on a Mac that has never seen it. Publish it, and tell MOrg, so the
website's launcher page follows the new release.

## On a developer's machine

The same packaging the workflow runs:

```bash
cargo xtask bundle-macos --target universal-apple-darwin # both slices joined by lipo; Method Launcher.app, ad hoc signed
cargo xtask dmg-macos                                    # target/bundle/macos/Method-Launcher-macOS.dmg
cargo xtask package-linux                                # the .deb (add --appimagetool and --runtime for the AppImage)
cargo xtask package-windows                              # the installer (Inno Setup 6)
```

`python3 scripts/make-icons.py` redraws the icons and the wordmark pictures from
`assets/brand/`.
