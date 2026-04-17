use crate::wire::protocol::{Argument, Message};
use crate::wire::socket::WaylandSocket;

pub struct WlSeat {
    id: u32,
}

impl WlSeat {
    pub fn new(id: u32) -> Self {
        WlSeat { id }
    }

    pub fn get_pointer(&self, pointer_id: u32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 0, vec![Argument::NewId(pointer_id)]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn get_keyboard(&self, keyboard_id: u32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 1, vec![Argument::NewId(keyboard_id)]);
        let _ = socket.send(&msg.serialize());
    }

    pub fn get_touch(&self, touch_id: u32, socket: &WaylandSocket) {
        let msg = Message::new(self.id, 2, vec![Argument::NewId(touch_id)]);
        let _ = socket.send(&msg.serialize());
    }
}

pub struct WlKeyboard {
    id: u32,
}

impl WlKeyboard {
    pub fn new(id: u32) -> Self {
        WlKeyboard { id }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

pub struct WlPointer {
    id: u32,
}

impl WlPointer {
    pub fn new(id: u32) -> Self {
        WlPointer { id }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}
