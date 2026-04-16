use crate::wire::protocol::Message;

pub enum Event {
    XdgConfigure {
        surface_id: u32,
        width: i32,
        height: i32,
        serial: u32,
    },
    XdgPing {
        serial: u32,
    },
    RegistryGlobal {
        name: u32,
        interface: String,
        version: u32,
    },
}

impl Event {
    pub fn from_message(msg: &Message) -> Option<Self> {
        match msg.header.opcode {
            0 => {
                // xdg_surface.configure
                if msg.args.len() >= 1 {
                    if let crate::wire::protocol::Argument::Uint(serial) = &msg.args[0] {
                        Some(Event::XdgConfigure {
                            surface_id: msg.header.sender_id,
                            width: 256,
                            height: 256,
                            serial: *serial,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            1 => {
                // xdg_wm_base.ping
                if msg.args.len() >= 1 {
                    if let crate::wire::protocol::Argument::Uint(serial) = &msg.args[0] {
                        Some(Event::XdgPing { serial: *serial })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
