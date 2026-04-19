use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub fn read_proc_cmdline() -> String {
    std::fs::read_to_string("/proc/cmdline").unwrap_or_default()
}

pub fn cmdline_value(cmdline: &str, key: &str) -> Option<String> {
    let prefix = format!("{}=", key);
    for tok in cmdline.split_whitespace() {
        if let Some(rest) = tok.strip_prefix(&prefix) {
            return Some(rest.to_string());
        }
    }
    None
}

/// Live session (ISO / diagnostic): initramfs used tmpfs for CONFIG/APPS/HOME; do not mount block devices again.
pub fn is_live_session(cmdline: &str) -> bool {
    cmdline_value(cmdline, "strat.live").as_deref() == Some("1")
}

pub fn resolve_disk_spec(spec: &str) -> PathBuf {
    let s = spec.trim();
    if let Some(uuid) = s.strip_prefix("PARTUUID=") {
        PathBuf::from(format!(
            "/dev/disk/by-partuuid/{}",
            uuid.trim().to_lowercase()
        ))
    } else {
        PathBuf::from(s)
    }
}

fn virtio_ide_partition(partnum: u8) -> PathBuf {
    let v = format!("/dev/vda{}", partnum);
    let s = format!("/dev/sda{}", partnum);
    if Path::new(&v).exists() {
        PathBuf::from(v)
    } else {
        PathBuf::from(s)
    }
}

/// Matches kernel cmdline keys written by stratboot (`config`, `apps`, `home`) with virtio/IDE fallbacks.
pub fn resolved_partition(cmdline: &str, key: &str, partnum: u8) -> PathBuf {
    if let Some(v) = cmdline_value(cmdline, key) {
        let resolved = resolve_disk_spec(&v);
        if resolved.exists() {
            return resolved;
        }
        // Early boot: udev may not have created symlinks or enumerated nodes yet (by-partuuid
        // and/or e.g. /dev/sdaN). If cmdline named any /dev path that is still missing, use
        // numbered virtio/IDE fallbacks instead of returning a non-existent path.
        if resolved.starts_with(Path::new("/dev/")) {
            return virtio_ide_partition(partnum);
        }
        return resolved;
    }
    virtio_ide_partition(partnum)
}

pub fn path_to_cstring(path: &Path) -> Option<CString> {
    CString::new(path.as_os_str().as_bytes()).ok()
}
