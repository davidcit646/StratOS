use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

use sha2::{Digest, Sha256};
use stratsup::efi_vars;

mod manifest;
mod fiemap;

fn update_slot_status(slot_id: u8, new_status: u8) -> Result<(), io::Error> {
    if slot_id > 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "slot_id must be 0, 1, or 2 (A, B, C)",
        ));
    }

    if new_status > 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "new_status must be 0=staging, 1=confirmed, or 2=bad",
        ));
    }

    let var_name = efi_vars::slot_status_var(slot_id).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidInput, e)
    })?;

    efi_vars::write_u8(var_name, new_status)
}

fn increment_boot_count() -> Result<(), io::Error> {
    let current = efi_vars::read_u8(efi_vars::VAR_BOOT_COUNT)?;
    let new_count = current.wrapping_add(1);
    efi_vars::write_u8(efi_vars::VAR_BOOT_COUNT, new_count)
}

#[derive(Clone)]
enum SlotStatus {
    Staging,
    Confirmed,
    Bad,
}

#[derive(Clone)]
struct Slot {
    id: u8,
    status: SlotStatus,
}

struct SystemState {
    active_slot: Option<Slot>,
    slots: [Slot; 3],
    boot_count: u8,
}

fn read_system_state() -> Result<SystemState, io::Error> {
    let slot_a_status_raw = efi_vars::read_u8(efi_vars::VAR_SLOT_A_STATUS)?;
    let slot_b_status_raw = efi_vars::read_u8(efi_vars::VAR_SLOT_B_STATUS)?;
    let slot_c_status_raw = efi_vars::read_u8(efi_vars::VAR_SLOT_C_STATUS)?;
    let boot_count = efi_vars::read_u8(efi_vars::VAR_BOOT_COUNT)?;

    let slot_a_status = match slot_a_status_raw {
        0 => SlotStatus::Staging,
        1 => SlotStatus::Confirmed,
        2 => SlotStatus::Bad,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid slot A status",
            ))
        }
    };

    let slot_b_status = match slot_b_status_raw {
        0 => SlotStatus::Staging,
        1 => SlotStatus::Confirmed,
        2 => SlotStatus::Bad,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid slot B status",
            ))
        }
    };

    let slot_c_status = match slot_c_status_raw {
        0 => SlotStatus::Staging,
        1 => SlotStatus::Confirmed,
        2 => SlotStatus::Bad,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid slot C status",
            ))
        }
    };

    let slots = [
        Slot { id: 0, status: slot_a_status },
        Slot { id: 1, status: slot_b_status },
        Slot { id: 2, status: slot_c_status },
    ];

    let active_slot_id = determine_active_slot()?;
    let active_slot = Some(slots[active_slot_id as usize].clone());

    Ok(SystemState {
        active_slot,
        slots,
        boot_count,
    })
}

fn validate_system_state(state: &SystemState) -> Result<(), io::Error> {
    for slot in &state.slots {
        if slot.id > 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid slot ID: {}", slot.id),
            ));
        }
    }

    if let Some(ref active) = state.active_slot {
        if !matches!(active.status, SlotStatus::Confirmed) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Active slot must have Confirmed status",
            ));
        }
    }

    let has_usable_slot = state.slots.iter().any(|slot| {
        matches!(slot.status, SlotStatus::Confirmed | SlotStatus::Staging)
    });

    if !has_usable_slot {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No usable slots available (all slots are Bad)",
        ));
    }

    Ok(())
}

fn determine_active_slot() -> Result<u8, io::Error> {
    let active_slot = efi_vars::read_u8(efi_vars::VAR_ACTIVE_SLOT)?;
    let slot_a_status = efi_vars::read_u8(efi_vars::VAR_SLOT_A_STATUS)?;
    let slot_b_status = efi_vars::read_u8(efi_vars::VAR_SLOT_B_STATUS)?;
    let slot_c_status = efi_vars::read_u8(efi_vars::VAR_SLOT_C_STATUS)?;

    if active_slot > 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "STRAT_ACTIVE_SLOT has invalid value",
        ));
    }

    if slot_a_status > 2 || slot_b_status > 2 || slot_c_status > 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Slot status has invalid value",
        ));
    }

    let statuses = [slot_a_status, slot_b_status, slot_c_status];

    if active_slot <= 2 && statuses[active_slot as usize] == 1 {
        return Ok(active_slot);
    }

    for (slot_id, &status) in statuses.iter().enumerate() {
        if status == 1 {
            return Ok(slot_id as u8);
        }
    }

    for (slot_id, &status) in statuses.iter().enumerate() {
        if status == 0 {
            return Ok(slot_id as u8);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No valid slots available (all slots are bad)",
    ))
}

fn set_active_slot(slot_id: u8) -> Result<(), io::Error> {
    if slot_id > 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "slot_id must be 0, 1, or 2 (A, B, C)",
        ));
    }

    let state = read_system_state()?;
    validate_system_state(&state)?;

    let target_slot = &state.slots[slot_id as usize];
    if !matches!(target_slot.status, SlotStatus::Confirmed) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Target slot must have Confirmed status",
        ));
    }

    efi_vars::write_u8(efi_vars::VAR_ACTIVE_SLOT, slot_id)
}

fn determine_staging_slot(state: &SystemState) -> Result<u8, io::Error> {
    let active_id = state.active_slot.as_ref().map(|s| s.id);

    for slot in &state.slots {
        if Some(slot.id) != active_id && matches!(slot.status, SlotStatus::Staging) {
            return Ok(slot.id);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No available staging slot",
    ))
}

fn mark_staging_pending(slot_id: u8) -> Result<(), io::Error> {
    if slot_id > 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "slot_id must be 0, 1, or 2 (A, B, C)",
        ));
    }

    let state = read_system_state()?;
    validate_system_state(&state)?;

    if let Some(ref active) = state.active_slot {
        if active.id == slot_id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Cannot mark active slot as pending update target",
            ));
        }
    }

    let target_slot = &state.slots[slot_id as usize];
    if !matches!(target_slot.status, SlotStatus::Staging) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Target slot must have Staging status",
        ));
    }

    efi_vars::set_target_slot(slot_id)
}

fn read_pending_slot() -> Result<Option<u8>, io::Error> {
    match efi_vars::read_u8(efi_vars::VAR_UPDATE_PENDING) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
        Ok(0) => Ok(None),
        Ok(1) => {
            let slot = efi_vars::read_u8(efi_vars::VAR_TARGET_SLOT)?;
            Ok(Some(slot))
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "STRAT_UPDATE_PENDING must be 0 or 1; use --stage-update to set activation",
        )),
    }
}

fn clear_pending_slot() -> Result<(), io::Error> {
    efi_vars::clear_update_request()
}

fn fetch_https_to_temp(url: &str) -> Result<PathBuf, io::Error> {
    let response = ureq::get(url).call().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("HTTPS fetch failed: {e}"),
        )
    })?;
    let mut body = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut body)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let tmp = std::env::temp_dir().join(format!("stratmon-stage-{}.img", std::process::id()));
    std::fs::write(&tmp, &body)?;
    Ok(tmp)
}

fn stage_update_from_path(image_path: &Path, target_slot: u8) -> Result<(), io::Error> {
    if target_slot > 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "target_slot must be 0, 1, or 2 (A, B, C)",
        ));
    }

    // Get file extents using FIEMAP
    let extents = fiemap::get_file_extents(image_path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get file extents: {}", e)))?;

    if extents.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No extents found for image file",
        ));
    }

    // Calculate SHA256 hash of the image
    let mut file = File::open(image_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    let hash_result = hasher.finalize();
    let mut expected_hash = [0u8; 32];
    expected_hash.copy_from_slice(&hash_result);

    // Write manifest to /EFI/STRAT/UPDATE.MAN
    let manifest_path = Path::new("/EFI/STRAT/UPDATE.MAN");
    manifest::write_manifest(manifest_path, target_slot, expected_hash, &extents)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to write manifest: {}", e)))?;

    efi_vars::set_target_slot(target_slot)?;
    efi_vars::set_target_hash(expected_hash)?;
    efi_vars::set_update_pending()?;

    Ok(())
}

fn stage_update(image_ref: &str, target_slot: u8) -> Result<(), io::Error> {
    if image_ref.starts_with("http://") || image_ref.starts_with("https://") {
        let tmp = fetch_https_to_temp(image_ref)?;
        let result = stage_update_from_path(&tmp, target_slot);
        let _ = std::fs::remove_file(&tmp);
        result
    } else {
        stage_update_from_path(Path::new(image_ref), target_slot)
    }
}

fn print_usage() {
    println!("StratMon - StratOS System Monitor");
    println!();
    println!("USAGE:");
    println!("  stratmon [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --update-slot <slot_id> <status>  Update slot status");
    println!("    slot_id: 0=A, 1=B, 2=C");
    println!("    status: 0=staging, 1=confirmed, 2=bad");
    println!("  --increment-boot                   Increment boot counter");
    println!("  --set-active <slot_id>             Set active slot (must be confirmed)");
    println!("    slot_id: 0=A, 1=B, 2=C");
    println!("  --mark-pending <slot_id>           Set STRAT_TARGET_SLOT only (use --stage-update to hash + activate)");
    println!("    slot_id: 0=A, 1=B, 2=C");
    println!("  --show-pending                     Show pending update target");
    println!("  --clear-pending                    Clear pending update target");
    println!("  --staging-slot                     Show staging slot for updates");
    println!("  --active-slot                      Show active slot (default)");
    println!("  --stage-update <image-path-or-url> <target-slot>  Stage update for target slot");
    println!("    image-path: Local path, or https:// URL (TLS via system roots; temp file)");
    println!("    target-slot: 0=A, 1=B, 2=C");
    println!("  --help                             Show this help");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        match read_system_state() {
            Ok(state) => {
                if let Err(e) = validate_system_state(&state) {
                    eprintln!("Invalid system state: {}", e);
                    process::exit(1);
                }
                let slot_id = state.active_slot.as_ref().map(|s| s.id).unwrap_or(0);
                let slot_name = match slot_id {
                    0 => "A",
                    1 => "B",
                    2 => "C",
                    _ => "unknown",
                };
                println!("Active slot: {}", slot_name);
            }
            Err(e) => {
                eprintln!("Error reading active slot: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    let mut update_slot_arg: Option<(u8, u8)> = None;
    let mut increment_boot = false;
    let mut set_active_arg: Option<u8> = None;
    let mut mark_pending_arg: Option<u8> = None;
    let mut stage_update_arg: Option<(String, u8)> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--update-slot" => {
                if i + 2 >= args.len() {
                    eprintln!("Error: --update-slot requires <slot_id> and <status>");
                    print_usage();
                    process::exit(1);
                }
                let slot_id: u8 = match args[i + 1].parse() {
                    Ok(n) => n,
                    Err(_) => {
                        eprintln!("Error: slot_id must be a number (0, 1, or 2)");
                        print_usage();
                        process::exit(1);
                    }
                };
                let status: u8 = match args[i + 2].parse() {
                    Ok(n) => n,
                    Err(_) => {
                        eprintln!("Error: status must be a number (0, 1, or 2)");
                        print_usage();
                        process::exit(1);
                    }
                };
                update_slot_arg = Some((slot_id, status));
                i += 3;
            }
            "--increment-boot" => {
                increment_boot = true;
                i += 1;
            }
            "--set-active" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --set-active requires <slot_id>");
                    print_usage();
                    process::exit(1);
                }
                let slot_id: u8 = match args[i + 1].parse() {
                    Ok(n) => n,
                    Err(_) => {
                        eprintln!("Error: slot_id must be a number (0, 1, or 2)");
                        print_usage();
                        process::exit(1);
                    }
                };
                set_active_arg = Some(slot_id);
                i += 2;
            }
            "--mark-pending" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --mark-pending requires <slot_id>");
                    print_usage();
                    process::exit(1);
                }
                let slot_id: u8 = match args[i + 1].parse() {
                    Ok(n) => n,
                    Err(_) => {
                        eprintln!("Error: slot_id must be a number (0, 1, or 2)");
                        print_usage();
                        process::exit(1);
                    }
                };
                mark_pending_arg = Some(slot_id);
                i += 2;
            }
            "--stage-update" => {
                if i + 2 >= args.len() {
                    eprintln!("Error: --stage-update requires <image-path> and <target-slot>");
                    print_usage();
                    process::exit(1);
                }
                let image_path = args[i + 1].clone();
                let target_slot: u8 = match args[i + 2].parse() {
                    Ok(n) => n,
                    Err(_) => {
                        eprintln!("Error: target_slot must be a number (0, 1, or 2)");
                        print_usage();
                        process::exit(1);
                    }
                };
                stage_update_arg = Some((image_path, target_slot));
                i += 3;
            }
            "--show-pending" => {
                match read_pending_slot() {
                    Ok(None) => {
                        println!("No pending update");
                    }
                    Ok(Some(slot_id)) => {
                        let slot_name = match slot_id {
                            0 => "A",
                            1 => "B",
                            2 => "C",
                            _ => "unknown",
                        };
                        println!("Pending update target: {}", slot_name);
                    }
                    Err(e) => {
                        eprintln!("Error reading pending slot: {}", e);
                        process::exit(1);
                    }
                }
                i += 1;
            }
            "--clear-pending" => {
                if let Err(e) = clear_pending_slot() {
                    eprintln!("Error clearing pending slot: {}", e);
                    process::exit(1);
                }
                println!("Pending update cleared");
                i += 1;
            }
            "--staging-slot" => {
                match read_system_state() {
                    Ok(state) => {
                        if let Err(e) = validate_system_state(&state) {
                            eprintln!("Invalid system state: {}", e);
                            process::exit(1);
                        }
                        match determine_staging_slot(&state) {
                            Ok(slot_id) => {
                                let slot_name = match slot_id {
                                    0 => "A",
                                    1 => "B",
                                    2 => "C",
                                    _ => "unknown",
                                };
                                println!("Staging slot: {}", slot_name);
                            }
                            Err(e) => {
                                eprintln!("Error determining staging slot: {}", e);
                                process::exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading system state: {}", e);
                        process::exit(1);
                    }
                }
                i += 1;
            }
            "--active-slot" => {
                match read_system_state() {
                    Ok(state) => {
                        if let Err(e) = validate_system_state(&state) {
                            eprintln!("Invalid system state: {}", e);
                            process::exit(1);
                        }
                        let slot_id = state.active_slot.as_ref().map(|s| s.id).unwrap_or(0);
                        let slot_name = match slot_id {
                            0 => "A",
                            1 => "B",
                            2 => "C",
                            _ => "unknown",
                        };
                        println!("Active slot: {}", slot_name);
                    }
                    Err(e) => {
                        eprintln!("Error reading active slot: {}", e);
                        process::exit(1);
                    }
                }
                i += 1;
            }
            "--help" => {
                print_usage();
                process::exit(0);
            }
            _ => {
                eprintln!("Error: unknown option '{}'", args[i]);
                print_usage();
                process::exit(1);
            }
        }
    }

    if let Some(slot_id) = set_active_arg {
        if let Err(e) = set_active_slot(slot_id) {
            eprintln!("Error setting active slot: {}", e);
            process::exit(1);
        }
        let slot_name = match slot_id {
            0 => "A",
            1 => "B",
            2 => "C",
            _ => "unknown",
        };
        println!("Active slot set to: {}", slot_name);
    }

    if let Some(slot_id) = mark_pending_arg {
        if let Err(e) = mark_staging_pending(slot_id) {
            eprintln!("Error marking staging pending: {}", e);
            process::exit(1);
        }
        let slot_name = match slot_id {
            0 => "A",
            1 => "B",
            2 => "C",
            _ => "unknown",
        };
        println!(
            "STRAT_TARGET_SLOT set to {} (run --stage-update to set hash + boot activation)",
            slot_name
        );
    }

    if let Some((slot_id, status)) = update_slot_arg {
        if let Err(e) = update_slot_status(slot_id, status) {
            eprintln!("Error updating slot status: {}", e);
            process::exit(1);
        }
        let slot_name = match slot_id {
            0 => "A",
            1 => "B",
            2 => "C",
            _ => "unknown",
        };
        let status_name = match status {
            0 => "staging",
            1 => "confirmed",
            2 => "bad",
            _ => "unknown",
        };
        println!("Updated slot {} to status: {}", slot_name, status_name);
    }

    if increment_boot {
        if let Err(e) = increment_boot_count() {
            eprintln!("Error incrementing boot count: {}", e);
            process::exit(1);
        }
        println!("Boot count incremented");
    }

    if let Some((image_path, target_slot)) = stage_update_arg {
        if let Err(e) = stage_update(&image_path, target_slot) {
            eprintln!("Error staging update: {}", e);
            process::exit(1);
        }
        let slot_name = match target_slot {
            0 => "A",
            1 => "B",
            2 => "C",
            _ => "unknown",
        };
        println!("Update staged for slot {} from {}", slot_name, image_path);
    }
}
