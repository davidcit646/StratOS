#!/usr/bin/env sh
set -eu

OUT_DIR="out/phase6"
BIN_NAME="stratsup"
TARGET="x86_64-unknown-linux-musl"
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)"

mkdir -p "$REPO_ROOT/$OUT_DIR"

cd "$REPO_ROOT/stratsup"
RUSTFLAGS="-C target-feature=+crt-static" \
    cargo build --release --target "$TARGET"
cd - > /dev/null

BIN_PATH="$REPO_ROOT/stratsup/target/$TARGET/release/$BIN_NAME"
if [ ! -f "$BIN_PATH" ]; then
    echo "Supervisor build output missing: $BIN_PATH" >&2
    exit 1
fi

FILE_OUT="$(file "$BIN_PATH")"
case "$FILE_OUT" in
    *"statically linked"*) ;;
    *)
        echo "Supervisor binary is not statically linked:" >&2
        echo "$FILE_OUT" >&2
        exit 1
        ;;
esac

cp "$BIN_PATH" "$REPO_ROOT/$OUT_DIR/$BIN_NAME"
echo "Built $REPO_ROOT/$OUT_DIR/$BIN_NAME"
