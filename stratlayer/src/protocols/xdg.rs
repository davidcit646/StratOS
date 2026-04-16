use crate::wire::protocol::{Argument, Message};
use crate::wire::registry::ObjectRegistry;

pub struct XdgWmBase {
    id: u32,
}

impl XdgWmBase {
    pub fn new(id: u32) -> Self {
        XdgWmBase { id }
    }

    pub fn get_xdg_surface(&self, surface_id: u32, registry: &mut ObjectRegistry) -> u32 {
        let xdg_surface_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // get_xdg_surface opcode
            vec![
                Argument::NewId(xdg_surface_id),
                Argument::Object(surface_id),
            ],
        );
        // Send message to socket would happen here
        xdg_surface_id
    }

    pub fn pong(&self, serial: u32) {
        let msg = Message::new(
            self.id,
            1, // pong opcode
            vec![Argument::Uint(serial)],
        );
        // Send message to socket would happen here
    }
}

pub struct XdgSurface {
    id: u32,
}

impl XdgSurface {
    pub fn new(id: u32) -> Self {
        XdgSurface { id }
    }

    pub fn get_toplevel(&self, registry: &mut ObjectRegistry) -> u32 {
        let toplevel_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // get_toplevel opcode
            vec![Argument::NewId(toplevel_id)],
        );
        // Send message to socket would happen here
        toplevel_id
    }

    pub fn ack_configure(&self, serial: u32) {
        let msg = Message::new(
            self.id,
            1, // ack_configure opcode
            vec![Argument::Uint(serial)],
        );
        // Send message to socket would happen here
    }

    pub fn destroy(&self) {
        let msg = Message::new(self.id, 2, vec![]); // destroy opcode
        // Send message to socket would happen here
    }
}

pub struct XdgToplevel {
    id: u32,
}

impl XdgToplevel {
    pub fn new(id: u32) -> Self {
        XdgToplevel { id }
    }

    pub fn set_title(&self, title: &str) {
        let msg = Message::new(
            self.id,
            0, // set_title opcode
            vec![Argument::String(title.to_string())],
        );
        // Send message to socket would happen here
    }

    pub fn set_app_id(&self, app_id: &str) {
        let msg = Message::new(
            self.id,
            1, // set_app_id opcode
            vec![Argument::String(app_id.to_string())],
        );
        // Send message to socket would happen here
    }

    pub fn close(&self) {
        let msg = Message::new(self.id, 2, vec![]); // close opcode
        // Send message to socket would happen here
    }
}
