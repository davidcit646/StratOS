#!/bin/sh
# Text-mode install walkthrough for StratOS live ISO (virtual console, e.g. tty2).
# strat-installer performs the final typed confirmation phrase.

set -f

clear_screen() {
    printf '\033[2J\033[H'
}

die() {
    echo "strat-live-install-wizard: $*" >&2
    exit 1
}

require_live() {
    case "$(cat /proc/cmdline 2>/dev/null)" in
    *strat.live=1*) return 0 ;;
    esac
    die "This wizard is for StratOS live (kernel cmdline strat.live=1) only."
}

current_uid() {
    uid=""
    if command -v id >/dev/null 2>&1; then
        uid=$(id -u 2>/dev/null || true)
    fi
    if [ -z "$uid" ] && [ -r /proc/self/status ]; then
        uid=$(awk '/^Uid:/ {print $2; exit}' /proc/self/status 2>/dev/null || true)
    fi
    echo "${uid:-1}"
}

require_root() {
    uid=$(current_uid)
    [ "${uid:-1}" -eq 0 ] && return 0

    echo "strat-live-install-wizard: root required, attempting elevation..." >&2
    if command -v sudo >/dev/null 2>&1; then
        exec sudo -E /bin/strat-live-install-wizard "$@"
    fi
    if command -v doas >/dev/null 2>&1; then
        exec doas /bin/strat-live-install-wizard "$@"
    fi
    if command -v pkexec >/dev/null 2>&1; then
        exec pkexec /bin/strat-live-install-wizard "$@"
    fi
    die "must run as root (no sudo/doas/pkexec available)"
}

pause_sh() {
    echo ""
    printf 'Press Enter to continue... '
    read -r _dummy || true
}

pick_tool_source_dialog() {
    command -v dialog >/dev/null 2>&1 || return 1
    dialog --stdout --menu \
        "Optional tool source (stable mode). Choose where to install from later:\n\nnone = skip optional tool install request." \
        15 76 6 \
        none "No optional tools" \
        apt "APT packages" \
        pacman "Pacman packages" \
        cargo "Cargo crates"
}

pick_tool_categories_dialog() {
    command -v dialog >/dev/null 2>&1 || return 1
    sel=$(dialog --stdout --separate-output --checklist \
        "Optional tool categories (stable). Leave all unchecked for no category filter." \
        22 80 14 \
        disk-filesystem "duf, dust, ncdu, diskonaut" off \
        file-navigation "yazi, lf, ranger, zoxide, fzf, tree" off \
        editor "helix, micro, nano, vim" off \
        search "grep, ripgrep, fd, bat, ag" off \
        process-system "bottom, procs, htop, btop, glances, iotop" off \
        network "bandwhich, nethogs, iftop, nmap, curl, wget" off \
        git-tui "lazygit, gitui, tig, delta" off \
        text-data "jq, yq, glow, mdcat, tealdeer, hexyl" off \
        shell "fish, tmux, zellij, starship, direnv" off \
        system-info "fastfetch, onefetch, inxi, hwinfo" off \
        benchmarking "hyperfine, sysbench" off \
        compression "zstd, xz" off \
        security-crypto "age, gpg" off \
        build-dev "git, make, gcc, clang" off) || return 1

    cats=""
    for c in $sel; do
        if [ -z "$cats" ]; then
            cats="$c"
        else
            cats="${cats},$c"
        fi
    done
    echo "$cats"
}

pick_tool_list_dialog() {
    command -v dialog >/dev/null 2>&1 || return 1
    dialog --stdout --inputbox \
        "Optional exact tool IDs (comma-separated, no spaces), or blank to skip.\nExample: helix,ripgrep,fd,zoxide" \
        11 80 ""
}

pick_disk_dialog() {
    command -v dialog >/dev/null 2>&1 || return 1
    d=""
    while [ -z "$d" ]; do
        d=$(dialog --stdout --inputbox \
            "Whole-disk device to install StratOS (e.g. /dev/nvme0n1 or /dev/sda).\n\nNOT the USB/CD you booted from." \
            12 70 "") || return 1
        d=$(echo "$d" | tr -d '[:space:]')
    done
    dialog --title "Confirm" --yesno "Wipe and install to:\n\n  $d\n\nAll data on that disk will be destroyed." 12 60 || return 1
    echo "$d"
}

pick_disk_plain() {
    while true; do
        d=""
        while [ -z "$d" ]; do
            echo ""
            echo "Enter the WHOLE DISK to install StratOS (e.g. /dev/nvme0n1 or /dev/sda)."
            echo "Do NOT use the live USB or optical drive."
            printf '/dev/...> '
            read -r d || exit 1
            d=$(echo "$d" | tr -d '[:space:]')
        done
        echo ""
        echo "You entered: $d"
        printf 'Type YES (uppercase) to continue, or anything else to re-pick: '
        read -r ok || exit 1
        [ "$ok" = "YES" ] && break
    done
    echo "$d"
}

pick_tools_dialog() {
    TOOL_SOURCE_CHOICE="none"
    TOOL_CATEGORIES_CHOICE=""
    TOOL_LIST_CHOICE=""

    command -v dialog >/dev/null 2>&1 || return 1
    dialog --title "Optional tools" --yesno \
        "Configure optional stable tool install request?\n\nThis does NOT change the base OS image.\nSelections are saved in CONFIG for post-install apply." \
        13 74 || return 0

    src=$(pick_tool_source_dialog) || return 1
    src=$(echo "$src" | tr -d '[:space:]')
    [ -n "$src" ] || src="none"
    TOOL_SOURCE_CHOICE="$src"

    if [ "$TOOL_SOURCE_CHOICE" != "none" ]; then
        cats=$(pick_tool_categories_dialog) || return 1
        ids=$(pick_tool_list_dialog) || return 1
        ids=$(echo "$ids" | tr -d '[:space:]')
        TOOL_CATEGORIES_CHOICE="$cats"
        TOOL_LIST_CHOICE="$ids"
    fi
    return 0
}

pick_tools_plain() {
    TOOL_SOURCE_CHOICE="none"
    TOOL_CATEGORIES_CHOICE=""
    TOOL_LIST_CHOICE=""

    echo ""
    printf 'Configure optional stable tool install request? [y/N]: '
    read -r yn || exit 1
    case "$yn" in
        y|Y|yes|YES) ;;
        *) return 0 ;;
    esac

    echo "Source options: none, apt, pacman, cargo"
    while true; do
        printf 'Tool source> '
        read -r src || exit 1
        src=$(echo "$src" | tr -d '[:space:]')
        [ -z "$src" ] && src="none"
        case "$src" in
            none|apt|pacman|cargo) TOOL_SOURCE_CHOICE="$src"; break ;;
            *) echo "Invalid source. Use one of: none, apt, pacman, cargo" ;;
        esac
    done

    if [ "$TOOL_SOURCE_CHOICE" != "none" ]; then
        echo "Categories (comma-separated, optional):"
        echo "  disk-filesystem,file-navigation,editor,search,process-system,network,git-tui,text-data,shell,system-info,benchmarking,compression,security-crypto,build-dev"
        printf 'Categories> '
        read -r cats || exit 1
        cats=$(echo "$cats" | tr -d '[:space:]')

        echo "Exact tool IDs (comma-separated, optional), e.g. helix,ripgrep,fd,zoxide"
        printf 'Tool IDs> '
        read -r ids || exit 1
        ids=$(echo "$ids" | tr -d '[:space:]')

        TOOL_CATEGORIES_CHOICE="$cats"
        TOOL_LIST_CHOICE="$ids"
    fi
    return 0
}

main() {
    require_live
    require_root "$@"

    [ -x /bin/strat-installer ] || die "/bin/strat-installer missing — rebuild the live image."

    if command -v dialog >/dev/null 2>&1; then
        dialog --title "StratOS live" --msgbox \
            "Install StratOS to one internal disk (GPT wipe + EROFS + ESP).\n\nNext: device list, then you choose the disk.\n\nstrat-installer will ask for a final typed confirmation phrase." \
            14 70 || true
    else
        clear_screen
        echo "======== StratOS live — install to disk ========"
        echo ""
        echo "This installs StratOS to ONE whole internal disk (destructive)."
        echo "strat-installer will ask for a final confirmation phrase at the end."
        pause_sh
    fi

    tmpf=$(mktemp /tmp/strat-lsblk.XXXXXX) || die "mktemp failed"
    lsblk -o NAME,SIZE,TYPE,MODEL,MOUNTPOINTS 2>/dev/null >"$tmpf" || lsblk >"$tmpf" || true

    if command -v dialog >/dev/null 2>&1; then
        dialog --title "lsblk" --textbox "$tmpf" 22 78 || true
    else
        clear_screen
        echo "======== lsblk ========"
        cat "$tmpf"
        echo "======================="
        pause_sh
    fi
    rm -f "$tmpf"

    if command -v dialog >/dev/null 2>&1; then
        disk=$(pick_disk_dialog) || die "cancelled"
    else
        disk=$(pick_disk_plain)
    fi

    [ -b "$disk" ] || die "not a block device: $disk"

    if command -v dialog >/dev/null 2>&1; then
        pick_tools_dialog || die "cancelled"
    else
        pick_tools_plain
    fi

    echo ""
    echo "Starting strat-installer..."
    set -- /bin/strat-installer --disk "$disk" --tool-mode stable --tool-source "$TOOL_SOURCE_CHOICE"
    if [ -n "$TOOL_CATEGORIES_CHOICE" ]; then
        set -- "$@" --tool-categories "$TOOL_CATEGORIES_CHOICE"
    fi
    if [ -n "$TOOL_LIST_CHOICE" ]; then
        set -- "$@" --tool-list "$TOOL_LIST_CHOICE"
    fi
    exec "$@"
}

main "$@"
