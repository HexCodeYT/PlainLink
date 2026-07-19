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

if [ "$(uname -s)" != "Darwin" ]; then
  echo "PlainLink.app packages can only be built on macOS." >&2
  exit 1
fi

if [ -z "$VERSION" ]; then
  echo "could not read package version from Cargo.toml" >&2
  exit 1
fi

command -v ditto >/dev/null 2>&1 || {
  echo "ditto is required to package PlainLink.app." >&2
  exit 1
}

command -v shasum >/dev/null 2>&1 || {
  echo "shasum is required to create package checksums." >&2
  exit 1
}

"$ROOT_DIR/scripts/test-macos-app.sh"

test -d "$APP_DIR" || {
  echo "missing app bundle: $APP_DIR" >&2
  exit 1
}

mkdir -p "$PACKAGE_DIR"
rm -f "$ZIP_PATH" "$SHA_PATH"

cd "$ROOT_DIR/dist"
ditto -c -k --sequesterRsrc --keepParent "$APP_NAME.app" "$ZIP_PATH"

cd "$PACKAGE_DIR"
shasum -a 256 "$ARTIFACT_NAME" > "$ARTIFACT_NAME.sha256"

echo "Packaged $ZIP_PATH"
echo "Checksum $SHA_PATH"
