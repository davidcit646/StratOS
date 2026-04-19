//! StratOS modular settings: merge `settings.toml` + `settings.d/*.toml`, typed sections,
//! and unknown top-level tables for third-party hooks.
//!
//! Location (on disk): [`CONFIG_DIR`] / `settings.toml` and [`CONFIG_DIR`]/`settings.d/`.
//! Later files in `settings.d/` win for overlapping keys. Tables merge recursively; scalars override.

pub mod keyboard;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

pub use keyboard::{parse_hotkey, write_stratvm_keybind_file};

pub const CONFIG_DIR: &str = "/config/strat";
pub const SETTINGS_FILE: &str = "settings.toml";
pub const SETTINGS_D: &str = "settings.d";
pub const LEGACY_PANEL_CONF: &str = "panel.conf";

fn default_true() -> bool {
    true
}

fn default_title_h() -> u32 {
    28
}

fn default_scrollback_lines() -> usize {
    10_000
}

/// PTY + terminal grid (Stratterm).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TerminalSettings {
    /// Scrollback buffer depth (lines). `0` means use built-in default (10000).
    pub scrollback_max_lines: usize,
    /// Terminal bitmap scale multiplier (`1.0`..`4.0`, rounded to integer steps).
    pub terminal_font_scale: f32,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        TerminalSettings {
            scrollback_max_lines: default_scrollback_lines(),
            terminal_font_scale: 1.0,
        }
    }
}

/// File browser overlay (Stratterm F7): chrome + listing behavior.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct FileExplorerSettings {
    /// Upper status line in the explorer (`mode:… scroll:…`); `0` / `1` or false / true in TOML.
    #[serde(deserialize_with = "deserialize_bool01")]
    pub status_bar_enabled: bool,
    /// Client-drawn Files/Terminal bar. If false, explorer + PTY use full height (no title strip).
    #[serde(deserialize_with = "deserialize_bool01")]
    pub client_title_bar_enabled: bool,
    /// File explorer + title-bar text/chrome scale (`1.0`..`4.0`, rounded to integer steps).
    pub title_bar_font_scale: f32,
    /// Initial listing mode: `flat` or `tree`.
    pub default_view: String,
}

impl Default for FileExplorerSettings {
    fn default() -> Self {
        FileExplorerSettings {
            status_bar_enabled: true,
            client_title_bar_enabled: true,
            title_bar_font_scale: 1.0,
            default_view: "tree".to_string(),
        }
    }
}

/// Terminal + file explorer (Stratterm). Legacy flat keys under `[stratterm]` are promoted into
/// `[stratterm.term]` / `[stratterm.file_explorer]` on load.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct StrattermSettings {
    pub term: TerminalSettings,
    pub file_explorer: FileExplorerSettings,
}

impl Default for StrattermSettings {
    fn default() -> Self {
        StrattermSettings {
            term: TerminalSettings::default(),
            file_explorer: FileExplorerSettings::default(),
        }
    }
}

/// Top panel (Stratpanel); mirrors previous `panel.conf` keys.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct PanelSettings {
    pub position: String,
    #[serde(deserialize_with = "deserialize_bool01")]
    pub autohide: bool,
    pub summon_key: String,
    pub size: u32,
    pub opacity: f64,
    #[serde(deserialize_with = "deserialize_bool01")]
    pub blur: bool,
    /// Panel bitmap scale multiplier (`1.0`..`4.0`, rounded to integer steps).
    pub font_scale: f32,
    pub clock: ClockSettings,
    pub pinned: PinnedSettings,
    pub workspace: WorkspaceSwitcherSettings,
    pub tray: TraySettings,
}

impl Default for PanelSettings {
    fn default() -> Self {
        PanelSettings {
            position: "top".to_string(),
            autohide: false,
            summon_key: "super+grave".to_string(),
            size: 28,
            opacity: 0.85,
            blur: true,
            font_scale: 1.0,
            clock: ClockSettings::default(),
            pinned: PinnedSettings::default(),
            workspace: WorkspaceSwitcherSettings::default(),
            tray: TraySettings::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ClockSettings {
    pub format: String,
    #[serde(deserialize_with = "deserialize_bool01")]
    pub show_date: bool,
}

impl Default for ClockSettings {
    fn default() -> Self {
        ClockSettings {
            format: "12hr".to_string(),
            show_date: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct PinnedSettings {
    pub apps: Vec<String>,
}

impl Default for PinnedSettings {
    fn default() -> Self {
        PinnedSettings { apps: vec![] }
    }
}

/// Workspace switcher strip (Stratpanel ↔ stratvm IPC).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct WorkspaceSwitcherSettings {
    #[serde(deserialize_with = "deserialize_bool01")]
    pub enabled: bool,
    /// How often to refresh workspace list from the compositor (`0` → `1`).
    pub poll_interval_secs: u64,
    /// When false, buttons show `1`, `2`, … instead of compositor names.
    #[serde(deserialize_with = "deserialize_bool01")]
    pub show_labels: bool,
    /// Max workspace buttons (`0` = show all).
    pub max_visible: u32,
}

impl Default for WorkspaceSwitcherSettings {
    fn default() -> Self {
        WorkspaceSwitcherSettings {
            enabled: true,
            poll_interval_secs: 1,
            show_labels: true,
            max_visible: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TraySettings {
    #[serde(deserialize_with = "deserialize_bool01")]
    pub show_network: bool,
    #[serde(deserialize_with = "deserialize_bool01")]
    pub show_volume: bool,
    #[serde(deserialize_with = "deserialize_bool01")]
    pub show_updates: bool,
    #[serde(deserialize_with = "deserialize_bool01")]
    pub show_battery: bool,
}

impl Default for TraySettings {
    fn default() -> Self {
        TraySettings {
            show_network: true,
            show_volume: true,
            show_updates: true,
            show_battery: true,
        }
    }
}

/// Used by `stratman --network` from merged `settings.toml` (requires `/config` mounted).
/// `interface = "auto"` picks a wired/USB-Ethernet iface first, then Wi-Fi; set `interface = "eth0"`
/// (or similar) to pin a single device. Wi-Fi association: `/config/strat/wpa_supplicant.conf` + `strat-wpa`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct NetworkSettings {
    pub interface: String,
    #[serde(deserialize_with = "deserialize_bool01")]
    pub use_dhcp: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_netmask: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_gateway: Option<String>,
    pub retry_interval_secs: u64,
    /// `None` = unlimited retries (stratman default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        NetworkSettings {
            interface: "auto".to_string(),
            use_dhcp: true,
            static_ip: None,
            static_netmask: None,
            static_gateway: None,
            retry_interval_secs: 5,
            max_retries: None,
        }
    }
}

/// Compositor / window chrome (stratvm and related); reserved for future file-backed config.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ChromeSettings {
    /// Wayland decoration titlebar height (pixels); consumed by stratvm (`stratwm_load_modular_chrome`).
    pub decoration_titlebar_height: u32,
    /// Window border padding (pixels) around tiled/floating frames in stratvm.
    pub border_pad: u32,
    #[serde(deserialize_with = "deserialize_bool01")]
    pub decorations_enabled_default: bool,
}

impl Default for ChromeSettings {
    fn default() -> Self {
        ChromeSettings {
            decoration_titlebar_height: default_title_h(),
            border_pad: 2,
            decorations_enabled_default: default_true(),
        }
    }
}

/// Compositor shortcuts: exported to `/config/strat/stratvm-keybinds` for `stratvm`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct KeyboardSettings {
    /// Open Spotlite overlay (`/bin/spotlite`).
    pub spotlite: String,
    /// Cycle workspace layout (BSP → stack → fullscreen).
    pub cycle_layout: String,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        KeyboardSettings {
            spotlite: "super+period".to_string(),
            cycle_layout: "super+space".to_string(),
        }
    }
}

/// Headless indexer (`stratterm-indexer`); mirrors `indexer.conf` keys. Applied before legacy file
/// parse so `/config/strat/indexer.conf` can still override per-key.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct SpotliteHeadlessSettings {
    #[serde(deserialize_with = "deserialize_bool01")]
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_bool01")]
    pub boot_start: bool,
    pub frequency_ms: u64,
    pub rescan_secs: u64,
    pub batch_limit: usize,
    pub high_usage_load_per_cpu: f64,
    pub roots: Vec<String>,
    pub exclude_prefixes: Vec<String>,
}

impl Default for SpotliteHeadlessSettings {
    fn default() -> Self {
        SpotliteHeadlessSettings {
            enabled: true,
            boot_start: true,
            frequency_ms: 1200,
            rescan_secs: 180,
            batch_limit: 96,
            high_usage_load_per_cpu: 0.85,
            roots: vec![
                "/home".to_string(),
                "/config".to_string(),
                "/apps".to_string(),
            ],
            exclude_prefixes: Vec::new(),
        }
    }
}

/// UI-side indexing hints (CLI / future in-process consumers); same semantics as `indexer.conf` `ui_*`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct SpotliteUiSettings {
    #[serde(deserialize_with = "deserialize_bool01")]
    pub enabled: bool,
    pub tick_ms: u64,
    pub batch_limit: usize,
    pub idle_after_secs: u64,
    pub startup_grace_secs: u64,
    pub post_nav_delay_ms: u64,
    pub post_nav_scan_limit: usize,
    pub post_nav_force_secs: u64,
}

impl Default for SpotliteUiSettings {
    fn default() -> Self {
        SpotliteUiSettings {
            enabled: true,
            tick_ms: 750,
            batch_limit: 80,
            idle_after_secs: 12,
            startup_grace_secs: 8,
            post_nav_delay_ms: 180,
            post_nav_scan_limit: 1200,
            post_nav_force_secs: 6,
        }
    }
}

/// Spotlite / path index: unified hook for indexer + UI tooling (`[spotlite]` in merged settings).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct SpotliteSettings {
    pub headless: SpotliteHeadlessSettings,
    pub ui: SpotliteUiSettings,
}

impl Default for SpotliteSettings {
    fn default() -> Self {
        SpotliteSettings {
            headless: SpotliteHeadlessSettings::default(),
            ui: SpotliteUiSettings::default(),
        }
    }
}

/// Root document: known sections + anything else lands in `extensions` for custom crates.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct StratSettings {
    pub stratterm: StrattermSettings,
    pub panel: PanelSettings,
    pub chrome: ChromeSettings,
    pub keyboard: KeyboardSettings,
    pub spotlite: SpotliteSettings,
    pub network: NetworkSettings,
    /// Third-party top-level tables, e.g. `[myproject]`, for other binaries / future hooks.
    #[serde(flatten)]
    pub extensions: HashMap<String, Value>,
}

impl Default for StratSettings {
    fn default() -> Self {
        StratSettings {
            stratterm: StrattermSettings::default(),
            panel: PanelSettings::default(),
            chrome: ChromeSettings::default(),
            keyboard: KeyboardSettings::default(),
            spotlite: SpotliteSettings::default(),
            network: NetworkSettings::default(),
            extensions: HashMap::new(),
        }
    }
}

impl StratSettings {
    /// Load merged settings from [`CONFIG_DIR`]. Missing files yield defaults; parse errors return Err.
    pub fn load() -> Result<Self, String> {
        Self::load_from(Path::new(CONFIG_DIR))
    }

    /// Same as [`StratSettings::load`] but with a custom config root (tests / alternate roots).
    pub fn load_from(root: &Path) -> Result<Self, String> {
        let mut merged = Self::embedded_defaults_toml_value()?;
        let main = root.join(SETTINGS_FILE);
        if main.is_file() {
            let v: Value = parse_file(&main)?;
            merge_toml_value(&mut merged, v);
        }
        let d = root.join(SETTINGS_D);
        if d.is_dir() {
            let mut names = fs::read_dir(&d)
                .map_err(|e| format!("read {}: {e}", d.display()))?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map_or(false, |x| x == "toml"))
                .collect::<Vec<_>>();
            names.sort();
            for path in names {
                let v: Value = parse_file(&path)?;
                merge_toml_value(&mut merged, v);
            }
        }
        promote_stratterm_legacy_keys(&mut merged);
        let mut settings: StratSettings = StratSettings::deserialize(merged)
            .map_err(|e: toml::de::Error| e.to_string())?;
        let legacy = root.join(LEGACY_PANEL_CONF);
        if legacy.is_file() {
            overlay_legacy_panel_conf(&legacy, &mut settings.panel)?;
        }
        Ok(settings)
    }

    /// Persist to `root/settings.toml` (atomic rename). Existing `settings.d/*.toml` are left as-is and
    /// merged on next [`StratSettings::load_from`].
    pub fn save_to(&self, root: &Path) -> Result<(), String> {
        #[derive(Serialize)]
        struct SaveToml<'a> {
            stratterm: &'a StrattermSettings,
            panel: &'a PanelSettings,
            chrome: &'a ChromeSettings,
            keyboard: &'a KeyboardSettings,
            spotlite: &'a SpotliteSettings,
            network: &'a NetworkSettings,
        }
        let path: PathBuf = root.join(SETTINGS_FILE);
        let tmp: PathBuf = path.with_file_name(format!(
            ".{}.tmp",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("settings.toml")
        ));
        let st = SaveToml {
            stratterm: &self.stratterm,
            panel: &self.panel,
            chrome: &self.chrome,
            keyboard: &self.keyboard,
            spotlite: &self.spotlite,
            network: &self.network,
        };
        let mut merged = serde_json_to_toml_value(
            &serde_json::to_value(&st).map_err(|e: serde_json::Error| e.to_string())?,
        )
        .map_err(|e: String| e)?;
        if !self.extensions.is_empty() {
            let patch = Value::Table(
                self.extensions
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            );
            merge_toml_value(&mut merged, patch);
        }
        let tbl = merged.as_table().ok_or("internal: root must be a TOML table")?;
        let ser = write_root_table(tbl);
        if ser.is_empty() || !ser.trim_start().starts_with('[') {
            return Err(format!(
                "internal: empty or non-section TOML (len {}): {:?}…",
                ser.len(),
                ser.chars().take(60).collect::<String>()
            ));
        }
        fs::write(&tmp, ser).map_err(|e| format!("write {}: {e}", tmp.display()))?;
        fs::rename(&tmp, &path).map_err(|e| format!("rename {} -> {}: {e}", tmp.display(), path.display()))?;
        write_stratvm_keybind_file(root, &self.keyboard)?;
        Ok(())
    }

    fn embedded_defaults_toml_value() -> Result<Value, String> {
        let s = include_str!("../defaults/settings.default.toml");
        toml::from_str(s).map_err(|e: toml::de::Error| e.to_string())
    }
}

/// Human-readable TOML with `[section]` / `[section.sub]` headers (avoids a root `{ … }` blob).
fn write_root_table(root: &toml::map::Map<String, Value>) -> String {
    let mut out = String::new();
    for (name, v) in root {
        if let Value::Table(t) = v {
            write_table_section(&mut out, name.as_str(), t);
        }
    }
    out
}

fn write_table_section(out: &mut String, path: &str, t: &toml::map::Map<String, Value>) {
    let mut scalars: Vec<(&String, &Value)> = Vec::new();
    let mut nested: Vec<(&String, &toml::map::Map<String, Value>)> = Vec::new();
    for (k, v) in t {
        match v {
            Value::Table(nt) => nested.push((k, nt)),
            _ => scalars.push((k, v)),
        }
    }
    if !scalars.is_empty() {
        out.push_str(&format!("[{path}]\n"));
        for (k, v) in scalars {
            out.push_str(&format!("{} = {}\n", k, fmt_toml_value_inline(v)));
        }
        out.push('\n');
    }
    for (k, nt) in nested {
        write_table_section(out, &format!("{path}.{k}"), nt);
    }
}

fn fmt_toml_value_inline(v: &Value) -> String {
    match v {
        Value::String(s) => {
            let mut t = String::new();
            t.push('"');
            for ch in s.chars() {
                match ch {
                    '\\' => t.push_str("\\\\"),
                    '"' => t.push_str("\\\""),
                    '\n' => t.push_str("\\n"),
                    '\r' => t.push_str("\\r"),
                    '\t' => t.push_str("\\t"),
                    c if c < ' ' => {
                        use std::fmt::Write;
                        let _ = write!(t, "\\u{:04X}", c as u32);
                    }
                    c => t.push(c),
                }
            }
            t.push('"');
            t
        }
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => format!("{f}"),
        Value::Boolean(b) => b.to_string(),
        Value::Array(a) => {
            let parts: Vec<String> = a.iter().map(fmt_toml_value_inline).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Datetime(d) => d.to_string(),
        Value::Table(_) => "\"<invalid nested>\"".to_string(),
    }
}

/// Build a `toml::Value` tree without a root inline table (serde-toml often emits `{ … }` for structs).
fn serde_json_to_toml_value(j: &serde_json::Value) -> Result<Value, String> {
    match j {
        serde_json::Value::Object(o) => {
            let mut m = toml::map::Map::new();
            for (k, v) in o {
                m.insert(k.clone(), serde_json_to_toml_value(v)?);
            }
            Ok(Value::Table(m))
        }
        serde_json::Value::Array(a) => {
            let mut v = Vec::new();
            for x in a {
                v.push(serde_json_to_toml_value(x)?);
            }
            Ok(Value::Array(v))
        }
        serde_json::Value::String(s) => Ok(Value::String(s.clone())),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Ok(Value::Integer(i));
            }
            if let Some(u) = n.as_u64() {
                return i64::try_from(u)
                    .map(Value::Integer)
                    .map_err(|_| "integer too large".to_string());
            }
            n.as_f64()
                .map(Value::Float)
                .ok_or_else(|| "bad number".to_string())
        }
        serde_json::Value::Null => Err("unexpected null in settings".into()),
    }
}

fn parse_file(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.parse::<Value>().map_err(|e| format!("{}: {e}", path.display()))
}

/// Move legacy `[stratterm]` flat keys into `[stratterm.term]` / `[stratterm.file_explorer]` when
/// present at the `stratterm` root (pre-nested-schema configs).
fn promote_stratterm_legacy_keys(merged: &mut Value) {
    let Some(root) = merged.as_table_mut() else {
        return;
    };
    let Some(st) = root.get_mut("stratterm").and_then(|v| v.as_table_mut()) else {
        return;
    };

    fn ensure_child<'a>(
        st: &'a mut toml::map::Map<String, Value>,
        key: &str,
    ) -> &'a mut toml::map::Map<String, Value> {
        st.entry(key.to_string())
            .or_insert_with(|| Value::Table(Default::default()))
            .as_table_mut()
            .expect("table")
    }

    let status_bar_enabled = st.remove("status_bar_enabled");
    let client_title_bar_enabled = st.remove("client_title_bar_enabled");
    let title_bar_font_scale = st.remove("title_bar_font_scale");
    let terminal_font_scale = st.remove("terminal_font_scale");

    let fe = ensure_child(st, "file_explorer");
    if let Some(v) = status_bar_enabled {
        fe.entry("status_bar_enabled".to_string()).or_insert(v);
    }
    if let Some(v) = client_title_bar_enabled {
        fe.entry("client_title_bar_enabled".to_string()).or_insert(v);
    }
    if let Some(v) = title_bar_font_scale {
        fe.entry("title_bar_font_scale".to_string()).or_insert(v);
    }
    let term = ensure_child(st, "term");
    if let Some(v) = terminal_font_scale {
        term.entry("terminal_font_scale".to_string()).or_insert(v);
    }
}

/// Deep-merge TOML values: tables recurse; leaves override.
pub fn merge_toml_value(base: &mut Value, patch: Value) {
    match patch {
        Value::Table(patch_map) => {
            if !base.is_table() {
                *base = Value::Table(Default::default());
            }
            let base_map = base.as_table_mut().expect("just set");
            for (k, v) in patch_map {
                if let Some(existing) = base_map.get_mut(&k) {
                    if existing.is_table() && v.is_table() {
                        merge_toml_value(existing, v);
                    } else {
                        base_map.insert(k, v);
                    }
                } else {
                    base_map.insert(k, v);
                }
            }
        }
        _ => *base = patch,
    }
}

fn deserialize_bool01<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum B {
        B(bool),
        I(i64),
        S(String),
    }
    match B::deserialize(deserializer)? {
        B::B(b) => Ok(b),
        B::I(0) => Ok(false),
        B::I(1) => Ok(true),
        B::I(n) => Err(D::Error::custom(format!("expected 0 or 1, got {n}"))),
        B::S(s) => match s.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(D::Error::custom("expected bool or 0/1")),
        },
    }
}

/// Minimal overlay from legacy INI-like `panel.conf` (`[panel]`, `[clock]`, …).
fn overlay_legacy_panel_conf(path: &Path, panel: &mut PanelSettings) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
    let mut current: Option<&str> = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = Some(&line[1..line.len() - 1]);
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match current {
            Some("panel") => match key {
                "position" => panel.position = parse_string_val(value),
                "autohide" => panel.autohide = parse_bool_val(value),
                "summon_key" => panel.summon_key = parse_string_val(value),
                "size" => {
                    if let Some(v) = value.parse().ok() {
                        panel.size = v;
                    }
                }
                "opacity" => {
                    if let Some(v) = value.parse().ok() {
                        panel.opacity = v;
                    }
                }
                "blur" => panel.blur = parse_bool_val(value),
                "font_scale" => {
                    if let Some(v) = value.parse().ok() {
                        panel.font_scale = v;
                    }
                }
                _ => {}
            },
            Some("clock") => match key {
                "format" => panel.clock.format = parse_string_val(value),
                "show_date" => panel.clock.show_date = parse_bool_val(value),
                _ => {}
            },
            Some("pinned") if key == "apps" => panel.pinned.apps = parse_string_array_val(value),
            Some("workspace") => match key {
                "enabled" => panel.workspace.enabled = parse_bool_val(value),
                "poll_interval_secs" => {
                    if let Ok(v) = value.parse::<u64>() {
                        panel.workspace.poll_interval_secs = v;
                    }
                }
                "show_labels" => panel.workspace.show_labels = parse_bool_val(value),
                "max_visible" => {
                    if let Ok(v) = value.parse::<u32>() {
                        panel.workspace.max_visible = v;
                    }
                }
                _ => {}
            },
            Some("tray") => match key {
                "show_network" => panel.tray.show_network = parse_bool_val(value),
                "show_volume" => panel.tray.show_volume = parse_bool_val(value),
                "show_updates" => panel.tray.show_updates = parse_bool_val(value),
                "show_battery" => panel.tray.show_battery = parse_bool_val(value),
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}

fn parse_string_val(value: &str) -> String {
    let t = value.trim();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

fn parse_bool_val(value: &str) -> bool {
    matches!(value.trim(), "true" | "1" | "yes" | "on")
}

fn parse_string_array_val(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return vec![];
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.trim().is_empty() {
        return vec![];
    }
    let mut result = vec![];
    let mut current = String::new();
    let mut in_quotes = false;
    // After `\` in a quoted element: next char is consumed here so `\"` and `\\` keep both chars.
    let mut escape_next = false;
    for ch in inner.chars() {
        if escape_next {
            match ch {
                '"' => {
                    current.push('\\');
                    current.push('"');
                }
                _ => {
                    current.push('\\');
                    current.push(ch);
                }
            }
            escape_next = false;
            continue;
        }
        match ch {
            '\\' if in_quotes => escape_next = true,
            '"' if in_quotes => in_quotes = false,
            '"' if !in_quotes => in_quotes = true,
            ',' if !in_quotes => {
                if !current.trim().is_empty() {
                    result.push(parse_string_val(&current));
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if escape_next {
        current.push('\\');
    }
    if !current.trim().is_empty() {
        result.push(parse_string_val(&current));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn save_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("stratsettings-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut s = StratSettings::default();
        s.panel.autohide = true;
        s.save_to(&dir).unwrap();
        let raw = fs::read_to_string(dir.join(SETTINGS_FILE)).unwrap();
        assert!(
            raw.trim_start().starts_with('['),
            "corrupt settings.toml (expected [): {:?}",
            raw.chars().take(120).collect::<String>()
        );
        let l = StratSettings::load_from(&dir).unwrap();
        assert!(l.panel.autohide);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_nested_stratterm() {
        let mut a: Value = toml::from_str(
            r#"
[stratterm.file_explorer]
status_bar_enabled = true
"#,
        )
        .unwrap();
        let b: Value = toml::from_str(
            r#"
[stratterm.file_explorer]
client_title_bar_enabled = false
"#,
        )
        .unwrap();
        merge_toml_value(&mut a, b);
        promote_stratterm_legacy_keys(&mut a);
        let merged_str = toml::to_string(&a).expect("serialize merged Value");
        let s: StratSettings = toml::from_str(&merged_str).expect("deserialize StratSettings");
        assert!(s.stratterm.file_explorer.status_bar_enabled);
        assert!(!s.stratterm.file_explorer.client_title_bar_enabled);
    }

    #[test]
    fn promote_legacy_flat_stratterm_keys() {
        let mut v: Value = toml::from_str(
            r#"
[stratterm]
status_bar_enabled = true
client_title_bar_enabled = false
terminal_font_scale = 1.5
title_bar_font_scale = 1.25
"#,
        )
        .unwrap();
        promote_stratterm_legacy_keys(&mut v);
        let s: StratSettings = toml::from_str(&toml::to_string(&v).unwrap()).unwrap();
        assert!(s.stratterm.file_explorer.status_bar_enabled);
        assert!(!s.stratterm.file_explorer.client_title_bar_enabled);
        assert!((s.stratterm.term.terminal_font_scale - 1.5).abs() < f32::EPSILON);
        assert!((s.stratterm.file_explorer.title_bar_font_scale - 1.25).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_string_array_val_preserves_backslash_before_quoted_char() {
        let v = parse_string_array_val(r#"[ "path\"with\"quotes" ]"#);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], r#"path\"with\"quotes"#);
    }

    #[test]
    fn parse_string_array_val_backslash_escape_sequence() {
        let v = parse_string_array_val(r#"[ "a\\b" ]"#);
        assert_eq!(v[0], r#"a\\b"#);
    }

    #[test]
    fn parse_string_array_val_two_backslashes_before_closing_quote() {
        let v = parse_string_array_val(r#"[ "x\\" ]"#);
        assert_eq!(v[0], r#"x\\"#);
    }

    #[test]
    fn parse_string_array_val_trailing_backslash_without_following_char() {
        // Inner is ` "x\` — no closing `"` before `]`; stray `\` is preserved via `escape_next` flush.
        let v = parse_string_array_val(concat!('[', ' ', '"', 'x', '\\', ']'));
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], r#"x\"#);
    }
}
