#!/bin/sh
# Live session only: open the install wizard in a stratterm window after the compositor is up.

case "$(cat /proc/cmdline 2>/dev/null)" in
*strat.live=1*) ;;
*) exit 0 ;;
esac

[ -x /bin/stratterm ] || exit 0
[ -x /bin/strat-live-install-wizard ] || exit 0

exec /bin/stratterm --exec /bin/strat-live-install-wizard
