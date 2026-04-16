use crate::manifest::ExtentEntry;
use nix::fcntl::OFlag;
use nix::sys::ioctl::{ioctl, ReadWriteDir, IoctlNumber};
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;

// FIEMAP ioctl definitions
const FIEMAP_MAX_OFFSET: u64 = u64::MAX;
const FS_IOC_FIEMAP: u32 = 0xC020660B;

#[repr(C)]
pub struct FiemapRequest {
    pub start: u64,
    pub length: u64,
    pub flags: u32,
    pub mapped_extents: u32,
    pub extent_count: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct FiemapExtent {
    pub logical: u64,
    pub physical: u64,
    pub length: u64,
    pub flags: u64,
    pub reserved: [u64; 2],
}

impl IoctlNumber for FiemapRequest {
    const IOCTL: u32 = FS_IOC_FIEMAP;
    type Output = FiemapRequest;
}

pub fn get_file_extents(image_path: &Path) -> io::Result<Vec<ExtentEntry>> {
    let file = File::open(image_path)?;
    let fd = file.as_raw_fd();
    let file_size = file.metadata()?.len();

    // Calculate number of extents needed
    let extent_struct_size = std::mem::size_of::<FiemapExtent>() as u64;
    let buffer_size = 4096; // 4KB buffer for extents
    let max_extents = (buffer_size / extent_struct_size) as u32;

    let mut extents = Vec::new();
    let mut current_offset = 0u64;

    while current_offset < file_size {
        let mut fiemap = FiemapRequest {
            start: current_offset,
            length: file_size - current_offset,
            flags: 0,
            mapped_extents: 0,
            extent_count: max_extents,
            reserved: 0,
        };

        // Allocate buffer for extents
        let mut extent_buffer: Vec<FiemapExtent> = vec![
            FiemapExtent {
                logical: 0,
                physical: 0,
                length: 0,
                flags: 0,
                reserved: [0, 0],
            };
            max_extents as usize
        ];

        // Perform ioctl
        unsafe {
            let result = ioctl(fd, &mut fiemap as *mut FiemapRequest)?;
            if result != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "FIEMAP ioctl failed",
                ));
            }
        }

        // Process returned extents
        for i in 0..fiemap.mapped_extents {
            let extent = &extent_buffer[i as usize];
            if extent.physical != 0 && extent.length > 0 {
                extents.push(ExtentEntry {
                    logical_offset: extent.logical,
                    physical_offset: extent.physical,
                    length: extent.length,
                });
            }
        }

        // Move to next batch
        if fiemap.mapped_extents == 0 {
            break;
        }

        let last_extent = &extent_buffer[(fiemap.mapped_extents - 1) as usize];
        current_offset = last_extent.logical + last_extent.length;

        if current_offset <= last_extent.logical {
            // Prevent infinite loop
            break;
        }
    }

    Ok(extents)
}
