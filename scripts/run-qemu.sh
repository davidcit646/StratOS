#!/bin/bash
# Runs QEMU with the StratOS test disk

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PHASE4_DIR="$REPO_ROOT/out/phase4"
DISK_IMAGE="$PHASE4_DIR/test-disk.img"
OVMF_VARS="$REPO_ROOT/ovmf_vars.fd"
SERIAL_LOG="${SERIAL_LOG_PATH:-$REPO_ROOT/ide-logs/qemu-desktop-serial.txt}"

mkdir -p "$(dirname "$SERIAL_LOG")"

qemu-system-x86_64 \
    -machine q35,accel=kvm:kvm:tcg \
    -cpu host \
    -m 4G \
    -smp 4 \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=off,file="$OVMF_VARS" \
    -drive if=virtio,file="$DISK_IMAGE",format=raw \
    -serial file:"$SERIAL_LOG" \
    -nographic \
    -monitor none
