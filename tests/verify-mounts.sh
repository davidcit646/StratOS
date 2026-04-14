#!/bin/sh

failures=0

if [ -L /system ]; then
    echo "FAIL: /system is a symlink"
    failures=$((failures + 1))
else
    echo "PASS: /system is not a symlink"
fi

if [ -L /config ]; then
    echo "FAIL: /config is a symlink"
    failures=$((failures + 1))
else
    echo "PASS: /config is not a symlink"
fi

if [ -L /apps ]; then
    echo "FAIL: /apps is a symlink"
    failures=$((failures + 1))
else
    echo "PASS: /apps is not a symlink"
fi

if [ -L /home ]; then
    echo "FAIL: /home is a symlink"
    failures=$((failures + 1))
else
    echo "PASS: /home is not a symlink"
fi

if [ -L /var ]; then
    echo "FAIL: /var is a symlink"
    failures=$((failures + 1))
else
    echo "PASS: /var is not a symlink"
fi

if [ -L /run ]; then
    echo "FAIL: /run is a symlink"
    failures=$((failures + 1))
else
    echo "PASS: /run is not a symlink"
fi

if [ -L /usr ]; then
    echo "FAIL: /usr is a symlink"
    failures=$((failures + 1))
else
    echo "PASS: /usr is not a symlink"
fi

system_opts="$(awk '$2=="/system"{print $4; found=1; exit} END{if(!found) exit 1}' /proc/mounts 2>/dev/null)"
if [ $? -ne 0 ]; then
    echo "FAIL: /system mount entry missing"
    failures=$((failures + 1))
else
    case ",$system_opts," in
        *,ro,*)
            echo "PASS: /system mounted read-only"
            ;;
        *)
            echo "FAIL: /system is not read-only"
            failures=$((failures + 1))
            ;;
    esac
fi

config_opts="$(awk '$2=="/config"{print $4; found=1; exit} END{if(!found) exit 1}' /proc/mounts 2>/dev/null)"
if [ $? -ne 0 ]; then
    echo "FAIL: /config mount entry missing"
    failures=$((failures + 1))
else
    case ",$config_opts," in
        *,rw,*)
            echo "PASS: /config mounted read-write"
            ;;
        *)
            echo "FAIL: /config is not read-write"
            failures=$((failures + 1))
            ;;
    esac
fi

apps_opts="$(awk '$2=="/apps"{print $4; found=1; exit} END{if(!found) exit 1}' /proc/mounts 2>/dev/null)"
if [ $? -ne 0 ]; then
    echo "FAIL: /apps mount entry missing"
    failures=$((failures + 1))
else
    case ",$apps_opts," in
        *,rw,*)
            echo "PASS: /apps mounted read-write"
            ;;
        *)
            echo "FAIL: /apps is not read-write"
            failures=$((failures + 1))
            ;;
    esac
fi

home_opts="$(awk '$2=="/home"{print $4; found=1; exit} END{if(!found) exit 1}' /proc/mounts 2>/dev/null)"
if [ $? -ne 0 ]; then
    echo "FAIL: /home mount entry missing"
    failures=$((failures + 1))
else
    case ",$home_opts," in
        *,rw,*)
            echo "PASS: /home mounted read-write"
            ;;
        *)
            echo "FAIL: /home is not read-write"
            failures=$((failures + 1))
            ;;
    esac
fi

config_var_dev=$(stat -c '%d' /config/var 2>/dev/null)
var_dev=$(stat -c '%d' /var 2>/dev/null)
config_var_ino=$(stat -c '%i' /config/var 2>/dev/null)
var_ino=$(stat -c '%i' /var 2>/dev/null)
if [ -n "$config_var_dev" ] && [ "$config_var_dev" = "$var_dev" ] && [ "$config_var_ino" = "$var_ino" ]; then
    echo "PASS: /var is bind-mounted from /config/var"
else
    echo "FAIL: /var is not bind-mounted from /config/var"
    failures=$((failures + 1))
fi

run_type="$(awk '$2=="/run"{print $3; found=1; exit} END{if(!found) exit 1}' /proc/mounts 2>/dev/null)"
if [ $? -ne 0 ]; then
    echo "FAIL: /run mount entry missing"
    failures=$((failures + 1))
else
    if [ "$run_type" = "tmpfs" ]; then
        echo "PASS: /run is tmpfs"
    else
        echo "FAIL: /run filesystem is $run_type (expected tmpfs)"
        failures=$((failures + 1))
    fi
fi

system_dev=$(stat -c '%d' /system 2>/dev/null)
usr_dev=$(stat -c '%d' /usr 2>/dev/null)
system_ino=$(stat -c '%i' /system 2>/dev/null)
usr_ino=$(stat -c '%i' /usr 2>/dev/null)
if [ -n "$system_dev" ] && [ "$system_dev" = "$usr_dev" ] && [ "$system_ino" = "$usr_ino" ]; then
    echo "PASS: /usr is bind-mounted from /system"
else
    echo "FAIL: /usr is not bind-mounted from /system"
    failures=$((failures + 1))
fi

if [ "$failures" -ne 0 ]; then
    exit 1
fi

exit 0
