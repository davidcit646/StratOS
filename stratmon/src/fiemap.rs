use crate::manifest::ExtentEntry;
use nix::errno::Errno;
use nix::libc::{self, c_ulong};
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;

// linux/fiemap.h + linux/fs.h (FS_IOC_FIEMAP _IOWR('f', 11, struct fiemap))
const FS_IOC_FIEMAP: u32 = 0xC020660B;

#[repr(C)]
struct Fiemap {
    fm_start: u64,
    fm_length: u64,
    fm_flags: u32,
    fm_mapped_extents: u32,
    fm_extent_count: u32,
    fm_reserved: u32,
}

#[repr(C)]
struct FiemapExtent {
    fe_logical: u64,
    fe_physical: u64,
    fe_length: u64,
    fe_reserved64: [u64; 2],
    fe_flags: u32,
    fe_reserved: [u32; 3],
}

pub fn get_file_extents(image_path: &Path) -> io::Result<Vec<ExtentEntry>> {
    let file = File::open(image_path)?;
    let fd = file.as_raw_fd();
    let file_size = file.metadata()?.len();

    let hdr = std::mem::size_of::<Fiemap>();
    let ext_sz = std::mem::size_of::<FiemapExtent>();
    let buffer_size = 4096u32;
    let max_extents = ((buffer_size as usize - hdr) / ext_sz) as u32;

    let mut extents = Vec::new();
    let mut current_offset = 0u64;

    while current_offset < file_size {
        let buf_len = hdr + (max_extents as usize) * ext_sz;
        let mut buf = vec![0u8; buf_len];

        {
            let fiemap = unsafe { &mut *(buf.as_mut_ptr() as *mut Fiemap) };
            fiemap.fm_start = current_offset;
            fiemap.fm_length = file_size - current_offset;
            fiemap.fm_flags = 0;
            fiemap.fm_mapped_extents = 0;
            fiemap.fm_extent_count = max_extents;
            fiemap.fm_reserved = 0;
        }

        let ret = unsafe {
            libc::ioctl(fd, FS_IOC_FIEMAP as c_ulong, buf.as_mut_ptr() as *mut libc::c_void)
        };
        Errno::result(ret).map_err(|e| io::Error::from_raw_os_error(e as i32))?;

        let (mapped, next_off) = {
            let fiemap = unsafe { &*(buf.as_ptr() as *const Fiemap) };
            let mapped = fiemap.fm_mapped_extents;
            let base = buf.as_ptr().wrapping_add(hdr) as *const FiemapExtent;
            for i in 0..mapped {
                let extent = unsafe { &*base.add(i as usize) };
                if extent.fe_physical != 0 && extent.fe_length > 0 {
                    extents.push(ExtentEntry {
                        logical_offset: extent.fe_logical,
                        physical_offset: extent.fe_physical,
                        length: extent.fe_length,
                    });
                }
            }
            let next_off = if mapped == 0 {
                None
            } else {
                let last = unsafe { &*base.add((mapped - 1) as usize) };
                Some(last.fe_logical.saturating_add(last.fe_length))
            };
            (mapped, next_off)
        };

        if mapped == 0 {
            break;
        }
        let Some(next) = next_off else {
            break;
        };
        if next <= current_offset {
            break;
        }
        current_offset = next;
    }

    Ok(extents)
}
