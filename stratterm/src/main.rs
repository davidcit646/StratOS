mod font;
mod keyboard;
mod parser;
mod pty;
mod renderer;
mod screen;
mod wayland;

use font::{FONT_HEIGHT, FONT_WIDTH};
use keyboard::{is_control_key, keysym_to_char, keysym_to_control};
use nix::poll::{poll, PollFd, PollFlags};
use parser::VtParser;
use pty::Pty;
use renderer::Renderer;
use screen::ScreenBuffer;
use std::os::unix::io::BorrowedFd;
use wayland::WaylandWindow;

const INITIAL_WIDTH: i32 = 800;
const INITIAL_HEIGHT: i32 = 600;

fn main() -> Result<(), String> {
    let mut window = WaylandWindow::new(INITIAL_WIDTH, INITIAL_HEIGHT)
        .map_err(|e| format!("Failed to create Wayland window: {}", e))?;

    let (mut width, mut height) = window.get_size();
    let mut cols = (width as usize) / FONT_WIDTH;
    let mut rows = (height as usize) / FONT_HEIGHT;

    let mut screen = ScreenBuffer::new(rows, cols);
    let pty = Pty::new(rows as u16, cols as u16)
        .map_err(|e| format!("Failed to create PTY: {}", e))?;
    let mut parser = VtParser::new();
    let mut renderer = Renderer::new(width as u32, height as u32);

    renderer
        .render(&screen, &mut window)
        .map_err(|e| format!("Initial render failed: {}", e))?;

    let pty_fd = pty.raw_fd();
    let wayland_fd = window.raw_fd();

    let mut buf = [0u8; 8192];

    loop {
        let mut poll_fds = unsafe {
            [
                PollFd::new(BorrowedFd::borrow_raw(pty_fd), PollFlags::POLLIN),
                PollFd::new(BorrowedFd::borrow_raw(wayland_fd), PollFlags::POLLIN),
            ]
        };

        let n = poll(&mut poll_fds, None::<Option<u16>>)
            .map_err(|e| format!("poll failed: {}", e))?;
        if n == 0 {
            continue;
        }

        let pty_ready = poll_fds[0]
            .revents()
            .map(|r| r.contains(PollFlags::POLLIN))
            .unwrap_or(false);
        let wayland_ready = poll_fds[1]
            .revents()
            .map(|r| r.contains(PollFlags::POLLIN))
            .unwrap_or(false);

        if pty_ready {
            let read = pty
                .read(&mut buf)
                .map_err(|e| format!("PTY read failed: {}", e))?;
            if read == 0 {
                break;
            }
            parser.parse(&mut screen, &buf[..read]);
            renderer
                .render(&screen, &mut window)
                .map_err(|e| format!("Render failed: {}", e))?;
        }

        if wayland_ready {
            let events = window
                .poll_events()
                .map_err(|e| format!("Wayland poll failed: {}", e))?;

            for event in events {
                if let stratlayer::Event::KeyboardKey { key, state, .. } = event {
                    if state == 1 {
                        if is_control_key(key) {
                            if let Some(seq) = keysym_to_control(key) {
                                if !seq.is_empty() {
                                    pty.write(&seq)
                                        .map_err(|e| format!("PTY write failed: {}", e))?;
                                }
                            }
                        } else if let Some(ch) = keysym_to_char(key) {
                            let mut utf = [0u8; 4];
                            let s = ch.encode_utf8(&mut utf);
                            pty.write(s.as_bytes())
                                .map_err(|e| format!("PTY write failed: {}", e))?;
                        }
                    }
                }
            }

            if let Some((new_w, new_h)) = window.commit_pending_size() {
                if new_w != width || new_h != height {
                    width = new_w;
                    height = new_h;
                    cols = (width as usize) / FONT_WIDTH;
                    rows = (height as usize) / FONT_HEIGHT;

                    screen.resize(rows, cols);
                    renderer.resize(width as u32, height as u32);
                    pty.resize(rows as u16, cols as u16)
                        .map_err(|e| format!("PTY resize failed: {}", e))?;
                    renderer
                        .render(&screen, &mut window)
                        .map_err(|e| format!("Render failed: {}", e))?;
                }
            }
        }
    }

    let _ = pty.wait();
    Ok(())
}
