use anyhow::{Context, Result};
use rusqlite::Connection;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Db {
    conn: Mutex<Connection>,
    _lock: File, // held for process lifetime
}

impl Db {
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir).context("create db dir")?;

        let lock_path = dir.join("claude-monitor.lock");
        let lock_file = File::create(&lock_path).context("create lockfile")?;
        lock_file
            .try_lock()
            .context("another instance holds the DB lock")?;

        let db_path = dir.join("data.db");
        let conn = Connection::open(&db_path).context("open sqlite")?;
        conn.execute_batch(include_str!("schema.sql"))
            .context("apply schema")?;

        let mut db = Db { conn: Mutex::new(conn), _lock: lock_file };
        db.ensure_version(1)?;
        Ok(db)
    }

    fn ensure_version(&mut self, target: i64) -> Result<()> {
        let conn = self.conn.get_mut().unwrap();
        let current: i64 = conn
            .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
            .unwrap_or(0);
        if current < target {
            conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (?1)", [target])?;
        }
        Ok(())
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("db mutex poisoned")
    }
}

pub mod queries;
pub use queries::*;

pub fn default_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "claude-usage-monitor", "ClaudeUsageMonitor")
        .map(|p| p.data_local_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".claude-monitor"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn opens_fresh_db_and_applies_schema() {
        let dir = tempdir().unwrap();
        let db = Db::open(dir.path()).expect("open db");
        let conn = db.conn();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(count >= 6, "expected >=6 tables, got {count}");
    }

    #[test]
    fn rejects_second_instance() {
        let dir = tempdir().unwrap();
        let _first = Db::open(dir.path()).expect("first open");
        let second = Db::open(dir.path());
        assert!(second.is_err(), "second open should fail");
    }
}
