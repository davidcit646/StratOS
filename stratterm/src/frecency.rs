use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct FrecencyStore {
    conn: Connection,
}

impl FrecencyStore {
    pub fn open_default() -> Result<Self, String> {
        let db_path = frecency_db_path();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("create dir failed: {error}"))?;
        }
        let conn = Connection::open(db_path).map_err(|error| format!("open db failed: {error}"))?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS path_use (
                    path TEXT PRIMARY KEY,
                    use_count INTEGER NOT NULL DEFAULT 0,
                    last_used_epoch INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_path_use_last_used ON path_use(last_used_epoch);
                ",
            )
            .map_err(|error| format!("schema failed: {error}"))
    }

    pub fn record_use(&self, path: &Path) {
        let now = now_epoch();
        let path = path.to_string_lossy().to_string();
        let _ = self.conn.execute(
            "
            INSERT INTO path_use(path, use_count, last_used_epoch)
            VALUES(?1, 1, ?2)
            ON CONFLICT(path) DO UPDATE SET
                use_count = path_use.use_count + 1,
                last_used_epoch = excluded.last_used_epoch
            ",
            params![path, now],
        );
    }

    pub fn rank_paths(&self, prefix: &str, limit: usize) -> Vec<PathBuf> {
        let mut results = Vec::new();
        let pattern = format!("{}%", prefix);
        let mut statement = match self.conn.prepare(
            "
            SELECT path
            FROM path_use
            WHERE path LIKE ?1
            ORDER BY use_count DESC, last_used_epoch DESC
            LIMIT ?2
            ",
        ) {
            Ok(value) => value,
            Err(_) => return results,
        };

        let rows = match statement.query_map(params![pattern, limit as i64], |row| {
            row.get::<usize, String>(0)
        }) {
            Ok(value) => value,
            Err(_) => return results,
        };

        for row in rows.flatten() {
            results.push(PathBuf::from(row));
        }
        results
    }

    pub fn best_completion_for_cd(&self, line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("cd ") {
            return None;
        }
        let query = trimmed.strip_prefix("cd ")?.trim();
        if query.is_empty() {
            return None;
        }

        let mut candidates = self.rank_paths(query, 20);
        if candidates.is_empty() {
            return None;
        }

        candidates.sort();
        let first = candidates.first()?.to_string_lossy().to_string();
        if !first.starts_with(query) {
            return None;
        }
        Some(first[query.len()..].to_string())
    }

    pub fn expand_cd_shortcut(&self, shortcut: &str) -> Option<PathBuf> {
        let segments: Vec<&str> = shortcut.split('/').filter(|value| !value.is_empty()).collect();
        if segments.is_empty() {
            return None;
        }

        let ranked = self.rank_paths("/", 500);
        let mut seen = HashSet::new();
        for path in ranked {
            if !seen.insert(path.clone()) {
                continue;
            }
            if shortcut_matches(&path, &segments) {
                return Some(path);
            }
        }
        None
    }
}

fn frecency_db_path() -> PathBuf {
    let config_db = PathBuf::from("/config/strat/frecency.db");
    if config_db.parent().is_some_and(|parent| parent.exists()) {
        return config_db;
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config/strat/frecency.db");
    }
    PathBuf::from("/tmp/strat-frecency.db")
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}

fn shortcut_matches(path: &Path, segments: &[&str]) -> bool {
    let parts: Vec<String> = path
        .components()
        .filter_map(|part| {
            let value = part.as_os_str().to_string_lossy();
            if value == "/" || value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
        .collect();
    if parts.len() < segments.len() {
        return false;
    }

    for (index, segment) in segments.iter().enumerate() {
        let candidate = &parts[index];
        if !candidate.starts_with(*segment) {
            return false;
        }
    }
    true
}
