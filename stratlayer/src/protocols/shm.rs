use crate::wire::protocol::{Argument, Message};
use crate::wire::socket::WaylandSocket;
use std::os::unix::io::RawFd;

pub struct WlShm {
    id: u32,
}

impl WlShm {
    pub fn new(id: u32) -> Self {
        WlShm { id }
    }

    /// wl_shm.create_pool(new_id, fd, size)
    /// Caller allocates `pool_id`. fd travels as SCM_RIGHTS.
    pub fn create_pool(
        &self,
        pool_id: u32,
        fd: RawFd,
        size: i32,
        socket: &WaylandSocket,
    ) {
        let msg = Message::new(
            self.id,
            0,
            vec![
                Argument::NewId(pool_id),
                Argument::Int(size),
            ],
        );
        let _ = socket.send_with_fd(&msg.serialize(), fd);
    }
}

pub struct WlShmPool {
    id: u32,
}

impl WlShmPool {
    pub fn new(id: u32) -> Self {
        WlShmPool { id }
    }

    /// wl_shm_pool.create_buffer(new_id, offset, width, height, stride, format)
    pub fn create_buffer(
        &self,
        buffer_id: u32,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: u32,
        socket: &WaylandSocket,
    ) {
        let msg = Message::new(
            self.id,
            0,
            vec![
                Argument::NewId(buffer_id),
                Argument::Int(offset),
                Argument::Int(width),
                Argument::Int(height),
                Argument::Int(stride),
                Argument::Uint(format),
            ],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn destroy(&self, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 1, vec![]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn resize(&self, size: i32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 2, vec![Argument::Int(size)]);
        let _ = socket.send(&msg.serialize());
    }
}

pub struct WlBuffer {
    id: u32,
}

impl WlBuffer {
    pub fn new(id: u32) -> Self {
        WlBuffer { id }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn destroy(&self, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 0, vec![]);
        let _ = socket.send(&msg.serialize());
    }
}
