use crate::wire::protocol::{Argument, Message};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interface {
    WlDisplay,
    WlRegistry,
    WlCallback,
    WlCompositor,
    WlSurface,
    WlShm,
    WlShmPool,
    WlBuffer,
    WlSeat,
    WlKeyboard,
    WlPointer,
    XdgWmBase,
    XdgSurface,
    XdgToplevel,
    ZwlrLayerSurfaceV1,
    Unknown,
}

#[derive(Clone, Debug)]
pub enum Event {
    /// wl_registry.global(name, interface, version)
    RegistryGlobal {
        name: u32,
        interface: String,
        version: u32,
    },
    /// wl_registry.global_remove(name)
    RegistryGlobalRemove {
        name: u32,
    },
    /// wl_callback.done(serial)
    CallbackDone {
        callback_id: u32,
        serial: u32,
    },
    /// wl_display.error(object_id, code, message)
    DisplayError {
        object_id: u32,
        code: u32,
        message: String,
    },
    /// wl_display.delete_id(id)
    DisplayDeleteId {
        id: u32,
    },
    /// xdg_wm_base.ping(serial)
    XdgPing {
        serial: u32,
    },
    /// xdg_surface.configure(serial) — client must ack_configure
    XdgSurfaceConfigure {
        surface_id: u32,
        serial: u32,
    },
    /// xdg_toplevel.configure(width, height, states)
    XdgToplevelConfigure {
        toplevel_id: u32,
        width: i32,
        height: i32,
    },
    /// xdg_toplevel.close — compositor asked window to close
    XdgToplevelClose,
    /// zwlr_layer_surface_v1.configure(serial, width, height)
    LayerSurfaceConfigure {
        object_id: u32,
        serial: u32,
        width: u32,
        height: u32,
    },
    /// zwlr_layer_surface_v1.closed()
    LayerSurfaceClosed {
        object_id: u32,
    },
    /// wl_keyboard.key(serial, time, key, state)
    KeyboardKey {
        serial: u32,
        time: u32,
        key: u32,
        state: u32,
    },
    /// wl_keyboard.modifiers(serial, depressed, latched, locked, group)
    KeyboardModifiers {
        serial: u32,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
    },
    /// wl_pointer.motion(time, surface_x, surface_y)
    PointerMotion {
        surface_x: f64,
        surface_y: f64,
    },
    /// wl_pointer.enter(serial, surface, surface_x, surface_y)
    PointerEnter {
        surface_x: f64,
        surface_y: f64,
    },
    /// wl_pointer.button(serial, time, button, state)
    PointerButton {
        button: u32,
        state: u32,
    },
    /// wl_buffer.release — compositor done with buffer, client can reuse
    BufferRelease {
        buffer_id: u32,
    },
    /// wl_shm.format(format) — supported pixel format
    ShmFormat {
        format: u32,
    },
}

impl Event {
    /// Signature string for a given (interface, opcode) event.
    /// Empty string means we don't care about this event.
    fn signature_for(interface: Interface, opcode: u16) -> &'static str {
        match (interface, opcode) {
            (Interface::WlDisplay, 0) => "ous",   // error
            (Interface::WlDisplay, 1) => "u",     // delete_id
            (Interface::WlRegistry, 0) => "usu",  // global
            (Interface::WlRegistry, 1) => "u",    // global_remove
            (Interface::WlCallback, 0) => "u",    // done
            (Interface::WlShm, 0) => "u",         // format
            (Interface::WlBuffer, 0) => "",       // release (no args)
            (Interface::WlKeyboard, 3) => "uuuu", // key
            (Interface::WlKeyboard, 4) => "uuuuu",// modifiers
            (Interface::WlPointer, 0) => "uoff", // enter: serial, surface, surface_x, surface_y
            (Interface::WlPointer, 2) => "uff", // motion: time, surface_x, surface_y
            (Interface::WlPointer, 3) => "uuuu", // button: serial, time, button, state
            (Interface::XdgWmBase, 0) => "u",     // ping
            (Interface::XdgSurface, 0) => "u",    // configure
            (Interface::XdgToplevel, 0) => "iia", // configure
            (Interface::XdgToplevel, 1) => "",    // close
            (Interface::ZwlrLayerSurfaceV1, 0) => "uuu", // configure: serial, width, height
            (Interface::ZwlrLayerSurfaceV1, 1) => "",     // closed: no args
            _ => "",
        }
    }

    pub fn from_message(msg: &Message, interface: Interface) -> Option<Event> {
        let sig = Self::signature_for(interface, msg.header.opcode);
        let args = msg.parse_args(sig);

        match (interface, msg.header.opcode) {
            (Interface::WlDisplay, 0) => {
                if args.len() < 3 { return None; }
                if let (Argument::Object(oid), Argument::Uint(code), Argument::String(m)) =
                    (&args[0], &args[1], &args[2])
                {
                    Some(Event::DisplayError { object_id: *oid, code: *code, message: m.clone() })
                } else { None }
            }
            (Interface::WlDisplay, 1) => {
                if let Some(Argument::Uint(id)) = args.first() {
                    Some(Event::DisplayDeleteId { id: *id })
                } else { None }
            }
            (Interface::WlRegistry, 0) => {
                if args.len() < 3 { return None; }
                if let (Argument::Uint(name), Argument::String(iface), Argument::Uint(v)) =
                    (&args[0], &args[1], &args[2])
                {
                    Some(Event::RegistryGlobal {
                        name: *name,
                        interface: iface.clone(),
                        version: *v,
                    })
                } else { None }
            }
            (Interface::WlRegistry, 1) => {
                if let Some(Argument::Uint(name)) = args.first() {
                    Some(Event::RegistryGlobalRemove { name: *name })
                } else { None }
            }
            (Interface::WlCallback, 0) => {
                if let Some(Argument::Uint(s)) = args.first() {
                    Some(Event::CallbackDone { callback_id: msg.header.sender_id, serial: *s })
                } else { None }
            }
            (Interface::WlShm, 0) => {
                if let Some(Argument::Uint(f)) = args.first() {
                    Some(Event::ShmFormat { format: *f })
                } else { None }
            }
            (Interface::WlBuffer, 0) => {
                Some(Event::BufferRelease { buffer_id: msg.header.sender_id })
            }
            (Interface::WlKeyboard, 3) => {
                if args.len() < 4 { return None; }
                if let (Argument::Uint(serial), Argument::Uint(time),
                        Argument::Uint(key), Argument::Uint(state)) =
                    (&args[0], &args[1], &args[2], &args[3])
                {
                    Some(Event::KeyboardKey {
                        serial: *serial, time: *time, key: *key, state: *state,
                    })
                } else { None }
            }
            (Interface::WlKeyboard, 4) => {
                if args.len() < 5 { return None; }
                if let (Argument::Uint(s), Argument::Uint(d), Argument::Uint(l),
                        Argument::Uint(lk), Argument::Uint(g)) =
                    (&args[0], &args[1], &args[2], &args[3], &args[4])
                {
                    Some(Event::KeyboardModifiers {
                        serial: *s, mods_depressed: *d, mods_latched: *l,
                        mods_locked: *lk, group: *g,
                    })
                } else { None }
            }
            (Interface::XdgWmBase, 0) => {
                if let Some(Argument::Uint(s)) = args.first() {
                    Some(Event::XdgPing { serial: *s })
                } else { None }
            }
            (Interface::XdgSurface, 0) => {
                if let Some(Argument::Uint(s)) = args.first() {
                    Some(Event::XdgSurfaceConfigure {
                        surface_id: msg.header.sender_id,
                        serial: *s,
                    })
                } else { None }
            }
            (Interface::XdgToplevel, 0) => {
                if args.len() < 2 { return None; }
                if let (Argument::Int(w), Argument::Int(h)) = (&args[0], &args[1]) {
                    Some(Event::XdgToplevelConfigure {
                        toplevel_id: msg.header.sender_id,
                        width: *w,
                        height: *h,
                    })
                } else { None }
            }
            (Interface::XdgToplevel, 1) => Some(Event::XdgToplevelClose),
            (Interface::ZwlrLayerSurfaceV1, 0) => {
                if args.len() < 3 { return None; }
                if let (Argument::Uint(serial), Argument::Uint(w), Argument::Uint(h)) =
                    (&args[0], &args[1], &args[2])
                {
                    Some(Event::LayerSurfaceConfigure {
                        object_id: msg.header.sender_id,
                        serial: *serial,
                        width: *w,
                        height: *h,
                    })
                } else { None }
            }
            (Interface::ZwlrLayerSurfaceV1, 1) => {
                Some(Event::LayerSurfaceClosed { object_id: msg.header.sender_id })
            }
            (Interface::WlPointer, 2) => {
                if args.len() < 3 { return None; }
                if let (_, Argument::Fixed(fx), Argument::Fixed(fy)) =
                    (&args[0], &args[1], &args[2])
                {
                    Some(Event::PointerMotion {
                        surface_x: (*fx as f64) / 256.0,
                        surface_y: (*fy as f64) / 256.0,
                    })
                } else { None }
            }
            (Interface::WlPointer, 0) => {
                if args.len() < 4 { return None; }
                if let (_, _, Argument::Fixed(fx), Argument::Fixed(fy)) =
                    (&args[0], &args[1], &args[2], &args[3])
                {
                    Some(Event::PointerEnter {
                        surface_x: (*fx as f64) / 256.0,
                        surface_y: (*fy as f64) / 256.0,
                    })
                } else { None }
            }
            (Interface::WlPointer, 3) => {
                if args.len() < 4 { return None; }
                if let (_, _, Argument::Uint(button), Argument::Uint(state)) =
                    (&args[0], &args[1], &args[2], &args[3])
                {
                    Some(Event::PointerButton { button: *button, state: *state })
                } else { None }
            }
            _ => None,
        }
    }
}
