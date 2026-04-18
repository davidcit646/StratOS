use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn default_values() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    map.insert("enabled".into(), "true".into());
    map.insert("boot_start".into(), "true".into());
    map.insert("frequency_ms".into(), "1200".into());
    map.insert("rescan_secs".into(), "180".into());
    map.insert("batch_limit".into(), "96".into());
    map.insert("high_usage_load_per_cpu".into(), "0.85".into());
    map.insert("roots".into(), "/home,/config,/apps".into());
    map.insert("exclude_prefixes".into(), "".into());
    map.insert("ui_enabled".into(), "true".into());
    map.insert("ui_tick_ms".into(), "750".into());
    map.insert("ui_batch_limit".into(), "80".into());
    map.insert("ui_idle_after_secs".into(), "12".into());
    map.insert("ui_startup_grace_secs".into(), "8".into());
    map.insert("ui_post_nav_delay_ms".into(), "180".into());
    map.insert("ui_post_nav_scan_limit".into(), "1200".into());
    map.insert("ui_post_nav_force_secs".into(), "6".into());
    map
}

fn config_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("/config/strat/indexer.conf")];
    if let Some(home) = env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".config/strat/indexer.conf"));
    }
    candidates
}

fn target_config_path() -> PathBuf {
    let primary = PathBuf::from("/config/strat/indexer.conf");
    if primary.parent().is_some_and(|parent| parent.exists()) {
        return primary;
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config/strat/indexer.conf");
    }
    PathBuf::from("/tmp/indexer.conf")
}

fn load_existing() -> BTreeMap<String, String> {
    let mut values = default_values();
    for candidate in config_candidates() {
        let content = match fs::read_to_string(&candidate) {
            Ok(text) => text,
            Err(_) => continue,
        };
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                values.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
        return values;
    }
    values
}

fn save(path: &Path, values: &BTreeMap<String, String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create dir failed: {error}"))?;
    }
    let mut output = String::new();
    output.push_str("# StratOS indexer backend settings\n");
    output.push_str("# Managed by strat-settings\n");
    for (key, value) in values {
        output.push_str(key);
        output.push('=');
        output.push_str(value);
        output.push('\n');
    }
    fs::write(path, output).map_err(|error| format!("write failed: {error}"))?;
    Ok(())
}

fn print_usage() {
    println!("strat-settings [--show] [--set key=value] [--reset-defaults] [--interactive]");
}

fn show(values: &BTreeMap<String, String>, path: &Path) {
    println!("config_path={}", path.display());
    for (key, value) in values {
        println!("{key}={value}");
    }
}

fn apply_set(values: &mut BTreeMap<String, String>, assignment: &str) -> Result<(), String> {
    let (key, value) = assignment
        .split_once('=')
        .ok_or_else(|| format!("invalid --set value: {assignment}"))?;
    let key = key.trim();
    if key.is_empty() {
        return Err("invalid --set key: empty".into());
    }
    values.insert(key.to_string(), value.trim().to_string());
    Ok(())
}

fn interactive_edit(mut values: BTreeMap<String, String>) -> Result<BTreeMap<String, String>, String> {
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let mut keys = values.keys().cloned().collect::<Vec<_>>();
    keys.sort();

    for key in keys {
        let current = values.get(&key).cloned().unwrap_or_default();
        write!(stdout, "{} [{}]: ", key, current).map_err(|error| format!("write failed: {error}"))?;
        stdout.flush().map_err(|error| format!("flush failed: {error}"))?;
        let mut line = String::new();
        stdin
            .read_line(&mut line)
            .map_err(|error| format!("read failed: {error}"))?;
        let next = line.trim();
        if !next.is_empty() {
            values.insert(key, next.to_string());
        }
    }
    Ok(values)
}

fn run() -> Result<(), String> {
    let mut show_only = false;
    let mut reset_defaults = false;
    let mut interactive = false;
    let mut assignments: Vec<String> = Vec::new();

    let mut args = env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--show" => show_only = true,
            "--reset-defaults" => reset_defaults = true,
            "--interactive" => interactive = true,
            "--set" => {
                let assignment = args
                    .next()
                    .ok_or_else(|| "--set expects key=value".to_string())?;
                assignments.push(assignment);
            }
            "-h" | "--help" => {
                print_usage();
                return Ok(());
            }
            _ => return Err(format!("unknown arg: {arg}")),
        }
    }

    let path = target_config_path();
    let mut values = if reset_defaults {
        default_values()
    } else {
        load_existing()
    };

    if interactive {
        values = interactive_edit(values)?;
    }

    for assignment in assignments {
        apply_set(&mut values, &assignment)?;
    }

    if show_only && !interactive && !reset_defaults {
        show(&values, &path);
        return Ok(());
    }

    save(&path, &values)?;
    show(&values, &path);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("strat-settings: {error}");
        std::process::exit(1);
    }
}
