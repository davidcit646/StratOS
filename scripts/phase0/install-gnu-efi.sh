#!/usr/bin/env sh
set -eu

CHECK_ONLY=0
if [ "${1-}" = "--check-only" ]; then
    CHECK_ONLY=1
fi

RUN_CONTEXT="local"
PKG_MANAGER=""

detect_pkg_manager_local() {
    for candidate in apt-get dnf pacman zypper; do
        if command -v "$candidate" >/dev/null 2>&1; then
            PKG_MANAGER="$candidate"
            RUN_CONTEXT="local"
            return 0
        fi
    done
    return 1
}

detect_pkg_manager_host() {
    if ! command -v flatpak-spawn >/dev/null 2>&1; then
        return 1
    fi

    for candidate in apt-get dnf pacman zypper; do
        if flatpak-spawn --host sh -lc "command -v $candidate >/dev/null 2>&1"; then
            PKG_MANAGER="$candidate"
            RUN_CONTEXT="host"
            return 0
        fi
    done
    return 1
}

run_cmd() {
    if [ "$RUN_CONTEXT" = "host" ]; then
        flatpak-spawn --host "$@"
        return
    fi
    "$@"
}

run_root_cmd() {
    if run_cmd sh -lc '[ "$(id -u)" -eq 0 ]'; then
        run_cmd "$@"
        return
    fi

    if run_cmd sh -lc 'command -v sudo >/dev/null 2>&1'; then
        run_cmd sudo "$@"
        return
    fi

    echo "Need root privileges to install packages. Re-run as root." >&2
    exit 1
}

find_gnu_efi_header() {
    run_cmd sh -lc 'test -f /usr/include/efi/efi.h && echo /usr/include/efi/efi.h'
}

find_lib_file() {
    name="$1"
    run_cmd sh -lc "find /usr/lib /usr/lib64 /usr/lib/x86_64-linux-gnu -maxdepth 4 -type f -name \"$name\" 2>/dev/null | head -n1"
}

verify_gnu_efi() {
    header_path="$(find_gnu_efi_header || true)"
    libgnuefi_path="$(find_lib_file libgnuefi.a || true)"
    libefi_path="$(find_lib_file libefi.a || true)"

    if [ -z "$header_path" ] || [ -z "$libgnuefi_path" ] || [ -z "$libefi_path" ]; then
        echo "GNU-EFI verification failed." >&2
        if [ -z "$header_path" ]; then
            echo "  missing: /usr/include/efi/efi.h" >&2
        fi
        if [ -z "$libgnuefi_path" ]; then
            echo "  missing: libgnuefi.a" >&2
        fi
        if [ -z "$libefi_path" ]; then
            echo "  missing: libefi.a" >&2
        fi
        return 1
    fi

    echo "GNU-EFI verified"
    echo "  header: $header_path"
    echo "  libgnuefi: $libgnuefi_path"
    echo "  libefi: $libefi_path"
}

install_gnu_efi() {
    case "$PKG_MANAGER" in
        apt-get)
            run_root_cmd apt-get update
            run_root_cmd apt-get install -y gnu-efi
            ;;
        dnf)
            run_root_cmd dnf install -y gnu-efi-devel
            ;;
        pacman)
            run_root_cmd pacman -Sy --noconfirm gnu-efi
            ;;
        zypper)
            run_root_cmd zypper --non-interactive install gnu-efi-devel
            ;;
        *)
            echo "Unsupported package manager: $PKG_MANAGER" >&2
            exit 1
            ;;
    esac
}

if ! detect_pkg_manager_local && ! detect_pkg_manager_host; then
    echo "No supported package manager found (apt-get, dnf, pacman, zypper)." >&2
    echo "Install GNU-EFI manually, then re-run with --check-only." >&2
    exit 1
fi

echo "Package manager: $PKG_MANAGER ($RUN_CONTEXT)"

if [ "$CHECK_ONLY" -eq 1 ]; then
    verify_gnu_efi
    exit $?
fi

install_gnu_efi
verify_gnu_efi
