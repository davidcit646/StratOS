use nix::sys::mman::{mmap, MapFlags, ProtFlags};
use nix::sys::stat::Mode;
use nix::unistd::{close, ftruncate};
use nix::fcntl::{OFlag, open};
use std::os::unix::io::RawFd;
use std::ptr;

pub struct ShmPool {
    fd: RawFd,
    ptr: *mut u8,
    size: usize,
}

impl ShmPool {
    pub fn create(name: &str, size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let fd = open(
            name,
            OFlag::O_CREAT | OFlag::O_RDWR | OFlag::O_EXCL,
            Mode::S_IRUSR | Mode::S_IWUSR,
        )?;

        ftruncate(fd, size as i64)?;

        let ptr = unsafe {
            mmap(
                ptr::null_mut(),
                size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                Some(fd),
                0,
            )? as *mut u8
        };

        Ok(ShmPool { fd, ptr, size })
    }

    pub fn fd(&self) -> RawFd {
        self.fd
    }

    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for ShmPool {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                let _ = nix::sys::mman::munmap(self.ptr as *mut libc::c_void, self.size);
            }
        }
        let _ = close(self.fd);
    }
}
