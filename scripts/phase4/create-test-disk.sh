#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase4"
IMAGE_PATH="$OUT_DIR/test-disk.img"
IMAGE_SIZE_MB="${IMAGE_SIZE_MB:-4096}"

BOOT_EFI="${BOOT_EFI:-$REPO_ROOT/out/phase3/BOOTX64.EFI}"
KERNEL_EFI="${KERNEL_EFI:-$REPO_ROOT/out/phase4/vmlinuz}"
INITRD_IMG="${INITRD_IMG:-$REPO_ROOT/out/phase7/initramfs.cpio.gz}"
SLOT_A_EROFS="${SLOT_A_EROFS:-$REPO_ROOT/out/phase7/slot-system.erofs}"

ESP_MB="${ESP_MB:-256}"
SLOT_A_MB="${SLOT_A_MB:-1024}"
SLOT_B_MB="${SLOT_B_MB:-512}"
SLOT_C_MB="${SLOT_C_MB:-512}"
CONFIG_MB="${CONFIG_MB:-256}"
CACHE_MB="${CACHE_MB:-256}"

use_host=0

usage() {
    cat <<USAGE
Usage: $0 [--image PATH] [--size-mb N]
          [--boot-efi PATH] [--kernel PATH] [--initrd PATH] [--slota-erofs PATH]

Builds a Phase 4/7 multi-partition raw test disk:
  p1 ESP (FAT32, EFI files)
  p2 SLOT_A (raw EROFS payload)
  p3 SLOT_B (empty placeholder)
  p4 SLOT_C (empty placeholder)
  p5 CONFIG (placeholder partition)
  p6 STRAT_CACHE (placeholder partition)
  p7 HOME (placeholder partition)
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --image)
            IMAGE_PATH="$2"
            shift 2
            ;;
        --size-mb)
            IMAGE_SIZE_MB="$2"
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
        --slota-erofs)
            SLOT_A_EROFS="$2"
            shift 2
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
    flatpak-spawn --host sh -c "command -v '$1' >/dev/null 2>&1"
}

run_cmd() {
    if [ "$use_host" -eq 1 ]; then
        flatpak-spawn --host "$@"
    else
        "$@"
    fi
}

capture_cmd() {
    if [ "$use_host" -eq 1 ]; then
        flatpak-spawn --host "$@"
    else
        "$@"
    fi
}

to_host_path() {
    path="$1"
    if [ "$use_host" -eq 0 ]; then
        printf '%s\n' "$path"
        return
    fi
    case "$path" in
        /home/*)
            printf '/var%s\n' "$path"
            ;;
        *)
            printf '%s\n' "$path"
            ;;
    esac
}

if ! has_local sgdisk || ! has_local mkfs.vfat || ! has_local mkfs.ext4 || ! has_local mmd || ! has_local mcopy || ! has_local dd; then
    if has_local flatpak-spawn && has_host sgdisk && has_host mkfs.vfat && has_host mkfs.ext4 && has_host mmd && has_host mcopy && has_host dd; then
        use_host=1
    fi
fi

for req in sgdisk mkfs.vfat mkfs.ext4 mmd mcopy dd awk; do
    if [ "$use_host" -eq 1 ]; then
        has_host "$req" || { echo "Missing required host tool: $req" >&2; exit 1; }
    else
        has_local "$req" || { echo "Missing required tool: $req" >&2; exit 1; }
    fi
done

home_fs="btrfs"
if [ "$use_host" -eq 1 ]; then
    if ! has_host mkfs.btrfs; then
        home_fs="ext4"
        echo "Warning: host mkfs.btrfs not found; formatting HOME as ext4" >&2
    fi
else
    if ! has_local mkfs.btrfs; then
        home_fs="ext4"
        echo "Warning: mkfs.btrfs not found; formatting HOME as ext4" >&2
    fi
fi

for req_file in "$BOOT_EFI" "$KERNEL_EFI" "$INITRD_IMG" "$SLOT_A_EROFS"; do
    if [ ! -s "$req_file" ]; then
        echo "Missing required input file: $req_file" >&2
        exit 1
    fi
done

mkdir -p "$(dirname "$IMAGE_PATH")"
ESP_TEMP_IMAGE="$(dirname "$IMAGE_PATH")/.esp-test.$$.img"
HOST_IMAGE="$(to_host_path "$IMAGE_PATH")"
HOST_ESP_TEMP_IMAGE="$(to_host_path "$ESP_TEMP_IMAGE")"
HOST_BOOT_EFI="$(to_host_path "$BOOT_EFI")"
HOST_KERNEL_EFI="$(to_host_path "$KERNEL_EFI")"
HOST_INITRD_IMG="$(to_host_path "$INITRD_IMG")"
HOST_SLOT_A_EROFS="$(to_host_path "$SLOT_A_EROFS")"

run_cmd dd if=/dev/zero of="$HOST_IMAGE" bs=1M count="$IMAGE_SIZE_MB" status=none
run_cmd sgdisk -o "$HOST_IMAGE"

run_cmd sgdisk \
    -n 1:0:+"${ESP_MB}M"    -t 1:ef00 -c 1:ESP \
    -n 2:0:+"${SLOT_A_MB}M" -t 2:8300 -c 2:SLOT_A \
    -n 3:0:+"${SLOT_B_MB}M" -t 3:8300 -c 3:SLOT_B \
    -n 4:0:+"${SLOT_C_MB}M" -t 4:8300 -c 4:SLOT_C \
    -n 5:0:+"${CONFIG_MB}M" -t 5:8300 -c 5:CONFIG \
    -n 6:0:+"${CACHE_MB}M"  -t 6:8300 -c 6:STRAT_CACHE \
    -n 7:0:0                -t 7:8300 -c 7:HOME \
    "$HOST_IMAGE"

# Build ESP contents in a standalone FAT image, then inject at p1 LBA.
part1_info="$(capture_cmd sgdisk -i 1 "$HOST_IMAGE")"
part1_first_lba="$(printf '%s\n' "$part1_info" | awk -F': ' '/First sector:/ {print $2; exit}' | awk '{print $1}')"
part1_sectors="$(printf '%s\n' "$part1_info" | awk -F': ' '/Partition size:/ {print $2; exit}' | awk '{print $1}')"
if [ -z "$part1_first_lba" ] || [ -z "$part1_sectors" ]; then
    echo "Failed to parse partition 1 geometry" >&2
    exit 1
fi

run_cmd dd if=/dev/zero of="$HOST_ESP_TEMP_IMAGE" bs=512 count="$part1_sectors" status=none
run_cmd mkfs.vfat -F 32 "$HOST_ESP_TEMP_IMAGE"
run_cmd mmd -i "$HOST_ESP_TEMP_IMAGE" ::/EFI ::/EFI/BOOT ::/EFI/STRAT ::/EFI/STRAT/SLOT_A
run_cmd mcopy -i "$HOST_ESP_TEMP_IMAGE" "$HOST_BOOT_EFI" ::/EFI/BOOT/BOOTX64.EFI
run_cmd mcopy -i "$HOST_ESP_TEMP_IMAGE" "$HOST_KERNEL_EFI" ::/EFI/STRAT/SLOT_A/vmlinuz.efi
run_cmd mcopy -i "$HOST_ESP_TEMP_IMAGE" "$HOST_INITRD_IMG" ::/EFI/STRAT/SLOT_A/initramfs.img
run_cmd dd if="$HOST_ESP_TEMP_IMAGE" of="$HOST_IMAGE" bs=512 seek="$part1_first_lba" conv=notrunc status=none
run_cmd rm -f "$HOST_ESP_TEMP_IMAGE"

# Inject SLOT_A EROFS payload into partition 2.
part2_info="$(capture_cmd sgdisk -i 2 "$HOST_IMAGE")"
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

run_cmd dd if="$HOST_SLOT_A_EROFS" of="$HOST_IMAGE" bs=512 seek="$part2_first_lba" conv=notrunc status=none

# Format CONFIG partition (p5) as ext4.
part5_info="$(capture_cmd sgdisk -i 5 "$HOST_IMAGE")"
part5_first_lba="$(printf '%s\n' "$part5_info" | awk -F': ' '/First sector:/ {print $2; exit}' | awk '{print $1}')"
part5_sectors="$(printf '%s\n' "$part5_info" | awk -F': ' '/Partition size:/ {print $2; exit}' | awk '{print $1}')"
if [ -z "$part5_first_lba" ] || [ -z "$part5_sectors" ]; then
    echo "Failed to parse partition 5 geometry" >&2
    exit 1
fi
CONFIG_TEMP="$(dirname "$IMAGE_PATH")/.config-test.$$.img"
HOST_CONFIG_TEMP="$(to_host_path "$CONFIG_TEMP")"
run_cmd dd if=/dev/zero of="$HOST_CONFIG_TEMP" bs=512 count="$part5_sectors" status=none
run_cmd mkfs.ext4 -q -L CONFIG "$HOST_CONFIG_TEMP"
run_cmd dd if="$HOST_CONFIG_TEMP" of="$HOST_IMAGE" bs=512 seek="$part5_first_lba" conv=notrunc status=none
run_cmd rm -f "$HOST_CONFIG_TEMP"

# Format STRAT_CACHE partition (p6) as ext4.
part6_info="$(capture_cmd sgdisk -i 6 "$HOST_IMAGE")"
part6_first_lba="$(printf '%s\n' "$part6_info" | awk -F': ' '/First sector:/ {print $2; exit}' | awk '{print $1}')"
part6_sectors="$(printf '%s\n' "$part6_info" | awk -F': ' '/Partition size:/ {print $2; exit}' | awk '{print $1}')"
if [ -z "$part6_first_lba" ] || [ -z "$part6_sectors" ]; then
    echo "Failed to parse partition 6 geometry" >&2
    exit 1
fi
CACHE_TEMP="$(dirname "$IMAGE_PATH")/.cache-test.$$.img"
HOST_CACHE_TEMP="$(to_host_path "$CACHE_TEMP")"
run_cmd dd if=/dev/zero of="$HOST_CACHE_TEMP" bs=512 count="$part6_sectors" status=none
run_cmd mkfs.ext4 -q -L STRAT_CACHE "$HOST_CACHE_TEMP"
run_cmd dd if="$HOST_CACHE_TEMP" of="$HOST_IMAGE" bs=512 seek="$part6_first_lba" conv=notrunc status=none
run_cmd rm -f "$HOST_CACHE_TEMP"

# Format HOME partition (p7) as btrfs, fallback to ext4 if mkfs.btrfs is unavailable.
part7_info="$(capture_cmd sgdisk -i 7 "$HOST_IMAGE")"
part7_first_lba="$(printf '%s\n' "$part7_info" | awk -F': ' '/First sector:/ {print $2; exit}' | awk '{print $1}')"
part7_sectors="$(printf '%s\n' "$part7_info" | awk -F': ' '/Partition size:/ {print $2; exit}' | awk '{print $1}')"
if [ -z "$part7_first_lba" ] || [ -z "$part7_sectors" ]; then
    echo "Failed to parse partition 7 geometry" >&2
    exit 1
fi
HOME_TEMP="$(dirname "$IMAGE_PATH")/.home-test.$$.img"
HOST_HOME_TEMP="$(to_host_path "$HOME_TEMP")"
run_cmd dd if=/dev/zero of="$HOST_HOME_TEMP" bs=512 count="$part7_sectors" status=none
if [ "$home_fs" = "btrfs" ]; then
    run_cmd mkfs.btrfs -f -q -L HOME "$HOST_HOME_TEMP"
else
    run_cmd mkfs.ext4 -q -L HOME "$HOST_HOME_TEMP"
fi
run_cmd dd if="$HOST_HOME_TEMP" of="$HOST_IMAGE" bs=512 seek="$part7_first_lba" conv=notrunc status=none
run_cmd rm -f "$HOST_HOME_TEMP"

echo "$IMAGE_PATH"
