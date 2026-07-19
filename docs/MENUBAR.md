# macOS Menu Bar App

PlainLink includes a native macOS menu bar shell built with Swift and AppKit. It does not need Xcode; Apple Command Line Tools are enough.

The menu bar app is intentionally thin. The Rust CLI remains the engine for cleaning, LaunchAgent management, restore, and diagnostics.

## Build

```sh
scripts/build-macos-app.sh
```

The script creates:

```text
dist/PlainLink.app
```

The app bundle contains:

```text
Contents/MacOS/PlainLinkMenu   Native AppKit status bar app
Contents/MacOS/plainlink       Release Rust CLI used by the app
Contents/Info.plist            LSUIElement menu bar app metadata
```

## Smoke Test

```sh
scripts/test-macos-app.sh
```

This builds the bundle, validates `Info.plist`, confirms both executables exist, runs the menu app smoke test, and checks the embedded CLI version command.

## Package

```sh
scripts/package-macos-app.sh
```

The package script rebuilds and smoke-tests the app, then writes:

```text
dist/packages/PlainLink-<version>-macos-<arch>.zip
dist/packages/PlainLink-<version>-macos-<arch>.zip.sha256
```

The zip is unsigned and not notarized. It is meant for MVP testing and GitHub Actions artifacts.

Verify a downloaded package:

```sh
cd dist/packages
shasum -a 256 -c PlainLink-<version>-macos-<arch>.zip.sha256
```

## Runtime Flow

```mermaid
flowchart TB
    Menu["PlainLink.app menu bar UI"] --> Runner["PlainLink command runner"]
    Runner --> CLI["Embedded plainlink CLI"]
    CLI --> Install["install / agent commands"]
    CLI --> Doctor["doctor"]
    CLI --> Clean["clean-clipboard"]
    CLI --> Restore["restore"]
    Install --> Agent["User LaunchAgent"]
    Agent --> Watch["plainlink watch"]
    Watch --> Clipboard["macOS clipboard"]
    Clean --> Clipboard
    Restore --> Clipboard
```

## Menu Actions

- Enable, pause, start, and restart clipboard cleaning.
- Select watcher interval.
- Clean the current clipboard once.
- Restore the last original URL.
- Run doctor diagnostics.
- Copy diagnostics to the clipboard.
- Open support and log folders.
- Quit the menu bar app without stopping the LaunchAgent.

## Design Notes

- The app is a user-level status bar app with `LSUIElement`.
- The app shells out to the embedded `plainlink` binary instead of duplicating core logic.
- `plainlink install` copies the embedded CLI to the stable user path before starting the watcher.
- Pausing uses `plainlink agent uninstall`, which stops the watcher without deleting the installed binary.
- Full uninstall remains available through the CLI with `plainlink uninstall`.
- Signing and notarization are intentionally out of scope for the current MVP.
