#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

if [ "${PLAINLINK_RELEASE_VERSION:-}" ]; then
  printf '%s\n' "${PLAINLINK_RELEASE_VERSION#v}"
  exit 0
fi

if TAG=$(git -C "$ROOT_DIR" describe --tags --exact-match 2>/dev/null); then
  printf '%s\n' "${TAG#v}"
  exit 0
fi

awk -F '=' '$1 ~ /^[[:space:]]*version[[:space:]]*$/ { gsub(/[ "]/, "", $2); print $2; exit }' "$ROOT_DIR/Cargo.toml"
