#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase7"
INITRAMFS_OUT="$OUT_DIR/initramfs.cpio.gz"
ROOTFS_DIR="$OUT_DIR/rootfs-minimal"
SLOT_OUT="$OUT_DIR/slot-system.erofs"
INIT_MODE="${INIT_MODE:-auto}"
RUN_SMOKE=0
SMOKE_SECONDS="${SMOKE_SECONDS:-20}"
BUILD_STRATTERM_INDEXER="${BUILD_STRATTERM_INDEXER:-1}"

usage() {
    cat <<USAGE
Usage: $0 [--init-mode auto|busybox|static] [--smoke] [--smoke-seconds N]

Builds Phase 7 local artifacts:
  1) initramfs.cpio.gz
  2) rootfs-minimal (assembled tree)
  3) slot-system.erofs

Optional:
  --smoke            Run Phase 7 QEMU smoke test after builds complete
  --smoke-seconds N  Duration for smoke run (default: 20)
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --init-mode)
            INIT_MODE="$2"
            shift 2
            ;;
        --smoke)
            RUN_SMOKE=1
            shift
            ;;
        --smoke-seconds)
            SMOKE_SECONDS="$2"
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

case "$INIT_MODE" in
    auto|busybox|static)
        ;;
    *)
        echo "Invalid init mode: $INIT_MODE" >&2
        exit 1
        ;;
esac

case "$SMOKE_SECONDS" in
    ''|*[!0-9]*)
        echo "Invalid --smoke-seconds value: $SMOKE_SECONDS" >&2
        exit 1
        ;;
esac
if [ "$SMOKE_SECONDS" -le 0 ]; then
    echo "--smoke-seconds must be > 0" >&2
    exit 1
fi

"$REPO_ROOT/scripts/phase7/build-initramfs.sh" \
    --init-mode "$INIT_MODE" \
    --output "$INITRAMFS_OUT"

if [ "$BUILD_STRATTERM_INDEXER" = "1" ]; then
    cargo build --release --manifest-path "$REPO_ROOT/stratterm/Cargo.toml"
fi

"$REPO_ROOT/scripts/phase7/prepare-minimal-rootfs.sh" \
    --rootfs-dir "$ROOTFS_DIR"

"$REPO_ROOT/scripts/phase7/build-slot-erofs.sh" \
    --rootfs "$ROOTFS_DIR" \
    --output "$SLOT_OUT"

if [ "$RUN_SMOKE" -eq 1 ]; then
    "$REPO_ROOT/scripts/phase7/run-qemu-phase7-smoke.sh" \
        --seconds "$SMOKE_SECONDS"
fi

printf '%s\n%s\n' "$INITRAMFS_OUT" "$SLOT_OUT"
