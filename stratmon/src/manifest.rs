use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

#[repr(C, packed)]
pub struct ManifestHeader {
    pub magic: [u8; 4],        // b"STRM"
    pub version: u8,           // 1
    pub target_slot: u8,       // 0=A, 1=B, 2=C
    pub reserved: [u8; 2],
    pub expected_hash: [u8; 32],
    pub extent_count: u64,
}

#[repr(C, packed)]
pub struct ExtentEntry {
    pub logical_offset: u64,
    pub physical_offset: u64,
    pub length: u64,
}

impl ManifestHeader {
    pub fn new(target_slot: u8, expected_hash: [u8; 32], extent_count: u64) -> Self {
        ManifestHeader {
            magic: *b"STRM",
            version: 1,
            target_slot,
            reserved: [0; 2],
            expected_hash,
            extent_count,
        }
    }
}

pub fn write_manifest(
    manifest_path: &Path,
    target_slot: u8,
    expected_hash: [u8; 32],
    extents: &[ExtentEntry],
) -> io::Result<()> {
    let header = ManifestHeader::new(target_slot, expected_hash, extents.len() as u64);
    
    let mut file = File::create(manifest_path)?;
    
    // Write header as bytes
    unsafe {
        let header_bytes = std::slice::from_raw_parts(
            &header as *const ManifestHeader as *const u8,
            std::mem::size_of::<ManifestHeader>(),
        );
        file.write_all(header_bytes)?;
    }
    
    // Write each extent entry as bytes
    for extent in extents {
        unsafe {
            let extent_bytes = std::slice::from_raw_parts(
                extent as *const ExtentEntry as *const u8,
                std::mem::size_of::<ExtentEntry>(),
            );
            file.write_all(extent_bytes)?;
        }
    }
    
    file.sync_all()?;
    Ok(())
}
