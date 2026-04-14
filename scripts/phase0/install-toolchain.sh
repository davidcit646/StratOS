#!/usr/bin/env sh
set -eu

CHECK_ONLY=0
if [ "${1-}" = "--check-only" ]; then
    CHECK_ONLY=1
fi

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

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

resolve_rustup_bin() {
    rustup_bin="$(run_cmd sh -lc 'if command -v rustup >/dev/null 2>&1; then command -v rustup; elif [ -x "$HOME/.cargo/bin/rustup" ]; then echo "$HOME/.cargo/bin/rustup"; fi' || true)"
    if [ -z "$rustup_bin" ]; then
        return 1
    fi
    echo "$rustup_bin"
}

rustup_toolchain_list() {
    rustup_bin="$(resolve_rustup_bin || true)"
    if [ -z "$rustup_bin" ]; then
        return 1
    fi
    run_cmd sh -lc "\"$rustup_bin\" toolchain list"
}

rustup_toolchain_install() {
    toolchain="$1"
    rustup_bin="$(resolve_rustup_bin || true)"
    if [ -z "$rustup_bin" ]; then
        return 1
    fi
    run_cmd sh -lc "\"$rustup_bin\" toolchain install \"$toolchain\""
}

find_ovmf_code() {
    for path in \
        /usr/share/edk2/ovmf/OVMF_CODE.fd \
        /usr/share/OVMF/OVMF_CODE.fd \
        /usr/share/ovmf/x64/OVMF_CODE.fd \
        /usr/share/qemu/OVMF_CODE.fd
    do
        if run_cmd sh -lc "[ -f '$path' ]"; then
            echo "$path"
            return 0
        fi
    done
    return 1
}

find_ovmf_vars() {
    for path in \
        /usr/share/edk2/ovmf/OVMF_VARS.fd \
        /usr/share/OVMF/OVMF_VARS.fd \
        /usr/share/ovmf/x64/OVMF_VARS.fd \
        /usr/share/qemu/OVMF_VARS.fd
    do
        if run_cmd sh -lc "[ -f '$path' ]"; then
            echo "$path"
            return 0
        fi
    done
    return 1
}

find_wlroots_pkgconfig() {
    for pc_name in wlroots wlroots-0.18 wlroots-0.17 wlroots-0.16 wlroots-0.15; do
        version="$(run_cmd pkg-config --modversion "$pc_name" 2>/dev/null || true)"
        if [ -n "$version" ]; then
            echo "$pc_name:$version"
            return 0
        fi
    done
    return 1
}

version_ge_0_17() {
    version="$1"
    major="$(echo "$version" | cut -d. -f1)"
    minor="$(echo "$version" | cut -d. -f2 | cut -d- -f1)"

    case "$major:$minor" in
        ''|*:|'':*) return 1 ;;
    esac

    if [ "$major" -gt 0 ]; then
        return 0
    fi
    [ "$minor" -ge 17 ]
}

verify_toolchain() {
    failures=0

    if ! run_cmd sh -lc 'command -v qemu-system-x86_64 >/dev/null 2>&1'; then
        echo "missing: qemu-system-x86_64" >&2
        failures=$((failures + 1))
    fi

    ovmf_code="$(find_ovmf_code || true)"
    ovmf_vars="$(find_ovmf_vars || true)"
    if [ -z "$ovmf_code" ] || [ -z "$ovmf_vars" ]; then
        echo "missing: OVMF firmware files (OVMF_CODE.fd and OVMF_VARS.fd)" >&2
        failures=$((failures + 1))
    fi

    if ! run_cmd sh -lc 'command -v x86_64-linux-gnu-gcc >/dev/null 2>&1'; then
        echo "missing: x86_64-linux-gnu-gcc" >&2
        failures=$((failures + 1))
    fi

    wlroots_info="$(find_wlroots_pkgconfig || true)"
    if [ -z "$wlroots_info" ]; then
        echo "missing: wlroots development headers/pkg-config metadata" >&2
        failures=$((failures + 1))
    else
        wlroots_version="${wlroots_info#*:}"
        if ! version_ge_0_17 "$wlroots_version"; then
            echo "wlroots version too old: $wlroots_version (need >= 0.17)" >&2
            failures=$((failures + 1))
        fi
    fi

    if ! resolve_rustup_bin >/dev/null; then
        echo "missing: rustup" >&2
        failures=$((failures + 1))
    else
        if ! rustup_toolchain_list 2>/dev/null | grep -q "^stable"; then
            echo "missing: Rust stable toolchain" >&2
            failures=$((failures + 1))
        fi
        if ! rustup_toolchain_list 2>/dev/null | grep -q "^nightly"; then
            echo "missing: Rust nightly toolchain" >&2
            failures=$((failures + 1))
        fi
    fi

    if ! run_cmd sh -lc 'command -v meson >/dev/null 2>&1'; then
        echo "missing: meson" >&2
        failures=$((failures + 1))
    fi

    if ! run_cmd sh -lc 'command -v ninja >/dev/null 2>&1'; then
        echo "missing: ninja" >&2
        failures=$((failures + 1))
    fi

    if ! "$SCRIPT_DIR/verify-gnu-efi.sh" >/dev/null 2>&1; then
        echo "missing: GNU-EFI headers/libraries" >&2
        failures=$((failures + 1))
    fi

    if [ "$failures" -ne 0 ]; then
        echo "Toolchain verification failed with $failures issue(s)." >&2
        return 1
    fi

    echo "Toolchain verification passed."
    echo "  qemu: $(run_cmd sh -lc 'command -v qemu-system-x86_64')"
    echo "  ovmf_code: $ovmf_code"
    echo "  ovmf_vars: $ovmf_vars"
    echo "  cross_cc: $(run_cmd sh -lc 'command -v x86_64-linux-gnu-gcc')"
    echo "  wlroots: $wlroots_info"
    echo "  rust_toolchains:"
    rustup_toolchain_list | sed 's/^/    /'
    echo "  meson: $(run_cmd sh -lc 'command -v meson')"
    echo "  ninja: $(run_cmd sh -lc 'command -v ninja')"
}

install_phase0_packages() {
    case "$PKG_MANAGER" in
        apt-get)
            run_root_cmd apt-get update
            run_root_cmd apt-get install -y \
                qemu-system-x86 \
                ovmf \
                gcc-x86-64-linux-gnu \
                libwlroots-dev \
                meson \
                ninja-build
            ;;
        dnf)
            if run_cmd sh -lc 'command -v rpm-ostree >/dev/null 2>&1 && [ -f /run/ostree-booted ]'; then
                echo "Immutable rpm-ostree host detected." >&2
                echo "Installing with rpm-ostree (reboot required)..." >&2
                run_root_cmd rpm-ostree install \
                    qemu-system-x86 \
                    edk2-ovmf \
                    gcc-x86_64-linux-gnu \
                    wlroots0.17-devel \
                    meson \
                    ninja-build \
                    gnu-efi-devel
                echo "rpm-ostree transaction staged. Reboot, then run --check-only." >&2
                exit 0
            fi
            run_root_cmd dnf install -y \
                qemu-system-x86 \
                edk2-ovmf \
                gcc-x86_64-linux-gnu \
                wlroots0.17-devel \
                meson \
                ninja-build
            ;;
        pacman)
            run_root_cmd pacman -Sy --noconfirm \
                qemu-full \
                edk2-ovmf \
                x86_64-linux-gnu-gcc \
                wlroots \
                meson \
                ninja
            ;;
        zypper)
            run_root_cmd zypper --non-interactive install \
                qemu-x86 \
                ovmf \
                gcc-x86_64-linux-gnu \
                wlroots-devel \
                meson \
                ninja
            ;;
        *)
            echo "Unsupported package manager: $PKG_MANAGER" >&2
            exit 1
            ;;
    esac
}

verify_apt_wlroots_floor() {
    if [ "$PKG_MANAGER" != "apt-get" ]; then
        return 0
    fi

    wlroots_info="$(find_wlroots_pkgconfig || true)"
    if [ -z "$wlroots_info" ]; then
        echo "wlroots pkg-config entry not found after apt install." >&2
        return 1
    fi

    wlroots_version="${wlroots_info#*:}"
    if version_ge_0_17 "$wlroots_version"; then
        return 0
    fi

    echo "apt installed wlroots $wlroots_version, but StratOS requires >= 0.17." >&2
    echo "Use a newer wlroots package source (backport/PPA/source build), then re-run verification." >&2
    return 1
}

ensure_rust_toolchains() {
    if ! resolve_rustup_bin >/dev/null; then
        echo "rustup not found; install rustup before running this script." >&2
        exit 1
    fi

    if ! rustup_toolchain_list 2>/dev/null | grep -q "^stable"; then
        rustup_toolchain_install stable
    fi

    if ! rustup_toolchain_list 2>/dev/null | grep -q "^nightly"; then
        rustup_toolchain_install nightly
    fi
}

if ! detect_pkg_manager_local && ! detect_pkg_manager_host; then
    echo "No supported package manager found (apt-get, dnf, pacman, zypper)." >&2
    echo "Install dependencies manually, then re-run with --check-only." >&2
    exit 1
fi

echo "Package manager: $PKG_MANAGER ($RUN_CONTEXT)"

if [ "$CHECK_ONLY" -eq 1 ]; then
    verify_toolchain
    exit $?
fi

install_phase0_packages
verify_apt_wlroots_floor
ensure_rust_toolchains
verify_toolchain
