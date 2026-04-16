#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
PROJECT_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

verify_mkfs_erofs() {
    if command -v mkfs.erofs >/dev/null 2>&1; then
        echo "PASS: mkfs.erofs found"
        return 0
    else
        echo "FAIL: mkfs.erofs not found (install erofs-utils)" >&2
        exit 1
    fi
}

verify_erofsfuse() {
    if command -v erofsfuse >/dev/null 2>&1; then
        echo "PASS: erofsfuse found"
        return 0
    else
        echo "WARN: erofsfuse not found (optional, for mounting EROFS images)" >&2
        return 0
    fi
}

main() {
    echo "=== StratOS Phase 1: EROFS Tooling Verification ==="
    
    verify_mkfs_erofs
    verify_erofsfuse
    
    echo ""
    echo "=== EROFS Tooling Verification Complete ==="
}

main "$@"
