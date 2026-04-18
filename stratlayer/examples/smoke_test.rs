use stratlayer::{
    Event, Interface,
    WaylandClient, WlDisplay, WlRegistry, WlCompositor, WlSurface, WlShm, WlShmPool,
    XdgWmBase, XdgSurface, XdgToplevel,
    ShmPool, ShmBuffer,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("StratLayer smoke test - connecting to Wayland...");

    let mut client = WaylandClient::connect(None)?;

    println!("Connected to Wayland display");

    // Get registry
    let registry_id = client.registry().allocate();
    client.registry().set_interface(registry_id, Interface::WlRegistry);
    WlDisplay::new(1).get_registry(registry_id, client.socket());

    // Collect globals via roundtrip
    let globals = client.roundtrip()?;

    let mut compositor_name: Option<u32> = None;
    let mut shm_name: Option<u32> = None;
    let mut xdg_wm_base_name: Option<u32> = None;

    for event in &globals {
        if let Event::RegistryGlobal { name, interface, .. } = event {
            match interface.as_str() {
                "wl_compositor" => compositor_name = Some(*name),
                "wl_shm" => shm_name = Some(*name),
                "xdg_wm_base" => xdg_wm_base_name = Some(*name),
                _ => {}
            }
        }
    }

    let compositor_name = compositor_name.ok_or("wl_compositor not found")?;
    let shm_name = shm_name.ok_or("wl_shm not found")?;
    let xdg_wm_base_name = xdg_wm_base_name.ok_or("xdg_wm_base not found")?;

    // Bind compositor
    let compositor_id = client.registry().allocate();
    client.registry().set_interface(compositor_id, Interface::WlCompositor);
    WlRegistry::new(registry_id).bind(compositor_name, "wl_compositor", 4, compositor_id, client.socket());

    // Bind xdg_wm_base
    let xdg_wm_base_id = client.registry().allocate();
    WlRegistry::new(registry_id).bind(xdg_wm_base_name, "xdg_wm_base", 1, xdg_wm_base_id, client.socket());

    // Bind shm
    let shm_id = client.registry().allocate();
    client.registry().set_interface(shm_id, Interface::WlShm);
    WlRegistry::new(registry_id).bind(shm_name, "wl_shm", 1, shm_id, client.socket());

    // Create surface
    let surface_id = client.registry().allocate();
    client.registry().set_interface(surface_id, Interface::WlSurface);
    WlCompositor::new(compositor_id).create_surface(surface_id, client.socket());

    // Get xdg_surface
    let xdg_surface_id = client.registry().allocate();
    XdgWmBase::new(xdg_wm_base_id).get_xdg_surface(xdg_surface_id, surface_id, client.socket());

    // Get toplevel
    let toplevel_id = client.registry().allocate();
    XdgSurface::new(xdg_surface_id).get_toplevel(toplevel_id, client.socket());
    let toplevel = XdgToplevel::new(toplevel_id);

    // Set window title
    toplevel.set_title("StratLayer Smoke Test", client.socket());
    toplevel.set_app_id("stratlayer.smoke_test", client.socket());

    // Create SHM pool (256x256 ARGB8888 = 256*256*4 = 262144 bytes)
    let pool_size = 256 * 256 * 4;
    let pool = ShmPool::create(pool_size)?;

    let pool_id = client.registry().allocate();
    client.registry().set_interface(pool_id, Interface::WlShmPool);
    WlShm::new(shm_id).create_pool(pool_id, pool.fd(), pool_size as i32, client.socket());

    // Create buffer
    let buffer_id = client.registry().allocate();
    client.registry().set_interface(buffer_id, Interface::WlBuffer);
    WlShmPool::new(pool_id).create_buffer(buffer_id, 0, 256, 256, 256 * 4, 0, client.socket());

    // Fill buffer with solid blue
    let mut buffer = ShmBuffer::new(pool, 0, 256, 256, 256 * 4);
    buffer.fill_solid_blue();

    // Attach buffer to surface
    WlSurface::new(surface_id).attach(buffer_id, 0, 0, client.socket());
    WlSurface::new(surface_id).damage(0, 0, 256, 256, client.socket());
    WlSurface::new(surface_id).commit(client.socket());

    println!("Window configured and committed");

    // Event loop briefly
    let mut iterations = 0;
    while iterations < 100 {
        if let Err(e) = client.poll() {
            eprintln!("Poll error: {}", e);
            break;
        }
        iterations += 1;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    println!("Smoke test completed successfully");
    Ok(())
}
