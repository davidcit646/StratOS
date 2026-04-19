#!/usr/bin/env bash
# Download wlroots release tarball into third_party/ (gitignored). Idempotent.
# Usage: ./scripts/fetch-wlroots.sh [VERSION]
#   VERSION default: 0.19.3 (must match stratvm API — see stratvm/Makefile)

set -euo pipefail

WLROOTS_VER="${1:-0.19.3}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${REPO_ROOT}/third_party/wlroots-${WLROOTS_VER}"
STAMP="${DEST}/.stratos-fetched"
TARBALL="${REPO_ROOT}/third_party/wlroots-${WLROOTS_VER}.tar.bz2"
# Same archive as documented in e.g. Linux From Scratch supplemental wlroots page.
URL="https://gitlab.freedesktop.org/wlroots/wlroots/-/archive/${WLROOTS_VER}/wlroots-${WLROOTS_VER}.tar.bz2"

if [[ -f "${DEST}/meson.build" ]]; then
	echo "wlroots ${WLROOTS_VER} already present: ${DEST}"
	exit 0
fi

mkdir -p "${REPO_ROOT}/third_party"

if [[ ! -f "${TARBALL}" ]]; then
	echo "Downloading wlroots ${WLROOTS_VER}..."
	if command -v curl >/dev/null 2>&1; then
		curl -fL --retry 3 --retry-delay 2 -o "${TARBALL}.part" "${URL}"
	elif command -v wget >/dev/null 2>&1; then
		wget -O "${TARBALL}.part" "${URL}"
	else
		echo "Need curl or wget to download wlroots." >&2
		exit 1
	fi
	mv "${TARBALL}.part" "${TARBALL}"
fi

echo "Extracting ${TARBALL}..."
tar -xjf "${TARBALL}" -C "${REPO_ROOT}/third_party"
touch "${STAMP}"
echo "wlroots ${WLROOTS_VER} ready at ${DEST}"
