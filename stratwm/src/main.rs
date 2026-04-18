mod compositor;
mod cursor;
mod output;
mod seat;
mod surface;
mod tiling;
mod input;
mod ipc;
mod config;
mod workspace;

use compositor::StratCompositor;
use config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from /config/strat/wm.conf
    let config = Config::load("/config/strat/wm.conf")?;

    // Initialize wlroots backend
    // TODO: Initialize wlroots backend via FFI

    // Create compositor
    let mut compositor = StratCompositor::new(config)?;

    // Run event loop
    compositor.run_event_loop()?;

    Ok(())
}
