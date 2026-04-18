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

/// Matches kernel cmdline keys written by stratboot (`config`, `apps`, `home`) with virtio/IDE fallbacks.
pub fn resolved_partition(cmdline: &str, key: &str, partnum: u8) -> PathBuf {
    if let Some(v) = cmdline_value(cmdline, key) {
        return resolve_disk_spec(&v);
    }
    let v = format!("/dev/vda{}", partnum);
    let s = format!("/dev/sda{}", partnum);
    if Path::new(&v).exists() {
        PathBuf::from(v)
    } else {
        PathBuf::from(s)
    }
}

pub fn path_to_cstring(path: &Path) -> Option<CString> {
    CString::new(path.as_os_str().as_bytes()).ok()
}
