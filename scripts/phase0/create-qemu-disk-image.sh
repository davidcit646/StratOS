#!/usr/bin/env sh
set -eu

IMAGE_PATH="${IMAGE_PATH:-out/stratos-disk.raw}"
SIZE_GB="${SIZE_GB:-256}"
FORCE=0
POC=0

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
        --poc)
            POC=1
            shift
            ;;
        --force)
            FORCE=1
            shift
            ;;
        *)
            echo "Unknown argument: $1" >&2
            echo "Usage: $0 [--image PATH] [--size-gb N] [--poc] [--force]" >&2
            exit 1
            ;;
    esac
done

RUN_CONTEXT="local"

run_cmd() {
    if [ "$RUN_CONTEXT" = "host" ]; then
        flatpak-spawn --host "$@"
        return
    fi
    "$@"
}

if ! command -v qemu-img >/dev/null 2>&1 || ! command -v sgdisk >/dev/null 2>&1; then
    if command -v flatpak-spawn >/dev/null 2>&1 &&
       flatpak-spawn --host sh -lc 'command -v qemu-img >/dev/null 2>&1 && command -v sgdisk >/dev/null 2>&1'
    then
        RUN_CONTEXT="host"
    fi
fi

if ! run_cmd sh -lc 'command -v qemu-img >/dev/null 2>&1'; then
    echo "qemu-img is required but not found." >&2
    exit 1
fi

if ! run_cmd sh -lc 'command -v sgdisk >/dev/null 2>&1'; then
    echo "sgdisk is required but not found." >&2
    exit 1
fi

IMAGE_DIR="$(dirname "$IMAGE_PATH")"
run_cmd mkdir -p "$IMAGE_DIR"

if run_cmd sh -lc "[ -e '$IMAGE_PATH' ]"; then
    if [ "$FORCE" -ne 1 ]; then
        echo "Image already exists: $IMAGE_PATH (use --force to overwrite)" >&2
        exit 1
    fi
    run_cmd rm -f "$IMAGE_PATH"
fi

echo "Creating raw disk image: $IMAGE_PATH (${SIZE_GB}G)"
run_cmd qemu-img create -f raw "$IMAGE_PATH" "${SIZE_GB}G"

if [ "$POC" -eq 1 ]; then
    echo "Applying StratOS GPT partition layout (POC sizes, not spec-compliant)..."
else
    echo "Applying StratOS GPT partition layout..."
fi
run_cmd sgdisk --zap-all "$IMAGE_PATH"
run_cmd sgdisk -o "$IMAGE_PATH"
run_cmd sgdisk -n 1:1MiB:+512MiB -t 1:EF00 -c 1:ESP "$IMAGE_PATH"
if [ "$POC" -eq 1 ]; then
    run_cmd sgdisk -n 2:0:+512MiB -t 2:8300 -c 2:SLOT_A "$IMAGE_PATH"
    run_cmd sgdisk -n 3:0:+512MiB -t 3:8300 -c 3:SLOT_B "$IMAGE_PATH"
    run_cmd sgdisk -n 4:0:+512MiB -t 4:8300 -c 4:SLOT_C "$IMAGE_PATH"
    run_cmd sgdisk -n 5:0:+512MiB -t 5:8300 -c 5:CONFIG "$IMAGE_PATH"
    run_cmd sgdisk -n 6:0:+1GiB -t 6:8300 -c 6:STRAT_CACHE "$IMAGE_PATH"
    run_cmd sgdisk -n 7:0:+1GiB -t 7:8300 -c 7:HOME "$IMAGE_PATH"
else
    run_cmd sgdisk -n 2:0:+20GiB -t 2:8300 -c 2:SLOT_A "$IMAGE_PATH"
    run_cmd sgdisk -n 3:0:+20GiB -t 3:8300 -c 3:SLOT_B "$IMAGE_PATH"
    run_cmd sgdisk -n 4:0:+20GiB -t 4:8300 -c 4:SLOT_C "$IMAGE_PATH"
    run_cmd sgdisk -n 5:0:+4GiB -t 5:8300 -c 5:CONFIG "$IMAGE_PATH"
    run_cmd sgdisk -n 6:0:+50GiB -t 6:8300 -c 6:STRAT_CACHE "$IMAGE_PATH"
    run_cmd sgdisk -n 7:0:0 -t 7:8300 -c 7:HOME "$IMAGE_PATH"
fi

echo "Note: this script only creates the GPT layout."
echo "Filesystem formatting and mount validation are handled in Phase 1."

echo "Partition table:"
run_cmd sgdisk -p "$IMAGE_PATH"
