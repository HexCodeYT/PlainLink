#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUTPUT_PATH="${1:-$ROOT_DIR/dist/PlainLink.app/Contents/Resources/PlainLink.icns}"
ICONSET_PARENT="$ROOT_DIR/target/macos-icon"
ICONSET_DIR="$ICONSET_PARENT/PlainLink.iconset"
SWIFT_MODULE_CACHE="$ROOT_DIR/target/swift-module-cache"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "PlainLink.app icons can only be generated on macOS." >&2
  exit 1
fi

command -v swift >/dev/null 2>&1 || {
  echo "swift is required to generate PlainLink.app icons. Install Apple Command Line Tools." >&2
  exit 1
}

command -v iconutil >/dev/null 2>&1 || {
  echo "iconutil is required to generate PlainLink.app icons. Install Apple Command Line Tools." >&2
  exit 1
}

rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR" "$SWIFT_MODULE_CACHE" "$(dirname -- "$OUTPUT_PATH")"

swift -module-cache-path "$SWIFT_MODULE_CACHE" "$ROOT_DIR/tools/macos/render-app-icon.swift" "$ICONSET_DIR"

if ! iconutil -c icns -o "$OUTPUT_PATH" "$ICONSET_DIR"; then
  command -v sips >/dev/null 2>&1 || {
    echo "sips is required for the fallback PlainLink.app icon build." >&2
    exit 1
  }

  command -v tiff2icns >/dev/null 2>&1 || {
    echo "tiff2icns is required for the fallback PlainLink.app icon build." >&2
    exit 1
  }

  TIFF_PATH="$ICONSET_PARENT/PlainLink.tiff"
  sips -s format tiff "$ICONSET_DIR/icon_512x512@2x.png" --out "$TIFF_PATH" >/dev/null
  tiff2icns "$TIFF_PATH" "$OUTPUT_PATH"
fi

echo "Generated $OUTPUT_PATH"
