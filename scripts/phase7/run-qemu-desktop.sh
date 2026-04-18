#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
DISK_IMAGE="${DISK_IMAGE:-$REPO_ROOT/out/phase4/test-disk.img}"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
ACCEL="${ACCEL:-kvm}"
MEMORY_MB="${MEMORY_MB:-2048}"
CPUS="${CPUS:-2}"
DISPLAY_BACKEND="${DISPLAY_BACKEND:-gtk}"
SYNC_SLOT_A="${SYNC_SLOT_A:-1}"
SYNC_ESP="${SYNC_ESP:-1}"
SERIAL_LOG_PATH="${SERIAL_LOG_PATH:-$REPO_ROOT/out/phase7/logs/qemu-desktop-serial.log}"
RUN_CONTEXT="local"

has_local() {
    command -v "$1" >/dev/null 2>&1
}

has_host() {
    if ! has_local flatpak-spawn; then
        return 1
    fi
    flatpak-spawn --host sh -c "command -v '$1' >/dev/null 2>&1"
}

run_cmd() {
    if [ "$RUN_CONTEXT" = "host" ]; then
        flatpak-spawn --host "$@"
    else
        "$@"
    fi
}

to_context_path() {
    path="$1"
    if [ "$RUN_CONTEXT" != "host" ]; then
        printf '%s\n' "$path"
        return
    fi
    case "$path" in
        /run/host/*)
            printf '%s\n' "${path#/run/host}"
            ;;
        /home/*)
            printf '/var%s\n' "$path"
            ;;
        *)
            printf '%s\n' "$path"
            ;;
    esac
}

find_ovmf_code() {
    for candidate in \
        /usr/share/edk2/ovmf/OVMF_CODE.fd \
        /usr/share/OVMF/OVMF_CODE.fd \
        /usr/share/ovmf/x64/OVMF_CODE.fd \
        /usr/share/qemu/OVMF_CODE.fd \
        /run/host/usr/share/edk2/ovmf/OVMF_CODE.fd \
        /run/host/usr/share/OVMF/OVMF_CODE.fd \
        /run/host/usr/share/ovmf/x64/OVMF_CODE.fd \
        /run/host/usr/share/qemu/OVMF_CODE.fd
    do
        if [ -f "$candidate" ]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    return 1
}

find_ovmf_vars_template() {
    for candidate in \
        /usr/share/edk2/ovmf/OVMF_VARS.fd \
        /usr/share/OVMF/OVMF_VARS.fd \
        /usr/share/ovmf/x64/OVMF_VARS.fd \
        /usr/share/qemu/OVMF_VARS.fd \
        /run/host/usr/share/edk2/ovmf/OVMF_VARS.fd \
        /run/host/usr/share/OVMF/OVMF_VARS.fd \
        /run/host/usr/share/ovmf/x64/OVMF_VARS.fd \
        /run/host/usr/share/qemu/OVMF_VARS.fd
    do
        if [ -f "$candidate" ]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    return 1
}

OVMF_CODE="${OVMF_CODE:-$(find_ovmf_code || true)}"
OVMF_VARS_TEMPLATE="${OVMF_VARS_TEMPLATE:-$(find_ovmf_vars_template || true)}"

if ! has_local "$QEMU_BIN"; then
    if has_host "$QEMU_BIN"; then
        RUN_CONTEXT="host"
    else
        echo "Missing QEMU binary: $QEMU_BIN" >&2
        exit 1
    fi
fi

if [ ! -f "$DISK_IMAGE" ]; then
    echo "Missing disk image: $DISK_IMAGE" >&2
    exit 1
fi
if [ -z "$OVMF_CODE" ] || [ ! -f "$OVMF_CODE" ]; then
    echo "Unable to find OVMF_CODE.fd (set OVMF_CODE=...)." >&2
    exit 1
fi
if [ -z "$OVMF_VARS_TEMPLATE" ] || [ ! -f "$OVMF_VARS_TEMPLATE" ]; then
    echo "Unable to find OVMF_VARS.fd (set OVMF_VARS_TEMPLATE=...)." >&2
    exit 1
fi

if [ "$SYNC_SLOT_A" = "1" ]; then
    sync_script="$REPO_ROOT/scripts/phase7/update-test-disk-slot-a.sh"
    if [ ! -x "$sync_script" ]; then
        echo "Missing sync helper: $sync_script" >&2
        exit 1
    fi
    if [ "$SYNC_ESP" = "1" ]; then
        "$sync_script" --disk "$DISK_IMAGE"
    else
        "$sync_script" --disk "$DISK_IMAGE" --no-esp-update
    fi
fi

OVMF_VARS_RUNTIME="$(mktemp /tmp/stratos-vars.XXXXXX.fd)"

QEMU_DISK_IMAGE="$(to_context_path "$DISK_IMAGE")"
QEMU_OVMF_CODE="$(to_context_path "$OVMF_CODE")"
QEMU_OVMF_VARS_TEMPLATE="$(to_context_path "$OVMF_VARS_TEMPLATE")"
QEMU_OVMF_VARS_RUNTIME="$(to_context_path "$OVMF_VARS_RUNTIME")"

run_cmd cp "$QEMU_OVMF_VARS_TEMPLATE" "$QEMU_OVMF_VARS_RUNTIME"

cleanup() {
    rm -f "$OVMF_VARS_RUNTIME"
}
trap cleanup EXIT INT TERM

mkdir -p "$(dirname "$SERIAL_LOG_PATH")"

status_file="$(mktemp /tmp/stratos-qemu-status.XXXXXX)"

(
set +e
run_cmd "$QEMU_BIN" \
    -machine "q35,accel=$ACCEL" \
    -m "$MEMORY_MB" -smp "$CPUS" \
    -drive "if=pflash,format=raw,readonly=on,file=$QEMU_OVMF_CODE" \
    -drive "if=pflash,format=raw,file=$QEMU_OVMF_VARS_RUNTIME" \
    -device virtio-scsi-pci,id=scsi0 \
    -drive "if=none,id=hd0,format=raw,file=$QEMU_DISK_IMAGE" \
    -device scsi-hd,bus=scsi0.0,drive=hd0 \
    -device virtio-gpu-pci \
    -device virtio-keyboard-pci \
    -device virtio-mouse-pci \
    -display "$DISPLAY_BACKEND" \
    -serial stdio \
    -no-reboot
printf '%s\n' "$?" > "$status_file"
) 2>&1 | tee "$SERIAL_LOG_PATH"

status="$(cat "$status_file" 2>/dev/null || printf '1')"
rm -f "$status_file"
echo "QEMU serial log captured at: $SERIAL_LOG_PATH" >&2
exit "$status"
