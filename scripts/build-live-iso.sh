#!/bin/bash
# Assemble a UEFI-bootable hybrid ISO for StratOS live (Milestone A).
# Prerequisite: full artifacts from ./build-all-and-run.sh.
#
# Produces: out/live/stratos-live.iso
#
# Requires: mkfs.vfat, mtools (mmd, mcopy), xorriso

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PHASE7="$REPO_ROOT/out/phase7"
PHASE4="$REPO_ROOT/out/phase4"
PHASE3="$REPO_ROOT/out/phase3"
OUT_DIR="$REPO_ROOT/out/live"
OUT_ISO="$OUT_DIR/stratos-live.iso"

SLOT_ERofs="$PHASE7/slot-system.erofs"
# Same artifact as initramfs.cpio.gz; ESP + ISO9660 use the name initramfs.img by convention (StratBoot / installer paths).
INITRD="$PHASE7/initramfs.cpio.gz"
KERNEL="$PHASE4/vmlinuz"
BOOT_EFI="$PHASE3/BOOTX64.EFI"

log() { echo "[build-live-iso] $*"; }

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        exit 1
    fi
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help)
            echo "Usage: $0"
            echo "  Writes $OUT_ISO from out/phase7, out/phase4, out/phase3."
            echo "  Run ./build-all-and-run.sh -s first (or full build)."
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

require_cmd mkfs.vfat
require_cmd xorriso
for t in mmd mcopy; do
    require_cmd "$t"
done

for f in "$SLOT_ERofs" "$INITRD" "$KERNEL" "$BOOT_EFI"; do
    if [ ! -f "$f" ]; then
        echo "Missing artifact: $f" >&2
        echo "Build first: ./build-all-and-run.sh -s" >&2
        exit 1
    fi
done

mkdir -p "$OUT_DIR"

WORK="$(mktemp -d)"
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

log "building FAT EFI system image (embedded El Torito payload)"
dd if=/dev/zero of="$WORK/esp.img" bs=1M count=80 status=none
mkfs.vfat -F 32 "$WORK/esp.img"

mmd -i "$WORK/esp.img" ::/EFI ::/EFI/BOOT ::/EFI/STRAT ::/EFI/STRAT/SLOT_A
mcopy -i "$WORK/esp.img" "$BOOT_EFI" ::/EFI/BOOT/BOOTX64.EFI
mcopy -i "$WORK/esp.img" "$KERNEL" ::/EFI/STRAT/SLOT_A/vmlinuz.efi
mcopy -i "$WORK/esp.img" "$INITRD" ::/EFI/STRAT/SLOT_A/initramfs.img
: > "$WORK/LIVE_MARK"
mcopy -i "$WORK/esp.img" "$WORK/LIVE_MARK" ::/EFI/STRAT/LIVE

log "staging ISO9660 payload (EROFS + copies for strat-installer on bare metal)"
mkdir -p "$WORK/data"
cp -f "$SLOT_ERofs" "$WORK/data/slot-system.erofs"
# Plain filenames so the installer can read them from the mounted ISO (see scripts/strat-installer.sh).
cp -f "$KERNEL" "$WORK/data/vmlinuz.efi"
cp -f "$INITRD" "$WORK/data/initramfs.img"
cp -f "$BOOT_EFI" "$WORK/data/BOOTX64.EFI"
# El Torito -e path must be relative to the ISO source dir ($WORK/data), not an absolute path outside it.
cp -f "$WORK/esp.img" "$WORK/data/esp.img"

log "writing $OUT_ISO"
xorriso -as mkisofs \
    -iso-level 3 \
    -full-iso9660-filenames \
    -volid "STRATOS_LIVE" \
    -appid "StratOS live" \
    -r \
    -J -joliet -joliet-long \
    -eltorito-alt-boot \
    -e esp.img \
    -no-emul-boot \
    -isohybrid-gpt-basdat \
    -o "$OUT_ISO" \
    "$WORK/data"

log "done: $OUT_ISO ($(du -h "$OUT_ISO" | cut -f1))"
