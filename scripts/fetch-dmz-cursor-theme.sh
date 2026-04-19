#!/usr/bin/env bash
# Download the DMZ-White Xcursor theme (classic arrow set) into icons/dmz-white.
# Used by stratwm (wlr_xcursor_manager) and Wayland clients via XCURSOR_THEME=dmz-white.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$REPO_ROOT/icons/dmz-white"
URL="https://github.com/rhizoome/dmz-cursors/releases/download/v1.0/dmz-white.tar.xz"

if [[ -d "$DEST/cursors" && -f "$DEST/index.theme" ]]; then
  exit 0
fi

mkdir -p "$REPO_ROOT/icons"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL -o "$tmp/dmz-white.tar.xz" "$URL"
tar -xJf "$tmp/dmz-white.tar.xz" -C "$tmp"
# If DEST already exists (partial prior run), plain `mv src DEST` nests into DEST instead of replacing it.
rm -rf "$DEST"
mv "$tmp/dmz-white" "$DEST"
