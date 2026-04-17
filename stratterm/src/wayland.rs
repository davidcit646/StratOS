use stratlayer::{
    Event, Interface, ShmPool, WaylandClient, WlBuffer, WlCompositor, WlDisplay, WlRegistry,
    WlSeat, WlShm, WlShmPool, WlSurface, XdgSurface, XdgToplevel, XdgWmBase,
};
use std::collections::HashMap;
use std::os::unix::io::RawFd;

const WL_SHM_FORMAT_ARGB8888: u32 = 0;

pub struct WaylandWindow {
    client: WaylandClient,
    #[allow(dead_code)]
    registry_id: u32,
    #[allow(dead_code)]
    compositor_id: u32,
    #[allow(dead_code)]
    shm_id: u32,
    #[allow(dead_code)]
    seat_id: u32,
    xdg_wm_base_id: u32,
    surface_id: u32,
    xdg_surface_id: u32,
    #[allow(dead_code)]
    xdg_toplevel_id: u32,
    #[allow(dead_code)]
    keyboard_id: u32,

    shm_pool_id: u32,
    shm_pool: ShmPool,
    pool_size: usize,

    width: i32,
    height: i32,
    pending_width: Option<i32>,
    pending_height: Option<i32>,

    current_buffer_id: Option<u32>,
    configured: bool,
}

impl WaylandWindow {
    pub fn new(initial_width: i32, initial_height: i32) -> Result<Self, String> {
        let mut client = WaylandClient::new()
            .map_err(|e| format!("Failed to connect to Wayland: {}", e))?;

        let registry_id = client.registry().allocate();
        client.registry().set_interface(registry_id, Interface::WlRegistry);

        let display = WlDisplay::new(1);
        display.get_registry(registry_id, client.socket());

        // Roundtrip #1: collect all registry globals.
        let globals_events = client
            .roundtrip()
            .map_err(|e| format!("Roundtrip for globals failed: {}", e))?;

        let mut globals: HashMap<String, (u32, u32)> = HashMap::new();
        for ev in globals_events {
            if let Event::RegistryGlobal { name, interface, version } = ev {
                globals.insert(interface, (name, version));
            }
        }

        let compositor_id = Self::bind_global(
            &mut client, registry_id, &globals, "wl_compositor", 4, Interface::WlCompositor,
        )?;
        let shm_id = Self::bind_global(
            &mut client, registry_id, &globals, "wl_shm", 1, Interface::WlShm,
        )?;
        let seat_id = Self::bind_global(
            &mut client, registry_id, &globals, "wl_seat", 5, Interface::WlSeat,
        )?;
        let xdg_wm_base_id = Self::bind_global(
            &mut client, registry_id, &globals, "xdg_wm_base", 1, Interface::XdgWmBase,
        )?;

        let surface_id = client.registry().allocate();
        client.registry().set_interface(surface_id, Interface::WlSurface);

        let xdg_surface_id = client.registry().allocate();
        client.registry().set_interface(xdg_surface_id, Interface::XdgSurface);

        let xdg_toplevel_id = client.registry().allocate();
        client.registry().set_interface(xdg_toplevel_id, Interface::XdgToplevel);

        let keyboard_id = client.registry().allocate();
        client.registry().set_interface(keyboard_id, Interface::WlKeyboard);

        {
            let socket = client.socket();
            WlCompositor::new(compositor_id).create_surface(surface_id, socket);
            XdgWmBase::new(xdg_wm_base_id).get_xdg_surface(xdg_surface_id, surface_id, socket);
            XdgSurface::new(xdg_surface_id).get_toplevel(xdg_toplevel_id, socket);
            XdgToplevel::new(xdg_toplevel_id).set_title("StratTerm", socket);
            XdgToplevel::new(xdg_toplevel_id).set_app_id("stratos.stratterm", socket);
            WlSurface::new(surface_id).commit(socket);
            WlSeat::new(seat_id).get_keyboard(keyboard_id, socket);
        }

        // SHM pool.
        let pool_size = (initial_width * initial_height * 4) as usize;
        let shm_pool = ShmPool::create(pool_size)
            .map_err(|e| format!("SHM pool create failed: {}", e))?;

        let shm_pool_id = client.registry().allocate();
        client.registry().set_interface(shm_pool_id, Interface::WlShmPool);
        WlShm::new(shm_id).create_pool(
            shm_pool_id,
            shm_pool.fd(),
            pool_size as i32,
            client.socket(),
        );

        let mut window = WaylandWindow {
            client,
            registry_id,
            compositor_id,
            shm_id,
            seat_id,
            xdg_wm_base_id,
            surface_id,
            xdg_surface_id,
            xdg_toplevel_id,
            keyboard_id,
            shm_pool_id,
            shm_pool,
            pool_size,
            width: initial_width,
            height: initial_height,
            pending_width: None,
            pending_height: None,
            current_buffer_id: None,
            configured: false,
        };

        // Roundtrip #2: pull in the initial xdg_surface / xdg_toplevel configure.
        let events = window
            .client
            .roundtrip()
            .map_err(|e| format!("Roundtrip for initial configure failed: {}", e))?;
        window.apply_events(&events)?;

        Ok(window)
    }

    fn bind_global(
        client: &mut WaylandClient,
        registry_id: u32,
        globals: &HashMap<String, (u32, u32)>,
        interface_name: &str,
        max_version: u32,
        interface: Interface,
    ) -> Result<u32, String> {
        let (name, server_version) = globals
            .get(interface_name)
            .copied()
            .ok_or_else(|| format!("{} not advertised by compositor", interface_name))?;
        let version = server_version.min(max_version);

        let new_id = client.registry().allocate();
        client.registry().set_interface(new_id, interface);

        WlRegistry::new(registry_id).bind(name, interface_name, version, new_id, client.socket());
        Ok(new_id)
    }

    fn apply_events(&mut self, events: &[Event]) -> Result<(), String> {
        for ev in events {
            match ev {
                Event::XdgPing { serial } => {
                    XdgWmBase::new(self.xdg_wm_base_id).pong(*serial, self.client.socket());
                }
                Event::XdgToplevelConfigure { width, height, .. } => {
                    if *width > 0 && *height > 0 {
                        self.pending_width = Some(*width);
                        self.pending_height = Some(*height);
                    }
                }
                Event::XdgSurfaceConfigure { serial, .. } => {
                    XdgSurface::new(self.xdg_surface_id)
                        .ack_configure(*serial, self.client.socket());
                    self.configured = true;
                }
                Event::BufferRelease { buffer_id } => {
                    if Some(*buffer_id) == self.current_buffer_id {
                        WlBuffer::new(*buffer_id).destroy(self.client.socket());
                        self.current_buffer_id = None;
                    }
                }
                Event::DisplayError { object_id, code, message } => {
                    return Err(format!(
                        "Wayland protocol error on object {}: code={}, message={}",
                        object_id, code, message
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn poll_events(&mut self) -> Result<Vec<Event>, String> {
        let events = self
            .client
            .poll()
            .map_err(|e| format!("poll failed: {}", e))?;
        self.apply_events(&events)?;
        Ok(events)
    }

    pub fn commit_pending_size(&mut self) -> Option<(i32, i32)> {
        if let (Some(w), Some(h)) = (self.pending_width.take(), self.pending_height.take()) {
            self.width = w;
            self.height = h;
            Some((w, h))
        } else {
            None
        }
    }

    pub fn get_size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    pub fn is_configured(&self) -> bool {
        self.configured
    }

    pub fn render_buffer(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        if !self.configured {
            return Ok(());
        }

        let required = data.len();
        if required > self.pool_size {
            self.shm_pool
                .resize(required)
                .map_err(|e| format!("SHM pool resize failed: {}", e))?;
            self.pool_size = required;
            WlShmPool::new(self.shm_pool_id).resize(required as i32, self.client.socket());
        }

        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.shm_pool.ptr(), data.len());
        }

        if let Some(old_id) = self.current_buffer_id.take() {
            WlBuffer::new(old_id).destroy(self.client.socket());
        }

        let buffer_id = self.client.registry().allocate();
        self.client.registry().set_interface(buffer_id, Interface::WlBuffer);

        let stride = (width * 4) as i32;
        WlShmPool::new(self.shm_pool_id).create_buffer(
            buffer_id,
            0,
            width as i32,
            height as i32,
            stride,
            WL_SHM_FORMAT_ARGB8888,
            self.client.socket(),
        );

        let surface = WlSurface::new(self.surface_id);
        surface.attach(buffer_id, 0, 0, self.client.socket());
        surface.damage(0, 0, width as i32, height as i32, self.client.socket());
        surface.commit(self.client.socket());

        self.current_buffer_id = Some(buffer_id);
        Ok(())
    }

    pub fn raw_fd(&self) -> RawFd {
        self.client.raw_fd()
    }
}
