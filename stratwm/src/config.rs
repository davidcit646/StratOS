use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub panel: PanelConfig,
    pub clock: ClockConfig,
    pub pinned: PinnedConfig,
    pub tray: TrayConfig,
    pub tiling: TilingConfig,
    pub keybinds: KeybindConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    pub position: String,
    pub autohide: bool,
    pub summon_key: String,
    pub size: u32,
    pub opacity: f32,
    pub blur: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockConfig {
    pub format: String,
    pub show_date: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedConfig {
    pub apps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayConfig {
    pub show_network: bool,
    pub show_volume: bool,
    pub show_updates: bool,
    pub show_battery: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilingConfig {
    pub default_mode: String,
    pub gap: u32,
    pub main_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindConfig {
    pub close_window: String,
    pub minimize: String,
    pub fullscreen: String,
    pub toggle_float: String,
    pub tabbed_mode: String,
    pub cover_flow: String,
    pub cover_flow_reverse: String,
    pub spotlite: String,
    pub panel_toggle: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            panel: PanelConfig::default(),
            clock: ClockConfig::default(),
            pinned: PinnedConfig::default(),
            tray: TrayConfig::default(),
            tiling: TilingConfig::default(),
            keybinds: KeybindConfig::default(),
        }
    }
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            position: "top".to_string(),
            autohide: true,
            summon_key: "super+grave".to_string(),
            size: 28,
            opacity: 0.85,
            blur: true,
        }
    }
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format: "12hr".to_string(),
            show_date: false,
        }
    }
}

impl Default for PinnedConfig {
    fn default() -> Self {
        Self {
            apps: vec!["chromium".to_string(), "onlyoffice".to_string(), "strat-terminal".to_string(), "vlc".to_string()],
        }
    }
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            show_network: true,
            show_volume: true,
            show_updates: true,
            show_battery: true,
        }
    }
}

impl Default for TilingConfig {
    fn default() -> Self {
        Self {
            default_mode: "tile".to_string(),
            gap: 8,
            main_ratio: 0.6,
        }
    }
}

impl Default for KeybindConfig {
    fn default() -> Self {
        Self {
            close_window: "super+w".to_string(),
            minimize: "super+m".to_string(),
            fullscreen: "super+f".to_string(),
            toggle_float: "super+shift+space".to_string(),
            tabbed_mode: "super+shift+w".to_string(),
            cover_flow: "super+tab".to_string(),
            cover_flow_reverse: "super+shift+tab".to_string(),
            spotlite: "super+space".to_string(),
            panel_toggle: "super+grave".to_string(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Path::new(path);
        
        if !config_path.exists() {
            // Return default config if file doesn't exist
            return Ok(Self::default());
        }
        
        let contents = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&contents)?;
        
        Ok(config)
    }
    
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}
