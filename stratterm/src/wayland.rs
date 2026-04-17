use stratlayer::{WaylandClient, protocols::{WlCompositor, WlDisplay, WlRegistry, WlSeat, WlShm, WlShmPool, WlBuffer, XdgWmBase, XdgSurface, XdgToplevel, WlSurface}, shm::ShmPool, events::{Event, Interface}};
use std::os::unix::io::RawFd;
use std::collections::HashMap;

pub struct WaylandWindow {
    client: WaylandClient,
    registry_id: u32,
    globals: HashMap<String, u32>, // interface name -> global name
    compositor_id: u32,
    shm_id: u32,
    shm_pool_id: u32,
    seat_id: u32,
    xdg_wm_base_id: u32,
    surface_id: u32,
    xdg_surface_id: u32,
    xdg_toplevel_id: u32,
    keyboard_id: u32,
    width: i32,
    height: i32,
    pending_width: Option<i32>,
    pending_height: Option<i32>,
    shm_pool: Option<ShmPool>,
    pool_size: usize,
}

impl WaylandWindow {
    pub fn new() -> Result<Self, String> {
        let mut client = WaylandClient::new()
            .map_err(|e| format!("Failed to connect to Wayland: {}", e))?;
        
        let registry = client.registry();
        let socket = client.socket();
        
        // Allocate IDs
        let display_id = 1; // Display is always 1
        let registry_id = registry.allocate();
        
        // Send wl_display.get_registry
        let display = WlDisplay::new(display_id);
        display.get_registry(registry, socket);
        
        // Read registry globals by polling for events
        let events = client.poll()
            .map_err(|e| format!("Failed to poll for registry globals: {}", e))?;
        
        // Collect global names from RegistryGlobal events
        let mut globals = HashMap::new();
        for event in &events {
            if let Event::RegistryGlobal { name, interface, .. } = event {
                globals.insert(interface.clone(), *name);
            }
        }
        
        // Bind globals using compositor-assigned names
        let compositor_name = globals.get("wl_compositor")
            .ok_or("wl_compositor not found in registry")?;
        let compositor_id = WlRegistry::new(registry_id).bind(*compositor_name, "wl_compositor", 1, registry, socket);
        
        let shm_name = globals.get("wl_shm")
            .ok_or("wl_shm not found in registry")?;
        let shm_id = WlRegistry::new(registry_id).bind(*shm_name, "wl_shm", 1, registry, socket);
        
        let seat_name = globals.get("wl_seat")
            .ok_or("wl_seat not found in registry")?;
        let seat_id = WlRegistry::new(registry_id).bind(*seat_name, "wl_seat", 1, registry, socket);
        
        let xdg_wm_base_name = globals.get("xdg_wm_base")
            .ok_or("xdg_wm_base not found in registry")?;
        let xdg_wm_base_id = WlRegistry::new(registry_id).bind(*xdg_wm_base_name, "xdg_wm_base", 1, registry, socket);
        
        // Allocate remaining IDs
        let surface_id = registry.allocate();
        let xdg_surface_id = registry.allocate();
        let xdg_toplevel_id = registry.allocate();
        let keyboard_id = registry.allocate();
        
        // Create surface
        let compositor = WlCompositor::new(compositor_id);
        compositor.create_surface(registry, socket);
        
        // Create xdg surface
        let xdg_wm_base = XdgWmBase::new(xdg_wm_base_id);
        xdg_wm_base.get_xdg_surface(surface_id, registry, socket);
        
        // Create xdg toplevel
        let xdg_surface = XdgSurface::new(xdg_surface_id);
        xdg_surface.get_toplevel(registry, socket);
        
        // Set title "StratTerm"
        let xdg_toplevel = XdgToplevel::new(xdg_toplevel_id);
        xdg_toplevel.set_title("StratTerm", socket);
        
        // Commit surface
        let surface = WlSurface::new(surface_id);
        surface.commit(socket);
        
        // Get keyboard
        let seat = WlSeat::new(seat_id);
        seat.get_keyboard(registry, socket);
        
        // Create SHM pool for buffer management
        let initial_pool_size = 800 * 600 * 4; // width * height * 4 bytes per pixel (ARGB8888)
        let shm_pool = ShmPool::create(initial_pool_size)
            .map_err(|e| format!("Failed to create SHM pool: {}", e))?;
        
        let shm_pool_id = registry.allocate();
        
        // Announce pool to compositor NOW (required before any buffer creation)
        let shm = WlShm::new(shm_id);
        let pool_fd = shm_pool.fd();
        shm.create_pool(pool_fd, initial_pool_size as i32, registry, socket);
        
        // Register interface mappings so events route correctly
        registry.set_interface(registry_id, Interface::WlRegistry);
        registry.set_interface(xdg_wm_base_id, Interface::XdgWmBase);
        registry.set_interface(xdg_surface_id, Interface::XdgSurface);
        registry.set_interface(keyboard_id, Interface::WlKeyboard);
        
        Ok(WaylandWindow {
            client,
            registry_id,
            globals,
            compositor_id,
            shm_id,
            shm_pool_id,
            seat_id,
            xdg_wm_base_id,
            surface_id,
            xdg_surface_id,
            xdg_toplevel_id,
            keyboard_id,
            width: 800,
            height: 600,
            pending_width: None,
            pending_height: None,
            shm_pool: Some(shm_pool),
            pool_size: initial_pool_size,
        })
    }
    
    pub fn poll_events(&mut self) -> Result<Vec<Event>, String> {
        self.client.poll().map_err(|e| e.to_string())
    }
    
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::XdgPing { serial } => {
                let xdg_wm_base = XdgWmBase::new(self.xdg_wm_base_id);
                xdg_wm_base.pong(*serial, self.client.socket());
            }
            _ => {}
        }
    }
    
    pub fn ack_configure(&mut self, serial: u32) {
        let xdg_surface = XdgSurface::new(self.xdg_surface_id);
        xdg_surface.ack_configure(serial, self.client.socket());
    }
    
    pub fn set_pending_size(&mut self, width: i32, height: i32) {
        self.pending_width = Some(width);
        self.pending_height = Some(height);
    }
    
    pub fn commit_pending_size(&mut self) -> Option<(i32, i32)> {
        if let (Some(width), Some(height)) = (self.pending_width, self.pending_height) {
            self.width = width;
            self.height = height;
            self.pending_width = None;
            self.pending_height = None;
            Some((width, height))
        } else {
            None
        }
    }
    
    pub fn get_size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
    
    pub fn render_buffer(&mut self, data: &[u8], width: u32, height: u32) -> Result<(), String> {
        let pool = self.shm_pool.as_mut().ok_or("SHM pool not initialized")?;
        let registry = self.client.registry();
        
        // Resize pool if needed
        let required_size = data.len();
        if required_size > self.pool_size {
            pool.resize(required_size)
                .map_err(|e| format!("Failed to resize SHM pool: {}", e))?;
            self.pool_size = required_size;
            
            // Recreate wl_shm_pool object after resize
            let shm = WlShm::new(self.shm_id);
            let pool_fd = pool.fd();
            shm.create_pool(pool_fd, required_size as i32, registry, self.client.socket());
        }
        
        // Write pixel data into pool
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), pool.ptr(), data.len());
        }
        
        // Create buffer from pool
        let shm_pool = WlShmPool::new(self.shm_pool_id);
        let buffer_id = registry.allocate();
        let stride = (width * 4) as i32; // ARGB8888 = 4 bytes per pixel
        shm_pool.create_buffer(0, width as i32, height as i32, stride, 0, registry, self.client.socket());
        
        // Attach buffer to surface
        let surface = WlSurface::new(self.surface_id);
        surface.attach(buffer_id, 0, 0, self.client.socket());
        
        // Damage full surface
        surface.damage(0, 0, width as i32, height as i32, self.client.socket());
        
        // Commit surface
        surface.commit(self.client.socket());
        
        Ok(())
    }
    
    pub fn raw_fd(&self) -> RawFd {
        self.client.raw_fd()
    }
}
