#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TAG="${1:-v0.1.0}"
VERSION="${TAG#v}"
BASE_VERSION="${VERSION%%-*}"
APP_NAME="PlainLink"
ARCH=$(uname -m)
PACKAGE_DIR="$ROOT_DIR/dist/packages"
ZIP_PATH="$PACKAGE_DIR/$APP_NAME-$VERSION-macos-$ARCH.zip"
SHA_PATH="$ZIP_PATH.sha256"
NOTES_PATH="$ROOT_DIR/docs/releases/$TAG.md"

if [ ! -f "$NOTES_PATH" ]; then
  NOTES_PATH="$ROOT_DIR/docs/releases/v$BASE_VERSION.md"
fi

command -v gh >/dev/null 2>&1 || {
  echo "gh is required to publish a GitHub Release." >&2
  exit 1
}

test -f "$ZIP_PATH" || {
  echo "missing release artifact: $ZIP_PATH" >&2
  exit 1
}

test -f "$SHA_PATH" || {
  echo "missing release checksum: $SHA_PATH" >&2
  exit 1
}

test -f "$NOTES_PATH" || {
  echo "missing release notes: $NOTES_PATH" >&2
  exit 1
}

gh auth status >/dev/null
git diff --quiet
git diff --cached --quiet
git rev-parse "$TAG" >/dev/null

gh release create "$TAG" \
  "$ZIP_PATH" \
  "$SHA_PATH" \
  --draft \
  --verify-tag \
  --title "PlainLink $TAG" \
  --notes-file "$NOTES_PATH"

echo "Created draft GitHub Release $TAG"
