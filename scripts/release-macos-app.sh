#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP_NAME="PlainLink"
APP_DIR="$ROOT_DIR/dist/$APP_NAME.app"
PACKAGE_DIR="$ROOT_DIR/dist/packages"
VERSION=$(awk -F '=' '$1 ~ /^[[:space:]]*version[[:space:]]*$/ { gsub(/[ "]/, "", $2); print $2; exit }' "$ROOT_DIR/Cargo.toml")
ARCH=$(uname -m)
ARTIFACT_NAME="$APP_NAME-$VERSION-macos-$ARCH.zip"
ZIP_PATH="$PACKAGE_DIR/$ARTIFACT_NAME"
SHA_PATH="$ZIP_PATH.sha256"
SIGNING_IDENTITY="${PLAINLINK_DEVELOPER_ID_APPLICATION:-}"
NOTARY_PROFILE="${PLAINLINK_NOTARY_PROFILE:-plainlink-notary}"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "PlainLink release builds can only be produced on macOS." >&2
  exit 1
fi

if [ -z "$VERSION" ]; then
  echo "could not read package version from Cargo.toml" >&2
  exit 1
fi

if [ -z "$SIGNING_IDENTITY" ]; then
  echo "set PLAINLINK_DEVELOPER_ID_APPLICATION to a Developer ID Application signing identity." >&2
  exit 1
fi

command -v codesign >/dev/null 2>&1 || {
  echo "codesign is required to sign PlainLink.app." >&2
  exit 1
}

command -v ditto >/dev/null 2>&1 || {
  echo "ditto is required to package PlainLink.app." >&2
  exit 1
}

command -v shasum >/dev/null 2>&1 || {
  echo "shasum is required to create package checksums." >&2
  exit 1
}

xcrun --find notarytool >/dev/null
xcrun --find stapler >/dev/null

"$ROOT_DIR/scripts/test-macos-app.sh"

codesign --force --timestamp --options runtime --sign "$SIGNING_IDENTITY" "$APP_DIR/Contents/MacOS/plainlink"
codesign --force --timestamp --options runtime --sign "$SIGNING_IDENTITY" "$APP_DIR/Contents/MacOS/PlainLinkMenu"
codesign --force --timestamp --options runtime --sign "$SIGNING_IDENTITY" "$APP_DIR"
codesign --verify --deep --strict --verbose=2 "$APP_DIR"

mkdir -p "$PACKAGE_DIR"
rm -f "$ZIP_PATH" "$SHA_PATH"

cd "$ROOT_DIR/dist"
ditto -c -k --sequesterRsrc --keepParent "$APP_NAME.app" "$ZIP_PATH"

xcrun notarytool submit "$ZIP_PATH" --keychain-profile "$NOTARY_PROFILE" --wait
xcrun stapler staple "$APP_DIR"
xcrun stapler validate "$APP_DIR"
spctl --assess --type execute --verbose "$APP_DIR"

rm -f "$ZIP_PATH" "$SHA_PATH"
ditto -c -k --sequesterRsrc --keepParent "$APP_NAME.app" "$ZIP_PATH"

cd "$PACKAGE_DIR"
shasum -a 256 "$ARTIFACT_NAME" > "$ARTIFACT_NAME.sha256"

echo "Signed, notarized, and packaged $ZIP_PATH"
echo "Checksum $SHA_PATH"
