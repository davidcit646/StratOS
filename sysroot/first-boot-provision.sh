#!/bin/sh

if [ -f /config/.provisioned ]; then
    echo "first-boot: already provisioned, skipping"
    exit 0
fi

mkdir -p /config/var
rc=$?
if [ "$rc" -ne 0 ]; then
    echo "first-boot: failed at step mkdir -p /config/var: exit $rc"
    exit 1
fi

mkdir -p /config/var/log
rc=$?
if [ "$rc" -ne 0 ]; then
    echo "first-boot: failed at step mkdir -p /config/var/log: exit $rc"
    exit 1
fi

mkdir -p /config/var/lib
rc=$?
if [ "$rc" -ne 0 ]; then
    echo "first-boot: failed at step mkdir -p /config/var/lib: exit $rc"
    exit 1
fi

mkdir -p /config/var/tmp
rc=$?
if [ "$rc" -ne 0 ]; then
    echo "first-boot: failed at step mkdir -p /config/var/tmp: exit $rc"
    exit 1
fi

mkdir -p /config/etc
rc=$?
if [ "$rc" -ne 0 ]; then
    echo "first-boot: failed at step mkdir -p /config/etc: exit $rc"
    exit 1
fi

mkdir -p /config/strat
rc=$?
if [ "$rc" -ne 0 ]; then
    echo "first-boot: failed at step mkdir -p /config/strat: exit $rc"
    exit 1
fi

mkdir -p /apps/lib
rc=$?
if [ "$rc" -ne 0 ]; then
    echo "first-boot: failed at step mkdir -p /apps/lib: exit $rc"
    exit 1
fi

mkdir -p /apps/bin
rc=$?
if [ "$rc" -ne 0 ]; then
    echo "first-boot: failed at step mkdir -p /apps/bin: exit $rc"
    exit 1
fi

mkdir -p /apps/share
rc=$?
if [ "$rc" -ne 0 ]; then
    echo "first-boot: failed at step mkdir -p /apps/share: exit $rc"
    exit 1
fi

if [ -d /system/etc/defaults ]; then
    cp -r /system/etc/defaults/. /config/etc/.
    rc=$?
    if [ "$rc" -ne 0 ]; then
        echo "first-boot: failed at step cp -r /system/etc/defaults/. /config/etc/.: exit $rc"
        exit 1
    fi
fi

touch /config/.provisioned
rc=$?
if [ "$rc" -ne 0 ]; then
    echo "first-boot: failed at step touch /config/.provisioned: exit $rc"
    exit 1
fi

echo "first-boot: provisioning complete"
