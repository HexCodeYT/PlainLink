# Release Process

PlainLink release builds need to be boring and verifiable. Regular users should receive a signed, notarized macOS app zip attached to a GitHub Release.

Current status: release automation exists, but this repository does not currently have Developer ID credentials configured. Until that changes, any downloadable app artifact should be described as an unsigned developer preview.

## Release Modes

### Unsigned Preview

Use this for technical testers:

```sh
PLAINLINK_RELEASE_VERSION=v0.1.0-preview.2 scripts/package-macos-app.sh
```

This produces an unsigned zip and checksum in `dist/packages/`. macOS Gatekeeper will warn users because the app is not signed with a Developer ID certificate and is not notarized.

Preview artifacts include the preview suffix in the filename, for example:

```text
dist/packages/PlainLink-0.1.0-preview.2-macos-arm64.zip
dist/packages/PlainLink-0.1.0-preview.2-macos-arm64.zip.sha256
```

### Signed Regular-User Release

Use this only when a release machine has Apple Developer Program credentials:

```sh
scripts/release-macos-app.sh
```

This produces the signed, notarized, stapled zip that is suitable for regular users.

## Requirements

Signed releases require:

- Apple Developer Program membership.
- A `Developer ID Application` certificate installed in the signing keychain.
- A stored notary profile created with `xcrun notarytool store-credentials`.
- Authenticated GitHub CLI access for publishing the release.

Apple's notarization flow requires Developer ID signing, hardened runtime, secure timestamps, submission through `notarytool`, and stapling the accepted ticket before distribution.

## Store Notary Credentials

Create a keychain profile once on the release machine:

```sh
xcrun notarytool store-credentials plainlink-notary \
  --apple-id you@example.com \
  --team-id TEAMID12345 \
  --password app-specific-password
```

## Build Signed And Notarized App

Set the Developer ID Application identity exactly as it appears in `security find-identity -v -p codesigning`:

```sh
export PLAINLINK_DEVELOPER_ID_APPLICATION="Developer ID Application: Example Name (TEAMID12345)"
export PLAINLINK_NOTARY_PROFILE="plainlink-notary"
scripts/release-macos-app.sh
```

The script:

- builds and smoke-tests `dist/PlainLink.app`,
- generates the app icon,
- signs the bundled CLI, menu binary, and app bundle with hardened runtime,
- packages a zip for notarization,
- submits the zip with `xcrun notarytool submit --wait`,
- staples and validates the app,
- verifies Gatekeeper assessment,
- repackages the stapled app and writes a SHA-256 checksum.

## Version Source

Packaging scripts use `scripts/release-version.sh` as the single source of truth:

1. `PLAINLINK_RELEASE_VERSION`, if set.
2. The exact Git tag at `HEAD`, if one exists.
3. The Cargo package version as a local development fallback.

Use `PLAINLINK_RELEASE_VERSION` when building a preview artifact from a release branch or before checking out the preview tag. Use an exact tag checkout for final release builds.

## Publish GitHub Release

After the signed zip and checksum exist, create and push the tag:

```sh
git tag -a v0.1.0 -m "PlainLink v0.1.0"
git push origin v0.1.0
scripts/publish-github-release.sh v0.1.0
```

For preview releases, the publish script expects preview-named artifacts and falls back to the base version notes if preview-specific notes do not exist:

```sh
PLAINLINK_RELEASE_VERSION=v0.1.0-preview.2 scripts/package-macos-app.sh
git tag -a v0.1.0-preview.2 -m "PlainLink v0.1.0 Preview 2"
git push origin v0.1.0-preview.2
scripts/publish-github-release.sh v0.1.0-preview.2
```

The publish script creates a draft GitHub Release with:

- `dist/packages/PlainLink-<release-version>-macos-<arch>.zip`
- `dist/packages/PlainLink-<release-version>-macos-<arch>.zip.sha256`
- release notes from `docs/releases/<tag>.md`, or `docs/releases/v<base-version>.md` when preview-specific notes do not exist

Review the draft release, confirm the checksum, confirm whether the artifact is signed/notarized or explicitly labeled unsigned preview, then publish it.
