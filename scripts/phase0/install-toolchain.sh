#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

resolve_rustup_bin() {
    if command -v rustup >/dev/null 2>&1; then
        command -v rustup
    elif [ -x "$HOME/.cargo/bin/rustup" ]; then
        echo "$HOME/.cargo/bin/rustup"
    else
        return 1
    fi
}

install_uefi_target() {
    rustup_bin="$(resolve_rustup_bin)"
    if [ -z "$rustup_bin" ]; then
        echo "Error: rustup not found. Install rustup first." >&2
        exit 1
    fi

    echo "Installing Rust UEFI target: x86_64-unknown-uefi"
    "$rustup_bin" target add x86_64-unknown-uefi
}

verify_uefi_target() {
    rustup_bin="$(resolve_rustup_bin)"
    if [ -z "$rustup_bin" ]; then
        echo "Error: rustup not found." >&2
        exit 1
    fi

    if "$rustup_bin" target list --installed 2>/dev/null | grep -q "x86_64-unknown-uefi"; then
        echo "Rust UEFI target verified: x86_64-unknown-uefi"
        return 0
    else
        echo "Error: Rust UEFI target not installed." >&2
        exit 1
    fi
}

install_uefi_target
verify_uefi_target
echo "Rust UEFI toolchain installation complete."
