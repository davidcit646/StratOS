use crate::boot_counter;
use crate::efi_vars;

const SLOT_C: u8 = 2;
const STATUS_STAGING: u8 = 0;
const STATUS_CONFIRMED: u8 = 1;

pub fn pivot_to_slot(slot_id: u8) -> Result<(), String> {
    if slot_id > SLOT_C {
        return Err(format!("pivot: invalid slot id {}", slot_id));
    }

    let target_status_var =
        efi_vars::slot_status_var(slot_id).map_err(|err| format!("pivot: {}", err))?;
    let target_status = efi_vars::read_u8(target_status_var)
        .map_err(|err| format!("pivot: failed to read {}: {}", target_status_var, err))?;
    if target_status != STATUS_STAGING {
        return Err(format!(
            "pivot: slot {} is not in staging status (got {}), refusing to pivot",
            slot_id, target_status
        ));
    }

    let previous_active = efi_vars::read_u8(efi_vars::VAR_ACTIVE_SLOT)
        .map_err(|err| format!("pivot: failed to read {}: {}", efi_vars::VAR_ACTIVE_SLOT, err))?;

    if previous_active <= SLOT_C {
        let previous_status_var =
            efi_vars::slot_status_var(previous_active).map_err(|err| format!("pivot: {}", err))?;
        let previous_status = efi_vars::read_u8(previous_status_var)
            .map_err(|err| format!("pivot: failed to read {}: {}", previous_status_var, err))?;
        if previous_status == STATUS_CONFIRMED {
            efi_vars::write_u8(efi_vars::VAR_LAST_GOOD_SLOT, previous_active).map_err(|err| {
                format!(
                    "pivot: failed to write {}: {}",
                    efi_vars::VAR_LAST_GOOD_SLOT,
                    err
                )
            })?;
        }
    }

    efi_vars::write_u8(efi_vars::VAR_ACTIVE_SLOT, slot_id)
        .map_err(|err| format!("pivot: failed to write {}: {}", efi_vars::VAR_ACTIVE_SLOT, err))?;

    boot_counter::reset_boot_count().map_err(|err| format!("pivot: {}", err))?;
    Ok(())
}
