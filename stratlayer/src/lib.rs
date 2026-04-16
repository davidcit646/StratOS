pub mod events;
pub mod protocols;
pub mod shm;
pub mod wire;

pub use events::Event;
pub use protocols::{WlCompositor, WlDisplay, WlRegistry, WlSurface, WlShm, WlShmPool, WlBuffer, XdgWmBase, XdgSurface, XdgToplevel};
pub use shm::{ShmPool, ShmBuffer};
pub use wire::{WaylandSocket, Dispatcher, ObjectRegistry, Message, Argument, MessageHeader};

pub struct WaylandClient {
    socket: WaylandSocket,
    registry: ObjectRegistry,
    dispatcher: Dispatcher,
}

impl WaylandClient {
    pub fn connect(display_name: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = WaylandSocket::connect(display_name)?;
        let registry = ObjectRegistry::new();
        let dispatcher = Dispatcher::new(WaylandSocket::connect(display_name)?);
        
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
}
