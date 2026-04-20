#!/bin/sh
# Filter the StratOS tool source catalog.
#
# Examples:
#   scripts/strat-tool-catalog.sh --source apt --categories editor,search
#   scripts/strat-tool-catalog.sh --source pacman --tools helix,ripgrep,fd --format install-cmd
#   scripts/strat-tool-catalog.sh --source upstream --categories network --format values

set -eu

usage() {
    echo "Usage: $0 --source {cargo|apt|pacman|upstream} [options]"
    echo "Options:"
    echo "  --catalog PATH       TSV catalog path (default: auto-detect)"
    echo "  --mode MODE          catalog mode (default: stable; only stable supported)"
    echo "  --categories CSV     category filter (comma-separated)"
    echo "  --tools CSV          tool id filter (comma-separated)"
    echo "  --format FMT         values|tools|rows|install-cmd (default: values)"
    echo "  -h, --help"
}

csv_has() {
    needle="$1"
    hay="$2"
    [ -n "$hay" ] || return 1
    old_ifs="$IFS"
    IFS=','
    for it in $hay; do
        [ "$it" = "$needle" ] && IFS="$old_ifs" && return 0
    done
    IFS="$old_ifs"
    return 1
}

find_catalog() {
    if [ -n "$CATALOG" ]; then
        [ -r "$CATALOG" ] || { echo "catalog not readable: $CATALOG" >&2; exit 1; }
        echo "$CATALOG"
        return 0
    fi
    if [ -r "/usr/share/strat/tools/tool-source-catalog.tsv" ]; then
        echo "/usr/share/strat/tools/tool-source-catalog.tsv"
        return 0
    fi
    if [ -r "./scripts/tool-source-catalog.tsv" ]; then
        echo "./scripts/tool-source-catalog.tsv"
        return 0
    fi
    echo "tool catalog not found (tried /usr/share/strat/tools/tool-source-catalog.tsv and ./scripts/tool-source-catalog.tsv)" >&2
    exit 1
}

SOURCE=""
MODE="stable"
CATEGORIES=""
TOOLS=""
FORMAT="values"
CATALOG=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --source) SOURCE="$2"; shift 2 ;;
        --mode) MODE="$2"; shift 2 ;;
        --categories) CATEGORIES="$2"; shift 2 ;;
        --tools) TOOLS="$2"; shift 2 ;;
        --format) FORMAT="$2"; shift 2 ;;
        --catalog) CATALOG="$2"; shift 2 ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

[ -n "$SOURCE" ] || { echo "missing required --source" >&2; usage >&2; exit 1; }
case "$SOURCE" in
    cargo|apt|pacman|upstream) ;;
    *) echo "invalid --source '$SOURCE'" >&2; exit 1 ;;
esac
case "$MODE" in
    stable) ;;
    *) echo "unsupported --mode '$MODE' (supported: stable)" >&2; exit 1 ;;
esac
case "$FORMAT" in
    values|tools|rows|install-cmd) ;;
    *) echo "invalid --format '$FORMAT'" >&2; exit 1 ;;
esac

CAT_FILE=$(find_catalog)
TAB=$(printf '\t')

ROWS=""
TOOLS_OUT=""
VALUES_OUT=""

while IFS="$TAB" read -r category tool cargo apt pacman upstream || [ -n "${category:-}" ]; do
    case "${category:-}" in
        ""|\#*) continue ;;
    esac

    if [ -n "$CATEGORIES" ] && ! csv_has "$category" "$CATEGORIES"; then
        continue
    fi
    if [ -n "$TOOLS" ] && ! csv_has "$tool" "$TOOLS"; then
        continue
    fi

    case "$SOURCE" in
        cargo) value="$cargo" ;;
        apt) value="$apt" ;;
        pacman) value="$pacman" ;;
        upstream) value="$upstream" ;;
    esac

    [ -n "$value" ] || continue
    [ "$value" = "-" ] && continue

    row="$category${TAB}$tool${TAB}$value"
    if [ -z "$ROWS" ]; then
        ROWS="$row"
        TOOLS_OUT="$tool"
        VALUES_OUT="$value"
    else
        ROWS="$ROWS
$row"
        TOOLS_OUT="$TOOLS_OUT
$tool"
        VALUES_OUT="$VALUES_OUT
$value"
    fi
done < "$CAT_FILE"

case "$FORMAT" in
    values)
        [ -n "$VALUES_OUT" ] && printf '%s\n' "$VALUES_OUT"
        ;;
    tools)
        [ -n "$TOOLS_OUT" ] && printf '%s\n' "$TOOLS_OUT"
        ;;
    rows)
        [ -n "$ROWS" ] && printf '%s\n' "$ROWS"
        ;;
    install-cmd)
        case "$SOURCE" in
            apt)
                cmd="apt install -y"
                if [ -n "$VALUES_OUT" ]; then
                    OLDIFS="$IFS"
                    IFS='
'
                    for v in $VALUES_OUT; do
                        [ -n "$v" ] && cmd="$cmd $v"
                    done
                    IFS="$OLDIFS"
                fi
                echo "$cmd"
                ;;
            pacman)
                cmd="pacman -S --needed"
                if [ -n "$VALUES_OUT" ]; then
                    OLDIFS="$IFS"
                    IFS='
'
                    for v in $VALUES_OUT; do
                        [ -n "$v" ] && cmd="$cmd $v"
                    done
                    IFS="$OLDIFS"
                fi
                echo "$cmd"
                ;;
            cargo)
                if [ -n "$VALUES_OUT" ]; then
                    OLDIFS="$IFS"
                    IFS='
'
                    for v in $VALUES_OUT; do
                        [ -n "$v" ] && echo "cargo install $v"
                    done
                    IFS="$OLDIFS"
                fi
                ;;
            upstream)
                [ -n "$VALUES_OUT" ] && printf '%s\n' "$VALUES_OUT"
                ;;
        esac
        ;;
esac
