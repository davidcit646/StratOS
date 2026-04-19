#!/bin/bash
# Full StratOS build script — kernel, userspace, rootfs, EROFS, test-disk refresh.
# Usage: ./build-all-and-run.sh [--clean] [--skip-kernel] [--recreate-disk]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT_DIR="$REPO_ROOT/out/phase7"
PHASE4_DIR="$REPO_ROOT/out/phase4"
PHASE3_DIR="$REPO_ROOT/out/phase3"
IDE_LOG_DIR="$REPO_ROOT/ide-logs"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

CLEAN_BUILD=0
SKIP_KERNEL=0
RECREATE_DISK=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --clean|-c)
            CLEAN_BUILD=1
            shift
            ;;
        -s|--skip-kernel)
            SKIP_KERNEL=1
            shift
            ;;
        --recreate-disk)
            RECREATE_DISK=1
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [-c] [-s] [--recreate-disk]"
            echo "  -c, --clean        Clean build (rebuild all from scratch)"
            echo "  -s, --skip-kernel  Skip kernel rebuild (faster)"
            echo "  --recreate-disk    Recreate GPT test disk before update"
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
require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        log_error "Missing required tool: $1"
        exit 1
    fi
}

if [ ! -d "$IDE_LOG_DIR" ]; then
    mkdir -p "$IDE_LOG_DIR"
fi
if [ ! -d "$OUT_DIR" ]; then
    mkdir -p "$OUT_DIR"
fi
if [ ! -d "$PHASE4_DIR" ]; then
    mkdir -p "$PHASE4_DIR"
fi
if [ ! -d "$PHASE3_DIR" ]; then
    mkdir -p "$PHASE3_DIR"
fi

require_cmd make
require_cmd gcc
require_cmd cargo
require_cmd cpio
require_cmd gzip
require_cmd mkfs.erofs

# ============================================================================
# 1. BUILD KERNEL
# ============================================================================
if [ $SKIP_KERNEL -eq 0 ]; then
    log_info "Building kernel..."
    KERNEL_SRC="$REPO_ROOT/linux"
    KERNEL_CONFIG="$REPO_ROOT/stratos-kernel/stratos.config"

    # Kernel build prerequisites (Kconfig lexer/parser generation)
    if ! command -v flex >/dev/null 2>&1; then
        log_error "Missing build tool: flex (required for Linux Kconfig)."
        log_error "Install flex (and usually bison) or re-run with --skip-kernel."
        exit 1
    fi
    if ! command -v bison >/dev/null 2>&1; then
        log_error "Missing build tool: bison (required for Linux Kconfig)."
        log_error "Install bison (and flex) or re-run with --skip-kernel."
        exit 1
    fi
    
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
    log_info "Skipping kernel build (-s)"
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
# 4b. BUILD STRATSETTINGS (strat-ui-config CLI + stratos-settings Wayland UI)
# ============================================================================
log_info "Building stratsettings..."
cd "$REPO_ROOT/stratsettings"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release --bins
log_ok "stratsettings built"

# ============================================================================
# 5. BUILD STRATTERM
# ============================================================================
log_info "Building stratterm..."
cd "$REPO_ROOT/stratterm"
if [ $CLEAN_BUILD -eq 1 ]; then
    cargo clean
fi
cargo build --release --bins
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
cargo build --release --bins
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
mkdir -p "$ROOTFS_DIR/config/strat/settings.d"
cp -f "$REPO_ROOT/stratsettings/defaults/settings.default.toml" "$ROOTFS_DIR/config/strat/settings.toml"
cp -f "$REPO_ROOT/stratsettings/defaults/stratvm-keybinds.default" "$ROOTFS_DIR/config/strat/stratvm-keybinds"
if [ -f "$REPO_ROOT/stratterm/indexer.conf.example" ]; then
    cp -f "$REPO_ROOT/stratterm/indexer.conf.example" "$ROOTFS_DIR/config/strat/indexer.conf"
fi
if [ -f "$REPO_ROOT/stratsettings/defaults/wpa_supplicant.default.conf" ]; then
    cp -f "$REPO_ROOT/stratsettings/defaults/wpa_supplicant.default.conf" "$ROOTFS_DIR/config/strat/wpa_supplicant.conf"
fi

# Create essential device nodes
mknod -m 666 "$ROOTFS_DIR/dev/null" c 1 3 2>/dev/null || true
mknod -m 666 "$ROOTFS_DIR/dev/zero" c 1 5 2>/dev/null || true
mknod -m 666 "$ROOTFS_DIR/dev/random" c 1 8 2>/dev/null || true
mknod -m 666 "$ROOTFS_DIR/dev/urandom" c 1 9 2>/dev/null || true
mknod -m 666 "$ROOTFS_DIR/dev/tty" c 5 0 2>/dev/null || true

# Copy binaries
cp "$REPO_ROOT/stratvm/stratwm" "$ROOTFS_DIR/bin/"
cp "$REPO_ROOT/stratpanel/target/release/stratpanel" "$ROOTFS_DIR/bin/"
cp "$REPO_ROOT/stratsettings/target/release/stratos-settings" "$ROOTFS_DIR/bin/" 2>/dev/null || true
cp "$REPO_ROOT/stratsettings/target/release/strat-ui-config" "$ROOTFS_DIR/bin/" 2>/dev/null || true
cp "$REPO_ROOT/stratterm/target/release/stratterm" "$ROOTFS_DIR/bin/"
cp "$REPO_ROOT/stratterm/target/release/spotlite" "$ROOTFS_DIR/bin/" 2>/dev/null || true
cp "$REPO_ROOT/stratman/target/release/stratman" "$ROOTFS_DIR/bin/"
cp "$REPO_ROOT/stratsup/target/x86_64-unknown-linux-gnu/release/stratsup" "$ROOTFS_DIR/bin/" 2>/dev/null || \
    cp "$REPO_ROOT/stratsup/target/release/stratsup" "$ROOTFS_DIR/bin/" 2>/dev/null || true
cp "$REPO_ROOT/stratterm/target/release/stratterm-indexer" "$ROOTFS_DIR/bin/" 2>/dev/null || true
cp "$REPO_ROOT/stratterm/target/release/strat-settings" "$ROOTFS_DIR/bin/" 2>/dev/null || true
if [ -x "$REPO_ROOT/stratsup/target/x86_64-unknown-linux-gnu/release/strat-validate-boot" ]; then
    cp "$REPO_ROOT/stratsup/target/x86_64-unknown-linux-gnu/release/strat-validate-boot" "$ROOTFS_DIR/bin/"
elif [ -x "$REPO_ROOT/stratsup/target/release/strat-validate-boot" ]; then
    cp "$REPO_ROOT/stratsup/target/release/strat-validate-boot" "$ROOTFS_DIR/bin/"
else
    log_warn "strat-validate-boot binary missing; installing fallback no-op helper"
    cat > "$ROOTFS_DIR/bin/strat-validate-boot" <<'EOF'
#!/bin/sh
echo "strat-validate-boot: fallback no-op"
exit 0
EOF
    chmod 0755 "$ROOTFS_DIR/bin/strat-validate-boot"
fi
if [ -f "$REPO_ROOT/sysroot/strat-indexer-boot.sh" ]; then
    cp "$REPO_ROOT/sysroot/strat-indexer-boot.sh" "$ROOTFS_DIR/bin/"
    chmod 0755 "$ROOTFS_DIR/bin/strat-indexer-boot.sh"
else
    log_warn "strat-indexer-boot.sh missing; installing fallback no-op helper"
    cat > "$ROOTFS_DIR/bin/strat-indexer-boot.sh" <<'EOF'
#!/bin/sh
echo "strat-indexer-boot: fallback no-op"
exit 0
EOF
    chmod 0755 "$ROOTFS_DIR/bin/strat-indexer-boot.sh"
fi

# Build and copy system-init
gcc -Os -static -s -Wall -Wextra -o "$ROOTFS_DIR/sbin/system-init" "$SYSTEM_INIT_SRC" 2>/dev/null || \
    gcc -Os -Wall -Wextra -o "$ROOTFS_DIR/sbin/system-init" "$SYSTEM_INIT_SRC"
chmod 0755 "$ROOTFS_DIR/sbin/system-init"

# Copy first-boot script
cp "$FIRST_BOOT_SRC" "$ROOTFS_DIR/sbin/first-boot-provision.sh"
chmod 0755 "$ROOTFS_DIR/sbin/first-boot-provision.sh"

# Disk installer (live session → bare metal); keep host tools in sync with strat-installer.sh
if [ -f "$REPO_ROOT/scripts/strat-installer.sh" ]; then
    cp "$REPO_ROOT/scripts/strat-installer.sh" "$ROOTFS_DIR/bin/strat-installer"
    chmod 0755 "$ROOTFS_DIR/bin/strat-installer"
fi
copy_host_tool_for_installer() {
    local name="$1"
    local src
    src=$(command -v "$name" 2>/dev/null) || return 0
    mkdir -p "$ROOTFS_DIR/usr/sbin"
    cp -L "$src" "$ROOTFS_DIR/usr/sbin/$name"
    if command -v ldd >/dev/null 2>&1 && [ -f "$src" ]; then
        ldd "$src" 2>/dev/null | awk '/=> \// {print $3}' | while read -r lib; do
            [ -n "$lib" ] && [ -f "$lib" ] && cp -L "$lib" "$ROOTFS_DIR/lib64/" 2>/dev/null || true
        done
    fi
}
for _t in sgdisk partprobe mkfs.vfat mkfs.ext4 mke2fs mkfs.btrfs blkid lsblk blockdev; do
    copy_host_tool_for_installer "$_t" || true
done

if [ -f "$REPO_ROOT/sysroot/strat-wpa.sh" ]; then
    cp "$REPO_ROOT/sysroot/strat-wpa.sh" "$ROOTFS_DIR/bin/strat-wpa.sh"
    chmod 0755 "$ROOTFS_DIR/bin/strat-wpa.sh"
fi
for _t in wpa_supplicant wpa_cli rfkill; do
    copy_host_tool_for_installer "$_t" || true
done

# Copy shell and essential tools
if [ -x /bin/busybox ]; then
    cp /bin/busybox "$ROOTFS_DIR/bin/"
    for applet in sh ls cat cp mv rm mkdir rmdir ps kill grep; do
        ln -sf busybox "$ROOTFS_DIR/bin/$applet"
    done
elif [ -x /bin/bash ]; then
    cp /bin/bash "$ROOTFS_DIR/bin/"
    ln -sf bash "$ROOTFS_DIR/bin/sh"
else
    ln -sf /bin/sh "$ROOTFS_DIR/bin/sh" 2>/dev/null || true
fi

# Copy seatd if available
if [ -x "$REPO_ROOT/third_party/seatd/build/seatd" ]; then
    cp "$REPO_ROOT/third_party/seatd/build/seatd" "$ROOTFS_DIR/bin/"
elif [ -x /usr/sbin/seatd ]; then
    cp /usr/sbin/seatd "$ROOTFS_DIR/bin/"
fi

# Copy essential libraries
for libdir in /usr/lib64 /lib64 /usr/lib/x86_64-linux-gnu /lib/x86_64-linux-gnu; do
    if [ -d "$libdir" ]; then
        for lib in "$libdir"/*.so*; do
            if [ -f "$lib" ]; then
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
    for libdir in /usr/lib64 /lib64 /usr/lib/x86_64-linux-gnu /lib/x86_64-linux-gnu; do
        if [ -f "$libdir/$lib" ]; then
            cp -L "$libdir/$lib" "$ROOTFS_DIR/lib64/" 2>/dev/null || true
            break
        fi
    done
done

# Ensure critical runtime SONAMEs exist (some hosts expose these as symlinks).
for soname in libgcc_s.so.1 libstdc++.so.6; do
    if [ -e "$ROOTFS_DIR/lib64/$soname" ]; then
        continue
    fi
    for libdir in /usr/lib64 /lib64 /usr/lib/x86_64-linux-gnu /lib/x86_64-linux-gnu; do
        if [ -e "$libdir/$soname" ]; then
            cp -L "$libdir/$soname" "$ROOTFS_DIR/lib64/" 2>/dev/null || true
            break
        fi
    done
done

# Copy Wayland and graphics libraries
for lib in libwayland-server.so libwayland-client.so libwlroots-*.so libpixman-1.so libxkbcommon.so libinput.so libevdev.so libdrm.so libEGL.so libgbm.so libGLESv2.so libvulkan.so libseat.so libudev.so; do
    for libdir in /usr/lib64 /lib64 /usr/lib/x86_64-linux-gnu /lib/x86_64-linux-gnu; do
        for match in "$libdir"/$lib*; do
            if [ -f "$match" ]; then
                cp -L "$match" "$ROOTFS_DIR/lib64/" 2>/dev/null || true
            fi
        done
    done
done

# Copy fontconfig and related libs
for lib in libfontconfig.so libfreetype.so libpng.so libz.so libexpat.so libbrotli*.so; do
    for libdir in /usr/lib64 /lib64 /usr/lib/x86_64-linux-gnu /lib/x86_64-linux-gnu; do
        for match in "$libdir"/$lib*; do
            if [ -f "$match" ]; then
                cp -L "$match" "$ROOTFS_DIR/lib64/" 2>/dev/null || true
            fi
        done
    done
done

# Copy terminfo
mkdir -p "$ROOTFS_DIR/usr/share/terminfo"
cp -r /usr/share/terminfo/x "$ROOTFS_DIR/usr/share/terminfo/" 2>/dev/null || true

# Copy fonts (bundled FOS set + optional host fonts/)
if [ -x "$REPO_ROOT/scripts/fetch-fos-fonts.sh" ]; then
    log_info "Bundled open fonts (scripts/fetch-fos-fonts.sh; skips existing files)"
    "$REPO_ROOT/scripts/fetch-fos-fonts.sh" || log_warn "fetch-fos-fonts.sh reported errors (offline?); continuing with any cached files"
fi
mkdir -p "$ROOTFS_DIR/usr/share/fonts"
cp -r /usr/share/fonts/* "$ROOTFS_DIR/usr/share/fonts/" 2>/dev/null || true
mkdir -p "$ROOTFS_DIR/usr/share/fonts/stratos"
shopt -s nullglob
_strat_fonts=( "$REPO_ROOT/fonts/stratos/"*.ttf "$REPO_ROOT/fonts/stratos/"*.otf "$REPO_ROOT/fonts/stratos/"*.ttc )
shopt -u nullglob
if [ "${#_strat_fonts[@]}" -gt 0 ]; then
    cp -f "${_strat_fonts[@]}" "$ROOTFS_DIR/usr/share/fonts/stratos/"
    log_ok "Copied ${#_strat_fonts[@]} bundled font files -> $ROOTFS_DIR/usr/share/fonts/stratos/"
else
    log_warn "No fonts under fonts/stratos (run ./scripts/fetch-fos-fonts.sh with network once)"
fi

# DMZ-White Xcursor theme (stratwm + clients; see scripts/fetch-dmz-cursor-theme.sh)
if [ -x "$REPO_ROOT/scripts/fetch-dmz-cursor-theme.sh" ]; then
    log_info "Cursor theme (scripts/fetch-dmz-cursor-theme.sh)"
    "$REPO_ROOT/scripts/fetch-dmz-cursor-theme.sh" || log_warn "fetch-dmz-cursor-theme.sh failed (offline?); continuing"
fi
mkdir -p "$ROOTFS_DIR/usr/share/icons"
if [ -d "$REPO_ROOT/icons/dmz-white" ]; then
    cp -a "$REPO_ROOT/icons/dmz-white" "$ROOTFS_DIR/usr/share/icons/"
    log_ok "Installed dmz-white cursor theme -> $ROOTFS_DIR/usr/share/icons/dmz-white"
fi

# Copy runtime data needed by libinput/xkb at boot.
mkdir -p "$ROOTFS_DIR/usr/share"
cp -aL /usr/share/libinput "$ROOTFS_DIR/usr/share/" 2>/dev/null || true
mkdir -p "$ROOTFS_DIR/usr/share/X11"
cp -aL /usr/share/X11/xkb "$ROOTFS_DIR/usr/share/X11/" 2>/dev/null || true

# Create ld.so.conf
mkdir -p "$ROOTFS_DIR/etc"
echo "/lib64" > "$ROOTFS_DIR/etc/ld.so.conf"
echo "/usr/lib64" >> "$ROOTFS_DIR/etc/ld.so.conf"
echo "/lib" >> "$ROOTFS_DIR/etc/ld.so.conf"
echo "/usr/lib" >> "$ROOTFS_DIR/etc/ld.so.conf"

# Create passwd file
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

if [ "$RECREATE_DISK" -eq 1 ]; then
    log_info "Recreating test disk..."
    "$REPO_ROOT/scripts/create-test-disk.sh"
elif [ ! -f "$DISK_IMAGE" ]; then
    log_warn "Test disk not found, creating..."
    "$REPO_ROOT/scripts/create-test-disk.sh"
fi

UPDATE_HELPER="$REPO_ROOT/scripts/update-test-disk.sh"
if [ ! -x "$UPDATE_HELPER" ]; then
    log_error "Missing update helper: $UPDATE_HELPER"
    exit 1
fi

"$UPDATE_HELPER" \
    --disk "$DISK_IMAGE" \
    --slot-a-erofs "$SLOT_A_EROFS" \
    --boot-efi "$BOOT_EFI" \
    --kernel "$KERNEL_EFI" \
    --initrd "$INITRD_IMG"

log_ok "Test disk updated -> $DISK_IMAGE"
log_ok "Build complete. Flash or attach $DISK_IMAGE on bare metal, or write out/live/stratos-live.iso to USB (see docs/human/live-iso.md)."
exit 0
