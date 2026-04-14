use rusqlite::{params, Connection};
use std::collections::{HashSet, VecDeque};
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use stratterm::index_settings::{
    indexer_is_disabled, load_index_settings, path_allowed_for_indexing, IndexSettings,
};

const INDEX_MAX_QUEUE: usize = 400_000;
const INDEX_CHILD_SCAN_LIMIT: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunMode {
    Daemon,
    BootDaemon,
    Once,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PathFingerprint {
    is_dir: i64,
    size: i64,
    mtime: i64,
    mode: i64,
}

#[derive(Default)]
struct IndexerState {
    index_queue: VecDeque<PathBuf>,
    index_seen: HashSet<String>,
}

fn main() {
    let mut run_mode = RunMode::Daemon;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--once" => run_mode = RunMode::Once,
            "--daemon" => run_mode = RunMode::Daemon,
            "--boot-daemon" => run_mode = RunMode::BootDaemon,
            "-h" | "--help" => {
                print_help();
                return;
            }
            other => {
                eprintln!("stratterm-indexer: unknown argument: {other}");
                print_help();
                std::process::exit(2);
            }
        }
    }

    let home_dir = env::var_os("HOME").map(PathBuf::from);
    let home_ref = home_dir.as_deref();
    let settings = load_index_settings(home_ref);

    if indexer_is_disabled(home_ref) || !settings.enabled {
        eprintln!("stratterm-indexer: disabled via /config/strat/disable-indexer");
        return;
    }

    if run_mode == RunMode::BootDaemon && !settings.boot_start {
        eprintln!("stratterm-indexer: boot_start=false, skipping boot launch");
        return;
    }

    let db_path = index_db_path();
    let mut conn = match open_index_db(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!(
                "stratterm-indexer: failed to open index db {}: {err}",
                db_path.display()
            );
            std::process::exit(1);
        }
    };

    let mut state = IndexerState::default();
    let roots = index_roots(&settings);
    for root in roots {
        enqueue_index_path(&mut state, root, &settings);
    }

    let mut last_rescan = unix_now();

    loop {
        if indexer_is_disabled(home_ref) {
            eprintln!("stratterm-indexer: disable flag detected, exiting");
            break;
        }

        if run_mode != RunMode::Once {
            let now = unix_now();
            if now.saturating_sub(last_rescan) >= settings.rescan_secs {
                for root in index_roots(&settings) {
                    enqueue_index_path(&mut state, root, &settings);
                }
                last_rescan = now;
            }
        }

        if is_usage_high(settings.high_usage_load_per_cpu) {
            thread::sleep(Duration::from_millis(settings.frequency_ms.saturating_mul(2)));
            continue;
        }

        let mut batch = Vec::new();
        for _ in 0..settings.batch_limit {
            let Some(path) = state.index_queue.pop_front() else {
                break;
            };
            let key = path.to_string_lossy().to_string();
            state.index_seen.remove(&key);
            batch.push(path);
        }

        if batch.is_empty() {
            if run_mode == RunMode::Once {
                break;
            }
            thread::sleep(Duration::from_millis(settings.frequency_ms));
            continue;
        }

        let now = unix_now();
        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(err) => {
                eprintln!("stratterm-indexer: failed to start tx: {err}");
                thread::sleep(Duration::from_millis(settings.frequency_ms));
                continue;
            }
        };

        for path in batch {
            let changed = index_one_path(&tx, &path, now);
            if !changed {
                continue;
            }

            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            if !meta.is_dir() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&path) {
                for item in entries.flatten().take(INDEX_CHILD_SCAN_LIMIT) {
                    enqueue_index_path(&mut state, item.path(), &settings);
                }
            }
        }

        if let Err(err) = tx.commit() {
            eprintln!("stratterm-indexer: tx commit failed: {err}");
            thread::sleep(Duration::from_millis(settings.frequency_ms));
            continue;
        }

        if run_mode == RunMode::Once && state.index_queue.is_empty() {
            break;
        }
    }
}

fn print_help() {
    println!(
        "stratterm-indexer\n\
         Usage: stratterm-indexer [--daemon] [--boot-daemon] [--once]\n\
         --daemon  Run continuously (default behavior)\n\
         --boot-daemon  Run daemon mode but only when boot_start=true in config\n\
         --once    Run one indexing sweep and exit"
    );
}

fn index_roots(settings: &IndexSettings) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for p in &settings.roots {
        if p.is_dir() {
            roots.push(p.to_path_buf());
        }
    }
    roots
}

fn enqueue_index_path(state: &mut IndexerState, path: PathBuf, settings: &IndexSettings) {
    if !path_allowed_for_indexing(&path, settings) {
        return;
    }

    if state.index_queue.len() >= INDEX_MAX_QUEUE {
        return;
    }
    let key = path.to_string_lossy().to_string();
    if state.index_seen.insert(key) {
        state.index_queue.push_back(path);
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_loadavg_one() -> Option<f64> {
    let content = fs::read_to_string("/proc/loadavg").ok()?;
    let first = content.split_whitespace().next()?;
    first.parse::<f64>().ok()
}

fn is_usage_high(threshold: f64) -> bool {
    let load_one = read_loadavg_one().unwrap_or(0.0);
    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1) as f64;
    let load_per_cpu = load_one / cpu_count.max(1.0);
    load_per_cpu >= threshold
}

fn metadata_fingerprint(path: &Path) -> Option<PathFingerprint> {
    let meta = fs::symlink_metadata(path).ok()?;
    let is_dir = if meta.is_dir() { 1_i64 } else { 0_i64 };
    let size = meta.len().min(i64::MAX as u64) as i64;
    let mode = (meta.permissions().mode() & 0o7777) as i64;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .min(i64::MAX as u64) as i64;
    Some(PathFingerprint {
        is_dir,
        size,
        mtime,
        mode,
    })
}

fn query_index_fingerprint(
    conn: &Connection,
    path: &Path,
) -> Result<Option<PathFingerprint>, rusqlite::Error> {
    let path_text = path.to_string_lossy().to_string();
    let mut stmt = conn.prepare("SELECT is_dir, size, mtime, mode FROM paths WHERE path = ?1 LIMIT 1")?;
    let mut rows = stmt.query(params![path_text])?;
    if let Some(row) = rows.next()? {
        let is_dir: i64 = row.get(0)?;
        let size: i64 = row.get(1)?;
        let mtime: i64 = row.get(2)?;
        let mode: i64 = row.get(3)?;
        Ok(Some(PathFingerprint {
            is_dir,
            size,
            mtime,
            mode,
        }))
    } else {
        Ok(None)
    }
}

fn lowercase_extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default()
}

fn index_one_path(conn: &Connection, path: &Path, indexed_at: u64) -> bool {
    let Some(fp) = metadata_fingerprint(path) else {
        return false;
    };

    let previously_indexed = query_index_fingerprint(conn, path).ok().flatten();
    if previously_indexed == Some(fp) {
        return false;
    }

    let path_text = path.to_string_lossy().to_string();
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let parent = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string());
    let ext = lowercase_extension(path);
    let indexed_at_db = indexed_at.min(i64::MAX as u64) as i64;

    conn.execute(
        "INSERT INTO paths(path, name, parent, is_dir, size, mtime, mode, ext, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(path) DO UPDATE SET
            name=excluded.name,
            parent=excluded.parent,
            is_dir=excluded.is_dir,
            size=excluded.size,
            mtime=excluded.mtime,
            mode=excluded.mode,
            ext=excluded.ext,
            indexed_at=excluded.indexed_at",
        params![
            path_text,
            name,
            parent,
            fp.is_dir,
            fp.size,
            fp.mtime,
            fp.mode,
            ext,
            indexed_at_db
        ],
    )
    .is_ok()
}

fn index_db_path() -> PathBuf {
    if Path::new("/config").is_dir() {
        return PathBuf::from("/config/strat/index.db");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config/strat/index.db");
    }

    PathBuf::from("/tmp/stratterm_index.db")
}

fn open_index_db(path: &Path) -> Result<Connection, rusqlite::Error> {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS paths (
            path TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            parent TEXT NOT NULL,
            is_dir INTEGER NOT NULL,
            size INTEGER NOT NULL,
            mtime INTEGER NOT NULL,
            mode INTEGER NOT NULL,
            ext TEXT NOT NULL,
            indexed_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_paths_name ON paths(name);
         CREATE INDEX IF NOT EXISTS idx_paths_parent ON paths(parent);
         CREATE INDEX IF NOT EXISTS idx_paths_mtime ON paths(mtime DESC);",
    )?;
    Ok(conn)
}
