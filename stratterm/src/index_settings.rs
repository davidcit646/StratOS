use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG_SYSTEM: &str = "/config/strat/indexer.conf";
const DEFAULT_CONFIG_REL_HOME: &str = ".config/strat/indexer.conf";
const DEFAULT_DISABLE_FLAG_SYSTEM: &str = "/config/strat/disable-indexer";
const DEFAULT_DISABLE_FLAG_REL_HOME: &str = ".config/strat/disable-indexer";

#[derive(Clone, Debug)]
pub struct IndexSettings {
    pub enabled: bool,
    pub boot_start: bool,
    pub frequency_ms: u64,
    pub rescan_secs: u64,
    pub batch_limit: usize,
    pub high_usage_load_per_cpu: f64,
    pub roots: Vec<PathBuf>,
    pub exclude_prefixes: Vec<PathBuf>,
    pub ui_enabled: bool,
    pub ui_tick_ms: u64,
    pub ui_batch_limit: usize,
    pub ui_idle_after_secs: u64,
    pub ui_startup_grace_secs: u64,
    pub ui_post_nav_delay_ms: u64,
    pub ui_post_nav_scan_limit: usize,
    pub ui_post_nav_force_secs: u64,
}

impl Default for IndexSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            boot_start: true,
            frequency_ms: 1200,
            rescan_secs: 180,
            batch_limit: 96,
            high_usage_load_per_cpu: 0.85,
            roots: vec![
                PathBuf::from("/home"),
                PathBuf::from("/config"),
                PathBuf::from("/apps"),
            ],
            exclude_prefixes: Vec::new(),
            ui_enabled: true,
            ui_tick_ms: 750,
            ui_batch_limit: 80,
            ui_idle_after_secs: 12,
            ui_startup_grace_secs: 8,
            ui_post_nav_delay_ms: 180,
            ui_post_nav_scan_limit: 1200,
            ui_post_nav_force_secs: 6,
        }
    }
}

pub fn load_index_settings(home_dir: Option<&Path>) -> IndexSettings {
    let path = config_path(home_dir);
    let mut settings = IndexSettings::default();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => return settings,
    };

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        let (key, value) = match line.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        match key {
            "enabled" => {
                if let Some(v) = parse_bool(value) {
                    settings.enabled = v;
                }
            }
            "boot_start" => {
                if let Some(v) = parse_bool(value) {
                    settings.boot_start = v;
                }
            }
            "frequency_ms" => {
                if let Some(v) = parse_u64(value) {
                    settings.frequency_ms = v.max(100);
                }
            }
            "rescan_secs" => {
                if let Some(v) = parse_u64(value) {
                    settings.rescan_secs = v.max(10);
                }
            }
            "batch_limit" => {
                if let Some(v) = parse_usize(value) {
                    settings.batch_limit = v.clamp(1, 5000);
                }
            }
            "high_usage_load_per_cpu" => {
                if let Some(v) = parse_f64(value) {
                    settings.high_usage_load_per_cpu = v.clamp(0.10, 4.0);
                }
            }
            "roots" => {
                let parsed = parse_path_list(value);
                if !parsed.is_empty() {
                    settings.roots = parsed;
                }
            }
            "exclude_prefixes" => {
                settings.exclude_prefixes = parse_path_list(value);
            }
            "ui_enabled" => {
                if let Some(v) = parse_bool(value) {
                    settings.ui_enabled = v;
                }
            }
            "ui_tick_ms" => {
                if let Some(v) = parse_u64(value) {
                    settings.ui_tick_ms = v.max(100);
                }
            }
            "ui_batch_limit" => {
                if let Some(v) = parse_usize(value) {
                    settings.ui_batch_limit = v.clamp(1, 5000);
                }
            }
            "ui_idle_after_secs" => {
                if let Some(v) = parse_u64(value) {
                    settings.ui_idle_after_secs = v.max(1);
                }
            }
            "ui_startup_grace_secs" => {
                if let Some(v) = parse_u64(value) {
                    settings.ui_startup_grace_secs = v.max(0);
                }
            }
            "ui_post_nav_delay_ms" => {
                if let Some(v) = parse_u64(value) {
                    settings.ui_post_nav_delay_ms = v.max(0);
                }
            }
            "ui_post_nav_scan_limit" => {
                if let Some(v) = parse_usize(value) {
                    settings.ui_post_nav_scan_limit = v.clamp(10, 100_000);
                }
            }
            "ui_post_nav_force_secs" => {
                if let Some(v) = parse_u64(value) {
                    settings.ui_post_nav_force_secs = v.max(0);
                }
            }
            _ => {}
        }
    }

    settings
}

pub fn indexer_is_disabled(home_dir: Option<&Path>) -> bool {
    if env::var("STRAT_INDEXER_DISABLE")
        .map(|v| parse_bool(v.as_str()).unwrap_or(false))
        .unwrap_or(false)
    {
        return true;
    }

    let system_flag = Path::new(DEFAULT_DISABLE_FLAG_SYSTEM);
    if system_flag.exists() {
        return true;
    }

    if let Some(home) = home_dir {
        let home_flag = home.join(DEFAULT_DISABLE_FLAG_REL_HOME);
        if home_flag.exists() {
            return true;
        }
    }

    false
}

pub fn config_path(home_dir: Option<&Path>) -> PathBuf {
    let system_path = PathBuf::from(DEFAULT_CONFIG_SYSTEM);
    if system_path.exists() {
        return system_path;
    }

    if let Some(home) = home_dir {
        let home_path = home.join(DEFAULT_CONFIG_REL_HOME);
        if home_path.exists() {
            return home_path;
        }
        return home_path;
    }

    system_path
}

pub fn path_allowed_for_indexing(path: &Path, settings: &IndexSettings) -> bool {
    if !settings.enabled {
        return false;
    }

    let in_roots = settings.roots.iter().any(|root| path.starts_with(root));
    if !in_roots {
        return false;
    }

    for excluded in &settings.exclude_prefixes {
        if path.starts_with(excluded) {
            return false;
        }
    }

    true
}

pub fn save_index_settings(
    settings: &IndexSettings,
    home_dir: Option<&Path>,
) -> Result<PathBuf, String> {
    let serialized = serialize_index_settings(settings);
    let target = preferred_write_config_path(home_dir);
    write_text_file_with_fallback(
        &target,
        serialized.as_str(),
        home_dir.map(|h| h.join(DEFAULT_CONFIG_REL_HOME)),
    )
}

pub fn serialize_index_settings(settings: &IndexSettings) -> String {
    let roots = join_path_list(&settings.roots);
    let excludes = join_path_list(&settings.exclude_prefixes);

    format!(
        "# StratOS indexer backend settings\n\
         # Managed by strat-settings.\n\
         enabled={}\n\
         boot_start={}\n\
         frequency_ms={}\n\
         rescan_secs={}\n\
         batch_limit={}\n\
         high_usage_load_per_cpu={}\n\
         roots={}\n\
         exclude_prefixes={}\n\
         ui_enabled={}\n\
         ui_tick_ms={}\n\
         ui_batch_limit={}\n\
         ui_idle_after_secs={}\n\
         ui_startup_grace_secs={}\n\
         ui_post_nav_delay_ms={}\n\
         ui_post_nav_scan_limit={}\n\
         ui_post_nav_force_secs={}\n",
        bool_text(settings.enabled),
        bool_text(settings.boot_start),
        settings.frequency_ms,
        settings.rescan_secs,
        settings.batch_limit,
        settings.high_usage_load_per_cpu,
        roots,
        excludes,
        bool_text(settings.ui_enabled),
        settings.ui_tick_ms,
        settings.ui_batch_limit,
        settings.ui_idle_after_secs,
        settings.ui_startup_grace_secs,
        settings.ui_post_nav_delay_ms,
        settings.ui_post_nav_scan_limit,
        settings.ui_post_nav_force_secs
    )
}

pub fn disable_flag_path(home_dir: Option<&Path>) -> PathBuf {
    let system_parent = Path::new("/config/strat");
    if system_parent.is_dir() {
        return PathBuf::from(DEFAULT_DISABLE_FLAG_SYSTEM);
    }

    if let Some(home) = home_dir {
        return home.join(DEFAULT_DISABLE_FLAG_REL_HOME);
    }

    PathBuf::from(DEFAULT_DISABLE_FLAG_SYSTEM)
}

pub fn set_disable_flag(disabled: bool, home_dir: Option<&Path>) -> Result<PathBuf, String> {
    let target = disable_flag_path(home_dir);
    if disabled {
        write_text_file_with_fallback(
            &target,
            "disabled=1\n",
            home_dir.map(|h| h.join(DEFAULT_DISABLE_FLAG_REL_HOME)),
        )
    } else {
        remove_file_if_exists_with_fallback(
            &target,
            home_dir.map(|h| h.join(DEFAULT_DISABLE_FLAG_REL_HOME)),
        )?;
        Ok(target)
    }
}

pub fn disable_flag_exists(home_dir: Option<&Path>) -> bool {
    if Path::new(DEFAULT_DISABLE_FLAG_SYSTEM).exists() {
        return true;
    }
    if let Some(home) = home_dir {
        return home.join(DEFAULT_DISABLE_FLAG_REL_HOME).exists();
    }
    false
}

pub fn preferred_write_config_path(home_dir: Option<&Path>) -> PathBuf {
    let system_parent = Path::new("/config/strat");
    if system_parent.is_dir() {
        return PathBuf::from(DEFAULT_CONFIG_SYSTEM);
    }

    if let Some(home) = home_dir {
        return home.join(DEFAULT_CONFIG_REL_HOME);
    }

    PathBuf::from(DEFAULT_CONFIG_SYSTEM)
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_u64(raw: &str) -> Option<u64> {
    raw.trim().parse::<u64>().ok()
}

fn parse_usize(raw: &str) -> Option<usize> {
    raw.trim().parse::<usize>().ok()
}

fn parse_f64(raw: &str) -> Option<f64> {
    raw.trim().parse::<f64>().ok()
}

fn parse_path_list(raw: &str) -> Vec<PathBuf> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn join_path_list(paths: &[PathBuf]) -> String {
    let mut out = Vec::new();
    for p in paths {
        out.push(p.to_string_lossy().to_string());
    }
    out.join(",")
}

fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn write_text_file_with_fallback(
    primary: &Path,
    content: &str,
    fallback: Option<PathBuf>,
) -> Result<PathBuf, String> {
    match write_text_file(primary, content) {
        Ok(()) => Ok(primary.to_path_buf()),
        Err(primary_err) => {
            if let Some(fallback_path) = fallback {
                write_text_file(&fallback_path, content)
                    .map(|_| fallback_path.clone())
                    .map_err(|fallback_err| {
                        format!(
                            "write failed for {} ({primary_err}); fallback {} failed ({fallback_err})",
                            primary.display(),
                            fallback_path.display()
                        )
                    })
            } else {
                Err(format!("write failed for {} ({primary_err})", primary.display()))
            }
        }
    }
}

fn write_text_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_all {}: {err}", parent.display()))?;
    }
    fs::write(path, content).map_err(|err| format!("write {}: {err}", path.display()))
}

fn remove_file_if_exists_with_fallback(
    primary: &Path,
    fallback: Option<PathBuf>,
) -> Result<(), String> {
    if remove_file_if_exists(primary)? {
        return Ok(());
    }

    if let Some(fallback_path) = fallback {
        let _ = remove_file_if_exists(&fallback_path)?;
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(path).map_err(|err| format!("remove {}: {err}", path.display()))?;
    Ok(true)
}
