#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
PROJECT_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)"
TARGETS_DIR="$PROJECT_ROOT/toolchains/targets"
CUSTOM_TARGET="x86_64-stratos-uefi"

resolve_rustup_bin() {
    if command -v rustup >/dev/null 2>&1; then
        command -v rustup
    elif [ -x "$HOME/.cargo/bin/rustup" ]; then
        echo "$HOME/.cargo/bin/rustup"
    else
        return 1
    fi
}

verify_rustup() {
    rustup_bin="$(resolve_rustup_bin)"
    if [ -z "$rustup_bin" ]; then
        echo "Error: rustup not found. Install rustup first." >&2
        exit 1
    fi
    echo "Rustup verified: $rustup_bin"
}

verify_custom_target_spec() {
    target_spec="$TARGETS_DIR/${CUSTOM_TARGET}.json"
    if [ ! -f "$target_spec" ]; then
        echo "Error: Custom target spec not found: $target_spec" >&2
        exit 1
    fi
    echo "Custom target spec verified: $target_spec"
}

install_rust_components() {
    rustup_bin="$(resolve_rustup_bin)"
    
    # Ensure rust-src is available for build-std
    echo "Installing rust-src component for -Z build-std..."
    "$rustup_bin" component add rust-src || true
    
    # Ensure we have the necessary tools
    echo "Verifying cargo..."
    if ! command -v cargo >/dev/null 2>&1; then
        echo "Error: cargo not found" >&2
        exit 1
    fi
}

verify_custom_target_build() {
    rustup_bin="$(resolve_rustup_bin)"
    
    echo "Testing custom target build capability..."
    
    # Create a temporary test project
    tmp_dir="$(mktemp -d)"
    trap "rm -rf '$tmp_dir'" EXIT
    
    cd "$tmp_dir"
    
    # Initialize minimal Cargo project
    cargo init --lib
    
    # Try to build with custom target using build-std
    echo "Attempting: cargo build --target $CUSTOM_TARGET -Z build-std=core,alloc"
    if cargo build --target "$CUSTOM_TARGET" -Z build-std=core,alloc 2>&1 | grep -q "Finished"; then
        echo "Custom target build verification PASSED"
        return 0
    else
        echo "Error: Custom target build verification FAILED" >&2
        echo "The custom target spec may be invalid or rust-src may not be installed" >&2
        exit 1
    fi
}

verify_toolchain_complete() {
    echo "Verifying complete toolchain..."
    verify_rustup
    verify_custom_target_spec
    echo "Toolchain verification complete."
}

main() {
    echo "=== StratOS Phase 1: Toolchain Setup ==="
    
    verify_rustup
    verify_custom_target_spec
    install_rust_components
    verify_custom_target_build
    verify_toolchain_complete
    
    echo "=== Phase 1 Toolchain Setup Complete ==="
    echo "Custom target: $CUSTOM_TARGET"
    echo "Target spec path: $TARGETS_DIR/${CUSTOM_TARGET}.json"
    echo ""
    echo "To build with custom target, use:"
    echo "  cargo build --target $CUSTOM_TARGET -Z build-std=core,alloc"
}

main "$@"
