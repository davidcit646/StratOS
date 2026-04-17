#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
ROOTFS_DIR="$REPO_ROOT/out/phase7/rootfs-minimal"
INIT_SOURCE="$REPO_ROOT/sysroot/system-init.c"
FIRST_BOOT_SOURCE="$REPO_ROOT/sysroot/first-boot-provision.sh"
VALIDATE_SERVICE_SOURCE="$REPO_ROOT/services/systemd/strat-validate-boot.service"
VALIDATE_BIN_SOURCE="${VALIDATE_BIN_SOURCE:-$REPO_ROOT/stratsup/target/x86_64-unknown-linux-gnu/release/strat-validate-boot}"
STRATWM_BIN_SOURCE="${STRATWM_BIN_SOURCE:-$REPO_ROOT/stratvm/stratwm}"
STRATTERM_BIN_SOURCE="${STRATTERM_BIN_SOURCE:-$REPO_ROOT/stratterm/target/release/stratterm}"
STRATTERM_INDEXER_BIN_SOURCE="${STRATTERM_INDEXER_BIN_SOURCE:-$REPO_ROOT/stratterm/target/release/stratterm-indexer}"
STRAT_SETTINGS_BIN_SOURCE="${STRAT_SETTINGS_BIN_SOURCE:-$REPO_ROOT/stratterm/target/release/strat-settings}"
STRATTERM_INDEXER_BOOT_SOURCE="$REPO_ROOT/sysroot/strat-indexer-boot.sh"
STRATSTOP_BIN_SOURCE="${STRATSTOP_BIN_SOURCE:-$REPO_ROOT/stratstop/bin/stratstop}"
SHELL_BIN_SOURCE="${SHELL_BIN_SOURCE:-/bin/sh}"
SEATD_BIN_SOURCE="${SEATD_BIN_SOURCE:-$REPO_ROOT/third_party/seatd/build/seatd}"
FOOT_BIN_SOURCE="${FOOT_BIN_SOURCE:-/usr/bin/foot}"

if [ ! -x "$SEATD_BIN_SOURCE" ] && [ -x "/usr/sbin/seatd" ]; then
    SEATD_BIN_SOURCE="/usr/sbin/seatd"
fi

usage() {
    cat <<USAGE
Usage: $0 [--rootfs-dir PATH] [--validate-bin PATH]

Assembles a minimal Phase 7 system rootfs with a static /sbin/init.
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --rootfs-dir)
            ROOTFS_DIR="$2"
            shift 2
            ;;
        --validate-bin)
            VALIDATE_BIN_SOURCE="$2"
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

if ! command -v gcc >/dev/null 2>&1; then
    echo "Missing required tool: gcc" >&2
    exit 1
fi

if [ ! -f "$INIT_SOURCE" ]; then
    echo "Missing init source: $INIT_SOURCE" >&2
    exit 1
fi

rm -rf "$ROOTFS_DIR"
mkdir -p "$ROOTFS_DIR"

mkdir -p "$ROOTFS_DIR/sbin" "$ROOTFS_DIR/bin" "$ROOTFS_DIR/usr/bin" \
         "$ROOTFS_DIR/lib64" "$ROOTFS_DIR/usr/lib64" \
         "$ROOTFS_DIR/lib/systemd/system" \
         "$ROOTFS_DIR/etc" "$ROOTFS_DIR/proc" "$ROOTFS_DIR/sys" "$ROOTFS_DIR/dev" \
         "$ROOTFS_DIR/run" "$ROOTFS_DIR/tmp" "$ROOTFS_DIR/var" "$ROOTFS_DIR/home" "$ROOTFS_DIR/config" \
         "$ROOTFS_DIR/apps" "$ROOTFS_DIR/usr"
mkdir -p "$ROOTFS_DIR/dev/shm"

gcc -Os -static -s -Wall -Wextra -o "$ROOTFS_DIR/sbin/init" "$INIT_SOURCE"
chmod 0755 "$ROOTFS_DIR/sbin/init"

if [ -f "$FIRST_BOOT_SOURCE" ]; then
    cp "$FIRST_BOOT_SOURCE" "$ROOTFS_DIR/bin/first-boot-provision.sh"
    chmod 0755 "$ROOTFS_DIR/bin/first-boot-provision.sh"
fi

if [ -f "$VALIDATE_SERVICE_SOURCE" ]; then
    cp "$VALIDATE_SERVICE_SOURCE" "$ROOTFS_DIR/lib/systemd/system/strat-validate-boot.service"
fi

if [ -x "$VALIDATE_BIN_SOURCE" ]; then
    validate_file_info="$(file "$VALIDATE_BIN_SOURCE" 2>/dev/null || true)"
    case "$validate_file_info" in
        *"statically linked"*)
            cp "$VALIDATE_BIN_SOURCE" "$ROOTFS_DIR/bin/strat-validate-boot"
            chmod 0755 "$ROOTFS_DIR/bin/strat-validate-boot"
            ;;
        *)
            echo "Skipping non-static validate binary: $VALIDATE_BIN_SOURCE" >&2
            ;;
    esac
fi

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

runtime_collect_deps() {
    bin_path="$1"
    [ -e "$bin_path" ] || return 0

    ldd_tmp="$ROOTFS_DIR/.runtime.ldd"
    local_missing=0
    host_missing=0
    host_ldd_checked=0
    verbose=0
    strict=0
    case "$bin_path" in
        "$ROOTFS_DIR/bin/stratwm"|"$ROOTFS_DIR/bin/sh")
            verbose=1
            strict=1
            ;;
        "$ROOTFS_DIR/bin/foot"|"$ROOTFS_DIR/bin/stratterm")
            verbose=1
            strict=0
            ;;
    esac

    if command -v ldd >/dev/null 2>&1; then
        ldd "$bin_path" >"$ldd_tmp" 2>&1 || true
        if [ "$strict" -eq 1 ] && grep -q "not found" "$ldd_tmp"; then
            local_missing=1
        fi
        if [ "$verbose" -eq 1 ] || [ "$local_missing" -eq 1 ]; then
            printf 'prepare-rootfs: ldd %s (local)\n' "$bin_path" >&2
            sed 's/^/  /' "$ldd_tmp" >&2 || true
        fi
        awk '
            /=>/ { if ($3 ~ /^\//) print $3 }
            { for (i = 1; i <= NF; i++) if ($i ~ /^\//) print $i }
        ' "$ldd_tmp" >> "$ROOTFS_DIR/.runtime.deps" || true
    fi
    if command -v readelf >/dev/null 2>&1; then
        readelf -l "$bin_path" 2>/dev/null | awk -F': ' '
            /Requesting program interpreter/ { gsub(/\]/, "", $2); print $2 }
        ' >> "$ROOTFS_DIR/.runtime.deps" || true
    fi

    if [ "$strict" -eq 1 ] && command -v flatpak-spawn >/dev/null 2>&1; then
        host_bin="$(to_host_path "$bin_path")"
        if flatpak-spawn --host sh -lc "command -v ldd >/dev/null 2>&1" && \
           flatpak-spawn --host sh -lc "[ -e '$host_bin' ]"; then
            host_ldd_checked=1
            flatpak-spawn --host sh -lc "ldd '$host_bin' 2>&1 || true" >"$ldd_tmp"
            if grep -q "not found" "$ldd_tmp"; then
                host_missing=1
            fi
            if [ "$verbose" -eq 1 ] || [ "$host_missing" -eq 1 ]; then
                printf 'prepare-rootfs: ldd %s (host)\n' "$host_bin" >&2
                sed 's/^/  /' "$ldd_tmp" >&2 || true
            fi
            awk '
                /=>/ { if ($3 ~ /^\//) print $3 }
                { for (i = 1; i <= NF; i++) if ($i ~ /^\//) print $i }
            ' "$ldd_tmp" >> "$ROOTFS_DIR/.runtime.deps" || true
        fi
        if flatpak-spawn --host sh -lc "command -v readelf >/dev/null 2>&1"; then
            flatpak-spawn --host sh -lc "readelf -l '$host_bin' 2>/dev/null || true" | awk -F': ' '
                /Requesting program interpreter/ { gsub(/\]/, "", $2); print $2 }
            ' >> "$ROOTFS_DIR/.runtime.deps" || true
        fi
    fi

    if [ "$strict" -eq 1 ] && [ "$host_ldd_checked" -eq 1 ]; then
        if [ "$host_missing" -eq 1 ]; then
            echo "ERROR: unresolved host dependency while resolving $bin_path" >&2
            exit 1
        fi
    elif [ "$strict" -eq 1 ] && [ "$local_missing" -eq 1 ]; then
        echo "ERROR: unresolved local dependency while resolving $bin_path" >&2
        exit 1
    fi

    rm -f "$ldd_tmp"
}

normalize_dep_path() {
    dep="$1"
    case "$dep" in
        /run/host/*)
            printf '%s\n' "${dep#/run/host}"
            ;;
        *)
            printf '%s\n' "$dep"
            ;;
    esac
}

find_and_stage_required_lib() {
    soname="$1"
    found=""

    search_dirs="/usr/lib64
/usr/lib
/usr/lib/x86_64-linux-gnu
/lib64
/lib
/lib/x86_64-linux-gnu
/run/host/usr/lib64
/run/host/usr/lib
/run/host/usr/lib/x86_64-linux-gnu
/run/host/lib64
/run/host/lib"

    if command -v pkg-config >/dev/null 2>&1; then
        for pkg in wlroots wlroots-0.19 wlroots-0.18 wlroots-0.17; do
            if pkg-config --exists "$pkg" 2>/dev/null; then
                pkg-config --libs-only-L "$pkg" 2>/dev/null | tr ' ' '\n' | sed -n 's/^-L//p' >> "$ROOTFS_DIR/.runtime.libdirs" || true
            fi
        done
    fi
    if command -v flatpak-spawn >/dev/null 2>&1; then
        flatpak-spawn --host sh -lc '
            if command -v pkg-config >/dev/null 2>&1; then
                for pkg in wlroots wlroots-0.19 wlroots-0.18 wlroots-0.17; do
                    if pkg-config --exists "$pkg" 2>/dev/null; then
                        pkg-config --libs-only-L "$pkg" 2>/dev/null
                    fi
                done
            fi
        ' | tr ' ' '\n' | sed -n 's/^-L//p' >> "$ROOTFS_DIR/.runtime.libdirs" || true
    fi

    if [ -f "$ROOTFS_DIR/.runtime.libdirs" ]; then
        while IFS= read -r d; do
            [ -n "$d" ] || continue
            printf '%s\n' "$d" >> "$ROOTFS_DIR/.runtime.searchdirs"
        done < "$ROOTFS_DIR/.runtime.libdirs"
    fi
    printf '%s\n' "$search_dirs" >> "$ROOTFS_DIR/.runtime.searchdirs"
    sort -u "$ROOTFS_DIR/.runtime.searchdirs" -o "$ROOTFS_DIR/.runtime.searchdirs"

    while IFS= read -r dir; do
        [ -n "$dir" ] || continue
        candidate="$dir/$soname"
        if [ -e "$candidate" ]; then
            found="$(normalize_dep_path "$candidate")"
            break
        fi
    done < "$ROOTFS_DIR/.runtime.searchdirs"

    if [ -z "$found" ]; then
        echo "ERROR: required library not found in build env: $soname" >&2
        exit 1
    fi

    echo "prepare-rootfs: staging required library $found" >&2
    runtime_stage_dep "$found"
}

runtime_stage_dep() {
    dep="$1"
    dep_src="$dep"
    if [ ! -e "$dep_src" ] && [ -e "/run/host$dep" ]; then
        dep_src="/run/host$dep"
    fi
    [ -e "$dep_src" ] || return 0

    dep_rel="${dep#/}"
    dep_dir="$ROOTFS_DIR/$(dirname "$dep_rel")"
    mkdir -p "$dep_dir"
    cp -aL "$dep_src" "$dep_dir/$(basename "$dep_rel")"

    if grep -Fxq "$dep_src" "$ROOTFS_DIR/.runtime.queue"; then
        :
    else
        printf '%s\n' "$dep_src" >> "$ROOTFS_DIR/.runtime.queue"
    fi
}

: > "$ROOTFS_DIR/.runtime.queue"
: > "$ROOTFS_DIR/.runtime.seen"
: > "$ROOTFS_DIR/.runtime.deps"
: > "$ROOTFS_DIR/.runtime.searchdirs"
: > "$ROOTFS_DIR/.runtime.libdirs"

if [ -x "$STRATWM_BIN_SOURCE" ]; then
    cp -aL "$STRATWM_BIN_SOURCE" "$ROOTFS_DIR/bin/stratwm"
    chmod 0755 "$ROOTFS_DIR/bin/stratwm"
fi

if [ -x "$STRATTERM_BIN_SOURCE" ]; then
    cp -aL "$STRATTERM_BIN_SOURCE" "$ROOTFS_DIR/bin/stratterm"
    chmod 0755 "$ROOTFS_DIR/bin/stratterm"
fi

if [ -x "$STRATTERM_INDEXER_BIN_SOURCE" ]; then
    cp -aL "$STRATTERM_INDEXER_BIN_SOURCE" "$ROOTFS_DIR/bin/stratterm-indexer"
    chmod 0755 "$ROOTFS_DIR/bin/stratterm-indexer"
fi

if [ -x "$STRAT_SETTINGS_BIN_SOURCE" ]; then
    cp -aL "$STRAT_SETTINGS_BIN_SOURCE" "$ROOTFS_DIR/bin/strat-settings"
    chmod 0755 "$ROOTFS_DIR/bin/strat-settings"
fi

if [ -f "$STRATTERM_INDEXER_BOOT_SOURCE" ]; then
    cp "$STRATTERM_INDEXER_BOOT_SOURCE" "$ROOTFS_DIR/bin/strat-indexer-boot.sh"
    chmod 0755 "$ROOTFS_DIR/bin/strat-indexer-boot.sh"
fi

if [ -x "$SHELL_BIN_SOURCE" ]; then
    cp -aL "$SHELL_BIN_SOURCE" "$ROOTFS_DIR/bin/sh"
    chmod 0755 "$ROOTFS_DIR/bin/sh"
fi

if [ -x "$STRATSTOP_BIN_SOURCE" ]; then
    cp -aL "$STRATSTOP_BIN_SOURCE" "$ROOTFS_DIR/bin/stratstop"
    chmod 0755 "$ROOTFS_DIR/bin/stratstop"
fi

if [ -x "$SEATD_BIN_SOURCE" ]; then
    cp -aL "$SEATD_BIN_SOURCE" "$ROOTFS_DIR/bin/seatd"
    chmod 0755 "$ROOTFS_DIR/bin/seatd"
    printf '%s\n' "$ROOTFS_DIR/bin/seatd" >> "$ROOTFS_DIR/.runtime.queue"
else
    echo "Warning: seatd not found at $SEATD_BIN_SOURCE" >&2
fi

if [ -x "$FOOT_BIN_SOURCE" ]; then
    cp -aL "$FOOT_BIN_SOURCE" "$ROOTFS_DIR/bin/foot"
    chmod 0755 "$ROOTFS_DIR/bin/foot"
    printf '%s\n' "$ROOTFS_DIR/bin/foot" >> "$ROOTFS_DIR/.runtime.queue"
elif [ -x "/run/host/usr/bin/foot" ]; then
    cp -aL "/run/host/usr/bin/foot" "$ROOTFS_DIR/bin/foot"
    chmod 0755 "$ROOTFS_DIR/bin/foot"
    printf '%s\n' "$ROOTFS_DIR/bin/foot" >> "$ROOTFS_DIR/.runtime.queue"
else
    echo "Warning: foot not found at $FOOT_BIN_SOURCE" >&2
fi

mkdir -p "$ROOTFS_DIR/usr/share/fonts" "$ROOTFS_DIR/etc/fonts"

mono_font_src=""
if command -v fc-list >/dev/null 2>&1; then
    mono_font_src="$(fc-list | grep -i mono | head -1 | cut -d: -f1 || true)"
fi
if [ -z "$mono_font_src" ]; then
    mono_font_src="$(find /usr/share/fonts -type f \( -name '*.ttf' -o -name '*.otf' \) | head -1 || true)"
fi
if [ -n "$mono_font_src" ] && [ -f "$mono_font_src" ]; then
    cp -aL "$mono_font_src" "$ROOTFS_DIR/usr/share/fonts/"
else
    echo "Warning: no font file found under /usr/share/fonts" >&2
fi

cat > "$ROOTFS_DIR/etc/fonts/fonts.conf" <<'EOF'
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig>
  <dir>/usr/share/fonts</dir>
</fontconfig>
EOF

mkdir -p "$ROOTFS_DIR/usr/share"
if [ -d "/usr/share/libinput" ]; then
    cp -a "/usr/share/libinput" "$ROOTFS_DIR/usr/share/"
elif [ -d "/run/host/usr/share/libinput" ]; then
    cp -a "/run/host/usr/share/libinput" "$ROOTFS_DIR/usr/share/"
else
    echo "Warning: libinput data directory not found on local or host paths" >&2
fi

mkdir -p "$ROOTFS_DIR/usr/share/X11"
if [ -d "/usr/share/X11/xkb" ]; then
    cp -aL "/usr/share/X11/xkb" "$ROOTFS_DIR/usr/share/X11/"
elif [ -d "/run/host/usr/share/X11/xkb" ]; then
    cp -aL "/run/host/usr/share/X11/xkb" "$ROOTFS_DIR/usr/share/X11/"
else
    echo "Warning: xkb data directory not found on local or host paths" >&2
fi

if [ -d "/usr/share/X11/locale" ]; then
    cp -a "/usr/share/X11/locale" "$ROOTFS_DIR/usr/share/X11/"
elif [ -d "/run/host/usr/share/X11/locale" ]; then
    cp -a "/run/host/usr/share/X11/locale" "$ROOTFS_DIR/usr/share/X11/"
else
    echo "Warning: X11 locale data directory not found on local or host paths" >&2
fi

mkdir -p "$ROOTFS_DIR/usr/lib/locale"
if [ -d "/usr/lib/locale/C.utf8" ]; then
    cp -a "/usr/lib/locale/C.utf8" "$ROOTFS_DIR/usr/lib/locale/"
elif [ -d "/run/host/usr/lib/locale/C.utf8" ]; then
    cp -a "/run/host/usr/lib/locale/C.utf8" "$ROOTFS_DIR/usr/lib/locale/"
else
    echo "Warning: C.utf8 locale directory not found on local or host paths" >&2
fi

if [ -x "$ROOTFS_DIR/bin/stratwm" ]; then
    printf '%s\n' "$ROOTFS_DIR/bin/stratwm" >> "$ROOTFS_DIR/.runtime.queue"
fi
if [ -x "$ROOTFS_DIR/bin/stratterm" ]; then
    printf '%s\n' "$ROOTFS_DIR/bin/stratterm" >> "$ROOTFS_DIR/.runtime.queue"
fi
if [ -x "$ROOTFS_DIR/bin/stratterm-indexer" ]; then
    printf '%s\n' "$ROOTFS_DIR/bin/stratterm-indexer" >> "$ROOTFS_DIR/.runtime.queue"
fi
if [ -x "$ROOTFS_DIR/bin/strat-settings" ]; then
    printf '%s\n' "$ROOTFS_DIR/bin/strat-settings" >> "$ROOTFS_DIR/.runtime.queue"
fi
if [ -x "$ROOTFS_DIR/bin/sh" ]; then
    printf '%s\n' "$ROOTFS_DIR/bin/sh" >> "$ROOTFS_DIR/.runtime.queue"
fi
if [ -x "$ROOTFS_DIR/bin/stratwm" ]; then
    find_and_stage_required_lib "libwlroots-0.19.so"
fi
if [ -x "$ROOTFS_DIR/bin/foot" ]; then
    find_and_stage_required_lib "libwayland-cursor.so.0"
fi

line_no=1
while :; do
    candidate="$(sed -n "${line_no}p" "$ROOTFS_DIR/.runtime.queue")"
    if [ -z "$candidate" ]; then
        break
    fi
    line_no=$((line_no + 1))

    if grep -Fxq "$candidate" "$ROOTFS_DIR/.runtime.seen"; then
        continue
    fi
    printf '%s\n' "$candidate" >> "$ROOTFS_DIR/.runtime.seen"

    : > "$ROOTFS_DIR/.runtime.deps"
    runtime_collect_deps "$candidate"
    sort -u "$ROOTFS_DIR/.runtime.deps" -o "$ROOTFS_DIR/.runtime.deps"

    while IFS= read -r dep; do
        [ -n "$dep" ] || continue
        runtime_stage_dep "$dep"
    done < "$ROOTFS_DIR/.runtime.deps"
done

rm -f "$ROOTFS_DIR/.runtime.queue" "$ROOTFS_DIR/.runtime.seen" "$ROOTFS_DIR/.runtime.deps" \
      "$ROOTFS_DIR/.runtime.searchdirs" "$ROOTFS_DIR/.runtime.libdirs" "$ROOTFS_DIR/.runtime.ldd"

echo "$ROOTFS_DIR"
