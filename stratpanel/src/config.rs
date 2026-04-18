use std::fs;
use std::path::Path;

pub struct PanelConfig {
    pub panel: PanelSection,
    pub clock: ClockSection,
    pub pinned: PinnedSection,
    pub tray: TraySection,
}

pub struct PanelSection {
    pub position: String,
    pub autohide: bool,
    pub summon_key: String,
    pub size: u32,
    pub opacity: f64,
    pub blur: bool,
}

pub struct ClockSection {
    pub format: String,
    pub show_date: bool,
}

/// Absolute paths to executables, e.g. `apps = ["/bin/stratterm", "/bin/sh"]`
pub struct PinnedSection {
    pub apps: Vec<String>,
}

pub struct TraySection {
    pub show_network: bool,
    pub show_volume: bool,
    pub show_updates: bool,
    pub show_battery: bool,
}

impl PanelConfig {
    pub fn defaults() -> Self {
        PanelConfig {
            panel: PanelSection {
                position: "top".to_string(),
                autohide: false,
                summon_key: "super+grave".to_string(),
                size: 28,
                opacity: 0.85,
                blur: true,
            },
            clock: ClockSection {
                format: "12hr".to_string(),
                show_date: false,
            },
            pinned: PinnedSection {
                apps: vec![],
            },
            tray: TraySection {
                show_network: true,
                show_volume: true,
                show_updates: true,
                show_battery: true,
            },
        }
    }

    pub fn load() -> Self {
        let path = Path::new("/config/strat/panel.conf");
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return PanelConfig::defaults(),
        };

        let mut config = PanelConfig::defaults();
        let mut current_section: Option<&str> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                current_section = Some(&line[1..line.len()-1]);
                continue;
            }

            if let Some(section) = current_section {
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    match section {
                        "panel" => parse_panel_key(&mut config.panel, key, value),
                        "clock" => parse_clock_key(&mut config.clock, key, value),
                        "pinned" => parse_pinned_key(&mut config.pinned, key, value),
                        "tray" => parse_tray_key(&mut config.tray, key, value),
                        _ => {}
                    }
                }
            }
        }

        config
    }
}

fn parse_panel_key(panel: &mut PanelSection, key: &str, value: &str) {
    match key {
        "position" => panel.position = parse_string(value),
        "autohide" => panel.autohide = parse_bool(value),
        "summon_key" => panel.summon_key = parse_string(value),
        "size" => { if let Some(v) = parse_u32(value) { panel.size = v; } }
        "opacity" => { if let Some(v) = parse_f64(value) { panel.opacity = v; } }
        "blur" => panel.blur = parse_bool(value),
        _ => {}
    }
}

fn parse_clock_key(clock: &mut ClockSection, key: &str, value: &str) {
    match key {
        "format" => clock.format = parse_string(value),
        "show_date" => clock.show_date = parse_bool(value),
        _ => {}
    }
}

fn parse_pinned_key(pinned: &mut PinnedSection, key: &str, value: &str) {
    match key {
        "apps" => pinned.apps = parse_string_array(value),
        _ => {}
    }
}

fn parse_tray_key(tray: &mut TraySection, key: &str, value: &str) {
    match key {
        "show_network" => tray.show_network = parse_bool(value),
        "show_volume" => tray.show_volume = parse_bool(value),
        "show_updates" => tray.show_updates = parse_bool(value),
        "show_battery" => tray.show_battery = parse_bool(value),
        _ => {}
    }
}

fn parse_string(value: &str) -> String {
    let trimmed = value.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"')) || (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
        trimmed[1..trimmed.len()-1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_bool(value: &str) -> bool {
    value.trim() == "true"
}

fn parse_u32(value: &str) -> Option<u32> {
    value.trim().parse().ok()
}

fn parse_f64(value: &str) -> Option<f64> {
    value.trim().parse().ok()
}

fn parse_string_array(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return vec![];
    }
    let inner = &trimmed[1..trimmed.len()-1];
    if inner.trim().is_empty() {
        return vec![];
    }
    let mut result = vec![];
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape = false;
    
    for ch in inner.chars() {
        match ch {
            '\\' if in_quotes => {
                escape = true;
            }
            '"' if in_quotes && !escape => {
                in_quotes = false;
            }
            '"' if !in_quotes => {
                in_quotes = true;
            }
            ',' if !in_quotes => {
                if !current.trim().is_empty() {
                    result.push(parse_string(&current));
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
                escape = false;
            }
        }
    }
    if !current.trim().is_empty() {
        result.push(parse_string(&current));
    }
    result
}
