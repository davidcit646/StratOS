use crate::wire::protocol::{Argument, Message};
use crate::wire::socket::WaylandSocket;

pub const LAYER_TOP: u32 = 2;
pub const ANCHOR_TOP: u32 = 1;
pub const ANCHOR_LEFT: u32 = 4;
pub const ANCHOR_RIGHT: u32 = 8;

pub struct ZwlrLayerShellV1 {
    id: u32,
}

impl ZwlrLayerShellV1 {
    pub fn new(id: u32) -> Self {
        ZwlrLayerShellV1 { id }
    }

    /// zwlr_layer_shell_v1.get_layer_surface(new_id, surface, output[nullable=0], layer, namespace)
    pub fn get_layer_surface(
        &self,
        new_id: u32,
        surface_id: u32,
        output_id: u32,
        layer: u32,
        namespace: &str,
        socket: &WaylandSocket,
    ) {
        let msg = Message::new(
            self.id,
            0,
            vec![
                Argument::NewId(new_id),
                Argument::Object(surface_id),
                Argument::Object(output_id),
                Argument::Uint(layer),
                Argument::String(namespace.to_string()),
            ],
        );
        let _ = socket.send(&msg.serialize());
    }
}

pub struct ZwlrLayerSurfaceV1 {
    id: u32,
}

impl ZwlrLayerSurfaceV1 {
    pub fn new(id: u32) -> Self {
        ZwlrLayerSurfaceV1 { id }
    }

    pub fn set_size(&self, width: u32, height: u32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 0, vec![Argument::Uint(width), Argument::Uint(height)]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn set_anchor(&self, anchor: u32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 1, vec![Argument::Uint(anchor)]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn set_exclusive_zone(&self, zone: i32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 2, vec![Argument::Int(zone)]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn set_keyboard_interactivity(&self, mode: u32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 4, vec![Argument::Uint(mode)]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn ack_configure(&self, serial: u32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 6, vec![Argument::Uint(serial)]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn destroy(&self, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 7, vec![]);
        let _ = socket.send(&msg.serialize());
    }
}
