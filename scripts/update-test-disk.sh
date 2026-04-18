#!/bin/bash
# Updates the test disk with new kernel, initramfs, and system image

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
echo "  Kernel: $KERNEL"
echo "  Initrd: $INITRD"

# Setup loop device
LOOP_DEV=$(losetup -f --show "$DISK_IMAGE")
trap "losetup -d $LOOP_DEV" EXIT

# Reread partition table
partprobe "$LOOP_DEV"

# Mount ESP partition
ESP_MOUNT=$(mktemp -d)
mount "${LOOP_DEV}p1" "$ESP_MOUNT"

# Update EFI boot loader
mkdir -p "$ESP_MOUNT/EFI/BOOT"
cp "$BOOT_EFI" "$ESP_MOUNT/EFI/BOOT/BOOTX64.EFI"

# Update kernel and initramfs
cp "$KERNEL" "$ESP_MOUNT/vmlinuz"
cp "$INITRD" "$ESP_MOUNT/initramfs.cpio.gz"

# Update system partition
SYSTEM_MOUNT=$(mktemp -d)
mount "${LOOP_DEV}p2" "$SYSTEM_MOUNT"
cp "$SLOT_A_EROFS" "$SYSTEM_MOUNT/slot-system.erofs"

# Cleanup
umount "$SYSTEM_MOUNT"
umount "$ESP_MOUNT"
rmdir "$SYSTEM_MOUNT" "$ESP_MOUNT"

echo "Disk updated successfully"
