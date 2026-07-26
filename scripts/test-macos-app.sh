#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP_DIR="$ROOT_DIR/dist/PlainLink.app"
MENU_BIN="$APP_DIR/Contents/MacOS/PlainLinkMenu"
CLI_BIN="$APP_DIR/Contents/MacOS/plainlink"
INFO_PLIST="$APP_DIR/Contents/Info.plist"
ICON_FILE="$APP_DIR/Contents/Resources/PlainLink.icns"
ICON_CHECK_DIR="$ROOT_DIR/target/macos-icon-check.iconset"

minimum_macos_version() {
  xcrun vtool -show-build "$1" | awk '$1 == "minos" { print $2; exit }'
}

"$ROOT_DIR/scripts/build-macos-app.sh"

test -d "$APP_DIR"
test -x "$MENU_BIN"
test -x "$CLI_BIN"
test -f "$INFO_PLIST"
test -s "$ICON_FILE"

plutil -lint "$INFO_PLIST" >/dev/null
codesign --verify --deep --strict --verbose=2 "$APP_DIR"
PLIST_MIN_OS=$(/usr/libexec/PlistBuddy -c "Print :LSMinimumSystemVersion" "$INFO_PLIST")
MENU_MIN_OS=$(minimum_macos_version "$MENU_BIN")
CLI_MIN_OS=$(minimum_macos_version "$CLI_BIN")
test "$MENU_MIN_OS" = "$PLIST_MIN_OS" || {
  echo "PlainLinkMenu minimum macOS version is $MENU_MIN_OS; expected $PLIST_MIN_OS" >&2
  exit 1
}
test "$CLI_MIN_OS" = "$PLIST_MIN_OS" || {
  echo "plainlink minimum macOS version is $CLI_MIN_OS; expected $PLIST_MIN_OS" >&2
  exit 1
}
rm -rf "$ICON_CHECK_DIR"
iconutil -c iconset -o "$ICON_CHECK_DIR" "$ICON_FILE"
test -s "$ICON_CHECK_DIR/icon_512x512.png"
"$MENU_BIN" --smoke-test >/dev/null
"$CLI_BIN" --version >/dev/null

echo "PlainLink.app smoke tests passed."
