#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP_NAME="PlainLink"
APP_DIR="$ROOT_DIR/dist/$APP_NAME.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
SWIFT_SOURCE="$ROOT_DIR/app/macos/PlainLinkMenu/Sources/PlainLinkMenu.swift"
INFO_PLIST="$ROOT_DIR/packaging/macos/PlainLink.app/Contents/Info.plist"
SWIFT_MODULE_CACHE="$ROOT_DIR/target/swift-module-cache"
ICON_FILE="$RESOURCES_DIR/PlainLink.icns"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "PlainLink.app can only be built on macOS." >&2
  exit 1
fi

command -v cargo >/dev/null 2>&1 || {
  echo "cargo is required to build PlainLink.app." >&2
  exit 1
}

command -v swiftc >/dev/null 2>&1 || {
  echo "swiftc is required to build PlainLink.app. Install Apple Command Line Tools." >&2
  exit 1
}

cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml"

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR" "$SWIFT_MODULE_CACHE"

cp "$INFO_PLIST" "$CONTENTS_DIR/Info.plist"
"$ROOT_DIR/scripts/generate-macos-icon.sh" "$ICON_FILE"
swiftc -O -framework AppKit -module-cache-path "$SWIFT_MODULE_CACHE" "$SWIFT_SOURCE" -o "$MACOS_DIR/PlainLinkMenu"
cp "$ROOT_DIR/target/release/plainlink" "$MACOS_DIR/plainlink"

chmod 755 "$MACOS_DIR/PlainLinkMenu" "$MACOS_DIR/plainlink"

echo "Built $APP_DIR"
