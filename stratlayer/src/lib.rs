pub mod events;
pub mod protocols;
pub mod shm;
pub mod wire;

pub use events::{Event, Interface};
pub use protocols::{WlCompositor, WlDisplay, WlRegistry, WlSurface, WlShm, WlShmPool, WlBuffer, XdgWmBase, XdgSurface, XdgToplevel, WlSeat, WlKeyboard};
pub use shm::{ShmPool, ShmBuffer};
pub use wire::{WaylandSocket, Dispatcher, ObjectRegistry, Message, Argument, MessageHeader};

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
        let registry = ObjectRegistry::new();
        let dispatcher = Dispatcher::from_fd(fd);
        
        Ok(WaylandClient {
            socket,
            registry,
            dispatcher,
        })
    }

    pub fn registry(&mut self) -> &mut ObjectRegistry {
        &mut self.registry
    }

    pub fn dispatcher(&mut self) -> &mut Dispatcher {
        &mut self.dispatcher
    }

    pub fn socket(&self) -> &WaylandSocket {
        &self.socket
    }

    pub fn raw_fd(&self) -> std::os::unix::io::RawFd {
        self.socket.raw_fd()
    }

    pub fn poll(&mut self) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let messages = self.dispatcher.dispatch_once()?;
        let events = messages.iter()
            .filter_map(|msg| {
                let iface = self.registry.get_interface(msg.header.sender_id);
                Event::from_message(msg, iface)
            })
            .collect();
        Ok(events)
    }
}
