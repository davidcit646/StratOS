#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase3"
ISO_PATH="$OUT_DIR/stratboot.iso"
STAGING_DIR=""

usage() {
    cat <<EOF
Usage: $0 [--iso PATH]

Builds a UEFI-bootable ISO image containing StratBoot as BOOTX64.EFI.
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --iso)
            ISO_PATH="$2"
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

if ! command -v xorriso >/dev/null 2>&1 &&
   ! command -v genisoimage >/dev/null 2>&1 &&
   ! command -v mkisofs >/dev/null 2>&1; then
    echo "Missing ISO tool (xorriso, genisoimage, or mkisofs)." >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

# Build FAT ESP image (4MB — contains BOOTX64.EFI + startup.nsh)
ESP_IMG="$("$REPO_ROOT/scripts/phase3/create-esp-image.sh" --size-mb 4)"
if [ -z "$ESP_IMG" ] || [ ! -f "$ESP_IMG" ] || [ ! -s "$ESP_IMG" ]; then
    echo "ESP image build failed or output missing: $ESP_IMG" >&2
    exit 1
fi

STAGING_DIR="$(mktemp -d /tmp/stratboot-iso.XXXXXX)"
cp "$ESP_IMG" "$STAGING_DIR/esp.img"
# Also place EFI/BOOT tree on ISO9660 for firmware that scans the ISO filesystem
mkdir -p "$STAGING_DIR/EFI/BOOT"
mcopy -i "$ESP_IMG" ::/EFI/BOOT/BOOTX64.EFI "$STAGING_DIR/EFI/BOOT/BOOTX64.EFI"
# Keep startup.nsh at ISO root too, so shells that only auto-run from ISO root can see it.
if mdir -i "$ESP_IMG" ::/startup.nsh >/dev/null 2>&1; then
    mcopy -i "$ESP_IMG" ::/startup.nsh "$STAGING_DIR/startup.nsh"
fi

if command -v xorriso >/dev/null 2>&1; then
    xorriso -as mkisofs \
        -R -J \
        -o "$ISO_PATH" \
        -eltorito-platform efi \
        -e esp.img \
        -no-emul-boot \
        -isohybrid-gpt-basdat \
        "$STAGING_DIR"
elif command -v genisoimage >/dev/null 2>&1; then
    genisoimage \
        -R -J \
        -o "$ISO_PATH" \
        -eltorito-platform efi \
        -e esp.img \
        -no-emul-boot \
        -isohybrid-gpt-basdat \
        "$STAGING_DIR"
else
    mkisofs \
        -R -J \
        -o "$ISO_PATH" \
        -eltorito-platform efi \
        -e esp.img \
        -no-emul-boot \
        -isohybrid-gpt-basdat \
        "$STAGING_DIR"
fi

rm -rf "$STAGING_DIR"

echo "$ISO_PATH"
