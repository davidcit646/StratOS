use stratlayer::{WaylandClient, WlDisplay, WlRegistry, WlCompositor, WlSurface, WlShm, WlShmPool, XdgWmBase, XdgSurface, XdgToplevel, ShmPool, ShmBuffer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("StratLayer smoke test - connecting to Wayland...");
    
    let mut client = WaylandClient::connect(None)?;
    let registry = client.registry();
    
    println!("Connected to Wayland display");
    
    // Allocate display object (ID 1 is always the display)
    let display_id = 1;
    let display = WlDisplay::new(display_id);
    
    // Get registry
    let registry_id = display.get_registry(registry);
    let wl_registry = WlRegistry::new(registry_id);
    
    // Bind compositor
    let compositor_id = wl_registry.bind("wl_compositor", 1, registry);
    let compositor = WlCompositor::new(compositor_id);
    
    // Bind xdg_wm_base
    let xdg_wm_base_id = wl_registry.bind("xdg_wm_base", 1, registry);
    let xdg_wm_base = XdgWmBase::new(xdg_wm_base_id);
    
    // Create surface
    let surface_id = compositor.create_surface(registry);
    let surface = WlSurface::new(surface_id);
    
    // Get xdg_surface
    let xdg_surface_id = xdg_wm_base.get_xdg_surface(surface_id, registry);
    let xdg_surface = XdgSurface::new(xdg_surface_id);
    
    // Get toplevel
    let toplevel_id = xdg_surface.get_toplevel(registry);
    let toplevel = XdgToplevel::new(toplevel_id);
    
    // Set window title
    toplevel.set_title("StratLayer Smoke Test");
    toplevel.set_app_id("stratlayer.smoke_test");
    
    // Bind shm
    let shm_id = wl_registry.bind("wl_shm", 1, registry);
    let wl_shm = WlShm::new(shm_id);
    
    // Create SHM pool (256x256 ARGB8888 = 256*256*4 = 262144 bytes)
    let pool_size = 256 * 256 * 4;
    let pool = ShmPool::create("/stratlayer_shm_pool", pool_size)?;
    let shm_pool_id = wl_shm.create_pool(pool.fd(), pool_size as i32, registry);
    let wl_shm_pool = WlShmPool::new(shm_pool_id);
    
    // Create buffer
    let buffer_id = wl_shm_pool.create_buffer(0, 256, 256, 256 * 4, 0, registry);
    let wl_buffer = WlBuffer::new(buffer_id);
    
    // Fill buffer with solid blue
    let mut buffer = ShmBuffer::new(pool, 0, 256, 256, 256 * 4);
    buffer.fill_solid_blue();
    
    // Attach buffer to surface
    surface.attach(buffer_id, 0, 0);
    surface.damage(0, 0, 256, 256);
    surface.commit();
    
    println!("Window configured and committed");
    
    // Event loop with configure handling
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    
    client.dispatcher().register_handler(xdg_surface_id, 0, move |_msg| {
        println!("Received configure event");
        // In a full implementation, we would ack_configure here
        running_clone.store(false, Ordering::SeqCst);
    });
    
    // Run event loop briefly
    let mut iterations = 0;
    while running.load(Ordering::SeqCst) && iterations < 100 {
        if let Err(e) = client.dispatcher().dispatch_once() {
            eprintln!("Dispatch error: {}", e);
            break;
        }
        iterations += 1;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    println!("Smoke test completed successfully");
    Ok(())
}
