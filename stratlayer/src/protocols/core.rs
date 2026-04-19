use crate::wire::protocol::{Argument, Message};
use crate::wire::socket::WaylandSocket;

pub struct WlDisplay {
    id: u32,
}

impl WlDisplay {
    pub fn new(id: u32) -> Self {
        WlDisplay { id }
    }

    /// wl_display.sync(new_id callback)
    /// Caller allocates `callback_id` and is responsible for registering the interface.
    pub fn sync(&self, callback_id: u32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            0,
            vec![Argument::NewId(callback_id)],
        );
        let _ = socket.send(&msg.serialize());
    }

    /// wl_display.get_registry(new_id registry)
    pub fn get_registry(&self, registry_id: u32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            1,
            vec![Argument::NewId(registry_id)],
        );
        let _ = socket.send(&msg.serialize());
    }
}

pub struct WlRegistry {
    id: u32,
}

impl WlRegistry {
    pub fn new(id: u32) -> Self {
        WlRegistry { id }
    }

    /// wl_registry.bind(name, interface, version, new_id)
    /// Per wayland-protocol, the NewId of `bind` carries the interface + version
    /// inline as a string + version before the id.
    pub fn bind(
        &self,
        name: u32,
        interface: &str,
        version: u32,
        new_id: u32,
        socket: &WaylandSocket,
    ) {
        let msg = Message::new(
            self.id,
            0,
            vec![
                Argument::Uint(name),
                Argument::String(interface.to_string()),
                Argument::Uint(version),
                Argument::NewId(new_id),
            ],
        );
        let _ = socket.send(&msg.serialize());
    }
}

pub struct WlCompositor {
    id: u32,
}

impl WlCompositor {
    pub fn new(id: u32) -> Self {
        WlCompositor { id }
    }

    /// wl_compositor.create_surface(new_id)
    pub fn create_surface(&self, surface_id: u32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            0,
            vec![Argument::NewId(surface_id)],
        );
        let _ = socket.send(&msg.serialize());
    }
}

pub struct WlSurface {
    id: u32,
}

impl WlSurface {
    pub fn new(id: u32) -> Self {
        WlSurface { id }
    }

    pub fn attach(&self, buffer_id: u32, x: i32, y: i32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            1, // attach
            vec![
                Argument::Object(buffer_id),
                Argument::Int(x),
                Argument::Int(y),
            ],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn damage(&self, x: i32, y: i32, width: i32, height: i32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            2, // damage
            vec![
                Argument::Int(x),
                Argument::Int(y),
                Argument::Int(width),
                Argument::Int(height),
            ],
        );
        let _ = socket.send(&msg.serialize());
    }

    /// wl_surface.set_buffer_transform(transform) — since compositor version 2.
    /// Use `0` for `WL_OUTPUT_TRANSFORM_NORMAL` (buffer authored in display coordinates).
    pub fn set_buffer_transform(&self, transform: i32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 7, vec![Argument::Int(transform)]);
        let _ = socket.send(&msg.serialize());
    }

    /// wl_surface.set_buffer_scale(scale) — since compositor version 3.
    pub fn set_buffer_scale(&self, scale: i32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 8, vec![Argument::Int(scale)]);
        let _ = socket.send(&msg.serialize());
    }

    /// wl_surface.damage_buffer — since compositor version 4 (buffer coordinates).
    pub fn damage_buffer(&self, x: i32, y: i32, width: i32, height: i32, socket: &WaylandSocket) {
        let msg = Message::new(
            self.id,
            9,
            vec![
                Argument::Int(x),
                Argument::Int(y),
                Argument::Int(width),
                Argument::Int(height),
            ],
        );
        let _ = socket.send(&msg.serialize());
    }

    pub fn commit(&self, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 6, vec![]); // commit
        let _ = socket.send(&msg.serialize());
    }
}
