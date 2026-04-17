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

    pub fn destroy(&self, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 0, vec![]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn create_positioner(&self, registry: &mut ObjectRegistry, socket: &WaylandSocket) -> u32 {
        let positioner_id = registry.allocate();
        let msg = Message::new(
            self.id,
            1,
            vec![Argument::NewId(positioner_id)],
        );
        let _ = socket.send(&msg.serialize());
        positioner_id
    }

    pub fn get_xdg_surface(&self, surface_id: u32, registry: &mut ObjectRegistry, socket: &WaylandSocket) -> u32 {
        let xdg_surface_id = registry.allocate();
        let msg = Message::new(
            self.id,
            2, // get_xdg_surface opcode
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
            3, // pong opcode
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

    pub fn destroy(&self, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 0, vec![]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn get_toplevel(&self, registry: &mut ObjectRegistry, socket: &WaylandSocket) -> u32 {
        let toplevel_id = registry.allocate();
        let msg = Message::new(
            self.id,
            1, // get_toplevel opcode
            vec![Argument::NewId(toplevel_id)],
        );
        let _ = socket.send(&msg.serialize());
        toplevel_id
    }

    pub fn get_popup(&self, parent_surface_id: u32, positioner_id: u32, registry: &mut ObjectRegistry, socket: &WaylandSocket) -> u32 {
        let popup_id = registry.allocate();
        let msg = Message::new(
            self.id,
            2,
            vec![
                Argument::NewId(popup_id),
                Argument::Object(parent_surface_id),
                Argument::Object(positioner_id),
            ],
        );
        let _ = socket.send(&msg.serialize());
        popup_id
    }

    pub fn set_window_geometry(&self, x: i32, y: i32, width: i32, height: i32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            3,
            vec![
                Argument::Int(x),
                Argument::Int(y),
                Argument::Int(width),
                Argument::Int(height),
            ],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn ack_configure(&self, serial: u32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            4, // ack_configure opcode
            vec![Argument::Uint(serial)],
        );
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

    pub fn destroy(&self, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 0, vec![]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn set_parent(&self, parent_id: u32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            1,
            vec![Argument::Object(parent_id)],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn set_title(&self, title: &str, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            2, // set_title opcode
            vec![Argument::String(title.to_string())],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn set_app_id(&self, app_id: &str, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            3, // set_app_id opcode
            vec![Argument::String(app_id.to_string())],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn close(&self, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 4, vec![]);
        let _ = socket.send(&msg.serialize());
    }
}
