# Release Process

PlainLink release builds need to be boring and verifiable. Regular users should receive a signed, notarized macOS app zip attached to a GitHub Release.

Current status: release automation exists, but this repository does not currently have Developer ID credentials configured. Until that changes, any downloadable app artifact should be described as an unsigned developer preview.

## Release Modes

### Unsigned Preview

Use this for technical testers:

```sh
scripts/package-macos-app.sh
```

This produces an unsigned zip and checksum in `dist/packages/`. macOS Gatekeeper will warn users because the app is not signed with a Developer ID certificate and is not notarized.

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

## Publish GitHub Release

After the signed zip and checksum exist, create and push the tag:

```sh
git tag -a v0.1.0 -m "PlainLink v0.1.0"
git push origin v0.1.0
scripts/publish-github-release.sh v0.1.0
```

The publish script creates a draft GitHub Release with:

- `dist/packages/PlainLink-0.1.0-macos-<arch>.zip`
- `dist/packages/PlainLink-0.1.0-macos-<arch>.zip.sha256`
- release notes from `docs/releases/v0.1.0.md`

Review the draft release, confirm the checksum, confirm whether the artifact is signed/notarized or explicitly labeled unsigned preview, then publish it.
