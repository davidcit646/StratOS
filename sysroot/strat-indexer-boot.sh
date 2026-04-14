#!/bin/sh
set -eu

DISABLE_FLAG="/config/strat/disable-indexer"
PID_FILE="/run/stratterm-indexer.pid"
LOG_FILE="/run/stratterm-indexer.log"

if [ "${STRAT_INDEXER_DISABLE:-0}" = "1" ]; then
    exit 0
fi

if [ -f "$DISABLE_FLAG" ]; then
    exit 0
fi

if [ ! -x /bin/stratterm-indexer ]; then
    exit 0
fi

mkdir -p /run

if [ -f "$PID_FILE" ]; then
    old_pid="$(cat "$PID_FILE" 2>/dev/null || true)"
    if [ -n "${old_pid:-}" ] && kill -0 "$old_pid" 2>/dev/null; then
        exit 0
    fi
fi

/bin/stratterm-indexer --boot-daemon >>"$LOG_FILE" 2>&1 &
echo "$!" > "$PID_FILE"

exit 0
