#!/usr/bin/env sh
set -eu

OUT_DIR="out/phase5"
BIN_NAME="strat-validate-boot"

mkdir -p "$OUT_DIR"

rustc -O "stratsup/validate_boot.rs" -o "$OUT_DIR/$BIN_NAME"

echo "Built $OUT_DIR/$BIN_NAME"
