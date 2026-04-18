#!/bin/bash
# Full StratOS build script - builds everything and runs QEMU
# Usage: ./scripts/build-all-and-run.sh [--clean] [--skip-kernel]

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase7"
LOG_FILE="$OUT_DIR/qemu_strattest.log"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

CLEAN_BUILD=0
SKIP_KERNEL=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --clean|-c)
            CLEAN_BUILD=1
            shift
            ;;
        --skip-kernel|-k)
            SKIP_KERNEL=1
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [--clean] [--skip-kernel]"
            echo "  --clean        Clean build (rebuild all from scratch)"
            echo "  --skip-kernel  Skip kernel rebuild (faster)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Kill existing QEMU
if pgrep -x "qemu-system-x86_64" > /dev/null 2>&1; then
    log_warn "Stopping existing QEMU..."
    pkill -x "qemu-system-x86_64" 2>/dev/null || true
    sleep 2
fi

# Build kernel if requested
if [ $SKIP_KERNEL -eq 0 ]; then
    log_info "Building kernel..."
    cd "$REPO_ROOT/stratos-kernel"
    if [ $CLEAN_BUILD -eq 1 ]; then
        make clean 2>/dev/null || true
    fi
    make -j$(nproc)
    cp arch/x86/boot/bzImage "$REPO_ROOT/out/phase4/vmlinuz"
    log_ok "Kernel built"
else
    log_info "Skipping kernel build (--skip-kernel)"
fi

# Build stratboot (EFI)
log_info "Building stratboot EFI..."
cd "$REPO_ROOT/stratboot"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release
cp target/x86_64-unknown-uefi/release/stratboot.efi "$REPO_ROOT/out/phase3/BOOTX64.EFI"
log_ok "stratboot built"

# Build stratvm
log_info "Building stratvm..."
cd "$REPO_ROOT/stratvm"
if [ $CLEAN_BUILD -eq 1 ]; then
    make clean
fi
make -j$(nproc)
log_ok "stratvm built"

# Build stratpanel
log_info "Building stratpanel..."
cd "$REPO_ROOT/stratpanel"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release
log_ok "stratpanel built"

# Build stratterm
log_info "Building stratterm..."
cd "$REPO_ROOT/stratterm"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release
log_ok "stratterm built"

# Build stratman
log_info "Building stratman..."
cd "$REPO_ROOT/stratman"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release
log_ok "stratman built"

# Build stratsup
log_info "Building stratsup..."
cd "$REPO_ROOT/stratsup"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release
log_ok "stratsup built"

# Build sysroot C code (initramfs-init)
log_info "Building sysroot C components..."
cd "$REPO_ROOT/sysroot"
make clean 2>/dev/null || true
make
log_ok "sysroot components built"

# Prepare minimal rootfs
log_info "Preparing minimal rootfs..."
"$REPO_ROOT/scripts/phase7/prepare-minimal-rootfs.sh"
log_ok "Rootfs prepared"

# Copy all binaries to rootfs
cp "$REPO_ROOT/stratvm/stratwm" "$OUT_DIR/rootfs-minimal/bin/"
cp "$REPO_ROOT/stratpanel/target/release/stratpanel" "$OUT_DIR/rootfs-minimal/bin/"
cp "$REPO_ROOT/stratterm/target/release/stratterm" "$OUT_DIR/rootfs-minimal/bin/"
cp "$REPO_ROOT/stratman/target/release/stratman" "$OUT_DIR/rootfs-minimal/bin/"
cp "$REPO_ROOT/stratsup/target/release/stratsup" "$OUT_DIR/rootfs-minimal/bin/"
log_ok "Binaries copied to rootfs"

# Build initramfs
log_info "Building initramfs..."
"$REPO_ROOT/scripts/phase7/build-initramfs.sh" \
    --init-mode auto \
    --output "$OUT_DIR/initramfs.cpio.gz"
log_ok "Initramfs built"

# Build EROFS slot image
log_info "Building EROFS slot image..."
cd "$OUT_DIR"
rm -f slot-system.erofs
mkfs.erofs -zlz4hc slot-system.erofs rootfs-minimal
log_ok "EROFs slot image built"

# Update test disk
log_info "Updating test disk..."
"$REPO_ROOT/scripts/phase7/update-test-disk-slot-a.sh"
log_ok "Test disk updated"

# Run QEMU
log_info "Starting QEMU..."
log_info "Logging to: $LOG_FILE"
"$REPO_ROOT/scripts/phase7/run-qemu-desktop.sh" 2>&1 | tee "$LOG_FILE"
