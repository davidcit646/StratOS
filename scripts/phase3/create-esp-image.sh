#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase3"
ESP_IMG="$OUT_DIR/esp.img"
ESP_SIZE_MB="${ESP_SIZE_MB:-64}"

usage() {
    cat <<EOF
Usage: $0 [--image PATH] [--size-mb N]

Builds a bootable ESP image containing StratBoot as BOOTX64.EFI.
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --image)
            ESP_IMG="$2"
            shift 2
            ;;
        --size-mb)
            ESP_SIZE_MB="$2"
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

if ! command -v mformat >/dev/null 2>&1; then
    echo "mformat not found (mtools). Install mtools to build ESP image." >&2
    exit 1
fi
if ! command -v mcopy >/dev/null 2>&1; then
    echo "mcopy not found (mtools). Install mtools to build ESP image." >&2
    exit 1
fi
if ! command -v mmd >/dev/null 2>&1; then
    echo "mmd not found (mtools). Install mtools to build ESP image." >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

EFI_APP="$("$REPO_ROOT/scripts/phase3/build-stratboot.sh")"
if [ -z "$EFI_APP" ] || [ ! -f "$EFI_APP" ] || [ ! -s "$EFI_APP" ]; then
    echo "StratBoot build failed or output missing: $EFI_APP" >&2
    exit 1
fi

dd if=/dev/zero of="$ESP_IMG" bs=1M count="$ESP_SIZE_MB" status=none
mformat -i "$ESP_IMG" -F
mmd -i "$ESP_IMG" ::/EFI ::/EFI/BOOT
mcopy -i "$ESP_IMG" "$EFI_APP" ::/EFI/BOOT/BOOTX64.EFI

cat > "$OUT_DIR/startup.nsh" <<'EOF'
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
EOF
mcopy -i "$ESP_IMG" "$OUT_DIR/startup.nsh" ::/startup.nsh

if ! mdir -i "$ESP_IMG" ::/EFI/BOOT/BOOTX64.EFI >/dev/null 2>&1; then
    echo "Failed to write BOOTX64.EFI into ESP image." >&2
    exit 1
fi

echo "$ESP_IMG"
