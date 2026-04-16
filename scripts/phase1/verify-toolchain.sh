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
        echo "FAIL: rustup not found" >&2
        exit 1
    fi
    echo "PASS: rustup found at $rustup_bin"
}

verify_custom_target_spec() {
    target_spec="$TARGETS_DIR/${CUSTOM_TARGET}.json"
    if [ -f "$target_spec" ]; then
        echo "PASS: custom target spec exists at $target_spec"
        return 0
    else
        echo "FAIL: custom target spec not found at $target_spec" >&2
        exit 1
    fi
}

verify_cargo() {
    if command -v cargo >/dev/null 2>&1; then
        echo "PASS: cargo found"
        return 0
    else
        echo "FAIL: cargo not found" >&2
        exit 1
    fi
}

verify_rustc() {
    if command -v rustc >/dev/null 2>&1; then
        echo "PASS: rustc found"
        return 0
    else
        echo "FAIL: rustc not found" >&2
        exit 1
    fi
}

verify_project_config() {
    project_dir="$1"
    config_file="$project_dir/.cargo/config.toml"
    
    if [ ! -f "$config_file" ]; then
        echo "FAIL: $config_file not found" >&2
        exit 1
    fi
    
    if grep -q "target = \"$CUSTOM_TARGET\"" "$config_file"; then
        echo "PASS: $config_file configured for $CUSTOM_TARGET"
        return 0
    else
        echo "FAIL: $config_file not configured for $CUSTOM_TARGET" >&2
        exit 1
    fi
}

verify_stratsup_config() {
    verify_project_config "$PROJECT_ROOT/stratsup"
}

verify_stratmon_config() {
    verify_project_config "$PROJECT_ROOT/stratmon"
}

verify_test_build() {
    rustup_bin="$(resolve_rustup_bin)"
    
    echo "Testing custom target build with test project..."
    
    # Create a temporary test project with valid name
    tmp_dir="$(mktemp -d)"
    trap "rm -rf '$tmp_dir'" EXIT
    
    cd "$tmp_dir"
    
    # Initialize minimal Cargo project with valid package name
    cargo init --lib --name strat_test_build
    
    # Try to build with custom target using build-std
    echo "Attempting: cargo build --target $CUSTOM_TARGET -Z build-std=core,alloc"
    if cargo build --target "$CUSTOM_TARGET" -Z build-std=core,alloc 2>&1 | grep -q "Finished"; then
        echo "PASS: custom target test build succeeded"
        return 0
    else
        echo "FAIL: custom target test build failed" >&2
        echo "The custom target spec may be invalid or rust-src may not be installed" >&2
        exit 1
    fi
}

main() {
    echo "=== StratOS Phase 1: Toolchain Verification ==="
    
    verify_rustup
    verify_rustc
    verify_cargo
    verify_custom_target_spec
    verify_stratsup_config
    verify_stratmon_config
    verify_test_build
    
    echo ""
    echo "=== All Toolchain Checks Passed ==="
}

main "$@"
