use std::path::Path;
use std::os::unix::net::{UnixListener, UnixStream};
use std::io::{Read, Write};

#[derive(Debug)]
pub enum IpcCommand {
    SetPanelAutoHide(bool),
    TriggerCoverFlow,
    TriggerPivotOverlay(bool),
    FloatWindow(u32),
    ToggleTilingFloat,
    SetTilingMode(String),
    ReloadConfig,
    Unknown(String),
}

pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    pub fn new(socket_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        // Remove existing socket if present
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }
        
        let listener = UnixListener::bind(socket_path)?;
        
        Ok(Self { listener })
    }
    
    pub fn try_accept(&self) -> Result<Option<UnixStream>, Box<dyn std::error::Error>> {
        match self.listener.accept() {
            Ok((stream, _addr)) => Ok(Some(stream)),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    pub fn set_nonblocking(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.listener.set_nonblocking(true)?;
        Ok(())
    }
}

impl IpcCommand {
    pub fn parse(line: &str) -> Self {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        
        if parts.is_empty() {
            return IpcCommand::Unknown(line.to_string());
        }
        
        match parts[0] {
            "set_panel_autohide" => {
                if parts.len() >= 2 {
                    let enabled = parts[1].parse().unwrap_or(false);
                    IpcCommand::SetPanelAutoHide(enabled)
                } else {
                    IpcCommand::Unknown(line.to_string())
                }
            }
            "trigger_coverflow" => IpcCommand::TriggerCoverFlow,
            "trigger_pivot_overlay" => {
                if parts.len() >= 2 {
                    let show = parts[1].parse().unwrap_or(true);
                    IpcCommand::TriggerPivotOverlay(show)
                } else {
                    IpcCommand::TriggerPivotOverlay(true)
                }
            }
            "float_window" => {
                if parts.len() >= 2 {
                    let window_id = parts[1].parse().unwrap_or(0);
                    IpcCommand::FloatWindow(window_id)
                } else {
                    IpcCommand::Unknown(line.to_string())
                }
            }
            "toggle_tiling_float" => IpcCommand::ToggleTilingFloat,
            "set_tiling_mode" => {
                if parts.len() >= 2 {
                    IpcCommand::SetTilingMode(parts[1].to_string())
                } else {
                    IpcCommand::Unknown(line.to_string())
                }
            }
            "reload_config" => IpcCommand::ReloadConfig,
            _ => IpcCommand::Unknown(line.to_string()),
        }
    }
}

pub fn read_command(stream: &mut UnixStream) -> Result<Option<IpcCommand>, Box<dyn std::error::Error>> {
    let mut buffer = [0u8; 1024];
    
    match stream.read(&mut buffer) {
        Ok(0) => Ok(None), // EOF
        Ok(n) => {
            let line = String::from_utf8_lossy(&buffer[..n]).to_string();
            Ok(Some(IpcCommand::parse(&line)))
        }
        Err(e) => Err(e.into()),
    }
}

pub fn send_response(stream: &mut UnixStream, response: &str) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(stream, "{}", response)?;
    Ok(())
}
