#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
TEST_SRC="$REPO_ROOT/stratsup/tests/efi_vars_test.rs"
OUT_DIR="$REPO_ROOT/out/phase2"
BIN="$OUT_DIR/efi_vars_test"

usage() {
    cat <<EOF
Usage: $0

Builds and runs the Rust efivarfs read test.
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
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

if ! command -v rustc >/dev/null 2>&1; then
    echo "rustc not found." >&2
    exit 1
fi

mkdir -p "$OUT_DIR"
rustc --edition=2021 "$TEST_SRC" -o "$BIN"
"$BIN"
