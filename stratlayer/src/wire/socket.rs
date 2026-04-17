use nix::sys::socket::{self, MsgFlags, SockFlag, SockType, socket as nix_socket, UnixAddr, sendmsg, ControlMessage};
use nix::unistd::close;
use std::io::IoSlice;
use std::os::unix::io::{IntoRawFd, RawFd};

#[derive(Debug)]
pub enum Error {
    EnvVarNotFound(String),
    SocketConnect(nix::Error),
    Send(nix::Error),
    Receive(nix::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EnvVarNotFound(s) => write!(f, "Environment variable not found: {}", s),
            Error::SocketConnect(e) => write!(f, "Socket connect error: {}", e),
            Error::Send(e) => write!(f, "Socket send error: {}", e),
            Error::Receive(e) => write!(f, "Socket receive error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::SocketConnect(e) => Some(e),
            Error::Send(e) => Some(e),
            Error::Receive(e) => Some(e),
            _ => None,
        }
    }
}

pub struct WaylandSocket {
    fd: RawFd,
    owns_fd: bool,
}

impl WaylandSocket {
    pub fn connect(display_name: Option<&str>) -> Result<Self, Error> {
        let display = if let Some(name) = display_name {
            name.to_string()
        } else if let Ok(env_display) = std::env::var("WAYLAND_DISPLAY") {
            env_display
        } else {
            "wayland-0".to_string()
        };

        let socket_path = if display.starts_with('/') {
            display
        } else {
            format!("/run/user/{}/{}", nix::unistd::getuid(), display)
        };

        let fd = nix_socket(
            socket::AddressFamily::Unix,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )
        .map_err(Error::SocketConnect)?
        .into_raw_fd();

        let sockaddr = UnixAddr::new(socket_path.as_str())
            .map_err(|_| Error::SocketConnect(nix::Error::EINVAL))?;

        socket::connect(fd, &sockaddr).map_err(Error::SocketConnect)?;

        Ok(WaylandSocket { fd, owns_fd: true })
    }

    pub fn send(&self, message: &[u8]) -> Result<(), Error> {
        socket::send(self.fd, message, MsgFlags::empty()).map_err(Error::Send)?;
        Ok(())
    }

    pub fn send_with_fd(&self, message: &[u8], fd: RawFd) -> Result<(), Error> {
        use nix::sys::socket::{sendmsg, ControlMessage, MsgFlags};
        use std::io::IoSlice;
        let iov = [IoSlice::new(message)];
        let cmsg = [ControlMessage::ScmRights(&[fd])];
        sendmsg::<()>(self.fd, &iov, &cmsg, MsgFlags::empty(), None)
            .map_err(Error::Send)?;
        Ok(())
    }

    pub fn receive(&self, buffer: &mut [u8]) -> Result<usize, Error> {
        let bytes_read = socket::recv(self.fd, buffer, MsgFlags::empty()).map_err(Error::Receive)?;
        Ok(bytes_read)
    }

    pub fn raw_fd(&self) -> RawFd {
        self.fd
    }

    pub fn from_raw_fd(fd: RawFd) -> Self {
        WaylandSocket { fd, owns_fd: false }
    }
}

impl Drop for WaylandSocket {
    fn drop(&mut self) {
        if self.owns_fd {
            let _ = close(self.fd);
        }
    }
}
