#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP_DIR="$ROOT_DIR/dist/PlainLink.app"
MENU_BIN="$APP_DIR/Contents/MacOS/PlainLinkMenu"
CLI_BIN="$APP_DIR/Contents/MacOS/plainlink"
INFO_PLIST="$APP_DIR/Contents/Info.plist"
ICON_FILE="$APP_DIR/Contents/Resources/PlainLink.icns"

"$ROOT_DIR/scripts/build-macos-app.sh"

test -d "$APP_DIR"
test -x "$MENU_BIN"
test -x "$CLI_BIN"
test -f "$INFO_PLIST"
test -s "$ICON_FILE"

plutil -lint "$INFO_PLIST" >/dev/null
"$MENU_BIN" --smoke-test >/dev/null
"$CLI_BIN" --version >/dev/null

echo "PlainLink.app smoke tests passed."
