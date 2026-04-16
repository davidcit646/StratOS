use stratsup::supervisor::Supervisor;
use stratsup::efi_vars;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "set-boot-success" => {
                match efi_vars::set_boot_success() {
                    Ok(()) => {
                        std::process::exit(0);
                    }
                    Err(err) => {
                        eprintln!("stratsup: failed to set boot success: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            "set-target-slot" => {
                if args.len() < 3 {
                    eprintln!("stratsup: set-target-slot requires <0|1|2> argument");
                    std::process::exit(1);
                }
                let slot: u8 = match args[2].parse() {
                    Ok(s) => s,
                    Err(_) => {
                        eprintln!("stratsup: invalid slot id: {}", args[2]);
                        std::process::exit(1);
                    }
                };
                match efi_vars::set_target_slot(slot) {
                    Ok(()) => {
                        std::process::exit(0);
                    }
                    Err(err) => {
                        eprintln!("stratsup: failed to set target slot: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            "set-target-hash" => {
                if args.len() < 3 {
                    eprintln!("stratsup: set-target-hash requires <hex-32-byte> argument");
                    std::process::exit(1);
                }
                let hex_str = &args[2];
                if hex_str.len() != 64 {
                    eprintln!("stratsup: hash must be 64 hex characters (32 bytes)");
                    std::process::exit(1);
                }
                let mut hash = [0u8; 32];
                for (i, byte) in hex_str.as_bytes().chunks(2).enumerate() {
                    let hex_byte = std::str::from_utf8(byte).unwrap();
                    match u8::from_str_radix(hex_byte, 16) {
                        Ok(b) => hash[i] = b,
                        Err(_) => {
                            eprintln!("stratsup: invalid hex at position {}: {}", i * 2, hex_byte);
                            std::process::exit(1);
                        }
                    }
                }
                match efi_vars::set_target_hash(hash) {
                    Ok(()) => {
                        std::process::exit(0);
                    }
                    Err(err) => {
                        eprintln!("stratsup: failed to set target hash: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            "set-update-pending" => {
                match efi_vars::set_update_pending() {
                    Ok(()) => {
                        std::process::exit(0);
                    }
                    Err(err) => {
                        eprintln!("stratsup: failed to set update pending: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            "update" => {
                if args.len() < 3 {
                    eprintln!("stratsup: update requires <apply|status> subcommand");
                    std::process::exit(1);
                }
                match args[2].as_str() {
                    "apply" => {
                        if args.len() < 5 {
                            eprintln!("stratsup: update apply requires <slot> <hash> arguments");
                            std::process::exit(1);
                        }
                        let slot: u8 = match args[3].parse() {
                            Ok(s) => s,
                            Err(_) => {
                                eprintln!("stratsup: invalid slot id: {}", args[3]);
                                std::process::exit(1);
                            }
                        };
                        if slot > 2 {
                            eprintln!("stratsup: slot must be 0, 1, or 2");
                            std::process::exit(1);
                        }
                        let hex_str = &args[4];
                        if hex_str.len() != 64 {
                            eprintln!("stratsup: hash must be 64 hex characters (32 bytes)");
                            std::process::exit(1);
                        }
                        let mut hash = [0u8; 32];
                        for (i, byte) in hex_str.as_bytes().chunks(2).enumerate() {
                            let hex_byte = std::str::from_utf8(byte).unwrap();
                            match u8::from_str_radix(hex_byte, 16) {
                                Ok(b) => hash[i] = b,
                                Err(_) => {
                                    eprintln!("stratsup: invalid hex at position {}: {}", i * 2, hex_byte);
                                    std::process::exit(1);
                                }
                            }
                        }

                        if hash.iter().all(|&b| b == 0) {
                            eprintln!("stratsup: Invalid hash: cannot be all zeros");
                            std::process::exit(1);
                        }

                        if hash.iter().all(|&b| b == 0xFF) {
                            eprintln!("stratsup: Invalid hash: cannot be all 0xFF");
                            std::process::exit(1);
                        }

                        let unique_bytes: std::collections::HashSet<u8> = hash.iter().cloned().collect();
                        if unique_bytes.len() < 4 {
                            eprintln!("stratsup: Invalid hash: insufficient entropy");
                            std::process::exit(1);
                        }

                        let dry_run = args.iter().any(|arg| arg == "--dry-run");
                        let confirm = args.iter().any(|arg| arg == "--confirm");

                        if !dry_run && !confirm {
                            eprintln!("stratsup: Confirmation required: re-run with --confirm to apply update");
                            std::process::exit(1);
                        }

                        match efi_vars::read_u8(efi_vars::VAR_ACTIVE_SLOT) {
                            Ok(active_slot) => {
                                if active_slot == slot {
                                    eprintln!("stratsup: Target slot is already active");
                                    std::process::exit(1);
                                }
                            }
                            Err(_) => {
                                // Variable not found, proceed with validation
                            }
                        }

                        match efi_vars::read_u8(efi_vars::VAR_TARGET_SLOT) {
                            Ok(target_slot) => {
                                if target_slot == slot {
                                    match efi_vars::read_u8(efi_vars::VAR_UPDATE_PENDING) {
                                        Ok(pending) => {
                                            if pending == 1 {
                                                eprintln!("stratsup: Update for this slot is already pending");
                                                std::process::exit(1);
                                            }
                                        }
                                        Err(_) => {
                                            // Variable not found, proceed
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                // Variable not found, proceed
                            }
                        }

                        match efi_vars::read_u8(efi_vars::VAR_UPDATE_PENDING) {
                            Ok(pending) => {
                                if pending == 1 {
                                    eprintln!("stratsup: Update already in progress");
                                    std::process::exit(1);
                                }
                            }
                            Err(_) => {
                                // Variable not found, treat as 0 (no update in progress)
                            }
                        }

                        match efi_vars::read_u8(efi_vars::VAR_LAST_UPDATE_STATUS) {
                            Ok(last_status) => {
                                if last_status == 2 || last_status == 4 {
                                    // BOOT_SUCCESS_CONFIRMED or UPDATE_FINALIZED
                                    match efi_vars::read_u8(efi_vars::VAR_ACTIVE_SLOT) {
                                        Ok(active_slot) => {
                                            if slot == active_slot {
                                                eprintln!("stratsup: Cannot update currently stable active slot");
                                                std::process::exit(1);
                                            }
                                        }
                                        Err(_) => {
                                            // Variable not found, proceed
                                        }
                                    }
                                } else if last_status == 3 {
                                    // ROLLBACK_TRIGGERED
                                    match efi_vars::read_u8(efi_vars::VAR_ACTIVE_SLOT) {
                                        Ok(active_slot) => {
                                            if slot != active_slot {
                                                eprintln!("stratsup: System in rollback state; only active slot may be targeted");
                                                std::process::exit(1);
                                            }
                                        }
                                        Err(_) => {
                                            // Variable not found, proceed
                                        }
                                    }
                                }
                                // NONE (0) or ROTATION_INITIATED (1): allow normal behavior
                            }
                            Err(_) => {
                                // Variable not found, treat as NONE: allow normal behavior
                            }
                        }

                        if dry_run {
                            println!("Dry run successful: update request is valid for slot {}", slot);
                            std::process::exit(0);
                        }

                        if let Err(err) = efi_vars::set_target_slot(slot) {
                            eprintln!("stratsup: failed to set target slot: {}", err);
                            std::process::exit(1);
                        }
                        if let Err(err) = efi_vars::set_target_hash(hash) {
                            eprintln!("stratsup: failed to set target hash: {}", err);
                            std::process::exit(1);
                        }
                        if let Err(err) = efi_vars::set_update_pending() {
                            eprintln!("stratsup: failed to set update pending: {}", err);
                            std::process::exit(1);
                        }
                        println!("Update request staged: slot {}", slot);
                        std::process::exit(0);
                    }
                    "status" => {
                        match efi_vars::read_u8(efi_vars::VAR_UPDATE_PENDING) {
                            Ok(pending) => {
                                if pending == 1 {
                                    println!("UPDATE_PENDING: YES");
                                } else {
                                    println!("UPDATE_PENDING: NO");
                                }
                            }
                            Err(_) => {
                                println!("UPDATE_PENDING: NO");
                            }
                        }

                        match efi_vars::read_u8(efi_vars::VAR_ACTIVE_SLOT) {
                            Ok(slot) => {
                                println!("ACTIVE_SLOT: {}", slot);
                            }
                            Err(_) => {
                                println!("ACTIVE_SLOT: UNKNOWN");
                            }
                        }

                        match efi_vars::read_u8(efi_vars::VAR_TARGET_SLOT) {
                            Ok(slot) => {
                                println!("TARGET_SLOT: {}", slot);
                            }
                            Err(_) => {
                                println!("TARGET_SLOT: NONE");
                            }
                        }

                        match efi_vars::read_u8(efi_vars::VAR_BOOT_ATTEMPTS) {
                            Ok(attempts) => {
                                println!("BOOT_ATTEMPTS: {}", attempts);
                            }
                            Err(_) => {
                                println!("BOOT_ATTEMPTS: 0");
                            }
                        }

                        match efi_vars::read_u8(efi_vars::VAR_BOOT_SUCCESS) {
                            Ok(success) => {
                                if success == 1 {
                                    println!("BOOT_SUCCESS: YES");
                                } else {
                                    println!("BOOT_SUCCESS: NO");
                                }
                            }
                            Err(_) => {
                                println!("BOOT_SUCCESS: NO");
                            }
                        }

                        match efi_vars::read_u8(efi_vars::VAR_LAST_UPDATE_STATUS) {
                            Ok(status) => {
                                let status_str = match status {
                                    0 => "NONE",
                                    1 => "ROTATION_INITIATED",
                                    2 => "BOOT_SUCCESS_CONFIRMED",
                                    3 => "ROLLBACK_TRIGGERED",
                                    4 => "UPDATE_FINALIZED",
                                    _ => "UNKNOWN",
                                };
                                println!("LAST_STATUS: {}", status_str);
                            }
                            Err(_) => {
                                println!("LAST_STATUS: NONE");
                            }
                        }

                        match efi_vars::read_bytes(efi_vars::VAR_UPDATE_HISTORY, efi_vars::UPDATE_HISTORY_SIZE) {
                            Ok(history) => {
                                println!("UPDATE_HISTORY:");
                                for entry in history.iter() {
                                    let status_str = match *entry {
                                        0 => "NONE",
                                        1 => "ROTATION_INITIATED",
                                        2 => "BOOT_SUCCESS_CONFIRMED",
                                        3 => "ROLLBACK_TRIGGERED",
                                        4 => "UPDATE_FINALIZED",
                                        _ => "UNKNOWN",
                                    };
                                    println!("  {}", status_str);
                                }
                            }
                            Err(_) => {
                                println!("UPDATE_HISTORY: NONE");
                            }
                        }

                        std::process::exit(0);
                    }
                    "reset" => {
                        if let Err(err) = efi_vars::write_u8(efi_vars::VAR_UPDATE_PENDING, 0) {
                            eprintln!("stratsup: failed to clear update pending: {}", err);
                            std::process::exit(1);
                        }
                        if let Err(err) = efi_vars::write_u8(efi_vars::VAR_BOOT_ATTEMPTS, 0) {
                            eprintln!("stratsup: failed to clear boot attempts: {}", err);
                            std::process::exit(1);
                        }
                        if let Err(err) = efi_vars::write_u8(efi_vars::VAR_BOOT_SUCCESS, 0) {
                            eprintln!("stratsup: failed to clear boot success: {}", err);
                            std::process::exit(1);
                        }
                        println!("Update state cleared");
                        std::process::exit(0);
                    }
                    "cancel" => {
                        match efi_vars::read_u8(efi_vars::VAR_UPDATE_PENDING) {
                            Ok(pending) => {
                                if pending == 0 {
                                    println!("No update pending");
                                    std::process::exit(0);
                                }
                            }
                            Err(_) => {
                                println!("No update pending");
                                std::process::exit(0);
                            }
                        }

                        if let Err(err) = efi_vars::write_u8(efi_vars::VAR_UPDATE_PENDING, 0) {
                            eprintln!("stratsup: failed to clear update pending: {}", err);
                            std::process::exit(1);
                        }

                        if let Err(err) = efi_vars::write_u8(efi_vars::VAR_LAST_UPDATE_STATUS, 0) {
                            eprintln!("stratsup: failed to set last update status: {}", err);
                            std::process::exit(1);
                        }

                        println!("Update canceled");
                        std::process::exit(0);
                    }
                    _ => {
                        eprintln!("stratsup: unknown update subcommand: {}", args[2]);
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                eprintln!("stratsup: unknown command: {}", args[1]);
                std::process::exit(1);
            }
        }
    }

    let mut supervisor = Supervisor::new();
    loop {
        match supervisor.run_once() {
            Ok(()) => {
                if supervisor.shutdown_requested() {
                    break;
                }
            }
            Err(err) => {
                eprintln!("stratsup: {}", err);
                break;
            }
        }
    }
}
