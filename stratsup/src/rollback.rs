use crate::boot_counter;
use crate::efi_vars;

const SLOT_C: u8 = 2;
const STATUS_BAD: u8 = 2;

pub fn should_rollback() -> Result<bool, String> {
    let boot_count = boot_counter::read_boot_count().map_err(|err| format!("rollback: {}", err))?;

    let active_slot = efi_vars::read_u8(efi_vars::VAR_ACTIVE_SLOT)
        .map_err(|err| format!("rollback: failed to read {}: {}", efi_vars::VAR_ACTIVE_SLOT, err))?;
    let active_slot_status_var =
        efi_vars::slot_status_var(active_slot).map_err(|err| format!("rollback: {}", err))?;
    let active_slot_status = efi_vars::read_u8(active_slot_status_var)
        .map_err(|err| format!("rollback: failed to read {}: {}", active_slot_status_var, err))?;

    Ok(boot_count >= boot_counter::MAX_BOOT_ATTEMPTS || active_slot_status == STATUS_BAD)
}

pub fn execute_rollback() -> Result<(), String> {
    let last_good_slot = efi_vars::read_u8(efi_vars::VAR_LAST_GOOD_SLOT).map_err(|err| {
        format!(
            "rollback: failed to read {}: {}",
            efi_vars::VAR_LAST_GOOD_SLOT,
            err
        )
    })?;
    if last_good_slot > SLOT_C {
        return Err("rollback: no valid last-good slot recorded, cannot roll back".to_string());
    }

    efi_vars::write_u8(efi_vars::VAR_ACTIVE_SLOT, last_good_slot).map_err(|err| {
        format!(
            "rollback: failed to write {}: {}",
            efi_vars::VAR_ACTIVE_SLOT,
            err
        )
    })?;

    boot_counter::reset_boot_count().map_err(|err| format!("rollback: {}", err))?;
    Ok(())
}
