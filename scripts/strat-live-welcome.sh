#!/bin/sh
# StratOS live session only: tell the user about the text-mode installer on VT2 and start it there.
# Exits immediately so stratwm can start; the wizard runs on a separate virtual console.

set -f

live=0
case "$(cat /proc/cmdline 2>/dev/null)" in
*strat.live=1*) live=1 ;;
esac
[ "$live" -eq 1 ] || exit 0

msg1="StratOS live: the graphical session is starting on this console."
msg2="If the keyboard or mouse does not work there, switch to the text-mode installer:"
msg3="  Press Alt+F2  (or Ctrl+Alt+F2 on some systems)"
msg4="Then follow the prompts to install StratOS to an internal disk."

for try in /dev/tty1 /dev/console /dev/tty0; do
    if [ -c "$try" ]; then
        {
            printf '\n%s\n%s\n%s\n%s\n\n' "$msg1" "$msg2" "$msg3" "$msg4"
        } >"$try" 2>/dev/null || true
    fi
done

WIZ=/bin/strat-live-install-wizard
[ -x "$WIZ" ] || exit 0

# Prefer util-linux openvt; some images only have busybox.
if command -v openvt >/dev/null 2>&1; then
    openvt -c 2 -- "$WIZ" &
fi

exit 0
