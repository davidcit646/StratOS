use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const STRAT_EFI_NAMESPACE_GUID: &str = "10731b6f-16b5-4aea-ab46-c62aa093c8e5";

pub const VAR_SLOT_A_STATUS: &str = "STRAT_SLOT_A_STATUS";
pub const VAR_SLOT_B_STATUS: &str = "STRAT_SLOT_B_STATUS";
pub const VAR_SLOT_C_STATUS: &str = "STRAT_SLOT_C_STATUS";
pub const VAR_ACTIVE_SLOT: &str = "STRAT_ACTIVE_SLOT";
pub const VAR_PINNED_SLOT: &str = "STRAT_PINNED_SLOT";
pub const VAR_RESET_FLAGS: &str = "STRAT_RESET_FLAGS";
pub const VAR_BOOT_COUNT: &str = "STRAT_BOOT_COUNT";
pub const VAR_LAST_GOOD_SLOT: &str = "STRAT_LAST_GOOD_SLOT";
pub const VAR_HOME_STATUS: &str = "STRAT_HOME_STATUS";
pub const VAR_UPDATE_PENDING: &str = "STRAT_UPDATE_PENDING";
pub const VAR_BOOT_SUCCESS: &str = "STRAT_BOOT_SUCCESS";
pub const VAR_BOOT_ATTEMPTS: &str = "STRAT_BOOT_ATTEMPTS";
pub const VAR_TARGET_SLOT: &str = "STRAT_TARGET_SLOT";
pub const VAR_TARGET_HASH: &str = "STRAT_TARGET_HASH";
pub const VAR_LAST_UPDATE_STATUS: &str = "STRAT_LAST_UPDATE_STATUS";
pub const VAR_UPDATE_HISTORY: &str = "STRAT_UPDATE_HISTORY";
pub const UPDATE_HISTORY_SIZE: usize = 5;

const STRAT_EFI_VAR_ATTRS: [u8; 4] = [0x07, 0x00, 0x00, 0x00];

pub fn read_u8(var_name: &str) -> io::Result<u8> {
    read_u8_from_dir(Path::new("/sys/firmware/efi/efivars"), var_name)
}

pub fn read_u8_from_dir(base_dir: &Path, var_name: &str) -> io::Result<u8> {
    let mut path = PathBuf::from(base_dir);
    let filename = format!("{}-{}", var_name, STRAT_EFI_NAMESPACE_GUID);
    path.push(filename);

    let data = fs::read(path)?;
    if data.len() < 5 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "EFI variable payload too short",
        ));
    }

    Ok(data[4])
}

pub fn read_bytes(var_name: &str, size: usize) -> io::Result<Vec<u8>> {
    read_bytes_from_dir(Path::new("/sys/firmware/efi/efivars"), var_name, size)
}

pub fn read_bytes_from_dir(base_dir: &Path, var_name: &str, size: usize) -> io::Result<Vec<u8>> {
    let mut path = PathBuf::from(base_dir);
    let filename = format!("{}-{}", var_name, STRAT_EFI_NAMESPACE_GUID);
    path.push(filename);

    let data = fs::read(path)?;
    if data.len() < 5 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "EFI variable payload too short",
        ));
    }

    if data.len() - 4 != size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "EFI variable payload size mismatch",
        ));
    }

    Ok(data[4..].to_vec())
}

pub fn write_u8(var_name: &str, value: u8) -> io::Result<()> {
    let mut path = PathBuf::from("/sys/firmware/efi/efivars");
    let filename = format!("{}-{}", var_name, STRAT_EFI_NAMESPACE_GUID);
    path.push(filename);

    match fs::symlink_metadata(&path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "efi_vars: refusing to write through symlink {}",
                        path.display()
                    ),
                ));
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    let mut payload = Vec::with_capacity(5);
    payload.extend_from_slice(&STRAT_EFI_VAR_ATTRS);
    payload.push(value);
    fs::write(path, payload)
}

pub fn slot_is_pinned(slot: u8) -> io::Result<bool> {
    slot_is_pinned_from_dir(Path::new("/sys/firmware/efi/efivars"), slot)
}

pub fn slot_is_pinned_from_dir(base_dir: &Path, slot: u8) -> io::Result<bool> {
    let pinned_slot = read_u8_from_dir(base_dir, VAR_PINNED_SLOT)?;
    Ok(pinned_slot == slot)
}

pub fn write_u8_to_dir(base_dir: &Path, var_name: &str, value: u8) -> io::Result<()> {
    let mut path = PathBuf::from(base_dir);
    let filename = format!("{}-{}", var_name, STRAT_EFI_NAMESPACE_GUID);
    path.push(filename);

    let mut payload = Vec::with_capacity(5);
    payload.extend_from_slice(&STRAT_EFI_VAR_ATTRS);
    payload.push(value);
    fs::write(path, payload)
}

pub fn delete_u8(var_name: &str) -> io::Result<()> {
    delete_u8_from_dir(Path::new("/sys/firmware/efi/efivars"), var_name)
}

pub fn delete_u8_from_dir(base_dir: &Path, var_name: &str) -> io::Result<()> {
    let mut path = PathBuf::from(base_dir);
    let filename = format!("{}-{}", var_name, STRAT_EFI_NAMESPACE_GUID);
    path.push(filename);
    fs::remove_file(path)
}

pub fn slot_status_var(slot_id: u8) -> Result<&'static str, String> {
    match slot_id {
        0 => Ok(VAR_SLOT_A_STATUS),
        1 => Ok(VAR_SLOT_B_STATUS),
        2 => Ok(VAR_SLOT_C_STATUS),
        _ => Err(format!("efi_vars: invalid slot id {}", slot_id)),
    }
}

pub fn set_boot_success() -> io::Result<()> {
    write_u8(VAR_BOOT_SUCCESS, 1)
}

pub fn set_target_slot(slot: u8) -> io::Result<()> {
    if slot > 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid slot id (must be 0, 1, or 2)",
        ));
    }
    write_u8(VAR_TARGET_SLOT, slot)
}

pub fn set_target_hash(hash: [u8; 32]) -> io::Result<()> {
    let mut path = PathBuf::from("/sys/firmware/efi/efivars");
    let filename = format!("{}-{}", VAR_TARGET_HASH, STRAT_EFI_NAMESPACE_GUID);
    path.push(filename);

    match fs::symlink_metadata(&path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "efi_vars: refusing to write through symlink {}",
                        path.display()
                    ),
                ));
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    let mut payload = Vec::with_capacity(36);
    payload.extend_from_slice(&STRAT_EFI_VAR_ATTRS);
    payload.extend_from_slice(&hash);
    fs::write(path, payload)
}

pub fn set_update_pending() -> io::Result<()> {
    write_u8(VAR_UPDATE_PENDING, 1)
}
