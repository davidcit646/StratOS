#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
. "$SCRIPT_DIR/lib.sh"

IMAGE_PATH="${IMAGE_PATH:-out/stratos-disk.raw}"
LOOP_DEV=""
TMP_DIR=""

usage() {
    cat <<EOF
Usage: $0 [--image PATH]

Formats the StratOS Phase 1 partitions on an existing raw disk image:
  p1 ESP         -> FAT32 label ESP
  p2 SLOT_A      -> EROFS label SLOT_A
  p3 SLOT_B      -> EROFS label SLOT_B
  p4 SLOT_C      -> EROFS label SLOT_C
  p5 CONFIG      -> ext4  label CONFIG
  p6 STRAT_CACHE -> XFS   label STRAT_CACHE
  p7 HOME        -> Btrfs label HOME
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --image)
            IMAGE_PATH="$2"
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

cleanup() {
    phase1_detach_loop "$LOOP_DEV"
    if [ -n "$TMP_DIR" ]; then
        phase1_run_cmd rm -rf "$TMP_DIR" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT INT TERM

phase1_detect_run_context \
    losetup partprobe blkid blockdev dd \
    mkfs.vfat mkfs.ext4 mkfs.xfs mkfs.btrfs

if ! phase1_run_cmd sh -lc 'command -v mkfs.erofs >/dev/null 2>&1'; then
    echo "mkfs.erofs is required (package: erofs-utils)." >&2
    echo "On Bazzite/Silverblue: rpm-ostree install erofs-utils && reboot" >&2
    exit 1
fi

phase1_ensure_image_exists "$IMAGE_PATH"

TMP_DIR="$(phase1_run_cmd mktemp -d /tmp/stratos-phase1-format.XXXXXX)"
LOOP_DEV="$(phase1_attach_loop "$IMAGE_PATH")"

P1="$(phase1_resolve_partition_dev "$LOOP_DEV" 1)"
P2="$(phase1_resolve_partition_dev "$LOOP_DEV" 2)"
P3="$(phase1_resolve_partition_dev "$LOOP_DEV" 3)"
P4="$(phase1_resolve_partition_dev "$LOOP_DEV" 4)"
P5="$(phase1_resolve_partition_dev "$LOOP_DEV" 5)"
P6="$(phase1_resolve_partition_dev "$LOOP_DEV" 6)"
P7="$(phase1_resolve_partition_dev "$LOOP_DEV" 7)"

echo "Formatting ESP as FAT32..."
phase1_run_root_cmd mkfs.vfat -F 32 -n ESP "$P1" >/dev/null

build_slot_image() {
    slot_label="$1"
    src_dir="$2"
    img_path="$3"

    phase1_run_cmd mkdir -p "$src_dir/system"
    phase1_run_cmd sh -lc "printf '%s\n' 'placeholder image for $slot_label' > '$src_dir/system/.stratos-slot'"
    phase1_run_cmd mkfs.erofs -L "$slot_label" "$img_path" "$src_dir" >/dev/null
}

write_erofs_partition() {
    slot_label="$1"
    part_dev="$2"

    src_dir="$TMP_DIR/src-$slot_label"
    img_path="$TMP_DIR/$slot_label.erofs"
    build_slot_image "$slot_label" "$src_dir" "$img_path"
    phase1_run_root_cmd dd if="$img_path" of="$part_dev" bs=4M conv=fsync,notrunc status=none
}

echo "Formatting SLOT_A/SLOT_B/SLOT_C as EROFS..."
write_erofs_partition SLOT_A "$P2"
write_erofs_partition SLOT_B "$P3"
write_erofs_partition SLOT_C "$P4"

echo "Formatting CONFIG as ext4..."
phase1_run_root_cmd mkfs.ext4 -F -L CONFIG "$P5" >/dev/null

echo "Formatting STRAT_CACHE as XFS..."
phase1_run_root_cmd mkfs.xfs -f -L STRAT_CACHE "$P6" >/dev/null

echo "Formatting HOME as Btrfs..."
phase1_run_root_cmd mkfs.btrfs -f -L HOME "$P7" >/dev/null

check_partition() {
    part_dev="$1"
    expected_type="$2"
    expected_label="$3"

    actual_type="$(phase1_run_root_cmd blkid -o value -s TYPE "$part_dev" 2>/dev/null || true)"
    actual_label="$(phase1_run_root_cmd blkid -o value -s LABEL "$part_dev" 2>/dev/null || true)"

    if [ "$actual_type" != "$expected_type" ]; then
        echo "Unexpected filesystem type on $part_dev: got '$actual_type', expected '$expected_type'" >&2
        exit 1
    fi

    if [ "$actual_label" != "$expected_label" ]; then
        echo "Unexpected label on $part_dev: got '$actual_label', expected '$expected_label'" >&2
        exit 1
    fi
}

check_partition "$P1" vfat ESP
check_partition "$P2" erofs SLOT_A
check_partition "$P3" erofs SLOT_B
check_partition "$P4" erofs SLOT_C
check_partition "$P5" ext4 CONFIG
check_partition "$P6" xfs STRAT_CACHE
check_partition "$P7" btrfs HOME

echo "Partition filesystem labels and types:"
phase1_run_cmd lsblk -o NAME,FSTYPE,LABEL,SIZE "$LOOP_DEV"
echo "Phase 1 formatting complete."
