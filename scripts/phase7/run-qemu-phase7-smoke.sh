#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/out/phase7"
BOOT_IMAGE="$REPO_ROOT/out/phase3/stratboot.img"
SLOT_IMAGE="$REPO_ROOT/out/phase7/slot-system.erofs"
SERIAL_LOG="$OUT_DIR/qemu-phase7-serial.log"
DEBUG_LOG="$OUT_DIR/qemu-phase7-debugcon.log"
OVMF_VARS_RUNTIME="$OUT_DIR/ovmf-vars.phase7.fd"
OVMF_CODE=""
OVMF_VARS_TEMPLATE=""
ACCEL="${ACCEL:-tcg}"
MEMORY_MB="${MEMORY_MB:-2048}"
CPUS="${CPUS:-2}"
DURATION_SEC="${DURATION_SEC:-20}"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
RUN_CONTEXT="local"
ATTACH_SLOT=1
BOOT_FORMAT="raw"

usage() {
    cat <<USAGE
Usage: $0 [options]

Runs a Phase 7 smoke boot with StratBoot disk image + OVMF for a fixed duration.
Pass condition:
  1) no fatal signatures in serial log
  2) serial/debug log contains "StratBoot: booting slot" OR OVMF vars contain `STRAT_SMOKE_BOOTING_SLOT`
  3) VM stays alive for full duration (timeout exit 124)

Options:
  --img PATH               StratBoot raw image (default: out/phase3/stratboot.img)
  --vhd PATH               StratBoot VHD image (alias; sets format=vpc)
  --slot-image PATH        Optional slot EROFS image to attach read-only (default: out/phase7/slot-system.erofs)
  --no-slot-image          Do not attach slot image
  --serial-log PATH        Serial output path (default: out/phase7/qemu-phase7-serial.log)
  --debug-log PATH         Debugcon output path (default: out/phase7/qemu-phase7-debugcon.log)
  --seconds N              Smoke duration in seconds (default: 20)
  --memory MB              RAM in MB (default: 2048)
  --cpus N                 CPU count (default: 2)
  --accel tcg|kvm          QEMU accel (default: tcg)
  --ovmf-code PATH         OVMF_CODE.fd path
  --ovmf-vars-template PATH
                           OVMF_VARS.fd template path
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --img)
            BOOT_IMAGE="$2"
            BOOT_FORMAT="raw"
            shift 2
            ;;
        --vhd)
            BOOT_IMAGE="$2"
            BOOT_FORMAT="vpc"
            shift 2
            ;;
        --slot-image)
            SLOT_IMAGE="$2"
            ATTACH_SLOT=1
            shift 2
            ;;
        --no-slot-image)
            ATTACH_SLOT=0
            shift
            ;;
        --serial-log)
            SERIAL_LOG="$2"
            shift 2
            ;;
        --debug-log)
            DEBUG_LOG="$2"
            shift 2
            ;;
        --seconds)
            DURATION_SEC="$2"
            shift 2
            ;;
        --memory)
            MEMORY_MB="$2"
            shift 2
            ;;
        --cpus)
            CPUS="$2"
            shift 2
            ;;
        --accel)
            ACCEL="$2"
            shift 2
            ;;
        --ovmf-code)
            OVMF_CODE="$2"
            shift 2
            ;;
        --ovmf-vars-template)
            OVMF_VARS_TEMPLATE="$2"
            shift 2
            ;;
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

to_host_path() {
    path="$1"
    if [ "$RUN_CONTEXT" != "host" ]; then
        printf '%s\n' "$path"
        return
    fi
    case "$path" in
        /home/*)
            printf '/var%s\n' "$path"
            ;;
        *)
            printf '%s\n' "$path"
            ;;
    esac
}

host_file_exists() {
    flatpak-spawn --host sh -c "[ -f '$1' ]"
}

file_exists_in_context() {
    path="$1"
    if [ "$RUN_CONTEXT" = "host" ]; then
        host_file_exists "$path"
    else
        [ -f "$path" ]
    fi
}

find_ovmf_code() {
    for candidate in \
        /usr/share/edk2/ovmf/OVMF_CODE.fd \
        /usr/share/OVMF/OVMF_CODE.fd \
        /usr/share/ovmf/x64/OVMF_CODE.fd \
        /usr/share/qemu/OVMF_CODE.fd
    do
        if file_exists_in_context "$candidate"; then
            printf '%s\n' "$candidate"
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
        if file_exists_in_context "$candidate"; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    return 1
}

if ! has_local "$QEMU_BIN"; then
    if has_host "$QEMU_BIN"; then
        RUN_CONTEXT="host"
    else
        echo "Missing QEMU binary: $QEMU_BIN" >&2
        exit 1
    fi
fi

if ! has_local timeout && [ "$RUN_CONTEXT" != "host" ]; then
    echo "Missing required tool: timeout" >&2
    exit 1
fi
if [ "$RUN_CONTEXT" = "host" ] && ! has_host timeout; then
    echo "Missing required host tool: timeout" >&2
    exit 1
fi

if [ ! -f "$BOOT_IMAGE" ]; then
    echo "Missing boot image: $BOOT_IMAGE" >&2
    exit 1
fi

if [ "$ATTACH_SLOT" -eq 1 ] && [ ! -f "$SLOT_IMAGE" ]; then
    echo "Slot image not found, continuing without it: $SLOT_IMAGE" >&2
    ATTACH_SLOT=0
fi

if [ -z "$OVMF_CODE" ]; then
    OVMF_CODE="$(find_ovmf_code || true)"
fi
if [ -z "$OVMF_VARS_TEMPLATE" ]; then
    OVMF_VARS_TEMPLATE="$(find_ovmf_vars || true)"
fi

if [ -z "$OVMF_CODE" ]; then
    echo "Unable to find OVMF_CODE.fd (set --ovmf-code)." >&2
    exit 1
fi
if [ -z "$OVMF_VARS_TEMPLATE" ]; then
    echo "Unable to find OVMF_VARS.fd (set --ovmf-vars-template)." >&2
    exit 1
fi

mkdir -p "$(dirname "$SERIAL_LOG")" "$OUT_DIR"
: > "$SERIAL_LOG"
: > "$DEBUG_LOG"

HOST_BOOT_IMAGE="$(to_host_path "$BOOT_IMAGE")"
HOST_SLOT="$(to_host_path "$SLOT_IMAGE")"
HOST_SERIAL="$(to_host_path "$SERIAL_LOG")"
HOST_DEBUG="$(to_host_path "$DEBUG_LOG")"
HOST_VARS_RUNTIME="$(to_host_path "$OVMF_VARS_RUNTIME")"

run_cmd cp "$OVMF_VARS_TEMPLATE" "$HOST_VARS_RUNTIME"

QEMU_ARGS="
    -machine q35,accel=$ACCEL
    -m $MEMORY_MB
    -smp $CPUS
    -drive if=pflash,format=raw,readonly=on,file=$OVMF_CODE
    -drive if=pflash,format=raw,file=$HOST_VARS_RUNTIME
    -device virtio-scsi-pci,id=scsi0
    -drive if=none,id=hd0,format=$BOOT_FORMAT,file=$HOST_BOOT_IMAGE
    -device scsi-hd,bus=scsi0.0,drive=hd0
    -display none
    -vga std
    -serial file:$HOST_SERIAL
    -debugcon file:$HOST_DEBUG
    -global isa-debugcon.iobase=0xe9
    -monitor none
    -no-reboot
"

if [ "$ATTACH_SLOT" -eq 1 ]; then
    QEMU_ARGS="$QEMU_ARGS -drive if=virtio,format=raw,readonly=on,file=$HOST_SLOT"
fi

set +e
# shellcheck disable=SC2086
run_cmd timeout "${DURATION_SEC}s" "$QEMU_BIN" $QEMU_ARGS
rc=$?
set -e

fatal_patterns='X64 Exception Type|Kernel panic|VFS: Unable to mount root fs|BUG: unable to handle|Oops:'
if grep -Eq "$fatal_patterns" "$SERIAL_LOG"; then
    echo "QEMU smoke FAIL: fatal signature detected in serial log." >&2
    echo "Serial log tail:" >&2
    tail -n 80 "$SERIAL_LOG" >&2 || true
    exit 1
fi

marker_seen=0
if grep -Fq "StratBoot: booting slot" "$SERIAL_LOG" || \
   grep -Fq "StratBoot: booting slot" "$DEBUG_LOG"; then
    marker_seen=1
fi

if [ "$marker_seen" -eq 0 ]; then
    if [ "$RUN_CONTEXT" = "host" ]; then
        if flatpak-spawn --host sh -lc "strings -el '$HOST_VARS_RUNTIME' | grep -Fq 'STRAT_SMOKE_BOOTING_SLOT'"; then
            marker_seen=1
        fi
    else
        if strings -el "$OVMF_VARS_RUNTIME" | grep -Fq "STRAT_SMOKE_BOOTING_SLOT"; then
            marker_seen=1
        fi
    fi
fi

if [ "$marker_seen" -eq 0 ]; then
    echo "QEMU smoke FAIL: missing PASS marker 'StratBoot: booting slot' in serial log." >&2
    echo "Serial log tail:" >&2
    tail -n 80 "$SERIAL_LOG" >&2 || true
    echo "Debug log tail:" >&2
    tail -n 80 "$DEBUG_LOG" >&2 || true
    # Keep vars file for inspection on failure
    exit 1
fi

if [ "$rc" -ne 124 ]; then
    echo "QEMU smoke FAIL: exited early (rc=$rc)." >&2
    echo "Serial log tail:" >&2
    tail -n 80 "$SERIAL_LOG" >&2 || true
    run_cmd rm -f "$HOST_VARS_RUNTIME"
    exit 1
fi

echo "QEMU smoke PASS: found 'StratBoot: booting slot' and no fatal signatures."
echo "Serial log: $SERIAL_LOG"
echo "Debug log: $DEBUG_LOG"
run_cmd rm -f "$HOST_VARS_RUNTIME"
exit 0
