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

partition_meta() {
    local num="$1"
    local info
    local start
    local size
    info=$(sgdisk -i "$num" "$DISK_IMAGE" 2>/dev/null || true)
    start=$(printf '%s\n' "$info" | awk -F: '/First sector:/ {print $2}' | awk '{print $1}')
    size=$(printf '%s\n' "$info" | awk -F: '/Partition size:/ {print $2}' | awk '{print $1}')
    [ -n "$start" ] && [ -n "$size" ] || return 1
    printf '%s %s\n' "$start" "$size"
}

probe_fs_type_at() {
    local offset_bytes="$1"
    local size_bytes="$2"
    blkid -p -O "$offset_bytes" -S "$size_bytes" -o value -s TYPE "$DISK_IMAGE" 2>/dev/null || true
}

write_partition_bytes() {
    local src="$1"
    local dst_start_sectors="$2"
    dd if="$src" of="$DISK_IMAGE" bs=512 seek="$dst_start_sectors" conv=notrunc,fsync status=none
}

format_partition_via_temp_image() {
    local fstype="$1"
    local start_sectors="$2"
    local size_bytes="$3"
    local tmp
    tmp=$(mktemp "/tmp/stratos-${fstype}.XXXXXX.img")
    truncate -s "$size_bytes" "$tmp"
    case "$fstype" in
        vfat)
            mkfs.vfat -F 32 "$tmp" >/dev/null
            ;;
        ext4)
            mkfs.ext4 -F -q "$tmp" >/dev/null
            ;;
        btrfs)
            mkfs.btrfs -f "$tmp" >/dev/null
            ;;
        *)
            echo "error: unsupported fstype in formatter: $fstype" >&2
            rm -f "$tmp"
            exit 1
            ;;
    esac
    write_partition_bytes "$tmp" "$start_sectors"
    rm -f "$tmp"
}

maybe_update_with_loop() {
    if ! command -v losetup >/dev/null 2>&1 || ! command -v kpartx >/dev/null 2>&1; then
        return 1
    fi
    if ! LOOP_DEV=$(losetup -f --show "$DISK_IMAGE" 2>/dev/null); then
        return 1
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
        [ -b "$p" ] || continue
        if ! blkid "$p" >/dev/null 2>&1; then
            echo "Formatting $p as ext4..."
            mkfs.ext4 -F "$p"
        fi
    done

    if [ -b "$PART7" ] && ! blkid "$PART7" >/dev/null 2>&1; then
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
    return 0
}

if maybe_update_with_loop; then
    echo "Disk updated successfully"
    exit 0
fi

echo "Loop devices unavailable; using raw GPT-image fallback updater."
for req in sgdisk blkid dd mmd mcopy truncate; do
    if ! command -v "$req" >/dev/null 2>&1; then
        echo "error: missing required tool for fallback updater: $req" >&2
        exit 1
    fi
done

if ! p1_meta=$(partition_meta 1); then
    echo "error: failed to read GPT metadata for required partition 1 (ESP) in $DISK_IMAGE" >&2
    exit 1
fi
if ! p2_meta=$(partition_meta 2); then
    echo "error: failed to read GPT metadata for required partition 2 (SLOT_A) in $DISK_IMAGE" >&2
    exit 1
fi
read -r P1_START_SECTORS P1_SIZE_SECTORS <<< "$p1_meta"
read -r P2_START_SECTORS P2_SIZE_SECTORS <<< "$p2_meta"

HAS_P5=0
HAS_P6=0
HAS_P7=0
if read -r P5_START_SECTORS P5_SIZE_SECTORS < <(partition_meta 5); then
    HAS_P5=1
fi
if read -r P6_START_SECTORS P6_SIZE_SECTORS < <(partition_meta 6); then
    HAS_P6=1
fi
if read -r P7_START_SECTORS P7_SIZE_SECTORS < <(partition_meta 7); then
    HAS_P7=1
fi

P1_OFFSET_BYTES=$((P1_START_SECTORS * 512))
P1_SIZE_BYTES=$((P1_SIZE_SECTORS * 512))
P2_OFFSET_BYTES=$((P2_START_SECTORS * 512))
P2_SIZE_BYTES=$((P2_SIZE_SECTORS * 512))
if [ "$HAS_P5" -eq 1 ]; then
    P5_OFFSET_BYTES=$((P5_START_SECTORS * 512))
    P5_SIZE_BYTES=$((P5_SIZE_SECTORS * 512))
fi
if [ "$HAS_P6" -eq 1 ]; then
    P6_OFFSET_BYTES=$((P6_START_SECTORS * 512))
    P6_SIZE_BYTES=$((P6_SIZE_SECTORS * 512))
fi
if [ "$HAS_P7" -eq 1 ]; then
    P7_OFFSET_BYTES=$((P7_START_SECTORS * 512))
    P7_SIZE_BYTES=$((P7_SIZE_SECTORS * 512))
fi

EROF_SIZE=$(stat -c%s "$SLOT_A_EROFS")
if [ "$EROF_SIZE" -gt "$P2_SIZE_BYTES" ]; then
    echo "error: EROFS image (${EROF_SIZE} bytes) is larger than SLOT_A partition (${P2_SIZE_BYTES} bytes)." >&2
    echo "  Enlarge SLOT_A in scripts/create-test-disk.sh or shrink the rootfs / EROFS image." >&2
    exit 1
fi

echo "Writing EROFS system image to SLOT_A (partition 2, raw offset)..."
dd if="$SLOT_A_EROFS" of="$DISK_IMAGE" bs=512 seek="$P2_START_SECTORS" conv=notrunc,fsync status=progress

if [ "$(probe_fs_type_at "$P1_OFFSET_BYTES" "$P1_SIZE_BYTES")" != "vfat" ]; then
    if ! command -v mkfs.vfat >/dev/null 2>&1; then
        echo "error: mkfs.vfat missing; cannot format ESP partition in fallback mode" >&2
        exit 1
    fi
    echo "Formatting ESP partition as FAT32 (raw offset)..."
    format_partition_via_temp_image vfat "$P1_START_SECTORS" "$P1_SIZE_BYTES"
fi

for pnum in 5 6; do
    has="HAS_P${pnum}"
    if [ "${!has}" -ne 1 ]; then
        echo "Note: partition $pnum not present; skipping format check."
        continue
    fi
    pstart_var="P${pnum}_START_SECTORS"
    poff_var="P${pnum}_OFFSET_BYTES"
    psz_var="P${pnum}_SIZE_BYTES"
    if [ "$(probe_fs_type_at "${!poff_var}" "${!psz_var}")" != "ext4" ]; then
        if ! command -v mkfs.ext4 >/dev/null 2>&1; then
            echo "error: mkfs.ext4 missing; cannot format partition $pnum in fallback mode" >&2
            exit 1
        fi
        echo "Formatting partition $pnum as ext4 (raw offset)..."
        format_partition_via_temp_image ext4 "${!pstart_var}" "${!psz_var}"
    fi
done

if [ "$HAS_P7" -eq 1 ]; then
    if [ "$(probe_fs_type_at "$P7_OFFSET_BYTES" "$P7_SIZE_BYTES")" != "btrfs" ]; then
        if ! command -v mkfs.btrfs >/dev/null 2>&1; then
            echo "error: mkfs.btrfs missing; cannot format HOME partition in fallback mode" >&2
            exit 1
        fi
        echo "Formatting HOME partition as btrfs (raw offset)..."
        format_partition_via_temp_image btrfs "$P7_START_SECTORS" "$P7_SIZE_BYTES"
    fi
else
    echo "Note: partition 7 not present; skipping HOME format check."
fi

ESP_IMAGE="${DISK_IMAGE}@@${P1_OFFSET_BYTES}"
mmd -i "$ESP_IMAGE" ::/EFI 2>/dev/null || true
mmd -i "$ESP_IMAGE" ::/EFI/BOOT 2>/dev/null || true
mmd -i "$ESP_IMAGE" ::/EFI/STRAT 2>/dev/null || true
mmd -i "$ESP_IMAGE" ::/EFI/STRAT/SLOT_A 2>/dev/null || true
mmd -i "$ESP_IMAGE" ::/EFI/STRAT/SLOT_B 2>/dev/null || true
mmd -i "$ESP_IMAGE" ::/EFI/STRAT/SLOT_C 2>/dev/null || true

mcopy -o -i "$ESP_IMAGE" "$BOOT_EFI" ::/EFI/BOOT/BOOTX64.EFI
mcopy -o -i "$ESP_IMAGE" "$KERNEL" ::/EFI/STRAT/SLOT_A/vmlinuz.efi
mcopy -o -i "$ESP_IMAGE" "$INITRD" ::/EFI/STRAT/SLOT_A/initramfs.img

sync
echo "Disk updated successfully"
