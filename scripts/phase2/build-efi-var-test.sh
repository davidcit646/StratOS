#!/usr/bin/env sh
set -eu

REPO_ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
SRC_DIR="$REPO_ROOT/stratboot/tests"
EFI_DIR="$REPO_ROOT/stratboot/efi"
OUT_DIR="$REPO_ROOT/out/phase2"

RUN_CONTEXT="local"

usage() {
    cat <<EOF
Usage: $0

Builds the StratOS EFI variable test app (x86_64).
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

run_cmd() {
    if [ "$RUN_CONTEXT" = "host" ]; then
        flatpak-spawn --host "$@"
        return
    fi
    "$@"
}

detect_run_context() {
    if command -v x86_64-linux-gnu-gcc >/dev/null 2>&1; then
        RUN_CONTEXT="local"
        return 0
    fi
    if command -v flatpak-spawn >/dev/null 2>&1 &&
       flatpak-spawn --host sh -lc 'command -v x86_64-linux-gnu-gcc >/dev/null 2>&1'; then
        RUN_CONTEXT="host"
        return 0
    fi
    echo "x86_64-linux-gnu-gcc not found." >&2
    return 1
}

detect_path() {
    name="$1"
    for path in /usr/lib64/gnuefi /usr/lib/gnuefi /usr/lib/x86_64-linux-gnu/gnuefi /usr/lib; do
        if run_cmd sh -lc "[ -e '$path/$name' ]"; then
            echo "$path/$name"
            return 0
        fi
    done
    return 1
}

detect_include_base() {
    for path in /usr/include/efi /usr/include/gnu-efi; do
        if run_cmd sh -lc "[ -d '$path' ]"; then
            echo "$path"
            return 0
        fi
    done
    return 1
}

detect_run_context

EFI_LDS="$(detect_path elf_x86_64_efi.lds || true)"
if [ -z "$EFI_LDS" ]; then
    echo "Unable to locate elf_x86_64_efi.lds (gnu-efi dev files)." >&2
    exit 1
fi

EFI_LIB_DIR="$(dirname "$EFI_LDS")"
EFI_INC_BASE="$(detect_include_base)"
EFI_INC_ARCH="$EFI_INC_BASE/x86_64"
EFI_INC_PROTO="$EFI_INC_BASE/protocol"
EXTRA_INC_FLAGS=""

if run_cmd sh -lc '[ -f /usr/include/stdint.h ]'; then
    EXTRA_INC_FLAGS="$EXTRA_INC_FLAGS -isystem /usr/include"
fi

NATIVE_GCC_INC="$(run_cmd sh -lc 'gcc -print-file-name=include 2>/dev/null || true')"
if [ -n "$NATIVE_GCC_INC" ] && run_cmd sh -lc "[ -f '$NATIVE_GCC_INC/stdint.h' ]"; then
    EXTRA_INC_FLAGS="$EXTRA_INC_FLAGS -isystem $NATIVE_GCC_INC"
fi

run_cmd mkdir -p "$OUT_DIR"

SRC_MAIN="$SRC_DIR/efi_var_test.c"
SRC_LIB="$EFI_DIR/strat_efi_vars.c"
OBJ_MAIN="$OUT_DIR/efi_var_test.o"
OBJ_LIB="$OUT_DIR/strat_efi_vars.o"
SO_FILE="$OUT_DIR/efi_var_test.so"
EFI_FILE="$OUT_DIR/efi_var_test.efi"
LD_CMD=""
USE_GCC_LINK=0

if run_cmd sh -lc 'command -v x86_64-linux-gnu-ld >/dev/null 2>&1'; then
    LD_CMD="x86_64-linux-gnu-ld"
elif run_cmd sh -lc 'command -v x86_64-linux-gnu-ld.bfd >/dev/null 2>&1'; then
    LD_CMD="x86_64-linux-gnu-ld.bfd"
elif run_cmd sh -lc 'command -v x86_64-linux-gnu-gcc >/dev/null 2>&1'; then
    USE_GCC_LINK=1
else
    echo "No suitable linker found (x86_64-linux-gnu-ld or x86_64-linux-gnu-gcc)." >&2
    exit 1
fi

run_cmd x86_64-linux-gnu-gcc \
    -I"$EFI_INC_ARCH" \
    -I"$EFI_INC_BASE" \
    -I"$EFI_INC_PROTO" \
    $EXTRA_INC_FLAGS \
    -fpic -fshort-wchar -mno-red-zone -Wall -Wextra -Wno-error=implicit-function-declaration \
    -DEFI_FUNCTION_WRAPPER \
    -c "$SRC_LIB" \
    -o "$OBJ_LIB"

run_cmd x86_64-linux-gnu-gcc \
    -I"$EFI_INC_ARCH" \
    -I"$EFI_INC_BASE" \
    -I"$EFI_INC_PROTO" \
    $EXTRA_INC_FLAGS \
    -fpic -fshort-wchar -mno-red-zone -Wall -Wextra -Wno-error=implicit-function-declaration \
    -DEFI_FUNCTION_WRAPPER \
    -c "$SRC_MAIN" \
    -o "$OBJ_MAIN"

if [ "$USE_GCC_LINK" -eq 1 ]; then
    run_cmd x86_64-linux-gnu-gcc \
        -nostdlib \
        -Wl,-znocombreloc \
        -Wl,-T,"$EFI_LDS" \
        -Wl,-shared -Wl,-Bsymbolic \
        -L"$EFI_LIB_DIR" -L/usr/lib64 -L/usr/lib \
        -lgnuefi -lefi \
        -o "$SO_FILE" \
        "$OBJ_LIB" "$OBJ_MAIN"
else
    run_cmd "$LD_CMD" \
        -nostdlib -znocombreloc \
        -T "$EFI_LDS" \
        -shared -Bsymbolic \
        -L"$EFI_LIB_DIR" -L/usr/lib64 -L/usr/lib \
        -lgnuefi -lefi \
        -o "$SO_FILE" \
        "$OBJ_LIB" "$OBJ_MAIN"
fi

ELF2EFI=""
for candidate in /usr/bin/elf2efi /usr/lib/gnu-efi/elf2efi /usr/lib64/gnu-efi/elf2efi; do
    if run_cmd sh -lc "[ -x '$candidate' ]"; then
        ELF2EFI="$candidate"
        break
    fi
done

if [ -n "$ELF2EFI" ]; then
    run_cmd "$ELF2EFI" "$SO_FILE" "$EFI_FILE"
else
    run_cmd x86_64-linux-gnu-objcopy \
        -j .text -j .sdata -j .data \
        -j .dynamic -j .dynsym -j .rel \
        -j .rela -j .reloc -j .eh_frame \
        --target efi-app-x86_64 \
        --subsystem 10 \
        "$SO_FILE" "$EFI_FILE"
fi

echo "$EFI_FILE"
