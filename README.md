# PlainLink

Clean copied links before you share them.

PlainLink is a local-first URL cleaner. The MVP is a Rust command line tool with a portable cleaning engine, a macOS clipboard watcher, and a native macOS menu bar app. The project goal is to become a community-maintained, ad-blocker-list-style ruleset for removing tracking parameters from copied URLs at the system clipboard level.

```mermaid
flowchart LR
    Copy["User copies a URL"] --> Clipboard["macOS clipboard"]
    Menu["PlainLink menu bar app"] --> Watcher["plainlink watch"]
    Clipboard --> Watcher
    Watcher --> Engine["plainlink-core"]
    Engine --> Rules["rules/base.plainlink"]
    Rules --> Engine
    Engine --> Cleaned["Clean URL"]
    Cleaned --> Clipboard
```

## Status

PlainLink is early MVP software.

- Cleans URLs from the CLI with `plainlink clean`.
- Explains removed parameters with `plainlink inspect`.
- Restores the last cleaned original URL with `plainlink restore`.
- Watches the macOS clipboard with `plainlink watch`.
- Cleans the current clipboard once with `plainlink clean-clipboard`.
- Provides a native macOS menu bar app built with Apple Command Line Tools.
- Installs PlainLink to a stable user path with `plainlink install`.
- Installs PlainLink as a user LaunchAgent with `plainlink agent install`.
- Validates community rule behavior with fixture-backed tests.
- Uses conservative rules that preserve unknown parameters by default.

## Quick Start

```sh
cargo test
cargo run -- clean 'https://youtu.be/LYa_ReqRlcs?si=VC4qVB_EUC90uwbo'
cargo run -- inspect 'https://example.com/read?utm_source=newsletter&id=42'
cargo run -- clean-clipboard
cargo run -- restore
cargo run -- doctor
cargo run -- agent status
```

Expected output:

```text
https://youtu.be/LYa_ReqRlcs
```

To watch the macOS clipboard:

```sh
cargo run -- watch --interval-ms 500
```

To install PlainLink to a stable user path and start the watcher:

```sh
cargo run -- install --interval-ms 500
```

To build and smoke-test the native macOS menu bar app:

```sh
scripts/test-macos-app.sh
```

This creates `dist/PlainLink.app`.

To build a local unsigned release zip:

```sh
scripts/package-macos-app.sh
```

This creates `dist/packages/PlainLink-<version>-macos-<arch>.zip` and a `.sha256` checksum.

## Project Layout

```text
app/
  macos/PlainLinkMenu  Swift/AppKit menu bar app
src/
  agent.rs        macOS LaunchAgent management
  cleaner.rs      URL cleaning engine
  install.rs      Stable user install and doctor checks
  rules.rs        PlainLink rule parser and matcher
  clipboard.rs    macOS clipboard watcher adapter
  state.rs        Last-cleaned URL restore state
  main.rs         CLI entrypoint
rules/
  base.plainlink  Default community rules
tests/
  fixtures/       Rule behavior fixtures used by cargo test
docs/
  ARCHITECTURE.md System design and data flow
  RULES.md        Rule syntax and contribution guidance
  MACOS.md        LaunchAgent notes
  MENUBAR.md      Native menu bar app notes
scripts/
  build-macos-app.sh  Build dist/PlainLink.app
  test-macos-app.sh   Build and smoke-test the app bundle
  package-macos-app.sh Create an unsigned zip and checksum
```

## Contributing

Rules are intentionally readable. A rule PR should include:

- the dirty URL,
- the expected cleaned URL,
- why the parameter is safe to remove,
- a fixture in `tests/fixtures/`.

Start with [CONTRIBUTING.md](CONTRIBUTING.md), then read [docs/RULES.md](docs/RULES.md).
