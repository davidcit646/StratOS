#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase3"

usage() {
    cat <<EOF
Usage: $0

Builds StratBoot (x86_64 EFI application).
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

mkdir -p "$OUT_DIR"

make -C "$REPO_ROOT/stratboot" clean all >/dev/null

EFI_FILE="$REPO_ROOT/stratboot/BOOTX64.EFI"
if [ ! -f "$EFI_FILE" ]; then
    echo "StratBoot build failed: BOOTX64.EFI missing." >&2
    exit 1
fi

cp "$EFI_FILE" "$OUT_DIR/BOOTX64.EFI"
echo "$OUT_DIR/BOOTX64.EFI"
