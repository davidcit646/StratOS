use crate::efi_vars;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::os::unix::fs::PermissionsExt;

const SLOT_A: u8 = 0;
const SLOT_C: u8 = 2;

const STATUS_CONFIRMED: u8 = 1;
const STATUS_BAD: u8 = 2;

const HOME_STATUS_HEALTHY: u8 = 0;
const HOME_STATUS_DEGRADED: u8 = 1;
const HOME_STATUS_CORRUPT: u8 = 2;

// After initramfs pivot, the system image is the root filesystem (no /system mount).
const STRAT_WM_BIN: &str = "/bin/stratwm";
const STRAT_PANEL_BIN: &str = "/bin/stratpanel";
/// Binaries required for a usable graphical session (fail fast before slot is marked good).
const CRITICAL_BINARIES: [&str; 2] = [STRAT_WM_BIN, STRAT_PANEL_BIN];

pub fn run() -> io::Result<()> {
    let mut failures = Vec::new();

    if !system_root_is_ro() {
        failures.push("SYSTEM not mounted read-only");
    }
    if !mount_is_rw("/config") {
        failures.push("CONFIG not mounted read-write");
    }
    let home_status = detect_home_status();
    if home_status != HOME_STATUS_HEALTHY {
        failures.push(match home_status {
            HOME_STATUS_DEGRADED => "HOME mount degraded",
            HOME_STATUS_CORRUPT => "HOME mount failed",
            _ => "HOME status invalid",
        });
    }
    if let Err(err) = check_binaries(&CRITICAL_BINARIES) {
        failures.push(err);
    }
    if !network_available() {
        failures.push("Network subsystem unavailable");
    }

    let active_slot = read_active_slot().unwrap_or(SLOT_A);
    let status_var = efi_vars::slot_status_var(active_slot)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let _ = efi_vars::write_u8(efi_vars::VAR_HOME_STATUS, home_status);

    if failures.is_empty() {
        let _ = efi_vars::write_u8(status_var, STATUS_CONFIRMED);
        Ok(())
    } else {
        let _ = efi_vars::write_u8(status_var, STATUS_BAD);
        for failure in failures {
            eprintln!("strat-validate-boot: {}", failure);
        }
        trigger_reboot();
        Ok(())
    }
}

fn read_active_slot() -> io::Result<u8> {
    let slot = efi_vars::read_u8(efi_vars::VAR_ACTIVE_SLOT)?;
    if slot > SLOT_C {
        return Ok(SLOT_A);
    }
    Ok(slot)
}

fn mount_is_ro(mount_point: &str) -> bool {
    mount_has_option(mount_point, "ro")
}

/// True when the read-only system volume is mounted: live/EROFS uses `/`, older layouts use `/system`.
fn system_root_is_ro() -> bool {
    mount_is_ro("/") || mount_is_ro("/system")
}

fn mount_is_rw(mount_point: &str) -> bool {
    mount_has_option(mount_point, "rw")
}

fn mount_present(mount_point: &str) -> bool {
    let Ok(mounts) = fs::read_to_string("/proc/mounts") else {
        return false;
    };

    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        if parts[1] == mount_point {
            return true;
        }
    }
    false
}

fn detect_home_status() -> u8 {
    if !mount_present("/home") {
        return HOME_STATUS_CORRUPT;
    }
    if !path_accessible("/home") {
        return HOME_STATUS_CORRUPT;
    }
    if mount_is_rw("/home") {
        HOME_STATUS_HEALTHY
    } else {
        HOME_STATUS_DEGRADED
    }
}

fn mount_has_option(mount_point: &str, option: &str) -> bool {
    let Ok(mounts) = fs::read_to_string("/proc/mounts") else {
        return false;
    };

    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        if parts[1] != mount_point {
            continue;
        }
        return parts[3].split(',').any(|opt| opt == option);
    }
    false
}

fn path_accessible(path: &str) -> bool {
    Path::new(path).exists()
}

fn check_binaries(paths: &[&str]) -> Result<(), &'static str> {
    for path in paths {
        if !binary_executable(path) {
            if *path == STRAT_WM_BIN {
                return Err("Strat WM binary missing or not executable");
            }
            if *path == STRAT_PANEL_BIN {
                return Err("Strat panel binary missing or not executable");
            }
            return Err("Critical system binary missing or not executable");
        }
    }
    Ok(())
}

fn binary_executable(path: &str) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    meta.permissions().mode() & 0o111 != 0
}

fn network_available() -> bool {
    let Ok(entries) = fs::read_dir("/sys/class/net") else {
        return false;
    };
    for entry in entries.flatten() {
        if let Ok(name) = entry.file_name().into_string() {
            if name != "lo" {
                return true;
            }
        }
    }
    false
}

fn trigger_reboot() {
    let _ = Command::new("reboot").status();
}
