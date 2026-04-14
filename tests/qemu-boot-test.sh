#!/usr/bin/env sh
set -eu

DISK_IMAGE=""
CDROM_IMAGE=""
SERIAL_LOG="out/qemu-serial.log"
TIMEOUT_SEC=300
MEMORY_MB=4096
CPU_COUNT=4
ACCEL="tcg"
SUCCESS_REGEX='login:'
PANIC_REGEX='Kernel panic|Oops:|BUG:|panic:'
OVMF_CODE=""
OVMF_VARS_TEMPLATE=""
OVMF_VARS_RUNTIME=""

usage() {
    cat <<EOF
Usage: $0 --disk IMAGE_PATH [options]

Options:
  --disk PATH              Raw/qcow2 disk image to boot (required)
  --cdrom PATH             Optional ISO image to attach
  --serial-log PATH        Serial log output file (default: out/qemu-serial.log)
  --timeout SEC            Timeout in seconds (default: 300)
  --memory MB              Memory in MB (default: 4096)
  --cpus N                 Number of CPUs (default: 4)
  --accel tcg|kvm          QEMU accelerator (default: tcg)
  --success-regex REGEX    Success pattern in serial log (default: login:)
  --panic-regex REGEX      Failure pattern in serial log
  --ovmf-code PATH         OVMF code firmware path
  --ovmf-vars-template PATH
                            OVMF vars template path
  --ovmf-vars-runtime PATH Runtime OVMF vars path (copied from template if absent)
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --disk) DISK_IMAGE="$2"; shift 2 ;;
        --cdrom) CDROM_IMAGE="$2"; shift 2 ;;
        --serial-log) SERIAL_LOG="$2"; shift 2 ;;
        --timeout) TIMEOUT_SEC="$2"; shift 2 ;;
        --memory) MEMORY_MB="$2"; shift 2 ;;
        --cpus) CPU_COUNT="$2"; shift 2 ;;
        --accel) ACCEL="$2"; shift 2 ;;
        --success-regex) SUCCESS_REGEX="$2"; shift 2 ;;
        --panic-regex) PANIC_REGEX="$2"; shift 2 ;;
        --ovmf-code) OVMF_CODE="$2"; shift 2 ;;
        --ovmf-vars-template) OVMF_VARS_TEMPLATE="$2"; shift 2 ;;
        --ovmf-vars-runtime) OVMF_VARS_RUNTIME="$2"; shift 2 ;;
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

if [ -z "$DISK_IMAGE" ]; then
    echo "--disk is required." >&2
    usage >&2
    exit 1
fi

if [ ! -f "$DISK_IMAGE" ]; then
    echo "Disk image not found: $DISK_IMAGE" >&2
    exit 1
fi

if [ -n "$CDROM_IMAGE" ] && [ ! -f "$CDROM_IMAGE" ]; then
    echo "CDROM image not found: $CDROM_IMAGE" >&2
    exit 1
fi

if [ -z "$OVMF_CODE" ]; then
    for candidate in \
        /usr/share/edk2/ovmf/OVMF_CODE.fd \
        /usr/share/OVMF/OVMF_CODE.fd \
        /usr/share/ovmf/x64/OVMF_CODE.fd \
        /usr/share/qemu/OVMF_CODE.fd
    do
        if [ -f "$candidate" ]; then
            OVMF_CODE="$candidate"
            break
        fi
    done
fi

if [ -z "$OVMF_VARS_TEMPLATE" ]; then
    for candidate in \
        /usr/share/edk2/ovmf/OVMF_VARS.fd \
        /usr/share/OVMF/OVMF_VARS.fd \
        /usr/share/ovmf/x64/OVMF_VARS.fd \
        /usr/share/qemu/OVMF_VARS.fd
    do
        if [ -f "$candidate" ]; then
            OVMF_VARS_TEMPLATE="$candidate"
            break
        fi
    done
fi

if [ -z "$OVMF_CODE" ] || [ ! -f "$OVMF_CODE" ]; then
    echo "Unable to find OVMF_CODE.fd. Pass --ovmf-code PATH." >&2
    exit 1
fi

if [ -z "$OVMF_VARS_TEMPLATE" ] || [ ! -f "$OVMF_VARS_TEMPLATE" ]; then
    echo "Unable to find OVMF_VARS.fd. Pass --ovmf-vars-template PATH." >&2
    exit 1
fi

mkdir -p "$(dirname "$SERIAL_LOG")"
: > "$SERIAL_LOG"

cleanup_vars_copy=0
if [ -z "$OVMF_VARS_RUNTIME" ]; then
    OVMF_VARS_RUNTIME="$(mktemp /tmp/stratos-ovmf-vars.XXXXXX.fd)"
    cleanup_vars_copy=1
    cp "$OVMF_VARS_TEMPLATE" "$OVMF_VARS_RUNTIME"
fi

QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
if ! command -v "$QEMU_BIN" >/dev/null 2>&1; then
    echo "qemu-system-x86_64 not found." >&2
    exit 1
fi

cleanup() {
    if [ -n "${QEMU_PID-}" ] && kill -0 "$QEMU_PID" 2>/dev/null; then
        kill "$QEMU_PID" >/dev/null 2>&1 || true
        wait "$QEMU_PID" >/dev/null 2>&1 || true
    fi
    if [ "$cleanup_vars_copy" -eq 1 ] && [ -n "$OVMF_VARS_RUNTIME" ]; then
        rm -f "$OVMF_VARS_RUNTIME"
    fi
}
trap cleanup EXIT INT TERM

QEMU_ARGS="
    -machine q35,accel=$ACCEL
    -m $MEMORY_MB
    -smp $CPU_COUNT
    -drive if=pflash,format=raw,readonly=on,file=$OVMF_CODE
    -drive if=pflash,format=raw,file=$OVMF_VARS_RUNTIME
    -drive if=virtio,format=raw,file=$DISK_IMAGE
    -nographic
    -serial file:$SERIAL_LOG
    -monitor none
    -no-reboot
"

if [ -n "$CDROM_IMAGE" ]; then
    QEMU_ARGS="$QEMU_ARGS -cdrom $CDROM_IMAGE"
fi

# shellcheck disable=SC2086
"$QEMU_BIN" $QEMU_ARGS &
QEMU_PID=$!

elapsed=0
while [ "$elapsed" -lt "$TIMEOUT_SEC" ]; do
    if grep -Eq "$PANIC_REGEX" "$SERIAL_LOG"; then
        echo "QEMU boot test failed: panic pattern matched." >&2
        exit 1
    fi

    if grep -Eq "$SUCCESS_REGEX" "$SERIAL_LOG"; then
        echo "QEMU boot test passed: success pattern matched."
        exit 0
    fi

    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
        wait "$QEMU_PID" || true
        echo "QEMU exited before success/panic pattern matched." >&2
        echo "Serial log tail:" >&2
        tail -n 50 "$SERIAL_LOG" >&2 || true
        exit 1
    fi

    sleep 1
    elapsed=$((elapsed + 1))
done

echo "QEMU boot test timed out after ${TIMEOUT_SEC}s." >&2
echo "Serial log tail:" >&2
tail -n 50 "$SERIAL_LOG" >&2 || true
exit 1
