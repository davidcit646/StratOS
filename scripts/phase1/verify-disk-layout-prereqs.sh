#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

. "$SCRIPT_DIR/lib.sh"

verify_qemu_img() {
    phase1_detect_run_context qemu-img
    
    if phase1_run_cmd sh -lc 'command -v qemu-img >/dev/null 2>&1'; then
        echo "PASS: qemu-img found"
        return 0
    else
        echo "FAIL: qemu-img not found" >&2
        exit 1
    fi
}

verify_sgdisk() {
    phase1_detect_run_context sgdisk
    
    if phase1_run_cmd sh -lc 'command -v sgdisk >/dev/null 2>&1'; then
        echo "PASS: sgdisk found"
        return 0
    else
        echo "FAIL: sgdisk not found" >&2
        exit 1
    fi
}

main() {
    echo "=== StratOS Phase 1: Disk Layout Prerequisites Verification ==="
    
    verify_qemu_img
    verify_sgdisk
    
    echo ""
    echo "=== Disk Layout Prerequisites Verification Complete ==="
}

main "$@"
