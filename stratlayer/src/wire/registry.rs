use std::any::Any;
use std::collections::HashMap;
use crate::events::Interface;

pub struct ObjectRegistry {
    next_id: u32,
    objects: HashMap<u32, Box<dyn Any>>,
    interface_map: HashMap<u32, Interface>,
}

impl ObjectRegistry {
    pub fn new() -> Self {
        ObjectRegistry {
            next_id: 2, // wl_display reserves id 1
            objects: HashMap::new(),
            interface_map: HashMap::new(),
        }
    }

    pub fn allocate(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn register<T: 'static>(&mut self, id: u32, object: T) {
        self.objects.insert(id, Box::new(object));
    }

    pub fn set_interface(&mut self, id: u32, interface: Interface) {
        self.interface_map.insert(id, interface);
    }

    pub fn get_interface(&self, id: u32) -> Interface {
        self.interface_map.get(&id).copied().unwrap_or(Interface::Unknown)
    }

    pub fn get<T: 'static>(&self, id: u32) -> Option<&T> {
        self.objects.get(&id).and_then(|obj| obj.downcast_ref())
    }

    pub fn get_mut<T: 'static>(&mut self, id: u32) -> Option<&mut T> {
        self.objects.get_mut(&id).and_then(|obj| obj.downcast_mut())
    }

    pub fn remove(&mut self, id: u32) -> Option<Box<dyn Any>> {
        self.interface_map.remove(&id);
        self.objects.remove(&id)
    }
}

impl Default for ObjectRegistry {
    fn default() -> Self {
        Self::new()
    }
}
