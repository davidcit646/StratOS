use std::fs;
use std::io;
use std::path::PathBuf;

use stratsup::efi_vars;

fn main() -> io::Result<()> {
    let temp_dir = std::env::temp_dir().join("stratos-efivars-test");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

    let filename = format!(
        "{}-{}",
        efi_vars::VAR_ACTIVE_SLOT,
        efi_vars::STRAT_EFI_NAMESPACE_GUID
    );
    let mut path = PathBuf::from(&temp_dir);
    path.push(filename);

    let mut payload = Vec::new();
    payload.extend_from_slice(&[0x07, 0x00, 0x00, 0x00]); // attributes
    payload.push(0x2A); // value
    fs::write(&path, payload)?;

    let value = efi_vars::read_u8_from_dir(&temp_dir, efi_vars::VAR_ACTIVE_SLOT)?;
    if value != 0x2A {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "value mismatch",
        ));
    }

    fs::remove_dir_all(&temp_dir)?;
    println!("STRAT EFIVARS RUST TEST: PASS");
    Ok(())
}
