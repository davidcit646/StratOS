use crate::config::Config;
use crate::cursor::StratCursor;
use crate::output::StratOutput;
use crate::seat::{StratSeat, SeatCapabilities};
use crate::surface::StratSurface;
use crate::input::{StratInput, InputEvent};
use crate::tiling::TilingLayout;
use crate::ipc::IpcServer;
use crate::workspace::Workspace;
use std::path::Path;

pub struct StratCompositor {
    // wlroots compositor handle (opaque pointer for FFI)
    wlr_compositor: *mut libc::c_void,
    
    // List of active outputs
    outputs: Vec<StratOutput>,
    
    // List of mapped surfaces (windows)
    surfaces: Vec<StratSurface>,
    
    // Input handler
    input: StratInput,
    
    // Cursor manager
    cursor: StratCursor,
    
    // Wayland seat (pointer + keyboard)
    seat: StratSeat,
    
    // Global tiling layout
    layout: TilingLayout,
    
    // IPC server
    ipc: IpcServer,
    
    // Workspaces
    workspaces: Vec<Workspace>,
    
    // Current active workspace
    active_workspace: u32,
    
    // Configuration
    config: Config,
}

impl StratCompositor {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: Initialize wlroots compositor via FFI
        let wlr_compositor = std::ptr::null_mut();
        
        let outputs = Vec::new();
        let surfaces = Vec::new();
        let input = StratInput::new()?;
        let mut cursor = StratCursor::new()?;
        let mut seat = StratSeat::new("seat0")?;
        let layout = TilingLayout::new();
        
        // Advertise pointer + keyboard capabilities on the seat
        seat.set_capabilities(SeatCapabilities::POINTER | SeatCapabilities::KEYBOARD);
        
        // Set default cursor image
        cursor.set_default_image();
        
        // Create IPC server
        let ipc = IpcServer::new(Path::new("/run/stratvm.sock"))?;
        ipc.set_nonblocking()?;
        
        // Initialize workspaces (start with workspace 0)
        let mut workspaces = Vec::new();
        workspaces.push(Workspace::new(0));
        
        let active_workspace = 0;
        
        Ok(Self {
            wlr_compositor,
            outputs,
            surfaces,
            input,
            cursor,
            seat,
            layout,
            ipc,
            workspaces,
            active_workspace,
            config,
        })
    }
    
    pub fn run_event_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::ipc::{read_command, send_response, IpcCommand};
        
        loop {
            // TODO: Run wlroots event loop via FFI
            
            // Process input events (pointer motion, key presses)
            if let Ok(events) = self.input.process_events() {
                for event in events {
                    match event {
                        InputEvent::Pointer(pe) => {
                            // Move cursor
                            if pe.button.is_none() {
                                self.cursor.move_delta(pe.x, pe.y);
                                let (cx, cy) = self.cursor.position();
                                self.seat.notify_pointer_motion(0, cx, cy);
                            }
                            
                            // Handle button press/release
                            if let Some(button) = pe.button {
                                let state = match pe.button_state {
                                    Some(crate::input::ButtonState::Pressed) => 1,
                                    Some(crate::input::ButtonState::Released) => 0,
                                    None => continue,
                                };
                                self.seat.notify_pointer_button(0, button, state);
                            }
                        }
                        InputEvent::Key(ke) => {
                            let state = match ke.state {
                                crate::input::KeyState::Pressed => 1,
                                crate::input::KeyState::Released => 0,
                            };
                            self.seat.notify_key(0, ke.keycode, state);
                            
                            // Update modifier tracking
                            if ke.state == crate::input::KeyState::Pressed {
                                self.input.set_keyboard_state(ke.modifiers);
                                self.seat.notify_modifiers(
                                    if ke.modifiers.shift { 1 } else { 0 },
                                    if ke.modifiers.caps_lock { 1 } else { 0 },
                                    0, 0,
                                );
                            }
                        }
                    }
                }
            }
            
            // Handle IPC connections
            if let Ok(Some(mut stream)) = self.ipc.try_accept() {
                if let Ok(Some(command)) = read_command(&mut stream) {
                    self.handle_ipc_command(command);
                    let _ = send_response(&mut stream, "OK");
                }
            }
            
            // TODO: Process other events (output, surface)
        }
    }
    
    fn handle_ipc_command(&mut self, command: crate::ipc::IpcCommand) {
        match command {
            crate::ipc::IpcCommand::SetPanelAutoHide(enabled) => {
                // TODO: Update panel config
            }
            crate::ipc::IpcCommand::TriggerCoverFlow => {
                // TODO: Trigger Cover Flow overlay
            }
            crate::ipc::IpcCommand::TriggerPivotOverlay(show) => {
                // TODO: Trigger pivot overlay
            }
            crate::ipc::IpcCommand::FloatWindow(window_id) => {
                // TODO: Float specific window
            }
            crate::ipc::IpcCommand::ToggleTilingFloat => {
                // TODO: Toggle tiling/floating mode
            }
            crate::ipc::IpcCommand::SetTilingMode(mode) => {
                // TODO: Set tiling mode
            }
            crate::ipc::IpcCommand::ReloadConfig => {
                // TODO: Reload config from disk
            }
            crate::ipc::IpcCommand::Unknown(_) => {
                // Ignore unknown commands
            }
        }
    }
    
    pub fn add_output(&mut self, output: StratOutput) {
        self.outputs.push(output);
    }
    
    pub fn remove_output(&mut self, output_id: u32) {
        self.outputs.retain(|o| o.id() != output_id);
    }
    
    pub fn add_surface(&mut self, surface: StratSurface) {
        self.surfaces.push(surface);
    }
    
    pub fn remove_surface(&mut self, surface_id: u32) {
        self.surfaces.retain(|s| s.id() != surface_id);
    }
    
    pub fn outputs(&self) -> &[StratOutput] {
        &self.outputs
    }
    
    pub fn surfaces(&self) -> &[StratSurface] {
        &self.surfaces
    }
    
    pub fn input(&mut self) -> &mut StratInput {
        &mut self.input
    }
    
    pub fn cursor(&mut self) -> &mut StratCursor {
        &mut self.cursor
    }
    
    pub fn seat(&mut self) -> &mut StratSeat {
        &mut self.seat
    }
    
    pub fn config(&self) -> &Config {
        &self.config
    }
    
    pub fn layout(&self) -> &TilingLayout {
        &self.layout
    }
    
    pub fn layout_mut(&mut self) -> &mut TilingLayout {
        &mut self.layout
    }
    
    pub fn ipc(&mut self) -> &mut IpcServer {
        &mut self.ipc
    }
    
    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }
    
    pub fn workspaces_mut(&mut self) -> &mut [Workspace] {
        &mut self.workspaces
    }
    
    pub fn active_workspace(&self) -> u32 {
        self.active_workspace
    }
    
    pub fn set_active_workspace(&mut self, workspace_id: u32) {
        if workspace_id < self.workspaces.len() as u32 {
            self.active_workspace = workspace_id;
        }
    }
    
    pub fn add_workspace(&mut self) {
        let new_id = self.workspaces.len() as u32;
        self.workspaces.push(Workspace::new(new_id));
    }
}
