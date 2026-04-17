mod pty;
mod parser;
mod screen;
mod font;
mod renderer;
mod keyboard;
mod wayland;

use pty::Pty;
use parser::VtParser;
use screen::ScreenBuffer;
use font::{FONT_WIDTH, FONT_HEIGHT};
use renderer::Renderer;
use keyboard::{keysym_to_char, keysym_to_control, is_control_key};
use wayland::WaylandWindow;
use nix::poll::{poll, PollFd, PollFlags};
use std::os::unix::io::BorrowedFd;

fn main() -> Result<(), String> {
    // Initialize Wayland window
    let mut window = WaylandWindow::new()
        .map_err(|e| format!("Failed to create Wayland window: {}", e))?;
    
    let (width, height) = window.get_size();
    
    // Calculate grid size based on font dimensions
    let cols = (width as usize) / FONT_WIDTH;
    let rows = (height as usize) / FONT_HEIGHT;
    
    // Initialize screen buffer
    let mut screen = ScreenBuffer::new(rows, cols);
    
    // Initialize PTY
    let pty = Pty::new(rows as u16, cols as u16)
        .map_err(|e| format!("Failed to create PTY: {}", e))?;
    
    // Initialize VT parser
    let mut parser = VtParser::new();
    
    // Initialize renderer
    let mut renderer = Renderer::new(width as u32, height as u32);
    
    // Initial render
    renderer.render(&screen, &mut window)
        .map_err(|e| format!("Failed to render: {}", e))?;
    
    // Set up polling
    let pty_fd = pty.raw_fd();
    let wayland_fd = window.raw_fd();
    
    let mut poll_fds = unsafe {
        [
            PollFd::new(BorrowedFd::borrow_raw(pty_fd), PollFlags::POLLIN),
            PollFd::new(BorrowedFd::borrow_raw(wayland_fd), PollFlags::POLLIN),
        ]
    };
    
    let mut buffer = [0u8; 8192];
    
    // Main event loop
    loop {
        // Poll for events
        let poll_result = poll(&mut poll_fds, None::<Option<u16>>)
            .map_err(|e| format!("poll failed: {}", e))?;
        
        if poll_result == 0 {
            continue;
        }
        
        // Check PTY for output
        if poll_fds[0].revents().unwrap().contains(PollFlags::POLLIN) {
            let bytes_read = pty.read(&mut buffer)
                .map_err(|e| format!("Failed to read from PTY: {}", e))?;
            
            if bytes_read > 0 {
                parser.parse(&mut screen, &buffer[..bytes_read]);
                
                renderer.render(&screen, &mut window)
                    .map_err(|e| format!("Failed to render: {}", e))?;
            } else {
                // PTY closed, exit
                break;
            }
        }
        
        // Check Wayland for events
        if poll_fds[1].revents().unwrap().contains(PollFlags::POLLIN) {
            let events = window.poll_events()
                .map_err(|e| format!("Failed to poll Wayland events: {}", e))?;
            
            for event in events {
                match event {
                    stratlayer::events::Event::XdgConfigure { serial, width, height, .. } => {
                        window.ack_configure(serial);
                        window.set_pending_size(width, height);
                    }
                    stratlayer::events::Event::XdgPing { .. } => {
                        window.handle_event(&event);
                    }
                    stratlayer::events::Event::KeyboardKey { key, state, .. } => {
                        if state == 1 { // Key pressed
                            if is_control_key(key) {
                                if let Some(control_seq) = keysym_to_control(key) {
                                    if !control_seq.is_empty() {
                                        pty.write(&control_seq)
                                            .map_err(|e| format!("Failed to write to PTY: {}", e))?;
                                    }
                                }
                            } else if let Some(ch) = keysym_to_char(key) {
                                let mut buf = [0u8; 4];
                                let ch_str = ch.encode_utf8(&mut buf);
                                pty.write(ch_str.as_bytes())
                                    .map_err(|e| format!("Failed to write to PTY: {}", e))?;
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            // Check for pending resize
            if let Some((new_width, new_height)) = window.commit_pending_size() {
                let new_cols = (new_width as usize) / FONT_WIDTH;
                let new_rows = (new_height as usize) / FONT_HEIGHT;
                
                screen.resize(new_rows, new_cols);
                renderer.resize(new_width as u32, new_height as u32);
                
                pty.resize(new_rows as u16, new_cols as u16)
                    .map_err(|e| format!("Failed to resize PTY: {}", e))?;
                
                renderer.render(&screen, &mut window)
                    .map_err(|e| format!("Failed to render: {}", e))?;
            }
        }
    }
    
    // Wait for PTY child to exit
    let _ = pty.wait();
    
    Ok(())
}
