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

require_root() {
    [ "$(id -u)" -eq 0 ] || die "must run as root"
}

pause_sh() {
    echo ""
    printf 'Press Enter to continue... '
    read -r _dummy || true
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

main() {
    require_live
    require_root

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

    echo ""
    echo "Starting strat-installer..."
    exec /bin/strat-installer --disk "$disk"
}

main "$@"
