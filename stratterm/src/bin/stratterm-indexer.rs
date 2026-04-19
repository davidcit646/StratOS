use rusqlite::{params, Connection};
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use stratsettings::SpotliteHeadlessSettings;

#[derive(Debug, Clone)]
struct IndexerConfig {
    enabled: bool,
    boot_start: bool,
    frequency_ms: u64,
    rescan_secs: u64,
    batch_limit: usize,
    high_usage_load_per_cpu: f64,
    roots: Vec<PathBuf>,
    exclude_prefixes: Vec<PathBuf>,
}

impl Default for IndexerConfig {
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
        }
    }
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_paths(value: &str) -> Vec<PathBuf> {
    value
        .split(',')
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn read_text(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

fn config_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("/config/strat/indexer.conf")];
    if let Some(home) = env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".config/strat/indexer.conf"));
    }
    candidates
}

fn apply_spotlite_headless(config: &mut IndexerConfig, h: &SpotliteHeadlessSettings) {
    config.enabled = h.enabled;
    config.boot_start = h.boot_start;
    config.frequency_ms = h.frequency_ms;
    config.rescan_secs = h.rescan_secs;
    config.batch_limit = h.batch_limit;
    config.high_usage_load_per_cpu = h.high_usage_load_per_cpu;
    if !h.roots.is_empty() {
        config.roots = h.roots.iter().map(PathBuf::from).collect();
    }
    config.exclude_prefixes = h.exclude_prefixes.iter().map(PathBuf::from).collect();
}

fn apply_indexer_conf_text(config: &mut IndexerConfig, content: &str) {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (key, value) = match trimmed.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "enabled" => {
                if let Some(parsed) = parse_bool(value) {
                    config.enabled = parsed;
                }
            }
            "boot_start" => {
                if let Some(parsed) = parse_bool(value) {
                    config.boot_start = parsed;
                }
            }
            "frequency_ms" => {
                if let Ok(parsed) = value.parse() {
                    config.frequency_ms = parsed;
                }
            }
            "rescan_secs" => {
                if let Ok(parsed) = value.parse() {
                    config.rescan_secs = parsed;
                }
            }
            "batch_limit" => {
                if let Ok(parsed) = value.parse() {
                    config.batch_limit = parsed;
                }
            }
            "high_usage_load_per_cpu" => {
                if let Ok(parsed) = value.parse() {
                    config.high_usage_load_per_cpu = parsed;
                }
            }
            "roots" => {
                let parsed = parse_paths(value);
                if !parsed.is_empty() {
                    config.roots = parsed;
                }
            }
            "exclude_prefixes" => {
                config.exclude_prefixes = parse_paths(value);
            }
            _ => {}
        }
    }
}

fn load_config() -> IndexerConfig {
    let mut config = IndexerConfig::default();
    if let Ok(settings) = stratsettings::StratSettings::load() {
        apply_spotlite_headless(&mut config, &settings.spotlite.headless);
    }
    for candidate in config_candidates() {
        let content = match read_text(&candidate) {
            Ok(text) => text,
            Err(_) => continue,
        };
        apply_indexer_conf_text(&mut config, &content);
        return config;
    }
    config
}

fn db_path() -> PathBuf {
    let config_db = PathBuf::from("/config/strat/path-index.db");
    if config_db.parent().is_some_and(|parent| parent.exists()) {
        return config_db;
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config/strat/path-index.db");
    }
    PathBuf::from("/tmp/strat-path-index.db")
}

fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn create_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS path_index (
            path TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            mtime_epoch INTEGER NOT NULL,
            indexed_epoch INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_path_index_kind ON path_index(kind);
        CREATE INDEX IF NOT EXISTS idx_path_index_mtime ON path_index(mtime_epoch);
        ",
    )
}

fn system_load_ratio() -> Option<f64> {
    let raw = fs::read_to_string("/proc/loadavg").ok()?;
    let first = raw.split_whitespace().next()?;
    let load = first.parse::<f64>().ok()?;
    let cpus = thread::available_parallelism().ok()?.get() as f64;
    if cpus <= 0.0 {
        return None;
    }
    Some(load / cpus)
}

fn skip_for_high_usage(config: &IndexerConfig) -> bool {
    match system_load_ratio() {
        Some(ratio) => ratio > config.high_usage_load_per_cpu,
        None => false,
    }
}

fn is_excluded(path: &Path, excludes: &[PathBuf]) -> bool {
    excludes.iter().any(|prefix| path.starts_with(prefix))
}

fn metadata_tuple(path: &Path) -> io::Result<(&'static str, u64, i64)> {
    let metadata = fs::symlink_metadata(path)?;
    let kind = if metadata.is_dir() {
        "dir"
    } else if metadata.is_file() {
        "file"
    } else if metadata.file_type().is_symlink() {
        "symlink"
    } else {
        "other"
    };
    let size = metadata.len();
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0);
    Ok((kind, size, mtime))
}

fn upsert_path(connection: &Connection, path: &Path, indexed_epoch: i64) {
    let (kind, size, mtime) = match metadata_tuple(path) {
        Ok(tuple) => tuple,
        Err(_) => return,
    };
    let _ = connection.execute(
        "
        INSERT INTO path_index(path, kind, size_bytes, mtime_epoch, indexed_epoch)
        VALUES(?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(path) DO UPDATE SET
            kind=excluded.kind,
            size_bytes=excluded.size_bytes,
            mtime_epoch=excluded.mtime_epoch,
            indexed_epoch=excluded.indexed_epoch
        ",
        params![path.to_string_lossy(), kind, size as i64, mtime, indexed_epoch],
    );
}

fn scan_once(connection: &Connection, config: &IndexerConfig) {
    let timestamp = now_ts();
    let mut queue = VecDeque::new();
    for root in &config.roots {
        if root.exists() {
            queue.push_back(root.clone());
        }
    }

    let mut staged = 0usize;
    while let Some(path) = queue.pop_front() {
        if is_excluded(&path, &config.exclude_prefixes) {
            continue;
        }

        upsert_path(connection, &path, timestamp);
        staged += 1;

        let recurse = fs::symlink_metadata(&path)
            .map(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
            .unwrap_or(false);
        if recurse {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let child = entry.path();
                    if !is_excluded(&child, &config.exclude_prefixes) {
                        queue.push_back(child);
                    }
                }
            }
        }

        if staged >= config.batch_limit {
            staged = 0;
            thread::sleep(Duration::from_millis(config.frequency_ms));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Once,
    Daemon,
    BootDaemon,
}

fn parse_mode() -> Mode {
    let mut mode = Mode::Daemon;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--once" => mode = Mode::Once,
            "--daemon" => mode = Mode::Daemon,
            "--boot-daemon" => mode = Mode::BootDaemon,
            _ => {}
        }
    }
    mode
}

fn run() -> Result<(), String> {
    let mode = parse_mode();
    let config = load_config();
    if !config.enabled {
        return Ok(());
    }
    if mode == Mode::BootDaemon && !config.boot_start {
        return Ok(());
    }

    let db = db_path();
    ensure_parent_dir(&db).map_err(|error| format!("create db dir failed: {error}"))?;
    let connection = Connection::open(db).map_err(|error| format!("open db failed: {error}"))?;
    create_schema(&connection).map_err(|error| format!("create schema failed: {error}"))?;

    match mode {
        Mode::Once => {
            if !skip_for_high_usage(&config) {
                scan_once(&connection, &config);
            }
        }
        Mode::Daemon | Mode::BootDaemon => loop {
            if !skip_for_high_usage(&config) {
                scan_once(&connection, &config);
            }
            thread::sleep(Duration::from_secs(config.rescan_secs.max(1)));
        },
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("stratterm-indexer: {error}");
        std::process::exit(1);
    }
}
