use crate::wire::protocol::{Argument, Message};
use crate::wire::registry::ObjectRegistry;
use crate::wire::socket::WaylandSocket;

pub struct WlSeat {
    id: u32,
}

impl WlSeat {
    pub fn new(id: u32) -> Self {
        WlSeat { id }
    }

    pub fn get_keyboard(&self, registry: &mut ObjectRegistry, socket: &WaylandSocket) -> u32 {
        let keyboard_id = registry.allocate();
        let msg = Message::new(
            self.id,
            0, // get_keyboard opcode
            vec![Argument::NewId(keyboard_id)],
        );
        let _ = socket.send(&msg.serialize());
        keyboard_id
    }
}

pub struct WlKeyboard {
    id: u32,
}

impl WlKeyboard {
    pub fn new(id: u32) -> Self {
        WlKeyboard { id }
    }
}
