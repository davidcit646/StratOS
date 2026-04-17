use nix::sys::mman::{mmap, MapFlags, ProtFlags};
use nix::sys::memfd::{memfd_create, MemFdCreateFlag};
use nix::unistd::{close, ftruncate};
use std::os::unix::io::{BorrowedFd, OwnedFd, RawFd};
use std::num::NonZeroUsize;

pub struct ShmPool {
    fd: OwnedFd,
    ptr: *mut u8,
    size: usize,
}

impl ShmPool {
    pub fn create(size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let fd = memfd_create("stratterm_shm", MemFdCreateFlag::MFD_CLOEXEC)?;

        ftruncate(BorrowedFd::from(&fd), size as i64)?;

        let ptr = unsafe {
            mmap(
                None,
                NonZeroUsize::new(size).unwrap(),
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                BorrowedFd::from(&fd),
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
        ftruncate(BorrowedFd::from(&self.fd), new_size as i64)?;

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
                BorrowedFd::from(&self.fd),
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
        let _ = close(self.fd.as_raw_fd());
    }
}
