#!/bin/bash
# Creates a GPT test disk for StratOS QEMU/dev builds.
#
# Layout matches stratboot (partition.c names + stratboot.c GPT PARTUUID lookup) and
# initramfs-init.c defaults (/dev/sda2 root EROFS, /dev/sda5/6/7 config/apps/home):
#   1  ESP           (~512 MiB, FAT32, type EF00, name ESP)
#   2  SLOT_A        (EROFS system image)
#   3  SLOT_B        (reserved / staging)
#   4  SLOT_C        (reserved / staging)
#   5  CONFIG        (ext4)
#   6  STRAT_CACHE   (ext4, apps — init default apps=)
#   7  HOME          (btrfs)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PHASE4_DIR="$REPO_ROOT/out/phase4"
DISK_IMAGE="${DISK_IMAGE:-$PHASE4_DIR/test-disk.img}"

mkdir -p "$PHASE4_DIR"

# Minimum ~3.6 GiB for layout below; use 4 GiB headroom for larger EROFS images.
DISK_MIB="${DISK_MIB:-4096}"

echo "Creating test disk image: $DISK_IMAGE (${DISK_MIB} MiB)"
rm -f "$DISK_IMAGE"

dd if=/dev/zero of="$DISK_IMAGE" bs=1M count="$DISK_MIB" status=progress
sync

sgdisk --zap-all "$DISK_IMAGE" 2>/dev/null || true
sgdisk -og "$DISK_IMAGE"
sync

sgdisk -n 1:0:+512M -t 1:EF00 -c 1:ESP "$DISK_IMAGE"
sgdisk -n 2:0:+1600M -t 2:8300 -c 2:SLOT_A "$DISK_IMAGE"
sgdisk -n 3:0:+128M -t 3:8300 -c 3:SLOT_B "$DISK_IMAGE"
sgdisk -n 4:0:+128M -t 4:8300 -c 4:SLOT_C "$DISK_IMAGE"
sgdisk -n 5:0:+128M -t 5:8300 -c 5:CONFIG "$DISK_IMAGE"
sgdisk -n 6:0:+256M -t 6:8300 -c 6:STRAT_CACHE "$DISK_IMAGE"
sgdisk -n 7:0:+512M -t 7:8300 -c 7:HOME "$DISK_IMAGE"
sync

echo "Test disk created successfully (7 partitions; GPT names SLOT_A … HOME)."
