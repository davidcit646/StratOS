#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Error: Target path argument required" >&2
    echo "Usage: $0 <target-path>" >&2
    exit 1
fi

TARGET="$1"

# Create config directory structure
mkdir -p "$TARGET/config/system/etc"
mkdir -p "$TARGET/config/system/services"
mkdir -p "$TARGET/config/apps"
mkdir -p "$TARGET/config/strat"
mkdir -p "$TARGET/config/user"
mkdir -p "$TARGET/config/var"

# Create system directory stubs for build-time
mkdir -p "$TARGET/system/bin"
mkdir -p "$TARGET/system/lib"
mkdir -p "$TARGET/system/etc"
mkdir -p "$TARGET/system/share"

# Set permissions
chmod 0755 "$TARGET/config/system/etc"
chmod 0755 "$TARGET/config/system/services"
chmod 0755 "$TARGET/config/apps"
chmod 0755 "$TARGET/config/strat"
chmod 0755 "$TARGET/config/user"
chmod 0755 "$TARGET/config/var"
chmod 0755 "$TARGET/system/bin"
chmod 0755 "$TARGET/system/lib"
chmod 0755 "$TARGET/system/etc"
chmod 0755 "$TARGET/system/share"

exit 0
