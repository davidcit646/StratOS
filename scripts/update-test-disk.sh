#!/bin/bash
# Populates the StratOS GPT test disk: ESP slot payloads (matches stratboot paths),
# raw EROFS on SLOT_A, formatted CONFIG / STRAT_CACHE / HOME.
#
# Partition layout must match scripts/create-test-disk.sh and stratboot partition.c.

set -euo pipefail

while [ "$#" -gt 0 ]; do
    case "$1" in
        --disk)
            DISK_IMAGE="$2"
            shift 2
            ;;
        --slot-a-erofs)
            SLOT_A_EROFS="$2"
            shift 2
            ;;
        --boot-efi)
            BOOT_EFI="$2"
            shift 2
            ;;
        --kernel)
            KERNEL="$2"
            shift 2
            ;;
        --initrd)
            INITRD="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

echo "Updating disk: $DISK_IMAGE"
echo "  Slot A EROFS: $SLOT_A_EROFS"
echo "  Boot EFI: $BOOT_EFI"
echo "  Kernel (bzImage / EFI stub -> vmlinuz.efi): $KERNEL"
echo "  Initrd: $INITRD"

LOOP_DEV=""
ESP_MOUNT=""

cleanup() {
    if [ -n "${ESP_MOUNT}" ]; then
        umount "${ESP_MOUNT}" 2>/dev/null || true
        rmdir "${ESP_MOUNT}" 2>/dev/null || true
    fi
    if [ -n "${LOOP_DEV}" ]; then
        kpartx -d "${LOOP_DEV}" 2>/dev/null || true
        losetup -d "${LOOP_DEV}" 2>/dev/null || true
    fi
}

if ! LOOP_DEV=$(losetup -f --show "$DISK_IMAGE" 2>/dev/null); then
    echo "Warning: Failed to set up loop device (container/WSL limitation?)" >&2
    echo "Skipping disk update. Components are built in out/ directories." >&2
    echo "You can manually update the disk or run this in a native Linux environment." >&2
    exit 0
fi
trap cleanup EXIT

kpartx -a "$LOOP_DEV"
sleep 1

BASE="/dev/mapper/$(basename "$LOOP_DEV")"
PART1="${BASE}p1"
PART2="${BASE}p2"
PART5="${BASE}p5"
PART6="${BASE}p6"
PART7="${BASE}p7"

if ! blkid "$PART1" >/dev/null 2>&1; then
    echo "Formatting ESP partition as FAT32..."
    mkfs.vfat -F 32 "$PART1"
fi

EROF_SIZE=$(stat -c%s "$SLOT_A_EROFS")
PART2_SIZE=$(blockdev --getsize64 "$PART2")
if [ "$EROF_SIZE" -gt "$PART2_SIZE" ]; then
    echo "error: EROFS image (${EROF_SIZE} bytes) is larger than SLOT_A partition (${PART2_SIZE} bytes)." >&2
    echo "  Enlarge SLOT_A in scripts/create-test-disk.sh or shrink the rootfs / EROFS image." >&2
    exit 1
fi

echo "Writing EROFS system image to SLOT_A (partition 2)..."
dd if="$SLOT_A_EROFS" of="$PART2" bs=4M conv=fsync status=progress

for p in "$PART5" "$PART6"; do
    if ! blkid "$p" >/dev/null 2>&1; then
        echo "Formatting $p as ext4..."
        mkfs.ext4 -F "$p"
    fi
done

if ! blkid "$PART7" >/dev/null 2>&1; then
    echo "Formatting HOME partition as btrfs..."
    mkfs.btrfs -f "$PART7"
fi

ESP_MOUNT=$(mktemp -d)
mount "$PART1" "$ESP_MOUNT"

mkdir -p "$ESP_MOUNT/EFI/BOOT" \
    "$ESP_MOUNT/EFI/STRAT/SLOT_A" \
    "$ESP_MOUNT/EFI/STRAT/SLOT_B" \
    "$ESP_MOUNT/EFI/STRAT/SLOT_C"

cp "$BOOT_EFI" "$ESP_MOUNT/EFI/BOOT/BOOTX64.EFI"
cp "$KERNEL" "$ESP_MOUNT/EFI/STRAT/SLOT_A/vmlinuz.efi"
cp "$INITRD" "$ESP_MOUNT/EFI/STRAT/SLOT_A/initramfs.img"

sync
umount "$ESP_MOUNT"
rmdir "$ESP_MOUNT"
ESP_MOUNT=""

echo "Disk updated successfully"
