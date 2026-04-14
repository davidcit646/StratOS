#!/usr/bin/env sh
set -eu

TEST_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$TEST_DIR/.." && pwd)"
PHASE1_DIR="$REPO_ROOT/scripts/phase1"
. "$PHASE1_DIR/lib.sh"

IMAGE_PATH="${IMAGE_PATH:-$REPO_ROOT/out/stratos-phase1.raw}"
SIZE_GB="${SIZE_GB:-256}"
SKIP_CREATE=0
POC=0
LOOP_DEV=""

usage() {
    cat <<EOF
Usage: $0 [--image PATH] [--size-gb N] [--skip-create] [--poc]

Runs Phase 1 end-to-end checks:
  1) create GPT layout (unless --skip-create)
  2) format all partitions with expected filesystems/labels
  3) validate mount behavior for SLOT_A, CONFIG, HOME
  4) assert partition table names/sizes

--poc uses test-only partition sizes and is not spec-compliant.
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --image)
            IMAGE_PATH="$2"
            shift 2
            ;;
        --size-gb)
            SIZE_GB="$2"
            shift 2
            ;;
        --skip-create)
            SKIP_CREATE=1
            shift
            ;;
        --poc)
            POC=1
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

cleanup() {
    phase1_detach_loop "$LOOP_DEV"
}
trap cleanup EXIT INT TERM

phase1_detect_run_context sgdisk losetup partprobe blkid

if [ "$SKIP_CREATE" -eq 0 ]; then
    if [ "$POC" -eq 1 ]; then
        "$PHASE1_DIR/create-partition-layout.sh" --image "$IMAGE_PATH" --size-gb "$SIZE_GB" --poc --force
    else
        "$PHASE1_DIR/create-partition-layout.sh" --image "$IMAGE_PATH" --size-gb "$SIZE_GB" --force
    fi
fi

"$PHASE1_DIR/format-partitions.sh" --image "$IMAGE_PATH"
"$PHASE1_DIR/validate-mounts.sh" --image "$IMAGE_PATH"

table_output="$(phase1_run_cmd sgdisk -p "$IMAGE_PATH")"
part_count="$(echo "$table_output" | awk '/^[[:space:]]*[0-9]+[[:space:]]/ {count++} END {print count+0}')"
if [ "$part_count" -ne 7 ]; then
    echo "Expected 7 partitions, found $part_count." >&2
    exit 1
fi

assert_table_line() {
    regex="$1"
    description="$2"
    if ! echo "$table_output" | grep -Eq "$regex"; then
        echo "Partition table check failed: $description" >&2
        exit 1
    fi
}

assert_table_line '1[[:space:]].*512\.0 MiB[[:space:]]+EF00[[:space:]]+ESP' 'ESP partition'
if [ "$POC" -eq 1 ]; then
    assert_table_line '2[[:space:]].*512\.0 MiB[[:space:]]+8300[[:space:]]+SLOT_A' 'SLOT_A partition (POC)'
    assert_table_line '3[[:space:]].*512\.0 MiB[[:space:]]+8300[[:space:]]+SLOT_B' 'SLOT_B partition (POC)'
    assert_table_line '4[[:space:]].*512\.0 MiB[[:space:]]+8300[[:space:]]+SLOT_C' 'SLOT_C partition (POC)'
    assert_table_line '5[[:space:]].*512\.0 MiB[[:space:]]+8300[[:space:]]+CONFIG' 'CONFIG partition (POC)'
    assert_table_line '6[[:space:]].*1024\.0 MiB[[:space:]]+8300[[:space:]]+STRAT_CACHE' 'STRAT_CACHE partition (POC)'
    assert_table_line '7[[:space:]].*1024\.0 MiB[[:space:]]+8300[[:space:]]+HOME' 'HOME partition (POC)'
else
    assert_table_line '2[[:space:]].*20\.0 GiB[[:space:]]+8300[[:space:]]+SLOT_A' 'SLOT_A partition'
    assert_table_line '3[[:space:]].*20\.0 GiB[[:space:]]+8300[[:space:]]+SLOT_B' 'SLOT_B partition'
    assert_table_line '4[[:space:]].*20\.0 GiB[[:space:]]+8300[[:space:]]+SLOT_C' 'SLOT_C partition'
    assert_table_line '5[[:space:]].*4\.0 GiB[[:space:]]+8300[[:space:]]+CONFIG' 'CONFIG partition'
    assert_table_line '6[[:space:]].*50\.0 GiB[[:space:]]+8300[[:space:]]+STRAT_CACHE' 'STRAT_CACHE partition'
    assert_table_line '7[[:space:]].*8300[[:space:]]+HOME' 'HOME partition'
fi

LOOP_DEV="$(phase1_attach_loop "$IMAGE_PATH")"
P1="$(phase1_resolve_partition_dev "$LOOP_DEV" 1)"
P2="$(phase1_resolve_partition_dev "$LOOP_DEV" 2)"
P3="$(phase1_resolve_partition_dev "$LOOP_DEV" 3)"
P4="$(phase1_resolve_partition_dev "$LOOP_DEV" 4)"
P5="$(phase1_resolve_partition_dev "$LOOP_DEV" 5)"
P6="$(phase1_resolve_partition_dev "$LOOP_DEV" 6)"
P7="$(phase1_resolve_partition_dev "$LOOP_DEV" 7)"

check_fs_signature() {
    part_dev="$1"
    expected_type="$2"
    expected_label="$3"
    actual_type="$(phase1_run_root_cmd blkid -o value -s TYPE "$part_dev" 2>/dev/null || true)"
    actual_label="$(phase1_run_root_cmd blkid -o value -s LABEL "$part_dev" 2>/dev/null || true)"
    if [ "$actual_type" != "$expected_type" ] || [ "$actual_label" != "$expected_label" ]; then
        echo "Filesystem signature mismatch on $part_dev: got type='$actual_type' label='$actual_label'" >&2
        echo "Expected type='$expected_type' label='$expected_label'" >&2
        exit 1
    fi
}

check_fs_signature "$P1" vfat ESP
check_fs_signature "$P2" erofs SLOT_A
check_fs_signature "$P3" erofs SLOT_B
check_fs_signature "$P4" erofs SLOT_C
check_fs_signature "$P5" ext4 CONFIG
check_fs_signature "$P6" xfs STRAT_CACHE
check_fs_signature "$P7" btrfs HOME

echo "Phase 1 partition layout test passed."
