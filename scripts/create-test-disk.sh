#!/bin/bash
# Creates a test GPT disk image for StratOS

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PHASE4_DIR="$REPO_ROOT/out/phase4"
DISK_IMAGE="$PHASE4_DIR/test-disk.img"
DISK_SIZE="2G"

mkdir -p "$PHASE4_DIR"

echo "Creating test disk image: $DISK_IMAGE"
rm -f "$DISK_IMAGE"

# Create raw disk image
truncate -s "$DISK_SIZE" "$DISK_IMAGE"

# Create GPT partition table
sgdisk -og "$DISK_IMAGE"

# Create EFI System Partition (ESP) - 512MB
sgdisk -n 1:0:+512M -t 1:EF00 "$DISK_IMAGE"
sgdisk -c 1:"EFI System Partition" "$DISK_IMAGE"

# Create System partition A - 1GB
sgdisk -n 2:0:+1G -t 2:8300 "$DISK_IMAGE"
sgdisk -c 2:"System Slot A" "$DISK_IMAGE"

echo "Test disk created successfully"
