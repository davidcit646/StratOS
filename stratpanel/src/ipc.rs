use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

pub struct IpcClient {
    stream: Option<BufReader<UnixStream>>,
}

/// Parse JSON objects for workspace entries returned by `stratvm` for `get workspaces`
/// (`stratvm/src/main.c` `ipc_dispatch_command`). One-line response, newline-terminated:
///
/// `{"workspaces":[{"id":1,"name":"1","focused":true},...,{"id":9,"name":"9","focused":false}]}\n`
///
/// Indexing contract: `id` and `name` are **1-based** for humans; `switch_workspace N` uses the
/// same `N` (`stratvm` subtracts 1 internally).
fn parse_workspaces_payload(resp: &str) -> Vec<(u32, String, bool)> {
    let s = resp.trim();
    let Some(rest) = s.strip_prefix("{\"workspaces\":") else {
        return vec![];
    };
    let rest = rest.trim_start();
    let Some(inner) = rest.strip_prefix('[') else {
        return vec![];
    };
    let mut out = Vec::new();
    // Split on each workspace object opener; inner is like: [{"id":1,...},...]
    for chunk in inner.split("{\"id\":").skip(1) {
        // `1,"name":"1","focused":true},` or trailing `9,...}]`
        let Some(comma) = chunk.find(',') else {
            continue;
        };
        let Ok(id) = chunk[..comma].parse::<u32>() else {
            continue;
        };
        let tail = &chunk[comma..];
        let name = parse_json_string_field(tail, "\"name\":").unwrap_or_default();
        let focused = tail.contains("\"focused\":true");
        out.push((id, name, focused));
    }
    out
}

fn parse_json_string_field(haystack: &str, key: &str) -> Option<String> {
    let i = haystack.find(key)?;
    let mut s = &haystack[i + key.len()..];
    s = s.trim_start();
    if !s.starts_with('"') {
        return None;
    }
    s = &s[1..];
    let end = s.find('"')?;
    Some(s[..end].to_string())
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
        reader
            .get_mut()
            .write_all(format!("{}\n", cmd).as_bytes())
            .ok()?;
        let mut response = String::new();
        reader.read_line(&mut response).ok()?;
        Some(response.trim().to_string())
    }

    #[allow(dead_code)]
    pub fn ping(&mut self) -> bool {
        self.send("ping").map(|r| r == "OK pong").unwrap_or(false)
    }

    pub fn get_workspaces(&mut self) -> Vec<(u32, String, bool)> {
        let resp = self.send("get workspaces").unwrap_or_default();
        parse_workspaces_payload(&resp)
    }

    pub fn set_panel_autohide(&mut self, enabled: bool) -> bool {
        let cmd = format!("set panel autohide {}", enabled);
        self.send(&cmd).map(|r| r == "OK").unwrap_or(false)
    }

    pub fn switch_workspace(&mut self, id: u32) -> bool {
        let cmd = format!("switch_workspace {}", id);
        self.send(&cmd).map(|r| r == "ok").unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn float_window(&mut self, pid: i32) -> bool {
        let cmd = format!("float window {}", pid);
        self.send(&cmd).map(|r| r == "OK").unwrap_or(false)
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::parse_workspaces_payload;

    #[test]
    fn parses_stratvm_line() {
        let s = r#"{"workspaces":[{"id":1,"name":"1","focused":true},{"id":2,"name":"2","focused":false}]}"#;
        let w = parse_workspaces_payload(s);
        assert_eq!(w.len(), 2);
        assert_eq!(w[0], (1, "1".to_string(), true));
        assert_eq!(w[1], (2, "2".to_string(), false));
    }
}
