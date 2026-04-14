#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
KERNEL_SRC="$REPO_ROOT/linux"
CONFIG_PATH="$REPO_ROOT/stratos-kernel/stratos.config"
OUT_DIR="$REPO_ROOT/out/phase4"
JOBS="${JOBS:-}"
BC_SHIM_DIR=""
CC_SHIM_DIR=""
EXTRA_MAKE_ARGS=""

cleanup() {
    if [ -n "$BC_SHIM_DIR" ] && [ -d "$BC_SHIM_DIR" ]; then
        rm -rf "$BC_SHIM_DIR"
    fi
    if [ -n "$CC_SHIM_DIR" ] && [ -d "$CC_SHIM_DIR" ]; then
        rm -rf "$CC_SHIM_DIR"
    fi
}

ensure_bc_on_path() {
    if command -v bc >/dev/null 2>&1; then
        return
    fi

    if [ -x /run/host/usr/bin/bc ]; then
        BC_SHIM_DIR="$(mktemp -d "${TMPDIR:-/tmp}/stratos-bc.XXXXXX")"
        ln -s /run/host/usr/bin/bc "$BC_SHIM_DIR/bc"
        PATH="$BC_SHIM_DIR:$PATH"
        export PATH
        echo "bc not found in PATH; using /run/host/usr/bin/bc fallback" >&2
        return
    fi

    echo "Missing required tool: bc" >&2
    echo "Install bc or provide /run/host/usr/bin/bc fallback" >&2
    exit 1
}

detect_gcc_compat_make_args() {
    cc_bin="${CC:-cc}"
    if ! command -v "$cc_bin" >/dev/null 2>&1; then
        return
    fi
    cc_path="$(command -v "$cc_bin")"

    cc_version="$("$cc_bin" -dumpfullversion -dumpversion 2>/dev/null || true)"
    cc_major="${cc_version%%.*}"
    case "$cc_major" in
        ''|*[!0-9]*)
            return
            ;;
    esac

    if [ "$cc_major" -ge 15 ]; then
        # Linux 6.6.x needs these under GCC 15 defaults:
        # - CONFIG_WERROR=n avoids newer warning classes failing the build.
        # - CC wrapper (passed as make arg) pins default language mode to pre-C23.
        CC_SHIM_DIR="$(mktemp -d "${TMPDIR:-/tmp}/stratos-cc.XXXXXX")"
        printf '#!/usr/bin/env sh\nexec "%s" -std=gnu11 "$@"\n' "$cc_path" > "$CC_SHIM_DIR/cc"
        chmod +x "$CC_SHIM_DIR/cc"
        EXTRA_MAKE_ARGS="CONFIG_WERROR=n CC=$CC_SHIM_DIR/cc"
        echo "Detected GCC $cc_major; applying make args: $EXTRA_MAKE_ARGS" >&2
    fi
}

run_make() {
    if [ -n "$EXTRA_MAKE_ARGS" ]; then
        # shellcheck disable=SC2086
        make $EXTRA_MAKE_ARGS "$@"
    else
        make "$@"
    fi
}

usage() {
    cat <<USAGE
Usage: $0 [--src PATH] [--config PATH] [--out-dir PATH] [--jobs N]

Builds Linux kernel using StratOS config and exports artifacts to out/phase4.
Expected outputs:
- vmlinuz
- System.map
- .config.used
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --src)
            KERNEL_SRC="$2"
            shift 2
            ;;
        --config)
            CONFIG_PATH="$2"
            shift 2
            ;;
        --out-dir)
            OUT_DIR="$2"
            shift 2
            ;;
        --jobs)
            JOBS="$2"
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

if [ ! -d "$KERNEL_SRC" ] || [ ! -f "$KERNEL_SRC/Makefile" ]; then
    echo "Kernel source tree not found: $KERNEL_SRC" >&2
    echo "Tip: pass --src /path/to/linux-source" >&2
    exit 1
fi

if [ ! -f "$CONFIG_PATH" ]; then
    echo "Kernel config not found: $CONFIG_PATH" >&2
    exit 1
fi

if ! command -v make >/dev/null 2>&1; then
    echo "Missing required tool: make" >&2
    exit 1
fi

ensure_bc_on_path
detect_gcc_compat_make_args
trap cleanup EXIT INT TERM

if [ -z "$JOBS" ]; then
    if command -v nproc >/dev/null 2>&1; then
        JOBS="$(nproc)"
    else
        JOBS=4
    fi
fi

mkdir -p "$OUT_DIR"

(
    cd "$KERNEL_SRC"
    run_make defconfig
    ./scripts/kconfig/merge_config.sh -m .config "$CONFIG_PATH"
    run_make olddefconfig
    run_make -j"$JOBS" bzImage
)

if [ ! -f "$KERNEL_SRC/arch/x86/boot/bzImage" ]; then
    echo "Kernel build did not produce bzImage" >&2
    exit 1
fi

cp "$KERNEL_SRC/arch/x86/boot/bzImage" "$OUT_DIR/vmlinuz"
if [ -f "$KERNEL_SRC/System.map" ]; then
    cp "$KERNEL_SRC/System.map" "$OUT_DIR/System.map"
fi
cp "$KERNEL_SRC/.config" "$OUT_DIR/.config.used"

echo "$OUT_DIR/vmlinuz"
