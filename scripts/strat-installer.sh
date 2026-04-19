#!/bin/bash
# StratOS disk installer (live session). Destroys all data on the target disk.
#
# GPT layout matches scripts/create-test-disk.sh / update-test-disk.sh:
#   ESP, SLOT_A (EROFS), SLOT_B, SLOT_C, CONFIG, STRAT_CACHE, HOME (uses rest of disk).
#
# Source payloads: StratOS live ISO9660 (see resolve_source) or --source-dir with:
#   slot-system.erofs  vmlinuz.efi  initramfs.img  BOOTX64.EFI
#   (initramfs.img is the same gzip cpio as out/phase7/initramfs.cpio.gz; name is convention.)
#
# Usage (as root):
#   strat-installer --disk /dev/nvme0n1
#   strat-installer --disk /dev/sda --source-dir /mnt/custom
#
# Refuses --disk if it is the same whole-disk device as the mounted install source (live ISO/USB),
# unless you pass --allow-wipe-live-medium (last resort).
#
# Requires: sgdisk, partprobe, blockdev, mkfs.vfat, mkfs.ext4, mkfs.btrfs, blkid, lsblk (in /usr/sbin from rootfs build).

set -euo pipefail

CONFIRM_PHRASE="DESTROY_ALL_DATA_ON_THIS_DISK"

die() { echo "strat-installer: $*" >&2; exit 1; }

usage() {
    echo "strat-installer — destructive fresh install to a single disk (matches create-test-disk layout)."
    echo "Options:"
    echo "  --disk PATH     Whole block device (e.g. /dev/sda, /dev/nvme0n1)"
    echo "  --source-dir D  install payloads (default: mount live ISO by LABEL=STRATOS_LIVE, else /dev/sr*)"
    echo "  --allow-wipe-live-medium  allow --disk to match the block device backing the source (DANGEROUS)"
    echo "  -h, --help"
    exit "${1:-0}"
}

# Whole disk for a block dev: /dev/sdb1 -> /dev/sdb; /dev/sr0 -> /dev/sr0; nvme0n1p2 -> /dev/nvme0n1
whole_disk_for_dev() {
    local d="$1"
    [ -b "$d" ] || { echo "$d"; return 0; }
    d=$(readlink -f "$d")
    local pk
    pk=$(lsblk -ndo PKNAME "$d" 2>/dev/null | head -1 | tr -d '[:space:]')
    if [ -n "$pk" ] && [ -e "/dev/$pk" ]; then
        readlink -f "/dev/$pk"
    else
        echo "$d"
    fi
}

# Block device that backs PATH's mount (strip udev [bracket] suffix from findmnt)
backing_block_for_path() {
    local p="$1"
    command -v findmnt >/dev/null 2>&1 || return 1
    local src
    src=$(findmnt -n -o SOURCE -T "$p" 2>/dev/null) || return 1
    src="${src%%[*}"
    src="${src%% }"
    [ -n "$src" ] || return 1
    [ -b "$src" ] || return 1
    echo "$src"
}

disk_part() {
    local d="$1" n="$2"
    local base
    base=$(basename "$d")
    if [[ "$base" == mmcblk* ]] || [[ "$base" == nvme* ]]; then
        echo "${d}p${n}"
    else
        echo "${d}${n}"
    fi
}

DISK=""
SOURCE_DIR=""
ISO_MNT=""
ALLOW_WIPE_LIVE_MEDIUM=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --disk) DISK="$2"; shift 2 ;;
        --source-dir) SOURCE_DIR="$2"; shift 2 ;;
        --allow-wipe-live-medium) ALLOW_WIPE_LIVE_MEDIUM=1; shift ;;
        -h|--help) usage 0 ;;
        *) die "unknown option: $1 (try --help)" ;;
    esac
done

[ "$(id -u)" -eq 0 ] || die "must run as root"
[ -n "$DISK" ] || die "missing required --disk"
[ -b "$DISK" ] || die "not a block device: $DISK"

bn=$(basename "$DISK")
ok=0
if [[ "$bn" =~ ^(sd[a-z]+|vd[a-z]+|xvd[a-z]+|nvme[0-9]+n[0-9]+|mmcblk[0-9]+)$ ]]; then
    ok=1
fi
[ "$ok" -eq 1 ] || die "use a whole disk (e.g. /dev/sda, /dev/nvme0n1), not a partition — got $DISK"

if ! grep -q 'strat\.live=1' /proc/cmdline 2>/dev/null; then
    echo "Warning: kernel cmdline does not have strat.live=1 — expected on StratOS live ISO only." >&2
fi

INSTALL_ESP_MNT=""

cleanup_installer() {
    if [ -n "${INSTALL_ESP_MNT:-}" ] && mountpoint -q "$INSTALL_ESP_MNT" 2>/dev/null; then
        umount "$INSTALL_ESP_MNT" 2>/dev/null || true
        rmdir "$INSTALL_ESP_MNT" 2>/dev/null || true
    fi
    if [ -n "${ISO_MNT:-}" ] && mountpoint -q "$ISO_MNT" 2>/dev/null; then
        umount "$ISO_MNT" 2>/dev/null || true
    fi
    if [ -n "${ISO_MNT:-}" ] && [ -d "$ISO_MNT" ]; then
        rmdir "$ISO_MNT" 2>/dev/null || true
    fi
}
trap cleanup_installer EXIT

resolve_source() {
    if [ -n "$SOURCE_DIR" ]; then
        [ -d "$SOURCE_DIR" ] || die "source dir not found: $SOURCE_DIR"
        return 0
    fi
    ISO_MNT=$(mktemp -d /tmp/stratos-iso.XXXXXX)
    # Prefer ISO9660 volume id (matches build-live-iso.sh -volid) when several optical drives exist.
    preferred_dev=""
    if command -v blkid >/dev/null 2>&1; then
        preferred_dev=$(blkid -l -o device -t LABEL=STRATOS_LIVE 2>/dev/null) || true
    fi
    if [ -n "$preferred_dev" ] && [ -b "$preferred_dev" ]; then
        if mount -o ro "$preferred_dev" "$ISO_MNT" 2>/dev/null; then
            if [ -f "$ISO_MNT/slot-system.erofs" ]; then
                SOURCE_DIR="$ISO_MNT"
                echo "Using ISO mounted at $SOURCE_DIR ($preferred_dev, LABEL=STRATOS_LIVE)"
                return 0
            fi
            umount "$ISO_MNT" 2>/dev/null || true
        fi
    fi
    for i in {0..31}; do
        dev="/dev/sr$i"
        [ -b "$dev" ] || continue
        if mount -o ro "$dev" "$ISO_MNT" 2>/dev/null; then
            if [ -f "$ISO_MNT/slot-system.erofs" ]; then
                SOURCE_DIR="$ISO_MNT"
                echo "Using ISO mounted at $SOURCE_DIR ($dev)"
                return 0
            fi
            umount "$ISO_MNT" 2>/dev/null || true
        fi
    done
    die "could not mount live ISO (LABEL=STRATOS_LIVE or /dev/sr*) — try --source-dir with slot-system.erofs"
}

resolve_source

ER="$SOURCE_DIR/slot-system.erofs"
KV="$SOURCE_DIR/vmlinuz.efi"
IR="$SOURCE_DIR/initramfs.img"
BE="$SOURCE_DIR/BOOTX64.EFI"

for f in "$ER" "$KV" "$IR" "$BE"; do
    [ -f "$f" ] || die "missing file in source: $f"
done

# Never wipe the disk that currently supplies the install payloads (live USB / ISO volume).
SOURCE_BACKING=""
if src_dev=$(backing_block_for_path "$SOURCE_DIR"); then
    SOURCE_BACKING=$(whole_disk_for_dev "$src_dev")
fi
DISK_W=$(whole_disk_for_dev "$DISK")
if [ -n "$SOURCE_BACKING" ] && [ "$DISK_W" = "$SOURCE_BACKING" ]; then
    if [ "$ALLOW_WIPE_LIVE_MEDIUM" -ne 1 ]; then
        die "refuse: --disk $DISK is the same whole-disk device as the install source ($SOURCE_BACKING backing $SOURCE_DIR). Choose the internal disk, or pass --allow-wipe-live-medium if you really intend to destroy the boot medium."
    fi
    echo "Warning: --allow-wipe-live-medium set; wiping the device that hosts the install source." >&2
elif [ -z "$SOURCE_BACKING" ]; then
    if ! command -v findmnt >/dev/null 2>&1; then
        echo "Warning: findmnt not found; cannot verify install source disk — mistaken --disk could still wipe the live medium." >&2
    else
        echo "Note: could not resolve a block backing for $SOURCE_DIR (e.g. NFS/tmpfs); not applying live-medium guard." >&2
    fi
fi

ER_BYTES=$(stat -c%s "$ER")
ER_MIB=$(( (ER_BYTES + 1048575) / 1048576 ))
SLOT_A_MIB=$(( ER_MIB + 128 ))
if [ "$SLOT_A_MIB" -lt 1600 ]; then
    SLOT_A_MIB=1600
fi

DISK_BYTES=$(blockdev --getsize64 "$DISK")
DISK_MIB=$(( DISK_BYTES / 1048576 ))
# Minimum: ESP + SLOT_A + B + C + CONFIG + STRAT_CACHE + small HOME
MIN_NEED=$(( 512 + SLOT_A_MIB + 128 + 128 + 128 + 256 + 256 ))
if [ "$DISK_MIB" -lt "$MIN_NEED" ]; then
    die "disk too small: ${DISK_MIB} MiB; need at least ~${MIN_NEED} MiB for this image (SLOT_A=${SLOT_A_MIB} MiB)"
fi

echo ""
echo "Target:     $DISK ($(basename "$DISK"), ${DISK_MIB} MiB)"
echo "EROFS:      ${ER_BYTES} bytes → SLOT_A partition ${SLOT_A_MIB} MiB"
echo "Source:     $SOURCE_DIR"
echo ""
echo "ALL DATA ON THIS DISK WILL BE PERMANENTLY DESTROYED."
read -r -p "Type exactly: ${CONFIRM_PHRASE} : " line
[ "$line" = "$CONFIRM_PHRASE" ] || die "confirmation mismatch — aborted"

command -v sgdisk >/dev/null 2>&1 || die "sgdisk not found (install/build rootfs with GPT tools)"
command -v partprobe >/dev/null 2>&1 || die "partprobe not found"

# Refuse if any partition on this disk is mounted (except harmless cases)
while read -r dev mp _; do
    case "$dev" in
        ${DISK}*)
            [ -n "$mp" ] && [ "$mp" != "" ] && die "refuse: $dev is mounted on $mp — unmount first"
            ;;
    esac
done < /proc/mounts

sync
echo "Partitioning $DISK ..."
sgdisk --zap-all "$DISK" 2>/dev/null || true
sgdisk -og "$DISK"

sgdisk -n "1:0:+512M" -t "1:EF00" -c "1:ESP" "$DISK"
sgdisk -n "2:0:+${SLOT_A_MIB}M" -t "2:8300" -c "2:SLOT_A" "$DISK"
sgdisk -n "3:0:+128M" -t "3:8300" -c "3:SLOT_B" "$DISK"
sgdisk -n "4:0:+128M" -t "4:8300" -c "4:SLOT_C" "$DISK"
sgdisk -n "5:0:+128M" -t "5:8300" -c "5:CONFIG" "$DISK"
sgdisk -n "6:0:+256M" -t "6:8300" -c "6:STRAT_CACHE" "$DISK"
sgdisk -n "7:0:0" -t "7:8300" -c "7:HOME" "$DISK"

partprobe "$DISK" 2>/dev/null || true
sleep 1
sync

P1=$(disk_part "$DISK" 1)
P2=$(disk_part "$DISK" 2)
P5=$(disk_part "$DISK" 5)
P6=$(disk_part "$DISK" 6)
P7=$(disk_part "$DISK" 7)

for p in "$P1" "$P2" "$P5" "$P6" "$P7"; do
    [ -b "$p" ] || die "partition node missing: $p (try partprobe or udev)"
done

P2S=$(blockdev --getsize64 "$P2")
if [ "$ER_BYTES" -gt "$P2S" ]; then
    die "SLOT_A partition smaller than EROFS image"
fi

echo "Formatting ESP..."
mkfs.vfat -F 32 "$P1"

echo "Writing EROFS to SLOT_A..."
dd if="$ER" of="$P2" bs=4M conv=fsync status=progress

echo "Formatting CONFIG / STRAT_CACHE (ext4), HOME (btrfs)..."
mkfs.ext4 -F "$P5"
mkfs.ext4 -F "$P6"
mkfs.btrfs -f "$P7"

INSTALL_ESP_MNT=$(mktemp -d /tmp/stratos-esp.XXXXXX)
mount "$P1" "$INSTALL_ESP_MNT"

mkdir -p "$INSTALL_ESP_MNT/EFI/BOOT" \
    "$INSTALL_ESP_MNT/EFI/STRAT/SLOT_A" \
    "$INSTALL_ESP_MNT/EFI/STRAT/SLOT_B" \
    "$INSTALL_ESP_MNT/EFI/STRAT/SLOT_C"

# Installed system: no EFI/STRAT/LIVE marker (normal StratBoot path)
cp -f "$BE" "$INSTALL_ESP_MNT/EFI/BOOT/BOOTX64.EFI"
cp -f "$KV" "$INSTALL_ESP_MNT/EFI/STRAT/SLOT_A/vmlinuz.efi"
cp -f "$IR" "$INSTALL_ESP_MNT/EFI/STRAT/SLOT_A/initramfs.img"

sync
umount "$INSTALL_ESP_MNT"
rmdir "$INSTALL_ESP_MNT"
INSTALL_ESP_MNT=""

sync
cleanup_installer
trap - EXIT

echo ""
echo "Install finished. Remove the live USB (if any), select $DISK in firmware boot order, and reboot."
echo "On first NVMe/SATA boot, StratBoot will initialize EFI variables if the NVRAM is empty."
