use crate::wire::protocol::Message;
use crate::wire::socket::WaylandSocket;
use std::os::unix::io::RawFd;

pub struct Dispatcher {
    socket: WaylandSocket,
    /// Bytes left over from a partial message on the previous read.
    pending: Vec<u8>,
}

impl Dispatcher {
    pub fn from_fd(fd: RawFd) -> Self {
        Dispatcher {
            socket: WaylandSocket::from_raw_fd(fd),
            pending: Vec::new(),
        }
    }

    /// Blocking read from the Wayland socket; returns as many complete messages as fit.
    /// Call poll(POLLIN) before this if you want non-blocking behavior.
    pub fn dispatch_once(&mut self) -> Result<(Vec<Message>, Vec<RawFd>), Box<dyn std::error::Error>> {
        let mut buf = [0u8; 8192];
        let (n, fds) = self.socket.receive_with_fds(&mut buf)?;
        if n == 0 {
            return Ok((Vec::new(), fds));
        }
        self.pending.extend_from_slice(&buf[..n]);

        let mut messages = Vec::new();
        let mut offset = 0;
        while self.pending.len() - offset >= 8 {
            let length = u16::from_le_bytes([
                self.pending[offset + 6],
                self.pending[offset + 7],
            ]) as usize;

            if length < 8 {
                // Corrupt — bail out, drop the rest
                self.pending.clear();
                break;
            }
            if self.pending.len() - offset < length {
                break;
            }

            if let Some(msg) = Message::deserialize(&self.pending[offset..offset + length]) {
                messages.push(msg);
            }
            offset += length;
        }

        if offset > 0 {
            self.pending.drain(..offset);
        }

        Ok((messages, fds))
    }
}
