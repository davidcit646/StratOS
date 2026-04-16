#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
PROJECT_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)"
OUT_DIR="${OUT_DIR:-$PROJECT_ROOT/out}"
SYSROOT="${SYSROOT:-$PROJECT_ROOT/sysroot}"
IMAGE_NAME="stratos-system.erofs"
IMAGE_PATH="$OUT_DIR/$IMAGE_NAME"

VERIFY_ONLY=0
FORCE=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --verify-only)
            VERIFY_ONLY=1
            shift
            ;;
        --force)
            FORCE=1
            shift
            ;;
        --sysroot)
            SYSROOT="$2"
            shift 2
            ;;
        --out-dir)
            OUT_DIR="$2"
            IMAGE_PATH="$OUT_DIR/$IMAGE_NAME"
            shift 2
            ;;
        *)
            echo "Unknown argument: $1" >&2
            echo "Usage: $0 [--verify-only] [--force] [--sysroot PATH] [--out-dir PATH]" >&2
            exit 1
            ;;
    esac
done

verify_mkfs_erofs() {
    if ! command -v mkfs.erofs >/dev/null 2>&1; then
        echo "Error: mkfs.erofs not found. Install erofs-utils." >&2
        exit 1
    fi
    echo "mkfs.erofs verified"
}

verify_sysroot_exists() {
    if [ ! -d "$SYSROOT" ]; then
        echo "Error: Sysroot directory not found: $SYSROOT" >&2
        exit 1
    fi
    echo "Sysroot verified: $SYSROOT"
}

verify_sysroot_content() {
    if [ -z "$(ls -A "$SYSROOT" 2>/dev/null)" ]; then
        echo "Error: Sysroot directory is empty: $SYSROOT" >&2
        exit 1
    fi
    echo "Sysroot content verified"
}

ensure_out_dir() {
    mkdir -p "$OUT_DIR"
}

check_existing_image() {
    if [ -f "$IMAGE_PATH" ]; then
        if [ "$FORCE" -ne 1 ]; then
            echo "Error: Image already exists: $IMAGE_PATH (use --force to overwrite)" >&2
            exit 1
        fi
        echo "Removing existing image: $IMAGE_PATH"
        rm -f "$IMAGE_PATH"
    fi
}

build_erofs_image() {
    echo "Building EROFS system image..."
    echo "  Source: $SYSROOT"
    echo "  Output: $IMAGE_PATH"
    
    mkfs.erofs -z lz4hc,9 "$IMAGE_PATH" "$SYSROOT"
    
    if [ ! -f "$IMAGE_PATH" ]; then
        echo "Error: Failed to create EROFS image: $IMAGE_PATH" >&2
        exit 1
    fi
    
    image_size=$(stat -c%s "$IMAGE_PATH" 2>/dev/null || stat -f%z "$IMAGE_PATH" 2>/dev/null)
    echo "EROFS image created successfully"
    echo "  Size: $image_size bytes"
}

print_image_info() {
    if [ -f "$IMAGE_PATH" ]; then
        echo "=== EROFS System Image Info ==="
        echo "Path: $IMAGE_PATH"
        ls -lh "$IMAGE_PATH"
    fi
}

main() {
    echo "=== StratOS Phase 1: EROFS System Image Build ==="
    
    verify_mkfs_erofs
    verify_sysroot_exists
    verify_sysroot_content
    
    if [ "$VERIFY_ONLY" -eq 1 ]; then
        echo "Verification complete. No image built."
        exit 0
    fi
    
    ensure_out_dir
    check_existing_image
    build_erofs_image
    print_image_info
    
    echo "=== Phase 1 EROFS System Image Build Complete ==="
}

main "$@"
