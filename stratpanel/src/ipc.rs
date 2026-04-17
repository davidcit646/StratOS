use std::os::unix::net::UnixStream;
use std::io::{Write, BufRead, BufReader};

pub struct IpcClient {
    stream: Option<BufReader<UnixStream>>,
}

impl IpcClient {
    pub fn connect() -> Self {
        let stream = UnixStream::connect("/run/stratvm.sock")
            .ok()
            .map(BufReader::new);
        IpcClient { stream }
    }

    pub fn send(&mut self, cmd: &str) -> Option<String> {
        let reader = self.stream.as_mut()?;
        reader.get_mut().write_all(format!("{}\n", cmd).as_bytes()).ok()?;
        let mut response = String::new();
        reader.read_line(&mut response).ok()?;
        Some(response.trim().to_string())
    }

    #[allow(dead_code)]
    pub fn ping(&mut self) -> bool {
        self.send("ping").map(|r| r == "OK pong").unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn get_workspaces(&mut self) -> Vec<(u32, String, bool)> {
        let resp = self.send("get workspaces").unwrap_or_default();
        if !resp.starts_with("{\"workspaces\":") { return vec![]; }
        
        // Parse JSON-like: {"workspaces":[{"id":1,"name":"1","focused":true},...]}
        let mut result = vec![];
        let content = resp.strip_prefix("{\"workspaces\":").unwrap_or(&resp);
        let content = content.strip_suffix("}").unwrap_or(content);
        
        // Split by },{ to get individual workspace entries
        for entry in content.split("},{") {
            let entry = entry.replace("[", "").replace("]", "").replace("{", "").replace("}", "");
            let mut id: Option<u32> = None;
            let mut name: String = String::new();
            let mut focused: bool = false;
            
            for part in entry.split(",") {
                let part = part.trim();
                if part.starts_with("\"id\":") {
                    id = part[5..].parse().ok();
                } else if part.starts_with("\"name\":") {
                    let n = part[8..].trim_matches('"');
                    name = n.to_string();
                } else if part.starts_with("\"focused\":") {
                    focused = part[10..].trim() == "true";
                }
            }
            
            if let Some(i) = id {
                result.push((i, name, focused));
            }
        }
        result
    }

    pub fn set_panel_autohide(&mut self, enabled: bool) -> bool {
        let cmd = format!("set panel autohide {}", enabled);
        self.send(&cmd).map(|r| r == "OK").unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn switch_workspace(&mut self, id: u32) -> bool {
        let cmd = format!("switch_workspace {}", id);
        self.send(&cmd).map(|r| r == "ok").unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn float_window(&mut self, pid: i32) -> bool {
        let cmd = format!("float window {}", pid);
        self.send(&cmd).map(|r| r == "OK").unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }
}
