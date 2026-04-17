use nix::sys::mman::{mmap, MapFlags, ProtFlags};
use nix::sys::memfd::{memfd_create, MemFdCreateFlag};
use nix::unistd::ftruncate;
use std::ffi::CStr;
use std::num::NonZeroUsize;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::io::RawFd;

pub struct ShmPool {
    fd: OwnedFd,
    ptr: *mut u8,
    size: usize,
}

impl ShmPool {
    pub fn create(size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let name = CStr::from_bytes_with_nul(b"stratterm_shm\0")?;
        let fd: OwnedFd = memfd_create(name, MemFdCreateFlag::MFD_CLOEXEC)?;

        ftruncate(fd.as_fd(), size as i64)?;

        let ptr = unsafe {
            mmap(
                None,
                NonZeroUsize::new(size).unwrap(),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                fd.as_fd(),
                0,
            )?
        }.as_ptr() as *mut u8;

        Ok(ShmPool { fd, ptr, size })
    }

    pub fn fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn resize(&mut self, new_size: usize) -> Result<(), Box<dyn std::error::Error>> {
        ftruncate(self.fd.as_fd(), new_size as i64)?;

        unsafe {
            if !self.ptr.is_null() {
                let _ = libc::munmap(self.ptr as *mut libc::c_void, self.size);
            }
        }

        self.ptr = unsafe {
            mmap(
                None,
                NonZeroUsize::new(new_size).unwrap(),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                self.fd.as_fd(),
                0,
            )?
        }.as_ptr() as *mut u8;

        self.size = new_size;
        Ok(())
    }
}

impl Drop for ShmPool {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                let _ = libc::munmap(self.ptr as *mut libc::c_void, self.size);
            }
        }
    }
}
