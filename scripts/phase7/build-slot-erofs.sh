#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
ROOTFS_DIR="$REPO_ROOT/sysroot"
OUTPUT_IMAGE="$REPO_ROOT/out/phase7/slot-system.erofs"
VOLUME_NAME="STRAT_SYSTEM"
MKEROFSCMD=""
use_host=0

usage() {
    cat <<USAGE
Usage: $0 [--rootfs PATH] [--output PATH] [--label NAME]

Builds an EROFS image for a system slot from a prepared rootfs directory.
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --rootfs)
            ROOTFS_DIR="$2"
            shift 2
            ;;
        --output)
            OUTPUT_IMAGE="$2"
            shift 2
            ;;
        --label)
            VOLUME_NAME="$2"
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

if [ ! -d "$ROOTFS_DIR" ]; then
    echo "Rootfs directory not found: $ROOTFS_DIR" >&2
    exit 1
fi

if [ ! -f "$ROOTFS_DIR/sbin/init" ] && [ ! -f "$ROOTFS_DIR/init" ]; then
    echo "Rootfs missing init entrypoint (/sbin/init or /init): $ROOTFS_DIR" >&2
    exit 1
fi

# Required mount-point stubs expected by initramfs-init after root pivot.
for mount_dir in /proc /sys /dev /config /apps /home /usr /var /run /sbin; do
    if [ ! -d "$ROOTFS_DIR$mount_dir" ]; then
        echo "Rootfs missing required mount-point directory: $ROOTFS_DIR$mount_dir" >&2
        exit 1
    fi
done

if has_local mkfs.erofs; then
    MKEROFSCMD="mkfs.erofs"
elif has_local mkerofs; then
    MKEROFSCMD="mkerofs"
elif has_local flatpak-spawn && has_host mkfs.erofs; then
    MKEROFSCMD="mkfs.erofs"
    use_host=1
elif has_local flatpak-spawn && has_host mkerofs; then
    MKEROFSCMD="mkerofs"
    use_host=1
else
    echo "Missing EROFS builder (mkfs.erofs/mkerofs). Install erofs-utils." >&2
    exit 1
fi

HOST_ROOTFS_DIR="$(to_host_path "$ROOTFS_DIR")"
HOST_OUTPUT_IMAGE="$(to_host_path "$OUTPUT_IMAGE")"

run_cmd mkdir -p "$(dirname "$HOST_OUTPUT_IMAGE")"
run_cmd rm -f "$HOST_OUTPUT_IMAGE"

# Keep image deterministic and small for slot replication.
run_cmd "$MKEROFSCMD" -L "$VOLUME_NAME" -zlz4hc "$HOST_OUTPUT_IMAGE" "$HOST_ROOTFS_DIR"

if [ ! -s "$OUTPUT_IMAGE" ]; then
    echo "Failed to create EROFS image: $OUTPUT_IMAGE" >&2
    exit 1
fi

echo "$OUTPUT_IMAGE"
