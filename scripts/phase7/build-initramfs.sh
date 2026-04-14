#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase7"
ROOTFS_DIR="$OUT_DIR/initramfs-root"
OUTPUT_IMAGE="$OUT_DIR/initramfs.cpio.gz"
BUSYBOX_BIN=""
INIT_MODE="auto"
INIT_SOURCE_SH="$REPO_ROOT/sysroot/initramfs-init"
INIT_SOURCE_C="$REPO_ROOT/sysroot/initramfs-init.c"

usage() {
    cat <<USAGE
Usage: $0 [--output PATH] [--rootfs-dir PATH] [--busybox PATH] [--init-mode MODE]

Builds a minimal initramfs archive.
Output: gzip-compressed newc cpio archive.

Modes:
  auto    Use busybox shell init when available, otherwise build static C init
  busybox Require busybox shell init mode
  static  Require static C init mode
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --output)
            OUTPUT_IMAGE="$2"
            shift 2
            ;;
        --rootfs-dir)
            ROOTFS_DIR="$2"
            shift 2
            ;;
        --busybox)
            BUSYBOX_BIN="$2"
            shift 2
            ;;
        --init-mode)
            INIT_MODE="$2"
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

require_tool() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required tool: $1" >&2
        exit 1
    fi
}

has_local_tool() {
    command -v "$1" >/dev/null 2>&1
}

has_host_tool() {
    if ! has_local_tool flatpak-spawn; then
        return 1
    fi
    flatpak-spawn --host sh -c "command -v '$1' >/dev/null 2>&1"
}

to_host_path() {
    path="$1"
    case "$path" in
        /home/*)
            printf '/var%s\n' "$path"
            ;;
        *)
            printf '%s\n' "$path"
            ;;
    esac
}

copy_from_spec() {
    spec="$1"
    dest="$2"
    mode="${spec%%:*}"
    src="${spec#*:}"

    case "$mode" in
        local)
            cp -L "$src" "$dest"
            ;;
        host)
            if ! has_local_tool flatpak-spawn; then
                echo "flatpak-spawn is required for host copy: $src" >&2
                exit 1
            fi
            flatpak-spawn --host cp -L "$src" "$(to_host_path "$dest")"
            ;;
        *)
            echo "Unknown copy mode in spec: $spec" >&2
            exit 1
            ;;
    esac
}

resolve_busybox_spec() {
    if [ -n "$BUSYBOX_BIN" ]; then
        if [ ! -x "$BUSYBOX_BIN" ]; then
            echo "busybox binary not executable: $BUSYBOX_BIN" >&2
            exit 1
        fi
        printf 'local:%s\n' "$BUSYBOX_BIN"
        return
    fi

    if has_local_tool busybox; then
        printf 'local:%s\n' "$(command -v busybox)"
        return
    fi

    for candidate in /run/host/usr/bin/busybox /usr/bin/busybox /bin/busybox; do
        if [ -x "$candidate" ]; then
            printf 'local:%s\n' "$candidate"
            return
        fi
    done

    if has_host_tool busybox; then
        printf 'host:%s\n' "$(flatpak-spawn --host sh -c 'command -v busybox')"
        return
    fi

    return 1
}

require_tool cpio
require_tool gzip

if [ "$INIT_MODE" != "static" ] && [ ! -f "$INIT_SOURCE_SH" ]; then
    echo "Missing initramfs init script: $INIT_SOURCE_SH" >&2
    exit 1
fi
if [ ! -f "$INIT_SOURCE_C" ]; then
    echo "Missing initramfs static init source: $INIT_SOURCE_C" >&2
    exit 1
fi

case "$INIT_MODE" in
    auto|busybox|static)
        ;;
    *)
        echo "Invalid --init-mode value: $INIT_MODE (expected auto|busybox|static)" >&2
        exit 1
        ;;
esac

BUSYBOX_SPEC="$(resolve_busybox_spec || true)"
MODE="$INIT_MODE"
if [ "$MODE" = "auto" ]; then
    if [ -n "$BUSYBOX_SPEC" ]; then
        MODE="busybox"
    else
        MODE="static"
    fi
fi

if [ "$MODE" = "busybox" ] && [ -z "$BUSYBOX_SPEC" ]; then
    echo "Could not find busybox. Install it, pass --busybox /path/to/busybox, or use --init-mode static." >&2
    exit 1
fi

if [ "$MODE" = "static" ]; then
    require_tool gcc
fi

mkdir -p "$(dirname "$OUTPUT_IMAGE")"
rm -rf "$ROOTFS_DIR"
mkdir -p "$ROOTFS_DIR"

mkdir -p "$ROOTFS_DIR/bin" "$ROOTFS_DIR/sbin" "$ROOTFS_DIR/proc" "$ROOTFS_DIR/sys" \
         "$ROOTFS_DIR/dev" "$ROOTFS_DIR/system" "$ROOTFS_DIR/config" "$ROOTFS_DIR/apps" \
         "$ROOTFS_DIR/home" "$ROOTFS_DIR/var" "$ROOTFS_DIR/run" "$ROOTFS_DIR/usr"

if [ "$MODE" = "busybox" ]; then
    cp "$INIT_SOURCE_SH" "$ROOTFS_DIR/init"
    chmod 0755 "$ROOTFS_DIR/init"

    # No symlinks policy: duplicate busybox for each required applet path.
    copy_from_spec "$BUSYBOX_SPEC" "$ROOTFS_DIR/bin/sh"
    copy_from_spec "$BUSYBOX_SPEC" "$ROOTFS_DIR/bin/mount"
    copy_from_spec "$BUSYBOX_SPEC" "$ROOTFS_DIR/bin/mkdir"
    copy_from_spec "$BUSYBOX_SPEC" "$ROOTFS_DIR/bin/cat"
    copy_from_spec "$BUSYBOX_SPEC" "$ROOTFS_DIR/sbin/switch_root"
    chmod 0755 "$ROOTFS_DIR/bin/sh" "$ROOTFS_DIR/bin/mount" "$ROOTFS_DIR/bin/mkdir" \
               "$ROOTFS_DIR/bin/cat" "$ROOTFS_DIR/sbin/switch_root"
else
    # Busybox-free path for environments where busybox is unavailable.
    # Static init performs mount sequence and root switch internally.
    gcc -Os -static -s -Wall -Wextra -o "$ROOTFS_DIR/init" "$INIT_SOURCE_C"
    chmod 0755 "$ROOTFS_DIR/init"
fi

(
    cd "$ROOTFS_DIR"
    find . -print | cpio -o -H newc --owner=0:0 2>/dev/null | gzip -9 > "$OUTPUT_IMAGE"
)

if [ ! -s "$OUTPUT_IMAGE" ]; then
    echo "Failed to produce initramfs image: $OUTPUT_IMAGE" >&2
    exit 1
fi

echo "$OUTPUT_IMAGE"
