use crate::wire::protocol::{Argument, Message};
use crate::wire::registry::ObjectRegistry;

pub struct WlDisplay {
    id: u32,
}

impl WlDisplay {
    pub fn new(id: u32) -> Self {
        WlDisplay { id }
    }

    pub fn sync(&self, registry: &mut ObjectRegistry) -> u32 {
        let callback_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // sync opcode
            vec![Argument::NewId(callback_id)],
        );
        // Send message to socket would happen here
        callback_id
    }

    pub fn get_registry(&self, registry: &mut ObjectRegistry) -> u32 {
        let registry_id = registry.allocate();
        let msg = Message::new(
            self.id,
            1, // get_registry opcode
            vec![Argument::NewId(registry_id)],
        );
        // Send message to socket would happen here
        registry_id
    }
}

pub struct WlRegistry {
    id: u32,
}

impl WlRegistry {
    pub fn new(id: u32) -> Self {
        WlRegistry { id }
    }

    pub fn bind(&self, interface: &str, version: u32, registry: &mut ObjectRegistry) -> u32 {
        let new_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // bind opcode
            vec![
                Argument::String(interface.to_string()),
                Argument::Uint(new_id),
                Argument::Uint(version),
            ],
        );
        // Send message to socket would happen here
        new_id
    }
}

pub struct WlCompositor {
    id: u32,
}

impl WlCompositor {
    pub fn new(id: u32) -> Self {
        WlCompositor { id }
    }

    pub fn create_surface(&self, registry: &mut ObjectRegistry) -> u32 {
        let surface_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // create_surface opcode
            vec![Argument::NewId(surface_id)],
        );
        // Send message to socket would happen here
        surface_id
    }
}

pub struct WlSurface {
    id: u32,
}

impl WlSurface {
    pub fn new(id: u32) -> Self {
        WlSurface { id }
    }

    pub fn attach(&self, buffer_id: u32, x: i32, y: i32) {
        let msg = Message::new(
            self.id,
            0, // attach opcode
            vec![
                Argument::Object(buffer_id),
                Argument::Int(x),
                Argument::Int(y),
            ],
        );
        // Send message to socket would happen here
    }

    pub fn damage(&self, x: i32, y: i32, width: i32, height: i32) {
        let msg = Message::new(
            self.id,
            1, // damage opcode
            vec![
                Argument::Int(x),
                Argument::Int(y),
                Argument::Int(width),
                Argument::Int(height),
            ],
        );
        // Send message to socket would happen here
    }

    pub fn commit(&self) {
        let msg = Message::new(self.id, 2, vec![]); // commit opcode
        // Send message to socket would happen here
    }
}
