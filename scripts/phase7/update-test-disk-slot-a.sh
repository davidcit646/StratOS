#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
DISK_IMAGE="${DISK_IMAGE:-$REPO_ROOT/out/phase4/test-disk.img}"
SLOT_A_EROFS="${SLOT_A_EROFS:-$REPO_ROOT/out/phase7/slot-system.erofs}"
BOOT_EFI="${BOOT_EFI:-$REPO_ROOT/out/phase3/BOOTX64.EFI}"
KERNEL_EFI="${KERNEL_EFI:-$REPO_ROOT/out/phase4/vmlinuz}"
INITRD_IMG="${INITRD_IMG:-$REPO_ROOT/out/phase7/initramfs.cpio.gz}"
UPDATE_ESP="${UPDATE_ESP:-1}"
USE_HOST=0

usage() {
    cat <<USAGE
Usage: $0 [--disk PATH] [--slot-a-erofs PATH] [--no-esp-update]
          [--boot-efi PATH] [--kernel PATH] [--initrd PATH]

Updates an existing Phase 4 test disk in-place:
  - writes latest SLOT_A EROFS payload into partition 2
  - optionally refreshes ESP files in partition 1 (default enabled)
USAGE
}

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
            KERNEL_EFI="$2"
            shift 2
            ;;
        --initrd)
            INITRD_IMG="$2"
            shift 2
            ;;
        --no-esp-update)
            UPDATE_ESP=0
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

has_local() {
    command -v "$1" >/dev/null 2>&1
}

has_host() {
    if ! has_local flatpak-spawn; then
        return 1
    fi
    flatpak-spawn --host sh -c "command -v '$1' >/dev/null 2>&1"
}

run_cmd() {
    if [ "$USE_HOST" -eq 1 ]; then
        flatpak-spawn --host "$@"
    else
        "$@"
    fi
}

capture_cmd() {
    if [ "$USE_HOST" -eq 1 ]; then
        flatpak-spawn --host "$@"
    else
        "$@"
    fi
}

to_host_path() {
    path="$1"
    if [ "$USE_HOST" -eq 1 ]; then
        case "$path" in
            /run/host/*)
                printf '%s\n' "${path#/run/host}"
                ;;
            /home/*)
                printf '/var%s\n' "$path"
                ;;
            *)
                printf '%s\n' "$path"
                ;;
        esac
    else
        printf '%s\n' "$path"
    fi
}

if ! has_local sgdisk; then
    if has_host sgdisk; then
        USE_HOST=1
    else
        echo "Missing required tool: sgdisk" >&2
        exit 1
    fi
fi

for req in dd awk wc; do
    if [ "$USE_HOST" -eq 1 ]; then
        has_host "$req" || { echo "Missing required host tool: $req" >&2; exit 1; }
    else
        has_local "$req" || { echo "Missing required tool: $req" >&2; exit 1; }
    fi
done

if [ "$UPDATE_ESP" -eq 1 ]; then
    for req in mkfs.vfat mmd mcopy; do
        if [ "$USE_HOST" -eq 1 ]; then
            has_host "$req" || { echo "Missing required host tool: $req" >&2; exit 1; }
        else
            has_local "$req" || { echo "Missing required tool: $req" >&2; exit 1; }
        fi
    done
fi

if [ ! -f "$DISK_IMAGE" ]; then
    echo "Missing disk image: $DISK_IMAGE" >&2
    exit 1
fi
if [ ! -s "$SLOT_A_EROFS" ]; then
    echo "Missing SLOT_A erofs payload: $SLOT_A_EROFS" >&2
    exit 1
fi

if [ "$UPDATE_ESP" -eq 1 ]; then
    for req_file in "$BOOT_EFI" "$KERNEL_EFI" "$INITRD_IMG"; do
        if [ ! -s "$req_file" ]; then
            echo "Missing ESP input file: $req_file" >&2
            exit 1
        fi
    done
fi

HOST_DISK_IMAGE="$(to_host_path "$DISK_IMAGE")"
HOST_SLOT_A_EROFS="$(to_host_path "$SLOT_A_EROFS")"
HOST_BOOT_EFI="$(to_host_path "$BOOT_EFI")"
HOST_KERNEL_EFI="$(to_host_path "$KERNEL_EFI")"
HOST_INITRD_IMG="$(to_host_path "$INITRD_IMG")"

part2_info="$(capture_cmd sgdisk -i 2 "$HOST_DISK_IMAGE")"
part2_first_lba="$(printf '%s\n' "$part2_info" | awk -F': ' '/First sector:/ {print $2; exit}' | awk '{print $1}')"
part2_sectors="$(printf '%s\n' "$part2_info" | awk -F': ' '/Partition size:/ {print $2; exit}' | awk '{print $1}')"
if [ -z "$part2_first_lba" ] || [ -z "$part2_sectors" ]; then
    echo "Failed to parse partition 2 geometry" >&2
    exit 1
fi

slot_bytes="$(wc -c < "$SLOT_A_EROFS")"
part2_bytes=$((part2_sectors * 512))
if [ "$slot_bytes" -gt "$part2_bytes" ]; then
    echo "SLOT_A payload too large for partition 2" >&2
    echo "  slot bytes: $slot_bytes" >&2
    echo "  p2 bytes:   $part2_bytes" >&2
    exit 1
fi

run_cmd dd if="$HOST_SLOT_A_EROFS" of="$HOST_DISK_IMAGE" bs=512 seek="$part2_first_lba" conv=notrunc status=none

if [ "$UPDATE_ESP" -eq 1 ]; then
    part1_info="$(capture_cmd sgdisk -i 1 "$HOST_DISK_IMAGE")"
    part1_first_lba="$(printf '%s\n' "$part1_info" | awk -F': ' '/First sector:/ {print $2; exit}' | awk '{print $1}')"
    part1_sectors="$(printf '%s\n' "$part1_info" | awk -F': ' '/Partition size:/ {print $2; exit}' | awk '{print $1}')"
    if [ -z "$part1_first_lba" ] || [ -z "$part1_sectors" ]; then
        echo "Failed to parse partition 1 geometry" >&2
        exit 1
    fi

    esp_temp_image="$(dirname "$DISK_IMAGE")/.esp-update.$$.img"
    host_esp_temp_image="$(to_host_path "$esp_temp_image")"
    run_cmd dd if=/dev/zero of="$host_esp_temp_image" bs=512 count="$part1_sectors" status=none
    run_cmd mkfs.vfat -F 32 "$host_esp_temp_image"
    run_cmd mmd -i "$host_esp_temp_image" ::/EFI ::/EFI/BOOT ::/EFI/STRAT ::/EFI/STRAT/SLOT_A
    run_cmd mcopy -i "$host_esp_temp_image" "$HOST_BOOT_EFI" ::/EFI/BOOT/BOOTX64.EFI
    run_cmd mcopy -i "$host_esp_temp_image" "$HOST_KERNEL_EFI" ::/EFI/STRAT/SLOT_A/vmlinuz.efi
    run_cmd mcopy -i "$host_esp_temp_image" "$HOST_INITRD_IMG" ::/EFI/STRAT/SLOT_A/initramfs.img
    run_cmd dd if="$host_esp_temp_image" of="$HOST_DISK_IMAGE" bs=512 seek="$part1_first_lba" conv=notrunc status=none
    run_cmd rm -f "$host_esp_temp_image"
fi

echo "Updated test disk SLOT_A: $DISK_IMAGE"
if [ "$UPDATE_ESP" -eq 1 ]; then
    echo "Refreshed ESP assets from:"
    echo "  BOOTX64.EFI: $BOOT_EFI"
    echo "  vmlinuz.efi: $KERNEL_EFI"
    echo "  initramfs:   $INITRD_IMG"
fi
echo "Slot payload source: $SLOT_A_EROFS"
