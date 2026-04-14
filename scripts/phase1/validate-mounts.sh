#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
. "$SCRIPT_DIR/lib.sh"

IMAGE_PATH="${IMAGE_PATH:-out/stratos-disk.raw}"
LOOP_DEV=""
MOUNT_ROOT=""

usage() {
    cat <<EOF
Usage: $0 [--image PATH]

Validates Phase 1 mount behavior:
  - SLOT_A mounts read-only and rejects writes
  - CONFIG mounts read-write and accepts writes
  - HOME mounts read-write and accepts writes
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
    if [ -n "$MOUNT_ROOT" ]; then
        SLOT_A_MP="$MOUNT_ROOT/slot_a"
        CONFIG_MP="$MOUNT_ROOT/config"
        HOME_MP="$MOUNT_ROOT/home"

        if phase1_run_cmd sh -lc "findmnt -rn '$HOME_MP' >/dev/null 2>&1"; then
            phase1_run_root_cmd umount "$HOME_MP" >/dev/null 2>&1 || true
        fi
        if phase1_run_cmd sh -lc "findmnt -rn '$CONFIG_MP' >/dev/null 2>&1"; then
            phase1_run_root_cmd umount "$CONFIG_MP" >/dev/null 2>&1 || true
        fi
        if phase1_run_cmd sh -lc "findmnt -rn '$SLOT_A_MP' >/dev/null 2>&1"; then
            phase1_run_root_cmd umount "$SLOT_A_MP" >/dev/null 2>&1 || true
        fi

        phase1_run_cmd rm -rf "$MOUNT_ROOT" >/dev/null 2>&1 || true
    fi
    phase1_detach_loop "$LOOP_DEV"
}
trap cleanup EXIT INT TERM

phase1_detect_run_context losetup partprobe mount umount findmnt mktemp
phase1_ensure_image_exists "$IMAGE_PATH"

LOOP_DEV="$(phase1_attach_loop "$IMAGE_PATH")"
P2="$(phase1_resolve_partition_dev "$LOOP_DEV" 2)"
P5="$(phase1_resolve_partition_dev "$LOOP_DEV" 5)"
P7="$(phase1_resolve_partition_dev "$LOOP_DEV" 7)"

MOUNT_ROOT="$(phase1_run_cmd mktemp -d /tmp/stratos-phase1-mounts.XXXXXX)"
SLOT_A_MP="$MOUNT_ROOT/slot_a"
CONFIG_MP="$MOUNT_ROOT/config"
HOME_MP="$MOUNT_ROOT/home"
phase1_run_cmd mkdir -p "$SLOT_A_MP" "$CONFIG_MP" "$HOME_MP"

echo "Validating SLOT_A read-only mount..."
phase1_run_root_cmd mount -o ro "$P2" "$SLOT_A_MP"
slot_a_opts="$(phase1_run_cmd findmnt -no OPTIONS "$SLOT_A_MP" || true)"
if ! echo "$slot_a_opts" | grep -Eq '(^|,)ro(,|$)'; then
    echo "SLOT_A is not mounted read-only: options='$slot_a_opts'" >&2
    exit 1
fi
if phase1_run_root_cmd sh -lc "printf '%s\n' test > '$SLOT_A_MP/.phase1-write-test'"; then
    echo "Write unexpectedly succeeded on SLOT_A (expected failure)." >&2
    exit 1
fi

echo "Validating CONFIG read-write mount..."
phase1_run_root_cmd mount "$P5" "$CONFIG_MP"
config_opts="$(phase1_run_cmd findmnt -no OPTIONS "$CONFIG_MP" || true)"
if ! echo "$config_opts" | grep -Eq '(^|,)rw(,|$)'; then
    echo "CONFIG is not mounted read-write: options='$config_opts'" >&2
    exit 1
fi
phase1_run_root_cmd sh -lc "printf '%s\n' config-ok > '$CONFIG_MP/.phase1-write-test'"
phase1_run_root_cmd sh -lc "[ -f '$CONFIG_MP/.phase1-write-test' ]"

echo "Validating HOME read-write mount..."
phase1_run_root_cmd mount "$P7" "$HOME_MP"
home_opts="$(phase1_run_cmd findmnt -no OPTIONS "$HOME_MP" || true)"
if ! echo "$home_opts" | grep -Eq '(^|,)rw(,|$)'; then
    echo "HOME is not mounted read-write: options='$home_opts'" >&2
    exit 1
fi
phase1_run_root_cmd sh -lc "printf '%s\n' home-ok > '$HOME_MP/.phase1-write-test'"
phase1_run_root_cmd sh -lc "[ -f '$HOME_MP/.phase1-write-test' ]"

echo "Phase 1 mount validation passed."
