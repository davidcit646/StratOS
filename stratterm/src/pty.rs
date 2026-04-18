use nix::pty::{forkpty, ForkptyResult, Winsize};
use nix::sys::wait::{waitpid, WaitStatus};
use std::os::unix::io::{BorrowedFd, IntoRawFd, RawFd};
use std::ffi::CString;
use nix::unistd::{execvp, close, read, write};
use libc::{ioctl, TIOCSWINSZ};

pub struct Pty {
    master_fd: RawFd,
    child_pid: nix::unistd::Pid,
}

impl Pty {
    pub fn new(rows: u16, cols: u16) -> Result<Self, String> {
        let winsize = Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let fork_result = unsafe {
            forkpty(Some(&winsize), None)
                .map_err(|e| format!("forkpty failed: {}", e))?
        };

        let (master_fd, child_pid) = match fork_result {
            ForkptyResult::Parent { master, child } => (master.into_raw_fd(), child),
            ForkptyResult::Child => {
                let shell = CString::new("/bin/sh").unwrap();
                let shell_name = CString::new("sh").unwrap();

                std::env::set_var("TERM", "xterm-256color");
                std::env::set_var("COLORTERM", "truecolor");
                std::env::set_var("COLUMNS", cols.to_string());
                std::env::set_var("ROWS", rows.to_string());
                std::env::set_var("PATH", "/bin:/usr/bin");
                std::env::set_var("HOME", "/home/user");
                std::env::set_var("SHELL", "/bin/sh");

                let _ = execvp(&shell, &[&shell_name]);
                // execvp only returns on error
                std::process::exit(127);
            }
        };

        Ok(Pty {
            master_fd,
            child_pid,
        })
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), String> {
        let winsize = Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        unsafe {
            if ioctl(self.master_fd, TIOCSWINSZ, &winsize) < 0 {
                return Err(format!("TIOCSWINSZ failed: {}", std::io::Error::last_os_error()));
            }
        }
        
        Ok(())
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, String> {
        read(self.master_fd, buf)
            .map_err(|e| format!("read from PTY failed: {}", e))
    }

    pub fn write(&self, data: &[u8]) -> Result<usize, String> {
        unsafe {
            write(BorrowedFd::borrow_raw(self.master_fd), data)
                .map_err(|e| format!("write to PTY failed: {}", e))
        }
    }

    pub fn raw_fd(&self) -> RawFd {
        self.master_fd
    }

    pub fn child_pid(&self) -> i32 {
        self.child_pid.as_raw()
    }

    pub fn wait(&self) -> Result<WaitStatus, String> {
        waitpid(self.child_pid, None)
            .map_err(|e| format!("waitpid failed: {}", e))
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        let _ = close(self.master_fd);
    }
}
