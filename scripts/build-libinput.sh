#!/usr/bin/env bash
# Build libinput from source with udev disabled (path-mode only).
# This allows libinput to work without udevd by manually adding devices.
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
LIBINPUT_VERSION="1.26.2"
LIBINPUT_SRC="$REPO_ROOT/third_party/libinput-$LIBINPUT_VERSION"
LIBINPUT_BUILD="$REPO_ROOT/out/libinput-build"
LIBINPUT_PREFIX="$REPO_ROOT/out/libinput-dist"

# Download if not present
if [ ! -d "$LIBINPUT_SRC" ]; then
    echo "Downloading libinput $LIBINPUT_VERSION..."
    mkdir -p "$(dirname "$LIBINPUT_SRC")"
    curl -L "https://gitlab.freedesktop.org/libinput/libinput/-/archive/$LIBINPUT_VERSION/libinput-$LIBINPUT_VERSION.tar.gz" | \
        tar xz -C "$(dirname "$LIBINPUT_SRC")"
fi

# Setup meson build
mkdir -p "$LIBINPUT_BUILD"

# Configure with udev disabled (empty udev-dir disables udev support)
meson setup "$LIBINPUT_BUILD" "$LIBINPUT_SRC" \
    -Dprefix="$LIBINPUT_PREFIX" \
    -Dudev-dir="" \
    -Ddocumentation=false \
    -Dtests=false \
    -Ddebug-gui=false \
    -Dinstall-tests=false \
    -Dlibwacom=false \
    -Dcoverity=false \
    --default-library=static

# Build and install
meson compile -C "$LIBINPUT_BUILD"
meson install -C "$LIBINPUT_BUILD"

echo "libinput (no-udev) installed to $LIBINPUT_PREFIX"
