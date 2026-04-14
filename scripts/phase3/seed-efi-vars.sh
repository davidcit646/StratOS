#!/usr/bin/env sh
set -eu

EFI_DIR="/sys/firmware/efi/efivars"
DRY_RUN=0

GUID="10731b6f-16b5-4aea-ab46-c62aa093c8e5"

usage() {
    cat <<USAGE
Usage: $0 [--efi-dir PATH] [--dry-run]

Seeds StratOS EFI variables with sane first-boot defaults.
Defaults:
  STRAT_SLOT_A_STATUS=1 (confirmed)
  STRAT_SLOT_B_STATUS=0 (staging)
  STRAT_SLOT_C_STATUS=0 (staging)
  STRAT_ACTIVE_SLOT=0 (A)
  STRAT_PINNED_SLOT=255 (none)
  STRAT_RESET_FLAGS=0
  STRAT_BOOT_COUNT=0
  STRAT_LAST_GOOD_SLOT=0 (A)
  STRAT_HOME_STATUS=0 (healthy)
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --efi-dir)
            EFI_DIR="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=1
            shift
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

if [ ! -d "$EFI_DIR" ]; then
    echo "EFI var dir does not exist: $EFI_DIR" >&2
    exit 1
fi

write_var() {
    name="$1"
    value="$2"

    case "$value" in
        ''|*[!0-9]*)
            echo "Invalid numeric value for $name: $value" >&2
            exit 1
            ;;
    esac
    if [ "$value" -lt 0 ] || [ "$value" -gt 255 ]; then
        echo "Value out of range for $name: $value" >&2
        exit 1
    fi

    path="$EFI_DIR/$name-$GUID"
    if [ -L "$path" ]; then
        echo "Refusing to write through symlink: $path" >&2
        exit 1
    fi

    if [ "$DRY_RUN" -eq 1 ]; then
        echo "DRY-RUN: $name=$value -> $path"
        return
    fi

    value_oct=$(printf '%03o' "$value")
    # efivarfs payload format: 4-byte attributes + 1-byte value.
    {
        printf '\007\000\000\000'
        printf "\\$value_oct"
    } > "$path"

    echo "seeded: $name=$value"
}

write_var "STRAT_SLOT_A_STATUS" 1
write_var "STRAT_SLOT_B_STATUS" 0
write_var "STRAT_SLOT_C_STATUS" 0
write_var "STRAT_ACTIVE_SLOT" 0
write_var "STRAT_PINNED_SLOT" 255
write_var "STRAT_RESET_FLAGS" 0
write_var "STRAT_BOOT_COUNT" 0
write_var "STRAT_LAST_GOOD_SLOT" 0
write_var "STRAT_HOME_STATUS" 0

echo "done: StratOS EFI variables seeded"
