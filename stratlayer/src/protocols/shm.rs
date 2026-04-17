use crate::wire::protocol::{Argument, Message};
use crate::wire::registry::ObjectRegistry;
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
    /// Allocates a pool id, sends the request with the fd as an SCM_RIGHTS attachment.
    pub fn create_pool(
        &self,
        fd: RawFd,
        size: i32,
        registry: &mut ObjectRegistry,
        socket: &WaylandSocket,
    ) -> u32 {
        let pool_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // create_pool
            vec![
                Argument::NewId(pool_id),
                Argument::Int(size),
            ],
        );
        let _ = socket.send_with_fd(&msg.serialize(), fd);
        pool_id
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
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: u32,
        registry: &mut ObjectRegistry,
        socket: &WaylandSocket,
    ) -> u32 {
        let buffer_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // create_buffer
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
        buffer_id
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
