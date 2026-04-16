use crate::wire::protocol::{Argument, Message};
use crate::wire::registry::ObjectRegistry;
use std::os::unix::io::RawFd;

pub struct WlShm {
    id: u32,
}

impl WlShm {
    pub fn new(id: u32) -> Self {
        WlShm { id }
    }

    pub fn create_pool(&self, fd: RawFd, size: i32, registry: &mut ObjectRegistry) -> u32 {
        let pool_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // create_pool opcode
            vec![
                Argument::NewId(pool_id),
                Argument::Fd(fd),
                Argument::Int(size),
            ],
        );
        // Send message to socket would happen here
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

    pub fn create_buffer(
        &self,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: u32,
        registry: &mut ObjectRegistry,
    ) -> u32 {
        let buffer_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // create_buffer opcode
            vec![
                Argument::NewId(buffer_id),
                Argument::Int(offset),
                Argument::Int(width),
                Argument::Int(height),
                Argument::Int(stride),
                Argument::Uint(format),
            ],
        );
        // Send message to socket would happen here
        buffer_id
    }

    pub fn destroy(&self) {
        let msg = Message::new(self.id, 1, vec![]); // destroy opcode
        // Send message to socket would happen here
    }
}

pub struct WlBuffer {
    id: u32,
}

impl WlBuffer {
    pub fn new(id: u32) -> Self {
        WlBuffer { id }
    }

    pub fn destroy(&self) {
        let msg = Message::new(self.id, 0, vec![]); // destroy opcode
        // Send message to socket would happen here
    }
}
