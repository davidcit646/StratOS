use crate::wire::protocol::{Argument, Message};
use crate::wire::registry::ObjectRegistry;
use crate::wire::socket::WaylandSocket;

pub struct XdgWmBase {
    id: u32,
}

impl XdgWmBase {
    pub fn new(id: u32) -> Self {
        XdgWmBase { id }
    }

    pub fn get_xdg_surface(&self, surface_id: u32, registry: &mut ObjectRegistry, socket: &WaylandSocket) -> u32 {
        let xdg_surface_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // get_xdg_surface opcode
            vec![
                Argument::NewId(xdg_surface_id),
                Argument::Object(surface_id),
            ],
        );
        let _ = socket.send(&msg.serialize());
        xdg_surface_id
    }

    pub fn pong(&self, serial: u32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            1, // pong opcode
            vec![Argument::Uint(serial)],
        );
        let _ = socket.send(&msg.serialize());
    }
}

pub struct XdgSurface {
    id: u32,
}

impl XdgSurface {
    pub fn new(id: u32) -> Self {
        XdgSurface { id }
    }

    pub fn get_toplevel(&self, registry: &mut ObjectRegistry, socket: &WaylandSocket) -> u32 {
        let toplevel_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // get_toplevel opcode
            vec![Argument::NewId(toplevel_id)],
        );
        let _ = socket.send(&msg.serialize());
        toplevel_id
    }

    pub fn ack_configure(&self, serial: u32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            1, // ack_configure opcode
            vec![Argument::Uint(serial)],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn destroy(&self, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 2, vec![]); // destroy opcode
        let _ = socket.send(&msg.serialize());
    }
}

pub struct XdgToplevel {
    id: u32,
}

impl XdgToplevel {
    pub fn new(id: u32) -> Self {
        XdgToplevel { id }
    }

    pub fn set_title(&self, title: &str, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            0, // set_title opcode
            vec![Argument::String(title.to_string())],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn set_app_id(&self, app_id: &str, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            1, // set_app_id opcode
            vec![Argument::String(app_id.to_string())],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn close(&self, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 2, vec![]); // close opcode
        let _ = socket.send(&msg.serialize());
    }
}
