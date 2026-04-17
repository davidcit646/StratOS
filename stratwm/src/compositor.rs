use crate::config::Config;
use crate::output::StratOutput;
use crate::surface::StratSurface;
use crate::input::StratInput;
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
        let layout = TilingLayout::new();
        
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
            layout,
            ipc,
            workspaces,
            active_workspace,
            config,
        })
    }
    
    pub fn run_event_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::net::UnixStream;
        use crate::ipc::{read_command, send_response, IpcCommand};
        
        loop {
            // TODO: Run wlroots event loop via FFI
            
            // Handle IPC connections
            if let Ok(Some(mut stream)) = self.ipc.try_accept() {
                if let Ok(Some(command)) = read_command(&mut stream) {
                    self.handle_ipc_command(command);
                    let _ = send_response(&mut stream, "OK");
                }
            }
            
            // TODO: Process other events (input, output, surface)
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
