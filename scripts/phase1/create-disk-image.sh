#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
PROJECT_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)"
OUT_DIR="${OUT_DIR:-$PROJECT_ROOT/out}"
IMAGE_NAME="stratos-disk.raw"
IMAGE_PATH="$OUT_DIR/$IMAGE_NAME"
SIZE_GB="${SIZE_GB:-20}"

FORCE=0

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
        --force)
            FORCE=1
            shift
            ;;
        --out-dir)
            OUT_DIR="$2"
            IMAGE_PATH="$OUT_DIR/$IMAGE_NAME"
            shift 2
            ;;
        *)
            echo "Unknown argument: $1" >&2
            echo "Usage: $0 [--image PATH] [--size-gb N] [--force] [--out-dir PATH]" >&2
            exit 1
            ;;
    esac
done

. "$SCRIPT_DIR/lib.sh"

verify_prereqs() {
    phase1_detect_run_context qemu-img sgdisk
    
    if ! phase1_run_cmd sh -lc 'command -v qemu-img >/dev/null 2>&1'; then
        echo "Error: qemu-img is required but not found." >&2
        exit 1
    fi
    
    if ! phase1_run_cmd sh -lc 'command -v sgdisk >/dev/null 2>&1'; then
        echo "Error: sgdisk is required but not found." >&2
        exit 1
    fi
    
    echo "Prerequisites verified (qemu-img, sgdisk)"
}

ensure_out_dir() {
    IMAGE_DIR="$(dirname "$IMAGE_PATH")"
    phase1_run_cmd mkdir -p "$IMAGE_DIR"
}

check_existing_image() {
    if phase1_run_cmd sh -lc "[ -e '$IMAGE_PATH' ]"; then
        if [ "$FORCE" -ne 1 ]; then
            echo "Error: Image already exists: $IMAGE_PATH (use --force to overwrite)" >&2
            exit 1
        fi
        echo "Removing existing image: $IMAGE_PATH"
        phase1_run_cmd rm -f "$IMAGE_PATH"
    fi
}

create_raw_image() {
    echo "Creating raw disk image: $IMAGE_PATH (${SIZE_GB}G)"
    phase1_run_cmd qemu-img create -f raw "$IMAGE_PATH" "${SIZE_GB}G"
}

apply_gpt_layout() {
    echo "Applying StratOS GPT partition layout..."
    
    # Zap any existing partition table
    phase1_run_cmd sgdisk --zap-all "$IMAGE_PATH"
    
    # Create new GPT
    phase1_run_cmd sgdisk -o "$IMAGE_PATH"
    
    # ESP (EFI System Partition) - 256MiB, type EF00
    phase1_run_cmd sgdisk -n 1:1MiB:+256MiB -t 1:EF00 -c 1:ESP "$IMAGE_PATH"

    # SLOT_A - 4GiB, type 8300 (Linux filesystem)
    phase1_run_cmd sgdisk -n 2:0:+4GiB -t 2:8300 -c 2:SLOT_A "$IMAGE_PATH"

    # SLOT_B - 4GiB, type 8300
    phase1_run_cmd sgdisk -n 3:0:+4GiB -t 3:8300 -c 3:SLOT_B "$IMAGE_PATH"

    # SLOT_C - 4GiB, type 8300
    phase1_run_cmd sgdisk -n 4:0:+4GiB -t 4:8300 -c 4:SLOT_C "$IMAGE_PATH"

    # CONFIG - 1GiB, type 8300
    phase1_run_cmd sgdisk -n 5:0:+1GiB -t 5:8300 -c 5:CONFIG "$IMAGE_PATH"

    # HOME - Remaining space, type 8300
    phase1_run_cmd sgdisk -n 6:0:0 -t 6:8300 -c 6:HOME "$IMAGE_PATH"
}

print_partition_table() {
    echo ""
    echo "=== GPT Partition Table ==="
    phase1_run_cmd sgdisk -p "$IMAGE_PATH"
}

print_summary() {
    echo ""
    echo "=== Phase 1 Disk Image Creation Complete ==="
    echo "Image path: $IMAGE_PATH"
    echo "Size: ${SIZE_GB}G"
    echo ""
    echo "Partition layout:"
    echo "  1: ESP (256MiB) - EFI System Partition"
    echo "  2: SLOT_A (4GiB) - System slot A"
    echo "  3: SLOT_B (4GiB) - System slot B"
    echo "  4: SLOT_C (4GiB) - System slot C"
    echo "  5: CONFIG (1GiB) - Persistent configuration"
    echo "  6: HOME (remaining) - User home directory"
}

main() {
    echo "=== StratOS Phase 1: Disk Image Layout ==="
    
    verify_prereqs
    ensure_out_dir
    check_existing_image
    create_raw_image
    apply_gpt_layout
    print_partition_table
    print_summary
}

main "$@"
