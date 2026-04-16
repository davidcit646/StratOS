use crate::wire::protocol::{Message, MessageHeader};
use crate::wire::socket::WaylandSocket;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type Handler = Box<dyn Fn(&Message) + Send>;

pub struct Dispatcher {
    socket: WaylandSocket,
    handlers: Arc<Mutex<HashMap<(u32, u16), Handler>>>,
}

impl Dispatcher {
    pub fn new(socket: WaylandSocket) -> Self {
        Dispatcher {
            socket,
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_handler<F>(&self, sender_id: u32, opcode: u16, handler: F)
    where
        F: Fn(&Message) + Send + 'static,
    {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.insert((sender_id, opcode), Box::new(handler));
    }

    pub fn dispatch_once(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = vec![0u8; 4096];
        let bytes_read = self.socket.receive(&mut buffer)?;
        
        if bytes_read < 8 {
            return Err("Invalid message: too short".into());
        }

        if let Some(message) = Message::deserialize(&buffer[..bytes_read]) {
            let handlers = self.handlers.lock().unwrap();
            let key = (message.header.sender_id, message.header.opcode);
            if let Some(handler) = handlers.get(&key) {
                handler(&message);
            }
        }

        Ok(())
    }

    pub fn dispatch_loop<F>(&self, mut should_stop: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut() -> bool,
    {
        while !should_stop() {
            self.dispatch_once()?;
        }
        Ok(())
    }
}
