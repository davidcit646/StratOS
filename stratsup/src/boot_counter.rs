use crate::efi_vars;

pub const MAX_BOOT_ATTEMPTS: u8 = 3;

pub fn increment_boot_count() -> Result<u8, String> {
    let current = read_boot_count()?;
    if current == u8::MAX {
        return Err("boot counter: overflow — counter at 255, refusing to increment".to_string());
    }

    let next = current + 1;
    efi_vars::write_u8(efi_vars::VAR_BOOT_COUNT, next).map_err(|err| {
        format!(
            "boot counter: failed to write {}: {}",
            efi_vars::VAR_BOOT_COUNT,
            err
        )
    })?;
    Ok(next)
}

pub fn reset_boot_count() -> Result<(), String> {
    efi_vars::write_u8(efi_vars::VAR_BOOT_COUNT, 0).map_err(|err| {
        format!(
            "boot counter: failed to write {}: {}",
            efi_vars::VAR_BOOT_COUNT,
            err
        )
    })
}

pub fn read_boot_count() -> Result<u8, String> {
    efi_vars::read_u8(efi_vars::VAR_BOOT_COUNT).map_err(|err| {
        format!(
            "boot counter: failed to read {}: {}",
            efi_vars::VAR_BOOT_COUNT,
            err
        )
    })
}
