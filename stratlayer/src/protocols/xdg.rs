use crate::wire::protocol::{Argument, Message};
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

    /// xdg_wm_base.get_xdg_surface(new_id xdg_surface, object surface)
    pub fn get_xdg_surface(
        &self,
        xdg_surface_id: u32,
        surface_id: u32,
        socket: &WaylandSocket,
    ) {
        let msg = Message::new(
            self.id,
            2,
            vec![
                Argument::NewId(xdg_surface_id),
                Argument::Object(surface_id),
            ],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn pong(&self, serial: u32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 3, vec![Argument::Uint(serial)]);
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

    /// xdg_surface.get_toplevel(new_id)
    pub fn get_toplevel(&self, toplevel_id: u32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 1, vec![Argument::NewId(toplevel_id)]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn set_window_geometry(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        socket: &WaylandSocket,
    ) {
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
        let msg = Message::new(self.id, 4, vec![Argument::Uint(serial)]);
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

    pub fn set_title(&self, title: &str, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            2,
            vec![Argument::String(title.to_string())],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn set_app_id(&self, app_id: &str, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            3,
            vec![Argument::String(app_id.to_string())],
        );
        let _ = socket.send(&msg.serialize());
    }
}
