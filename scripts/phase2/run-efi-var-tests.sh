#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase2"
ESP_WRITE_IMG="$OUT_DIR/esp-write.img"
ESP_READ_IMG="$OUT_DIR/esp-read.img"
ESP_CORRUPT_IMG="$OUT_DIR/esp-corrupt.img"
SERIAL_LOG="$OUT_DIR/efi-var-test.log"
OVMF_VARS="$OUT_DIR/ovmf-vars.fd"
OVMF_CODE=""
OVMF_VARS_TEMPLATE=""

usage() {
    cat <<EOF
Usage: $0

Runs Phase 2 EFI variable tests in QEMU:
  1) write variables
  2) reboot + read variables (persistence)
  3) corrupt variable + verify detection
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

find_ovmf_code() {
    for candidate in \
        /usr/share/edk2/ovmf/OVMF_CODE.fd \
        /usr/share/OVMF/OVMF_CODE.fd \
        /usr/share/ovmf/x64/OVMF_CODE.fd \
        /usr/share/qemu/OVMF_CODE.fd
    do
        if [ -f "$candidate" ]; then
            echo "$candidate"
            return 0
        fi
    done
    return 1
}

find_ovmf_vars() {
    for candidate in \
        /usr/share/edk2/ovmf/OVMF_VARS.fd \
        /usr/share/OVMF/OVMF_VARS.fd \
        /usr/share/ovmf/x64/OVMF_VARS.fd \
        /usr/share/qemu/OVMF_VARS.fd
    do
        if [ -f "$candidate" ]; then
            echo "$candidate"
            return 0
        fi
    done
    return 1
}

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        exit 1
    fi
}

require_cmd qemu-system-x86_64
require_cmd mkfs.vfat
require_cmd mcopy
require_cmd mmd
require_cmd dd

mkdir -p "$OUT_DIR"

OVMF_CODE="$(find_ovmf_code)"
OVMF_VARS_TEMPLATE="$(find_ovmf_vars)"
cp "$OVMF_VARS_TEMPLATE" "$OVMF_VARS"

EFI_APP="$("$REPO_ROOT/scripts/phase2/build-efi-var-test.sh")"
if [ -z "$EFI_APP" ] || [ ! -f "$EFI_APP" ]; then
    echo "EFI app build failed or output missing: $EFI_APP" >&2
    exit 1
fi
if [ ! -s "$EFI_APP" ]; then
    echo "EFI app output is empty: $EFI_APP" >&2
    exit 1
fi

create_esp_image() {
    img_path="$1"
    mode="$2"

    dd if=/dev/zero of="$img_path" bs=1M count=16 status=none
    mkfs.vfat -F 32 "$img_path" >/dev/null
    mmd -i "$img_path" ::/EFI ::/EFI/BOOT

    printf "fs0:\n\\EFI\\BOOT\\BOOTX64.EFI %s\n" "$mode" > "$OUT_DIR/startup.nsh"
    mcopy -i "$img_path" "$EFI_APP" ::/EFI/BOOT/BOOTX64.EFI
    if ! mdir -i "$img_path" ::/EFI/BOOT/BOOTX64.EFI >/dev/null 2>&1; then
        echo "Failed to copy EFI app to ESP image: $img_path" >&2
        exit 1
    fi
    mcopy -i "$img_path" "$OUT_DIR/startup.nsh" ::/startup.nsh
}

run_qemu_once() {
    esp_img="$1"
    label="$2"

    : > "$SERIAL_LOG"
    qemu-system-x86_64 \
        -machine q35,accel=tcg \
        -m 1024 \
        -smp 2 \
        -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
        -drive if=pflash,format=raw,file="$OVMF_VARS" \
        -drive if=ide,format=raw,file="$esp_img" \
        -nographic \
        -serial file:"$SERIAL_LOG" \
        -monitor none \
        -no-reboot &

    qemu_pid=$!
    sleep 10
    if kill -0 "$qemu_pid" 2>/dev/null; then
        kill "$qemu_pid" >/dev/null 2>&1 || true
        wait "$qemu_pid" >/dev/null 2>&1 || true
    fi

    if ! grep -q "STRAT EFI VAR TEST: PASS" "$SERIAL_LOG"; then
        echo "EFI var test failed in mode: $label" >&2
        echo "Serial log tail:" >&2
        tail -n 50 "$SERIAL_LOG" >&2 || true
        exit 1
    fi
}

create_esp_image "$ESP_WRITE_IMG" "write"
create_esp_image "$ESP_READ_IMG" "read"
create_esp_image "$ESP_CORRUPT_IMG" "corrupt"

run_qemu_once "$ESP_WRITE_IMG" "write"
run_qemu_once "$ESP_READ_IMG" "read"
run_qemu_once "$ESP_CORRUPT_IMG" "corrupt"

echo "Phase 2 EFI variable tests passed."
