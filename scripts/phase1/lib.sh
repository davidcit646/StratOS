#!/usr/bin/env sh
set -eu

# Shared helpers for Phase 1 scripts.

RUN_CONTEXT="${RUN_CONTEXT:-local}"

phase1_detect_run_context() {
    if [ "$#" -eq 0 ]; then
        RUN_CONTEXT="local"
        return 0
    fi

    all_local=1
    for cmd in "$@"; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            all_local=0
            break
        fi
    done

    if [ "$all_local" -eq 1 ]; then
        RUN_CONTEXT="local"
        return 0
    fi

    if ! command -v flatpak-spawn >/dev/null 2>&1; then
        echo "Missing required command(s) locally and flatpak-spawn is unavailable." >&2
        return 1
    fi

    for cmd in "$@"; do
        if ! flatpak-spawn --host sh -lc "command -v $cmd >/dev/null 2>&1"; then
            echo "Required command not found: $cmd" >&2
            return 1
        fi
    done

    RUN_CONTEXT="host"
}

phase1_run_cmd() {
    if [ "$RUN_CONTEXT" = "host" ]; then
        flatpak-spawn --host "$@"
        return
    fi
    "$@"
}

phase1_run_root_cmd() {
    if phase1_run_cmd sh -lc '[ "$(id -u)" -eq 0 ]'; then
        phase1_run_cmd "$@"
        return
    fi

    if phase1_run_cmd sh -lc 'command -v sudo >/dev/null 2>&1'; then
        phase1_run_cmd sudo "$@"
        return
    fi

    echo "Root privileges are required. Re-run as root or with sudo configured." >&2
    return 1
}

phase1_ensure_image_exists() {
    image_path="$1"
    if ! phase1_run_cmd sh -lc "[ -f '$image_path' ]"; then
        echo "Disk image not found: $image_path" >&2
        return 1
    fi
}

phase1_wait_for_block_dev() {
    block_dev="$1"
    retries=25
    i=0
    while [ "$i" -lt "$retries" ]; do
        if phase1_run_cmd sh -lc "[ -b '$block_dev' ]"; then
            return 0
        fi
        sleep 0.2
        i=$((i + 1))
    done
    echo "Timed out waiting for block device: $block_dev" >&2
    return 1
}

phase1_resolve_partition_dev() {
    loop_dev="$1"
    part_num="$2"

    with_p="${loop_dev}p${part_num}"
    if phase1_wait_for_block_dev "$with_p"; then
        echo "$with_p"
        return 0
    fi

    without_p="${loop_dev}${part_num}"
    if phase1_wait_for_block_dev "$without_p"; then
        echo "$without_p"
        return 0
    fi

    echo "Unable to resolve partition device for ${loop_dev} partition ${part_num}" >&2
    return 1
}

phase1_attach_loop() {
    image_path="$1"
    loop_dev="$(phase1_run_root_cmd losetup --find --show --partscan "$image_path")"
    phase1_run_root_cmd partprobe "$loop_dev" >/dev/null 2>&1 || true
    echo "$loop_dev"
}

phase1_detach_loop() {
    loop_dev="$1"
    if [ -n "$loop_dev" ]; then
        phase1_run_root_cmd losetup -d "$loop_dev" >/dev/null 2>&1 || true
    fi
}
