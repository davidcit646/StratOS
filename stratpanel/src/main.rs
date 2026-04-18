use std::time::{Instant, Duration};

use stratlayer::{
    Event, Interface,
    WlCompositor, WlDisplay, WlRegistry, WlSeat, WlShm, WlShmPool, WlSurface,
    ZwlrLayerShellV1, ZwlrLayerSurfaceV1, LAYER_TOP, ANCHOR_TOP, ANCHOR_LEFT, ANCHOR_RIGHT,
    ShmPool, ShmBuffer,
    WaylandClient,
};

mod config;
mod ipc;
mod clock;
mod textinput;

fn draw_text(buf: &mut [u8], stride: u32, panel_width: i32, panel_height: i32,
             x: i32, y: i32, text: &str, color: u32) {
    const FONT: [(char, [u8; 7]); 95] = [
        ('0', [0x3E, 0x51, 0x49, 0x45, 0x3E, 0x00, 0x00]),
        ('1', [0x00, 0x42, 0x7F, 0x40, 0x00, 0x00, 0x00]),
        ('2', [0x42, 0x61, 0x51, 0x49, 0x46, 0x00, 0x00]),
        ('3', [0x21, 0x41, 0x45, 0x4B, 0x31, 0x00, 0x00]),
        ('4', [0x18, 0x14, 0x12, 0x7F, 0x10, 0x00, 0x00]),
        ('5', [0x27, 0x45, 0x45, 0x45, 0x39, 0x00, 0x00]),
        ('6', [0x3C, 0x4A, 0x49, 0x49, 0x30, 0x00, 0x00]),
        ('7', [0x01, 0x71, 0x09, 0x05, 0x03, 0x00, 0x00]),
        ('8', [0x36, 0x49, 0x49, 0x49, 0x36, 0x00, 0x00]),
        ('9', [0x06, 0x49, 0x49, 0x29, 0x1E, 0x00, 0x00]),
        ('A', [0x7E, 0x09, 0x09, 0x09, 0x7E, 0x00, 0x00]),
        ('B', [0x7F, 0x49, 0x49, 0x49, 0x36, 0x00, 0x00]),
        ('C', [0x3E, 0x41, 0x41, 0x41, 0x22, 0x00, 0x00]),
        ('D', [0x7F, 0x41, 0x41, 0x41, 0x3E, 0x00, 0x00]),
        ('E', [0x7F, 0x49, 0x49, 0x49, 0x41, 0x00, 0x00]),
        ('F', [0x7F, 0x09, 0x09, 0x09, 0x01, 0x00, 0x00]),
        ('G', [0x3E, 0x41, 0x49, 0x49, 0x3E, 0x00, 0x00]),
        ('H', [0x7F, 0x08, 0x08, 0x08, 0x7F, 0x00, 0x00]),
        ('I', [0x00, 0x41, 0x7F, 0x41, 0x00, 0x00, 0x00]),
        ('J', [0x1E, 0x20, 0x20, 0x20, 0x1F, 0x00, 0x00]),
        ('K', [0x7F, 0x08, 0x14, 0x22, 0x41, 0x00, 0x00]),
        ('L', [0x7F, 0x40, 0x40, 0x40, 0x40, 0x00, 0x00]),
        ('M', [0x7F, 0x02, 0x0C, 0x02, 0x7F, 0x00, 0x00]),
        ('N', [0x7F, 0x04, 0x08, 0x10, 0x7F, 0x00, 0x00]),
        ('O', [0x7E, 0x41, 0x41, 0x41, 0x7E, 0x00, 0x00]),
        ('P', [0x7F, 0x09, 0x09, 0x09, 0x06, 0x00, 0x00]),
        ('Q', [0x7E, 0x41, 0x51, 0x21, 0x5E, 0x00, 0x00]),
        ('R', [0x7F, 0x09, 0x19, 0x29, 0x46, 0x00, 0x00]),
        ('S', [0x46, 0x49, 0x49, 0x49, 0x31, 0x00, 0x00]),
        ('T', [0x01, 0x01, 0x7F, 0x01, 0x01, 0x00, 0x00]),
        ('U', [0x7F, 0x40, 0x40, 0x40, 0x3F, 0x00, 0x00]),
        ('V', [0x1F, 0x20, 0x40, 0x20, 0x1F, 0x00, 0x00]),
        ('W', [0x3F, 0x40, 0x38, 0x40, 0x3F, 0x00, 0x00]),
        ('X', [0x63, 0x14, 0x08, 0x14, 0x63, 0x00, 0x00]),
        ('Y', [0x03, 0x04, 0x78, 0x04, 0x03, 0x00, 0x00]),
        ('Z', [0x61, 0x51, 0x49, 0x45, 0x43, 0x00, 0x00]),
        ('a', [0x20, 0x54, 0x54, 0x54, 0x78, 0x00, 0x00]),
        ('b', [0x7F, 0x48, 0x44, 0x44, 0x38, 0x00, 0x00]),
        ('c', [0x38, 0x44, 0x44, 0x44, 0x20, 0x00, 0x00]),
        ('d', [0x38, 0x44, 0x44, 0x48, 0x7F, 0x00, 0x00]),
        ('e', [0x38, 0x54, 0x54, 0x54, 0x18, 0x00, 0x00]),
        ('f', [0x08, 0x7E, 0x09, 0x01, 0x02, 0x00, 0x00]),
        ('g', [0x08, 0x14, 0x54, 0x54, 0x3C, 0x00, 0x00]),
        ('h', [0x7F, 0x08, 0x04, 0x04, 0x78, 0x00, 0x00]),
        ('i', [0x00, 0x44, 0x7D, 0x40, 0x00, 0x00, 0x00]),
        ('j', [0x20, 0x40, 0x44, 0x3D, 0x00, 0x00, 0x00]),
        ('k', [0x7F, 0x10, 0x28, 0x44, 0x00, 0x00, 0x00]),
        ('l', [0x00, 0x41, 0x7F, 0x40, 0x00, 0x00, 0x00]),
        ('m', [0x7C, 0x04, 0x78, 0x04, 0x78, 0x00, 0x00]),
        ('n', [0x7C, 0x08, 0x04, 0x04, 0x78, 0x00, 0x00]),
        ('o', [0x38, 0x44, 0x44, 0x44, 0x38, 0x00, 0x00]),
        ('p', [0x7C, 0x14, 0x14, 0x14, 0x08, 0x00, 0x00]),
        ('q', [0x08, 0x14, 0x14, 0x18, 0x7C, 0x00, 0x00]),
        ('r', [0x7C, 0x08, 0x04, 0x04, 0x08, 0x00, 0x00]),
        ('s', [0x48, 0x54, 0x54, 0x54, 0x24, 0x00, 0x00]),
        ('t', [0x04, 0x7F, 0x44, 0x40, 0x20, 0x00, 0x00]),
        ('u', [0x3C, 0x40, 0x40, 0x20, 0x7C, 0x00, 0x00]),
        ('v', [0x1C, 0x20, 0x40, 0x20, 0x1C, 0x00, 0x00]),
        ('w', [0x3C, 0x40, 0x30, 0x40, 0x3C, 0x00, 0x00]),
        ('x', [0x44, 0x28, 0x10, 0x28, 0x44, 0x00, 0x00]),
        ('y', [0x0C, 0x50, 0x50, 0x50, 0x3C, 0x00, 0x00]),
        ('z', [0x44, 0x64, 0x54, 0x4C, 0x44, 0x00, 0x00]),
        (' ', [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        ('!', [0x00, 0x5F, 0x00, 0x00, 0x00, 0x00, 0x00]),
        ('@', [0x3C, 0x4A, 0x4A, 0x3C, 0x00, 0x00, 0x00]),
        ('#', [0x14, 0x7F, 0x14, 0x7F, 0x14, 0x00, 0x00]),
        ('$', [0x24, 0x2A, 0x7F, 0x2A, 0x12, 0x00, 0x00]),
        ('%', [0x62, 0x64, 0x08, 0x13, 0x23, 0x00, 0x00]),
        ('^', [0x04, 0x02, 0x01, 0x02, 0x04, 0x00, 0x00]),
        ('&', [0x36, 0x49, 0x55, 0x22, 0x50, 0x00, 0x00]),
        ('*', [0x44, 0x28, 0x7F, 0x28, 0x44, 0x00, 0x00]),
        ('(', [0x0E, 0x11, 0x11, 0x11, 0x0E, 0x00, 0x00]),
        (')', [0x70, 0x88, 0x88, 0x88, 0x70, 0x00, 0x00]),
        ('-', [0x00, 0x08, 0x7F, 0x08, 0x00, 0x00, 0x00]),
        ('_', [0x00, 0x00, 0x00, 0x00, 0x7F, 0x00, 0x00]),
        ('+', [0x00, 0x08, 0x2A, 0x08, 0x00, 0x00, 0x00]),
        ('=', [0x00, 0x14, 0x14, 0x14, 0x00, 0x00, 0x00]),
        ('[', [0x7F, 0x41, 0x41, 0x00, 0x00, 0x00, 0x00]),
        (']', [0x41, 0x41, 0x7F, 0x00, 0x00, 0x00, 0x00]),
        ('{', [0x14, 0x12, 0x7F, 0x12, 0x14, 0x00, 0x00]),
        ('}', [0x14, 0x48, 0x7F, 0x48, 0x14, 0x00, 0x00]),
        (':', [0x00, 0x36, 0x36, 0x00, 0x00, 0x00, 0x00]),
        (';', [0x00, 0x56, 0x36, 0x00, 0x00, 0x00, 0x00]),
        ('\'', [0x00, 0x06, 0x09, 0x00, 0x00, 0x00, 0x00]),
        ('"', [0x06, 0x09, 0x06, 0x09, 0x00, 0x00, 0x00]),
        ('`', [0x04, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00]),
        ('~', [0x08, 0x34, 0x28, 0x34, 0x08, 0x00, 0x00]),
        ('\\', [0x40, 0x20, 0x10, 0x08, 0x04, 0x00, 0x00]),
        ('|', [0x00, 0x7F, 0x00, 0x7F, 0x00, 0x00, 0x00]),
        (',', [0x00, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00]),
        ('.', [0x00, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00]),
        ('/', [0x04, 0x08, 0x10, 0x20, 0x40, 0x00, 0x00]),
        ('<', [0x08, 0x14, 0x22, 0x41, 0x00, 0x00, 0x00]),
        ('>', [0x41, 0x22, 0x14, 0x08, 0x00, 0x00, 0x00]),
        ('?', [0x02, 0x01, 0x51, 0x09, 0x06, 0x00, 0x00]),
    ];

    let bytes = color.to_le_bytes();
    let mut cursor_x = x;

    for ch in text.chars() {
        if ch == ' ' {
            cursor_x += 6;
            continue;
        }

        let glyph = FONT.iter().find(|(c, _)| *c == ch);
        if let Some((_, rows)) = glyph {
            for row_idx in 0..7 {
                let row = rows[row_idx];
                for col_idx in 0..5 {
                    if (row >> (4 - col_idx)) & 1 == 1 {
                        let px = cursor_x + col_idx as i32;
                        let py = y + row_idx as i32;
                        if px >= 0 && px < panel_width && py >= 0 && py < panel_height {
                            let offset = (py as u32 * stride + px as u32 * 4) as usize;
                            if offset + 4 <= buf.len() {
                                buf[offset..offset + 4].copy_from_slice(&bytes);
                            }
                        }
                    }
                }
            }
        }
        cursor_x += 6;
    }
}

fn fill_rect(buf: &mut [u8], stride: u32, panel_width: i32, panel_height: i32,
             x: i32, y: i32, w: i32, h: i32, color: u32) {
    let bytes = color.to_le_bytes();
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(panel_width);
    let y1 = (y + h).min(panel_height);

    for py in y0..y1 {
        for px in x0..x1 {
            let offset = (py as u32 * stride + px as u32 * 4) as usize;
            if offset + 4 <= buf.len() {
                buf[offset..offset + 4].copy_from_slice(&bytes);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Connect to Wayland
    let mut client = WaylandClient::new()?;

    // Setup: send get_registry using allocate() so next_id advances past it
    let registry_id = client.registry().allocate();
    client.registry().set_interface(registry_id, Interface::WlRegistry);
    WlDisplay::new(1).get_registry(registry_id, client.socket());

    // Step 2: Roundtrip to collect RegistryGlobal events
    let globals = client.roundtrip()?;

    // Load configuration
    let config = config::PanelConfig::load();

    // Connect to stratvm IPC
    let mut ipc = ipc::IpcClient::connect();
    ipc.set_panel_autohide(config.panel.autohide);

    let mut compositor_name: Option<u32> = None;
    let mut shm_name: Option<u32> = None;
    let mut layer_shell_name: Option<u32> = None;
    let mut seat_name: Option<u32> = None;

    for event in &globals {
        if let Event::RegistryGlobal { name, interface, .. } = event {
            match interface.as_str() {
                "wl_compositor" => compositor_name = Some(*name),
                "wl_shm" => shm_name = Some(*name),
                "zwlr_layer_shell_v1" => layer_shell_name = Some(*name),
                "wl_seat" => seat_name = Some(*name),
                _ => {}
            }
        }
    }

    let compositor_name = compositor_name.ok_or("wl_compositor not found")?;
    let shm_name = shm_name.ok_or("wl_shm not found")?;
    let layer_shell_name = layer_shell_name.ok_or("zwlr_layer_shell_v1 not found")?;

    // Bind wl_compositor
    let compositor_id = client.registry().allocate();
    client.registry().set_interface(compositor_id, Interface::WlCompositor);
    WlRegistry::new(registry_id).bind(compositor_name, "wl_compositor", 4, compositor_id, client.socket());

    // Bind wl_shm
    let shm_id = client.registry().allocate();
    client.registry().set_interface(shm_id, Interface::WlShm);
    WlRegistry::new(registry_id).bind(shm_name, "wl_shm", 1, shm_id, client.socket());

    // Bind zwlr_layer_shell_v1
    let layer_shell_id = client.registry().allocate();
    // No Interface variant for ZwlrLayerShellV1 in the enum; use Unknown
    WlRegistry::new(registry_id).bind(layer_shell_name, "zwlr_layer_shell_v1", 4, layer_shell_id, client.socket());

    // Bind wl_seat
    let seat_id = client.registry().allocate();
    client.registry().set_interface(seat_id, Interface::WlSeat);
    let seat_name = seat_name.ok_or("wl_seat not found")?;
    WlRegistry::new(registry_id).bind(seat_name, "wl_seat", 7, seat_id, client.socket());

    // Get pointer from seat
    let pointer_id = client.registry().allocate();
    client.registry().set_interface(pointer_id, Interface::WlPointer);
    WlSeat::new(seat_id).get_pointer(pointer_id, client.socket());

    // Get keyboard from seat
    let keyboard_id = client.registry().allocate();
    client.registry().set_interface(keyboard_id, Interface::WlKeyboard);
    WlSeat::new(seat_id).get_keyboard(keyboard_id, client.socket());

    // Step 3: Create wl_surface
    let surface_id = client.registry().allocate();
    client.registry().set_interface(surface_id, Interface::WlSurface);
    WlCompositor::new(compositor_id).create_surface(surface_id, client.socket());

    // Step 4: Create layer surface
    let layer_surface_id = client.registry().allocate();
    client.register_layer_surface(layer_surface_id);
    ZwlrLayerShellV1::new(layer_shell_id).get_layer_surface(
        layer_surface_id,
        surface_id,
        0,
        LAYER_TOP,
        "stratpanel",
        client.socket(),
    );

    // Step 5: Configure layer surface
    let ls = ZwlrLayerSurfaceV1::new(layer_surface_id);
    ls.set_size(0, config.panel.size, client.socket());
    ls.set_anchor(ANCHOR_TOP | ANCHOR_LEFT | ANCHOR_RIGHT, client.socket());
    ls.set_exclusive_zone(config.panel.size as i32, client.socket());
    ls.set_keyboard_interactivity(1, client.socket());
    WlSurface::new(surface_id).commit(client.socket());

    // Step 6: Wait for LayerSurfaceConfigure
    let (confirmed_width, confirmed_height);
    'configure: loop {
        for event in client.poll()? {
            if let Event::LayerSurfaceConfigure { serial, width, height, .. } = event {
                ls.ack_configure(serial, client.socket());
                confirmed_width = width;
                confirmed_height = height;
                break 'configure;
            }
        }
    }

    // Step 7: Allocate SHM buffer
    let panel_width = if confirmed_width == 0 { 1920 } else { confirmed_width };
    let panel_height = if confirmed_height == 0 { config.panel.size as i32 } else { confirmed_height as i32 };
    let stride = panel_width * 4;
    let size = (stride * panel_height as u32) as usize;

    let pool = ShmPool::create(size)?;
    let shm_fd = pool.fd();

    // Fill with configurable-opacity panel background (ARGB8888)
    let mut shm_buffer = ShmBuffer::new(pool, 0, panel_width, panel_height as u32, stride);
    {
        let data = shm_buffer.data_mut();
        let color = ((config.panel.opacity * 255.0) as u32) << 24 | 0x2B2B2B;
        let bytes = color.to_le_bytes();
        for chunk in data.chunks_exact_mut(4) {
            chunk[0] = bytes[0];
            chunk[1] = bytes[1];
            chunk[2] = bytes[2];
            chunk[3] = bytes[3];
        }
    }

    // Create wl_shm_pool
    let pool_id = client.registry().allocate();
    client.registry().set_interface(pool_id, Interface::WlShmPool);
    WlShm::new(shm_id).create_pool(pool_id, shm_fd, size as i32, client.socket());

    // Create wl_buffer from pool
    let buffer_id = client.registry().allocate();
    client.registry().set_interface(buffer_id, Interface::WlBuffer);
    WlShmPool::new(pool_id).create_buffer(
        buffer_id,
        0,
        panel_width as i32,
        panel_height as i32,
        stride as i32,
        0, // WL_SHM_FORMAT_ARGB8888 = 0
        client.socket(),
    );

    // Step 8: Attach buffer and commit
    WlSurface::new(surface_id).attach(buffer_id, 0, 0, client.socket());
    WlSurface::new(surface_id).damage(0, 0, panel_width as i32, config.panel.size as i32, client.socket());
    WlSurface::new(surface_id).commit(client.socket());

    // Initialize clock
    let mut clock = clock::Clock::new();

    // Initialize workspace state
    let mut workspaces: Vec<(u32, String, bool)> = vec![];
    let mut last_workspace_fetch = Instant::now();

    // Initialize pointer state
    let mut pointer_x: f64 = 0.0;
    let mut pointer_y: f64 = 0.0;

    // Initialize text input
    let mut text_input = textinput::TextInput::new();
    let mut keyboard_focused = false;

    // Cursor blink state
    let mut cursor_visible = true;
    let mut last_cursor_blink = Instant::now();
    
    // Track last rendered state to avoid unnecessary commits
    let mut last_clock_text = String::new();
    let mut needs_commit = true; // Initial render
    let mut last_configure_serial: u32 = 0;

    // Step 9: Main event loop
    loop {
        // Fetch workspaces once per second
        if last_workspace_fetch.elapsed() >= Duration::from_secs(1) {
            workspaces = ipc.get_workspaces();
            last_workspace_fetch = Instant::now();
            needs_commit = true; // Workspace buttons changed
        }

        // Blink cursor
        if last_cursor_blink.elapsed() >= Duration::from_millis(500) {
            cursor_visible = !cursor_visible;
            last_cursor_blink = Instant::now();
            if keyboard_focused {
                needs_commit = true; // Cursor visibility changed
            }
        }

        clock.tick(&config.clock.format, config.clock.show_date);
        let clock_text = clock.text();
        if clock_text != last_clock_text {
            last_clock_text = clock_text.to_string();
            needs_commit = true; // Clock changed
        }

        // Only render and commit if something changed
        if needs_commit {
            // Render text input field on the left side
            let input_x = 8;
            let input_y = 2;
            let input_w = 200;
            let input_h = panel_height - 4;
            {
                let data = shm_buffer.data_mut();
                // Input field background
                let input_bg = ((config.panel.opacity * 255.0) as u32) << 24 | 0x1B1B1B;
                fill_rect(data, stride, panel_width as i32, panel_height, input_x, input_y, input_w, input_h, input_bg);

                // Render input text
                let display_text = text_input.display_text();
                let text_y = input_y + (input_h - 7) / 2;
                draw_text(data, stride, panel_width as i32, panel_height, input_x + 4, text_y, &display_text, 0xFFFFFFFF);

                // Render blinking cursor
                if keyboard_focused && cursor_visible {
                    let cursor_x = input_x + 4 + text_input.cursor_pixel_offset();
                    fill_rect(data, stride, panel_width as i32, panel_height, cursor_x, text_y, 1, 9, 0xFFFFFFFF);
                }
            }

            let text_width = last_clock_text.len() as i32 * 6;
            let x = panel_width as i32 - text_width - 8;
            let y = ((panel_height - 7) / 2) as i32;
            {
                let data = shm_buffer.data_mut();
                draw_text(data, stride, panel_width as i32, panel_height, x, y, &last_clock_text, 0xFFFFFFFF);
            }

            // Render workspace buttons in center
            let button_width = 40;
            let button_height = panel_height - 4;
            let total_width = workspaces.len() as i32 * (button_width + 4);
            let start_x = (panel_width as i32 - total_width) / 2;

            {
                let data = shm_buffer.data_mut();
                for (i, (_id, name, focused)) in workspaces.iter().enumerate() {
                    let bx = start_x + (i as i32 * (button_width + 4));
                    let by = 2;

                    let button_color = if *focused {
                        ((config.panel.opacity * 255.0) as u32) << 24 | 0x3B3B3B
                    } else {
                        ((config.panel.opacity * 255.0) as u32) << 24 | 0x1B1B1B
                    };

                    fill_rect(data, stride, panel_width as i32, panel_height, bx, by, button_width, button_height, button_color);

                    let text_x = bx + (button_width - name.len() as i32 * 6) / 2;
                    let text_y = by + (button_height - 7) / 2;
                    draw_text(data, stride, panel_width as i32, panel_height, text_x, text_y, name, 0xFFFFFFFF);
                }
            }

            WlSurface::new(surface_id).damage(0, 0, panel_width as i32, panel_height, client.socket());
            WlSurface::new(surface_id).commit(client.socket());
            needs_commit = false;
        }

        for event in client.poll()? {
            match event {
                Event::LayerSurfaceConfigure { serial, .. } => {
                    if serial != last_configure_serial {
                        last_configure_serial = serial;
                        ls.ack_configure(serial, client.socket());
                    }
                }
                Event::LayerSurfaceClosed { .. } => return Ok(()),
                Event::PointerMotion { surface_x, surface_y } => {
                    pointer_x = surface_x;
                    pointer_y = surface_y;
                }
                Event::PointerEnter { surface_x, surface_y } => {
                    pointer_x = surface_x;
                    pointer_y = surface_y;
                }
                Event::PointerButton { button, state } => {
                    if button == 0x110 && state == 1 {
                        let px = pointer_x as i32;
                        let py = pointer_y as i32;

                        // Check if click is in text input field
                        if px >= 8 && px < 208 && py >= 2 && py < panel_height - 2 {
                            if !keyboard_focused {
                                keyboard_focused = true;
                                ls.set_keyboard_interactivity(1, client.socket());
                                WlSurface::new(surface_id).commit(client.socket());
                            }
                            // Position cursor within text
                            text_input.click_at(px - 12);
                        } else {
                            // Click outside input field — check workspace buttons
                            let btn_w = 40i32;
                            let total_w = workspaces.len() as i32 * (btn_w + 4);
                            let start_x = (panel_width as i32 - total_w) / 2;
                            for (i, (id, _, _)) in workspaces.iter().enumerate() {
                                let bx = start_x + i as i32 * (btn_w + 4);
                                if px >= bx && px < bx + btn_w && py >= 2 && py < panel_height - 2 {
                                    ipc.switch_workspace(*id);
                                    break;
                                }
                            }
                            // Release keyboard focus
                            if keyboard_focused {
                                keyboard_focused = false;
                                ls.set_keyboard_interactivity(0, client.socket());
                                WlSurface::new(surface_id).commit(client.socket());
                            }
                        }
                    }
                }
                Event::KeyboardKey { key, state, .. } => {
                    if state == 1 && keyboard_focused {
                        text_input.handle_key(key);
                        cursor_visible = true;
                        last_cursor_blink = Instant::now();
                    }
                }
                Event::KeyboardModifiers { mods_depressed, .. } => {
                    text_input.handle_modifiers(mods_depressed);
                }
                _ => {}
            }
        }
    }
}
