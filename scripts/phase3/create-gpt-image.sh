#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase3"
IMAGE_PATH="$OUT_DIR/stratboot.img"
VHD_PATH="$OUT_DIR/stratboot.vhd"
IMAGE_SIZE_MB="${IMAGE_SIZE_MB:-64}"
MAKE_VHD=1
INCLUDE_SLOT_ASSETS=1
SLOT_A_KERNEL="${SLOT_A_KERNEL:-$REPO_ROOT/out/phase4/vmlinuz}"
SLOT_A_INITRD="${SLOT_A_INITRD:-$REPO_ROOT/out/phase7/initramfs.cpio.gz}"

usage() {
    cat <<USAGE
Usage: $0 [--image PATH] [--vhd PATH] [--size-mb N] [--no-vhd]
          [--slot-kernel PATH] [--slot-initrd PATH] [--no-slot-assets]

Builds a GPT disk image with an EFI System Partition containing:
- EFI/BOOT/BOOTX64.EFI
- startup.nsh
- (optional) EFI/STRAT/SLOT_A/vmlinuz.efi + initramfs.img

By default it also generates a VirtualBox-friendly VHD.
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --image)
            IMAGE_PATH="$2"
            shift 2
            ;;
        --vhd)
            VHD_PATH="$2"
            shift 2
            ;;
        --size-mb)
            IMAGE_SIZE_MB="$2"
            shift 2
            ;;
        --no-vhd)
            MAKE_VHD=0
            shift
            ;;
        --slot-kernel)
            SLOT_A_KERNEL="$2"
            shift 2
            ;;
        --slot-initrd)
            SLOT_A_INITRD="$2"
            shift 2
            ;;
        --no-slot-assets)
            INCLUDE_SLOT_ASSETS=0
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

use_host=0

has_local() {
    command -v "$1" >/dev/null 2>&1
}

has_host() {
    flatpak-spawn --host sh -c "command -v '$1' >/dev/null 2>&1"
}

need_tool() {
    tool="$1"
    if has_local "$tool"; then
        return 0
    fi
    if [ "$use_host" -eq 1 ] && has_host "$tool"; then
        return 0
    fi
    return 1
}

if ! has_local sgdisk || ! has_local mkfs.vfat || ! has_local mmd || ! has_local mcopy || ! has_local dd || { [ "$MAKE_VHD" -eq 1 ] && ! has_local qemu-img; }; then
    if has_local flatpak-spawn && has_host sgdisk && has_host mkfs.vfat && has_host mmd && has_host mcopy && has_host dd; then
        if [ "$MAKE_VHD" -eq 0 ] || has_host qemu-img; then
            use_host=1
        fi
    fi
fi

if ! need_tool sgdisk || ! need_tool mkfs.vfat || ! need_tool mmd || ! need_tool mcopy || ! need_tool dd; then
    echo "Missing required tools (sgdisk, mkfs.vfat, mtools, dd)." >&2
    exit 1
fi
if [ "$MAKE_VHD" -eq 1 ] && ! need_tool qemu-img; then
    echo "Missing qemu-img for VHD conversion. Use --no-vhd or install qemu-img." >&2
    exit 1
fi

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

mkdir -p "$(dirname "$IMAGE_PATH")"
if [ "$MAKE_VHD" -eq 1 ]; then
    mkdir -p "$(dirname "$VHD_PATH")"
fi

EFI_APP="$($REPO_ROOT/scripts/phase3/build-stratboot.sh)"
if [ -z "$EFI_APP" ] || [ ! -f "$EFI_APP" ] || [ ! -s "$EFI_APP" ]; then
    echo "StratBoot build failed or output missing: $EFI_APP" >&2
    exit 1
fi

STARTUP_NSH="$(dirname "$IMAGE_PATH")/startup.nsh"
cat > "$STARTUP_NSH" <<'NSH'
@echo -off
if exist FS0:\EFI\BOOT\BOOTX64.EFI then
  FS0:\EFI\BOOT\BOOTX64.EFI
endif
if exist FS1:\EFI\BOOT\BOOTX64.EFI then
  FS1:\EFI\BOOT\BOOTX64.EFI
endif
if exist FS2:\EFI\BOOT\BOOTX64.EFI then
  FS2:\EFI\BOOT\BOOTX64.EFI
endif
if exist FS3:\EFI\BOOT\BOOTX64.EFI then
  FS3:\EFI\BOOT\BOOTX64.EFI
endif
NSH

HOST_IMAGE="$(to_host_path "$IMAGE_PATH")"
HOST_VHD="$(to_host_path "$VHD_PATH")"
HOST_EFI_APP="$(to_host_path "$EFI_APP")"
HOST_STARTUP_NSH="$(to_host_path "$STARTUP_NSH")"
HOST_SLOT_A_KERNEL="$(to_host_path "$SLOT_A_KERNEL")"
HOST_SLOT_A_INITRD="$(to_host_path "$SLOT_A_INITRD")"
ESP_TEMP_IMAGE="$(dirname "$IMAGE_PATH")/.esp.$$.img"
HOST_ESP_TEMP_IMAGE="$(to_host_path "$ESP_TEMP_IMAGE")"

run_cmd mkdir -p "$(dirname "$HOST_IMAGE")"
if [ "$MAKE_VHD" -eq 1 ]; then
    run_cmd mkdir -p "$(dirname "$HOST_VHD")"
fi

run_cmd dd if=/dev/zero of="$HOST_IMAGE" bs=1M count="$IMAGE_SIZE_MB" status=none
run_cmd sgdisk -o "$HOST_IMAGE"
# Fixed partition GUID so OVMF_VARS.fd boot entries can be pre-configured
run_cmd sgdisk -n 1:2048:0 -t 1:ef00 -c 1:ESP -u "1:4A3B2C1D-5E6F-7A8B-9C0D-E1F2A3B4C5D6" "$HOST_IMAGE"

PART_INFO="$(capture_cmd sgdisk -i 1 "$HOST_IMAGE")"
ESP_FIRST_LBA="$(printf '%s\n' "$PART_INFO" | awk -F': ' '/First sector:/ {print $2; exit}' | awk '{print $1}')"
ESP_SECTORS="$(printf '%s\n' "$PART_INFO" | awk -F': ' '/Partition size:/ {print $2; exit}' | awk '{print $1}')"
if [ -z "$ESP_FIRST_LBA" ] || [ -z "$ESP_SECTORS" ]; then
    echo "Failed to parse ESP partition geometry." >&2
    exit 1
fi

run_cmd dd if=/dev/zero of="$HOST_ESP_TEMP_IMAGE" bs=512 count="$ESP_SECTORS" status=none
run_cmd mkfs.vfat -F 32 "$HOST_ESP_TEMP_IMAGE"
run_cmd mmd -i "$HOST_ESP_TEMP_IMAGE" ::/EFI ::/EFI/BOOT
run_cmd mcopy -i "$HOST_ESP_TEMP_IMAGE" "$HOST_EFI_APP" ::/EFI/BOOT/BOOTX64.EFI
run_cmd mcopy -i "$HOST_ESP_TEMP_IMAGE" "$HOST_STARTUP_NSH" ::/startup.nsh

if [ "$INCLUDE_SLOT_ASSETS" -eq 1 ]; then
    if [ -f "$SLOT_A_KERNEL" ] && [ -s "$SLOT_A_KERNEL" ] &&
       [ -f "$SLOT_A_INITRD" ] && [ -s "$SLOT_A_INITRD" ]; then
        run_cmd mmd -i "$HOST_ESP_TEMP_IMAGE" ::/EFI/STRAT ::/EFI/STRAT/SLOT_A
        run_cmd mcopy -i "$HOST_ESP_TEMP_IMAGE" "$HOST_SLOT_A_KERNEL" ::/EFI/STRAT/SLOT_A/vmlinuz.efi
        run_cmd mcopy -i "$HOST_ESP_TEMP_IMAGE" "$HOST_SLOT_A_INITRD" ::/EFI/STRAT/SLOT_A/initramfs.img
    else
        echo "Skipping slot payload copy: missing kernel/initrd assets." >&2
        echo "  kernel: $SLOT_A_KERNEL" >&2
        echo "  initrd: $SLOT_A_INITRD" >&2
    fi
fi

run_cmd dd if="$HOST_ESP_TEMP_IMAGE" of="$HOST_IMAGE" bs=512 seek="$ESP_FIRST_LBA" conv=notrunc status=none
run_cmd rm -f "$HOST_ESP_TEMP_IMAGE"

if [ "$MAKE_VHD" -eq 1 ]; then
    run_cmd qemu-img convert -f raw -O vpc "$HOST_IMAGE" "$HOST_VHD"
fi

echo "$IMAGE_PATH"
if [ "$MAKE_VHD" -eq 1 ]; then
    echo "$VHD_PATH"
fi
