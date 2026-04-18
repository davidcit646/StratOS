#!/bin/bash
# Full StratOS build script - builds everything inline and runs QEMU
# Usage: ./scripts/build-all-and-run.sh [--clean] [--skip-kernel]

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase7"
PHASE4_DIR="$REPO_ROOT/out/phase4"
PHASE3_DIR="$REPO_ROOT/out/phase3"
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

# ============================================================================
# 1. BUILD KERNEL
# ============================================================================
if [ $SKIP_KERNEL -eq 0 ]; then
    log_info "Building kernel..."
    KERNEL_SRC="$REPO_ROOT/linux"
    KERNEL_CONFIG="$REPO_ROOT/stratos-kernel/stratos.config"
    
    if [ ! -d "$KERNEL_SRC" ] || [ ! -f "$KERNEL_SRC/Makefile" ]; then
        log_error "Kernel source not found at $KERNEL_SRC"
        exit 1
    fi
    
    # Detect GCC version for compatibility
    EXTRA_MAKE_ARGS=""
    CC_PATH="${CC:-cc}"
    if command -v "$CC_PATH" >/dev/null 2>&1; then
        CC_VERSION="$($CC_PATH -dumpfullversion -dumpversion 2>/dev/null || true)"
        CC_MAJOR="${CC_VERSION%%.*}"
        if [ "$CC_MAJOR" -ge 15 ] 2>/dev/null; then
            CC_SHIM_DIR="$(mktemp -d)"
            printf '#!/usr/bin/env sh\nexec "%s" -std=gnu11 "$@"\n' "$(command -v "$CC_PATH")" > "$CC_SHIM_DIR/cc"
            chmod +x "$CC_SHIM_DIR/cc"
            EXTRA_MAKE_ARGS="CONFIG_WERROR=n CC=$CC_SHIM_DIR/cc"
            echo "Detected GCC $CC_MAJOR; applying compatibility args" >&2
        fi
    fi
    
    # Ensure bc is available
    if ! command -v bc >/dev/null 2>&1 && [ -x /run/host/usr/bin/bc ]; then
        BC_SHIM_DIR="$(mktemp -d)"
        ln -s /run/host/usr/bin/bc "$BC_SHIM_DIR/bc"
        export PATH="$BC_SHIM_DIR:$PATH"
    fi
    
    JOBS=$(nproc 2>/dev/null || echo 4)
    
    (
        cd "$KERNEL_SRC"
        if [ $CLEAN_BUILD -eq 1 ]; then
            make clean 2>/dev/null || true
        fi
        make defconfig
        ./scripts/kconfig/merge_config.sh -m .config "$KERNEL_CONFIG"
        make $EXTRA_MAKE_ARGS olddefconfig
        make $EXTRA_MAKE_ARGS -j"$JOBS" bzImage
    )
    
    cp "$KERNEL_SRC/arch/x86/boot/bzImage" "$PHASE4_DIR/vmlinuz"
    if [ -f "$KERNEL_SRC/System.map" ]; then
        cp "$KERNEL_SRC/System.map" "$PHASE4_DIR/"
    fi
    cp "$KERNEL_SRC/.config" "$PHASE4_DIR/.config.used"
    
    # Cleanup temp dirs
    [ -n "$CC_SHIM_DIR" ] && rm -rf "$CC_SHIM_DIR" 2>/dev/null || true
    [ -n "$BC_SHIM_DIR" ] && rm -rf "$BC_SHIM_DIR" 2>/dev/null || true
    
    log_ok "Kernel built -> $PHASE4_DIR/vmlinuz"
else
    log_info "Skipping kernel build (--skip-kernel)"
fi

# ============================================================================
# 2. BUILD STRATBOOT (EFI)
# ============================================================================
log_info "Building stratboot EFI..."
cd "$REPO_ROOT/stratboot"
if [ $CLEAN_BUILD -eq 1 ]; then
    make clean
fi
make
cp BOOTX64.EFI "$PHASE3_DIR/BOOTX64.EFI"
log_ok "stratboot built -> $PHASE3_DIR/BOOTX64.EFI"

# ============================================================================
# 3. BUILD STRATVM (COMPOSITOR)
# ============================================================================
log_info "Building stratvm..."
cd "$REPO_ROOT/stratvm"
if [ $CLEAN_BUILD -eq 1 ]; then
    make clean
fi
make -j$(nproc)
log_ok "stratvm built"

# ============================================================================
# 4. BUILD STRATPANEL
# ============================================================================
log_info "Building stratpanel..."
cd "$REPO_ROOT/stratpanel"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release
log_ok "stratpanel built"

# ============================================================================
# 5. BUILD STRATTERM
# ============================================================================
log_info "Building stratterm..."
cd "$REPO_ROOT/stratterm"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release
log_ok "stratterm built"

# ============================================================================
# 6. BUILD STRATMAN
# ============================================================================
log_info "Building stratman..."
cd "$REPO_ROOT/stratman"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release
log_ok "stratman built"

# ============================================================================
# 7. BUILD STRATSUP
# ============================================================================
log_info "Building stratsup..."
cd "$REPO_ROOT/stratsup"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release
log_ok "stratsup built"

# ============================================================================
# 8. BUILD SYSROOT (initramfs-init C code)
# ============================================================================
log_info "Building sysroot C components..."
cd "$REPO_ROOT/sysroot"
if [ $CLEAN_BUILD -eq 1 ]; then
    make clean 2>/dev/null || true
fi
make
log_ok "sysroot components built"

# ============================================================================
# 9. BUILD INITRAMFS
# ============================================================================
log_info "Building initramfs..."
INITRAMFS_ROOT="$OUT_DIR/initramfs-root"
INITRAMFS_OUT="$OUT_DIR/initramfs.cpio.gz"
INIT_SOURCE_C="$REPO_ROOT/sysroot/initramfs-init.c"

rm -rf "$INITRAMFS_ROOT"
mkdir -p "$INITRAMFS_ROOT"/{bin,sbin,proc,sys,dev,system,config,apps,home,var,run,usr}

# Build static init
gcc -Os -static -s -Wall -Wextra -o "$INITRAMFS_ROOT/init" "$INIT_SOURCE_C"
chmod 0755 "$INITRAMFS_ROOT/init"

# Create initramfs archive
(
    cd "$INITRAMFS_ROOT"
    find . -print | cpio -o -H newc --owner=0:0 2>/dev/null | gzip -9 > "$INITRAMFS_OUT"
)

if [ ! -s "$INITRAMFS_OUT" ]; then
    log_error "Failed to produce initramfs"
    exit 1
fi
log_ok "Initramfs built -> $INITRAMFS_OUT"

# ============================================================================
# 10. PREPARE MINIMAL ROOTFS
# ============================================================================
log_info "Preparing minimal rootfs..."
ROOTFS_DIR="$OUT_DIR/rootfs-minimal"
SYSTEM_INIT_SRC="$REPO_ROOT/sysroot/system-init.c"
FIRST_BOOT_SRC="$REPO_ROOT/sysroot/first-boot-provision.sh"

rm -rf "$ROOTFS_DIR"
mkdir -p "$ROOTFS_DIR"/{bin,sbin,lib,lib64,etc,dev,proc,sys,run,tmp,var,home,usr/{bin,sbin,lib,lib64},boot,system,config,apps}

# Create essential device nodes
mknod -m 666 "$ROOTFS_DIR/dev/null" c 1 3 2>/dev/null || true
mknod -m 666 "$ROOTFS_DIR/dev/zero" c 1 5 2>/dev/null || true
mknod -m 666 "$ROOTFS_DIR/dev/random" c 1 8 2>/dev/null || true
mknod -m 666 "$ROOTFS_DIR/dev/urandom" c 1 9 2>/dev/null || true
mknod -m 666 "$ROOTFS_DIR/dev/tty" c 5 0 2>/dev/null || true

# Copy binaries
cp "$REPO_ROOT/stratvm/stratwm" "$ROOTFS_DIR/bin/"
cp "$REPO_ROOT/stratpanel/target/release/stratpanel" "$ROOTFS_DIR/bin/"
cp "$REPO_ROOT/stratterm/target/release/stratterm" "$ROOTFS_DIR/bin/"
cp "$REPO_ROOT/stratman/target/release/stratman" "$ROOTFS_DIR/bin/"
cp "$REPO_ROOT/stratsup/target/x86_64-unknown-linux-gnu/release/stratsup" "$ROOTFS_DIR/bin/" 2>/dev/null || \
    cp "$REPO_ROOT/stratsup/target/release/stratsup" "$ROOTFS_DIR/bin/" 2>/dev/null || true
cp "$REPO_ROOT/stratterm/target/release/stratterm-indexer" "$ROOTFS_DIR/bin/" 2>/dev/null || true
cp "$REPO_ROOT/stratterm/target/release/strat-settings" "$ROOTFS_DIR/bin/" 2>/dev/null || true

# Build and copy system-init
gcc -Os -static -s -Wall -Wextra -o "$ROOTFS_DIR/sbin/system-init" "$SYSTEM_INIT_SRC" 2>/dev/null || \
    gcc -Os -Wall -Wextra -o "$ROOTFS_DIR/sbin/system-init" "$SYSTEM_INIT_SRC"
chmod 0755 "$ROOTFS_DIR/sbin/system-init"

# Copy first-boot script
cp "$FIRST_BOOT_SRC" "$ROOTFS_DIR/sbin/first-boot-provision.sh"
chmod 0755 "$ROOTFS_DIR/sbin/first-boot-provision.sh"

# Copy shell and essential tools
if [ -x /bin/busybox ]; then
    cp /bin/busybox "$ROOTFS_DIR/bin/"
    for applet in sh ls cat cp mv rm mkdir rmdir ps kill; do
        ln -sf busybox "$ROOTFS_DIR/bin/$applet"
    done
elif [ -x /bin/bash ]; then
    cp /bin/bash "$ROOTFS_DIR/bin/"
    ln -sf bash "$ROOTFS_DIR/bin/sh"
else
    ln -sf /bin/sh "$ROOTFS_DIR/bin/sh" 2>/dev/null || true
fi

# Copy foot terminal if available
if [ -x /usr/bin/foot ]; then
    cp /usr/bin/foot "$ROOTFS_DIR/bin/"
fi

# Copy seatd if available
if [ -x "$REPO_ROOT/third_party/seatd/build/seatd" ]; then
    cp "$REPO_ROOT/third_party/seatd/build/seatd" "$ROOTFS_DIR/bin/"
elif [ -x /usr/sbin/seatd ]; then
    cp /usr/sbin/seatd "$ROOTFS_DIR/bin/"
fi

# Copy essential libraries
for libdir in /usr/lib64 /usr/lib /lib64 /lib; do
    if [ -d "$libdir" ]; then
        for lib in "$libdir"/*.so*; do
            if [ -f "$lib" ] && [ ! -L "$lib" ]; then
                cp -L "$lib" "$ROOTFS_DIR/lib64/" 2>/dev/null || true
            fi
        done
    fi
done

# Copy ld-linux
for ldso in /lib64/ld-linux-x86-64.so.2 /usr/lib64/ld-linux-x86-64.so.2 /lib/x86_64-linux-gnu/ld-linux-x86-64.so.2; do
    if [ -f "$ldso" ]; then
        cp -L "$ldso" "$ROOTFS_DIR/lib64/" 2>/dev/null || true
        break
    fi
done

# Copy key system libraries
for lib in libc.so libm.so libdl.so libpthread.so librt.so; do
    for libdir in /usr/lib64 /lib64 /usr/lib /lib; do
        if [ -f "$libdir/$lib" ]; then
            cp -L "$libdir/$lib" "$ROOTFS_DIR/lib64/" 2>/dev/null || true
            break
        fi
    done
done

# Copy Wayland and graphics libraries
for lib in libwayland-server.so libwayland-client.so libwlroots-*.so libpixman-1.so libxkbcommon.so libinput.so libevdev.so libdrm.so libEGL.so libgbm.so libGLESv2.so libvulkan.so libseat.so libudev.so; do
    for libdir in /usr/lib64 /lib64 /usr/lib /lib; do
        for match in "$libdir"/$lib*; do
            if [ -f "$match" ] && [ ! -L "$match" ]; then
                cp -L "$match" "$ROOTFS_DIR/lib64/" 2>/dev/null || true
            fi
        done
    done
done

# Copy fontconfig and related libs
for lib in libfontconfig.so libfreetype.so libpng.so libz.so libexpat.so libbrotli*.so; do
    for libdir in /usr/lib64 /lib64 /usr/lib /lib; do
        for match in "$libdir"/$lib*; do
            if [ -f "$match" ] && [ ! -L "$match" ]; then
                cp -L "$match" "$ROOTFS_DIR/lib64/" 2>/dev/null || true
            fi
        done
    done
done

# Copy terminfo
mkdir -p "$ROOTFS_DIR/usr/share/terminfo"
cp -r /usr/share/terminfo/x "$ROOTFS_DIR/usr/share/terminfo/" 2>/dev/null || true

# Copy fonts
mkdir -p "$ROOTFS_DIR/usr/share/fonts"
cp -r /usr/share/fonts/* "$ROOTFS_DIR/usr/share/fonts/" 2>/dev/null || true

# Create ld.so.conf
mkdir -p "$ROOTFS_DIR/etc"
echo "/lib64" > "$ROOTFS_DIR/etc/ld.so.conf"
echo "/usr/lib64" >> "$ROOTFS_DIR/etc/ld.so.conf"
echo "/lib" >> "$ROOTFS_DIR/etc/ld.so.conf"
echo "/usr/lib" >> "$ROOTFS_DIR/etc/ld.so.conf"

# Create passwd file for foot terminal
echo "root:x:0:0:root:/root:/bin/sh" > "$ROOTFS_DIR/etc/passwd"
mkdir -p "$ROOTFS_DIR/root"

log_ok "Rootfs prepared -> $ROOTFS_DIR"

# ============================================================================
# 11. BUILD EROFS SYSTEM IMAGE
# ============================================================================
log_info "Building EROFS system image..."
cd "$OUT_DIR"
rm -f slot-system.erofs
mkfs.erofs -zlz4hc slot-system.erofs rootfs-minimal
log_ok "EROFs image built -> $OUT_DIR/slot-system.erofs"

# ============================================================================
# 12. UPDATE TEST DISK
# ============================================================================
log_info "Updating test disk..."

DISK_IMAGE="$PHASE4_DIR/test-disk.img"
SLOT_A_EROFS="$OUT_DIR/slot-system.erofs"
BOOT_EFI="$PHASE3_DIR/BOOTX64.EFI"
KERNEL_EFI="$PHASE4_DIR/vmlinuz"
INITRD_IMG="$OUT_DIR/initramfs.cpio.gz"

if [ ! -f "$DISK_IMAGE" ]; then
    log_warn "Test disk not found, creating..."
    "$REPO_ROOT/scripts/phase4/create-test-disk.sh"
fi

# Get partition offsets
read_part_field() { part=$1; field=$2; echo "$(parted -sm "$DISK_IMAGE" unit B print 2>/dev/null | grep "^$part:" | cut -d: -f$field | tr -d 'B')"; }

PART1_START=$(read_part_field 1 2)
PART2_START=$(read_part_field 2 2)

if [ -z "$PART1_START" ] || [ -z "$PART2_START" ]; then
    log_error "Failed to parse partition table"
    exit 1
fi

PART1_START_LBA=$((PART1_START / 512))
PART2_START_LBA=$((PART2_START / 512))

# Update slot A (EROFS)
log_info "Writing EROFS to slot A (offset $PART2_START_LBA)..."
dd if="$SLOT_A_EROFS" of="$DISK_IMAGE" bs=512 seek="$PART2_START_LBA" conv=notrunc status=none

# Update ESP with fresh assets
ESP_TEMP=$(mktemp)
log_info "Updating ESP..."
dd if="$DISK_IMAGE" of="$ESP_TEMP" bs=512 skip="$PART1_START_LBA" count=262144 status=none 2>/dev/null || true

# Create fresh ESP
ESP_SIZE_MB=128
ESP_SIZE_BYTES=$((ESP_SIZE_MB * 1024 * 1024))
rm -f "$ESP_TEMP"
mkfs.vfat -F 32 -C "$ESP_TEMP" $((ESP_SIZE_BYTES / 1024)) 2>/dev/null || mkfs.vfat -F 32 "$ESP_TEMP" 2>/dev/null || true

# Copy files to ESP
if command -v mmd >/dev/null 2>&1 && command -v mcopy >/dev/null 2>&1; then
    mmd -i "$ESP_TEMP" ::/EFI ::/EFI/BOOT ::/EFI/STRAT ::/EFI/STRAT/SLOT_A 2>/dev/null || true
    mcopy -i "$ESP_TEMP" "$BOOT_EFI" ::/EFI/BOOT/BOOTX64.EFI 2>/dev/null || true
    mcopy -i "$ESP_TEMP" "$KERNEL_EFI" ::/EFI/STRAT/SLOT_A/vmlinuz.efi 2>/dev/null || true
    mcopy -i "$ESP_TEMP" "$INITRD_IMG" ::/EFI/STRAT/SLOT_A/initramfs.img 2>/dev/null || true
    
    # Write ESP back
    dd if="$ESP_TEMP" of="$DISK_IMAGE" bs=512 seek="$PART1_START_LBA" conv=notrunc status=none
    log_ok "ESP updated with fresh assets"
else
    log_warn "mtools not available, skipping ESP update"
fi

rm -f "$ESP_TEMP"
log_ok "Test disk updated -> $DISK_IMAGE"

# ============================================================================
# 13. RUN QEMU
# ============================================================================
log_info "Starting QEMU..."
log_info "Logging to: $LOG_FILE"

QEMU_SCRIPT="$REPO_ROOT/scripts/phase7/run-qemu-desktop.sh"

if [ -x "$QEMU_SCRIPT" ]; then
    "$QEMU_SCRIPT" 2>&1 | tee "$LOG_FILE"
else
    log_error "QEMU script not found: $QEMU_SCRIPT"
    exit 1
fi
