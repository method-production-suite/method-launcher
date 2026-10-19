# Method Launcher

One small Mac app that is the front door to the Method Production Suite: see the suite's
apps, install the ones you want, know when an update is ready, open them, and open your
recent files in the right app. One download instead of four.

- **Install with one click.** Each app comes from its own latest release on GitHub, and is
  checked before anything is copied: signed by the suite's Developer ID, notarised by Apple,
  and built for Apple silicon. If a check fails, nothing is installed.
- **Each app updates itself.** The launcher tells you when a new version is ready and opens
  the app; the app's own updater does the rest, so there is one way each app updates.
- **Your recent files, across the suite.** Scripts, schedules and budgets you opened lately,
  each opening in the app it belongs to.
- **Field Station, ready to open.** Every app's invented sample production, one click away.

## Yours to keep

- Free, and open source under MIT OR Apache-2.0.
- No account, no server of its own, no tracking. It reaches only GitHub, to check for and
  download releases, and you can turn update checks off.
- It never touches your files: it reads the apps' own recent-file lists and hands a file to
  its app.

## Status

Status: in development. See [docs/spec.md](docs/spec.md) for what it does and
[docs/ROADMAP.md](docs/ROADMAP.md) for what comes next.

| Platform | Built | Signed | Tested |
| --- | --- | --- | --- |
| macOS (Apple silicon) | From source | No | CI tests |

## Build from source

You need Rust (the version in `rust-toolchain.toml` installs itself through rustup).

```
cargo run -p method-launcher        # the window
cargo run -p launcher-cli -- list   # the command line
cargo xtask ci                      # every check CI runs
```

## Licence

MIT OR Apache-2.0, at your option ([LICENSE-MIT](LICENSE-MIT),
[LICENSE-APACHE](LICENSE-APACHE)). Third-party notices are in [NOTICE](NOTICE).
