#!/bin/sh
# StratOS: run wpa_supplicant when Wi-Fi hardware and /config/strat/wpa_supplicant.conf are ready.
set -eu

WPA_BIN="${STRAT_WPA_SUPPLICANT:-/usr/sbin/wpa_supplicant}"
CONF="${STRAT_WPA_CONF:-/config/strat/wpa_supplicant.conf}"
RFKILL="${STRAT_RFKILL:-/usr/sbin/rfkill}"

idle_forever() {
	echo "strat-wpa: $1 — sleeping."
	exec sleep infinity
}

has_network_block() {
	# Non-comment line containing network={...} start (simple check)
	grep -q '^[[:space:]]*network[[:space:]]*=' "$1" 2>/dev/null
}

find_wifi_iface() {
	for n in /sys/class/net/*; do
		[ -e "$n" ] || continue
		base=$(basename "$n")
		case "$base" in lo|lo0) continue ;; esac
		if [ -e "$n/phy80211" ] || [ -e "$n/wireless" ]; then
			echo "$base"
			return 0
		fi
	done
	return 1
}

if [ ! -x "$WPA_BIN" ]; then
	idle_forever "wpa_supplicant not installed (build host had no wpa_supplicant)"
fi

if [ ! -f "$CONF" ] || [ ! -s "$CONF" ]; then
	idle_forever "missing or empty $CONF"
fi

if ! has_network_block "$CONF"; then
	idle_forever "no active network={} block in $CONF (uncomment template or add SSID)"
fi

mkdir -p /run/wpa_supplicant

i=0
while [ "$i" -lt 120 ]; do
	IFACE=$(find_wifi_iface || true)
	if [ -n "${IFACE:-}" ]; then
		if command -v "$RFKILL" >/dev/null 2>&1; then
			"$RFKILL" unblock wifi 2>/dev/null || true
		fi
		echo "strat-wpa: starting wpa_supplicant on $IFACE"
		exec "$WPA_BIN" -i "$IFACE" -c "$CONF" -Dnl80211,wext
	fi
	i=$((i + 1))
	sleep 1
done

idle_forever "no Wi-Fi interface after 120s (USB Wi-Fi may need more time — reboot or add interface)"
