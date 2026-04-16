use nix::sys::socket::{self, MsgFlags, SockAddr, SockFlag, SockType};
use nix::unistd::close;
use std::os::unix::io::RawFd;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    EnvVarNotFound(String),
    SocketConnect(nix::Error),
    Send(nix::Error),
    Receive(nix::Error),
}

pub struct WaylandSocket {
    fd: RawFd,
}

impl WaylandSocket {
    pub fn connect(display_name: Option<&str>) -> Result<Self, Error> {
        let display = display_name
            .or_else(|| std::env::var("WAYLAND_DISPLAY").ok())
            .unwrap_or_else(|| "wayland-0".to_string());

        let socket_path = if display.starts_with('/') {
            display
        } else {
            format!("/run/user/{}/{}", nix::unistd::getuid(), display)
        };

        let fd = socket::socket(
            socket::AddressFamily::Unix,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )
        .map_err(Error::SocketConnect)?;

        let sockaddr = SockAddr::new_unix(Path::new(&socket_path))
            .map_err(|_| Error::SocketConnect(nix::Error::EINVAL))?;

        socket::connect(fd, &sockaddr).map_err(Error::SocketConnect)?;

        Ok(WaylandSocket { fd })
    }

    pub fn send(&self, message: &[u8]) -> Result<(), Error> {
        socket::send(self.fd, message, MsgFlags::empty()).map_err(Error::Send)?;
        Ok(())
    }

    pub fn receive(&self, buffer: &mut [u8]) -> Result<usize, Error> {
        let bytes_read = socket::recv(self.fd, buffer, MsgFlags::empty()).map_err(Error::Receive)?;
        Ok(bytes_read)
    }
}

impl Drop for WaylandSocket {
    fn drop(&mut self) {
        let _ = close(self.fd);
    }
}
