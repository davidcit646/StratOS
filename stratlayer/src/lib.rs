pub mod events;
pub mod protocols;
pub mod shm;
pub mod wire;

pub use events::{Event, Interface};
pub use protocols::{
    WlBuffer, WlCompositor, WlDisplay, WlKeyboard, WlPointer, WlRegistry, WlSeat, WlShm, WlShmPool,
    WlSurface, XdgSurface, XdgToplevel, XdgWmBase,
    ZwlrLayerShellV1, ZwlrLayerSurfaceV1, LAYER_TOP, ANCHOR_TOP, ANCHOR_LEFT, ANCHOR_RIGHT,
};
pub use shm::{ShmBuffer, ShmPool};
pub use wire::{Argument, Dispatcher, Message, MessageHeader, ObjectRegistry, WaylandSocket};

pub struct WaylandClient {
    socket: WaylandSocket,
    registry: ObjectRegistry,
    dispatcher: Dispatcher,
}

impl WaylandClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::connect(None)
    }

    pub fn connect(display_name: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = WaylandSocket::connect(display_name)?;
        let fd = socket.raw_fd();
        let mut registry = ObjectRegistry::new();
        registry.set_interface(1, Interface::WlDisplay);
        let dispatcher = Dispatcher::from_fd(fd);

        Ok(WaylandClient { socket, registry, dispatcher })
    }

    pub fn socket(&self) -> &WaylandSocket {
        &self.socket
    }

    pub fn registry(&mut self) -> &mut ObjectRegistry {
        &mut self.registry
    }

    pub fn register_layer_surface(&mut self, id: u32) {
        self.registry.set_interface(id, Interface::ZwlrLayerSurfaceV1);
    }

    pub fn raw_fd(&self) -> std::os::unix::io::RawFd {
        self.socket.raw_fd()
    }

    /// One round of receive + decode. Blocks on the socket.
    pub fn poll(&mut self) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let messages = self.dispatcher.dispatch_once()?;
        let mut events = Vec::with_capacity(messages.len());
        for msg in &messages {
            let iface = self.registry.get_interface(msg.header.sender_id);
            if let Some(ev) = Event::from_message(msg, iface) {
                events.push(ev);
            }
        }
        Ok(events)
    }

    /// wl_display.sync + block until the callback fires. Forces the compositor to
    /// flush everything it owes us (e.g. registry globals, configure events).
    pub fn roundtrip(&mut self) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let callback_id = self.registry.allocate();
        self.registry.set_interface(callback_id, Interface::WlCallback);

        let display = WlDisplay::new(1);
        display.sync(callback_id, &self.socket);

        let mut all_events = Vec::new();
        loop {
            let events = self.poll()?;
            let mut done = false;
            for ev in events {
                if let Event::CallbackDone { callback_id: cid, .. } = &ev {
                    if *cid == callback_id {
                        done = true;
                    }
                }
                all_events.push(ev);
            }
            if done {
                break;
            }
        }

        Ok(all_events)
    }
}
