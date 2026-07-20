# Roadmap

## Done

- Rust URL cleaning core.
- CLI commands for cleaning and inspection.
- macOS clipboard watcher.
- Native macOS menu bar app.
- App icon generation.
- First-run menu bar onboarding.
- Restore last original URL.
- User LaunchAgent install/status/restart/uninstall.
- Stable user install/uninstall and doctor checks.
- Unsigned macOS release packaging for testers and CI artifacts.
- Signed/notarized release automation.
- Conservative ClearURLs rule importer.
- Import manifests and fixture verification for generated rules.
- Human-readable default rules.
- Fixture-backed rule QA.
- Contributor docs with Mermaid diagrams.

## Current Distribution Status

- Technical testers can build from source or use unsigned preview artifacts.
- Regular-user distribution is blocked on Apple Developer Program membership, a Developer ID Application certificate, and notarization credentials.
- A GitHub Release should be published only when the artifact is clearly labeled as an unsigned preview or when the signed/notarized zip exists.

## Next

- Collect tester signal before paying for Developer ID distribution.
- Improve the menu bar app's visible status and failure states.
- Expand external source importers for EasyList/uBO removeparam and Firefox query stripping.
- Add optional ruleset updates with clear privacy and licensing controls.
- Add release provenance notes for any generated external rules included in an artifact.

## Later

- Windows and Linux clipboard adapters.
- Community rules registry.
- Rule linter for pull requests.
- Native tray apps for supported desktop platforms.
