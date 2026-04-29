use anyhow::{Context, Result};
use rusqlite::Connection;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Db {
    conn: Mutex<Connection>,
    _lock: File, // held for process lifetime
    /// True when the DB was corrupt on startup and had to be recreated.
    pub recovered: bool,
}

impl Db {
    /// Open (or recover) the database in `dir`.
    ///
    /// Returns `Ok(db)` in all non-fatal cases:
    ///   - clean open: `db.recovered == false`
    ///   - corruption detected + file renamed + DB recreated: `db.recovered == true`
    ///
    /// Returns `Err` only if the directory or lockfile cannot be created, or if
    /// another instance holds the process lock.
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir).context("create db dir")?;

        let lock_path = dir.join("claude-monitor.lock");
        let lock_file = File::create(&lock_path).context("create lockfile")?;
        lock_file
            .try_lock()
            .context("another instance holds the DB lock")?;

        let db_path = dir.join("data.db");
        let (conn, recovered) = Self::open_or_recover(&db_path)?;

        let mut db = Db { conn: Mutex::new(conn), _lock: lock_file, recovered };
        db.migrate()?;
        Ok(db)
    }

    /// Try to open `db_path` and verify its integrity.  On failure (open error
    /// or `PRAGMA integrity_check` ≠ "ok"), rename the corrupt file and create
    /// a fresh DB in its place.  Returns `(connection, was_recovered)`.
    fn open_or_recover(db_path: &Path) -> Result<(Connection, bool)> {
        let corrupt = Self::is_corrupt(db_path);

        if corrupt {
            // Rename the bad file so it's recoverable by the user.
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let backup = db_path.with_file_name(format!(
                "{}.corrupt-{ts}",
                db_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("data.db")
            ));
            tracing::warn!(
                "corrupt DB detected — renaming {:?} to {:?} and recreating",
                db_path,
                backup,
            );
            // Best-effort rename; if it fails we still try to create a fresh DB.
            let _ = std::fs::rename(db_path, &backup);

            let conn = Connection::open(db_path).context("open fresh sqlite after recovery")?;
            conn.execute_batch(include_str!("schema.sql"))
                .context("apply schema on recovered db")?;
            return Ok((conn, true));
        }

        // Normal path: open succeeded and integrity is clean.
        let conn = Connection::open(db_path).context("open sqlite")?;
        // schema.sql holds the *current* shape (v2). For a fresh DB this
        // creates everything; for an existing v1 DB it's a no-op (CREATE
        // TABLE IF NOT EXISTS) and the migration block below brings it
        // forward.
        conn.execute_batch(include_str!("schema.sql"))
            .context("apply schema")?;
        Ok((conn, false))
    }

    /// Returns `true` when `db_path` cannot be opened or `PRAGMA integrity_check`
    /// returns anything other than a single "ok" row.
    fn is_corrupt(db_path: &Path) -> bool {
        // A missing file is not corrupt — SQLite will create a fresh one.
        if !db_path.exists() {
            return false;
        }
        match Connection::open(db_path) {
            Err(_) => true,
            Ok(conn) => {
                // integrity_check returns one or more rows.  A healthy DB
                // returns exactly one row containing the text "ok".
                let result: rusqlite::Result<String> = conn.query_row(
                    "PRAGMA integrity_check",
                    [],
                    |r| r.get(0),
                );
                !matches!(result, Ok(s) if s == "ok")
            }
        }
    }

    /// Brings the DB up to the current schema version. Each block is
    /// idempotent (guarded by the schema_version row) so it's safe to run
    /// on fresh DBs too.
    fn migrate(&mut self) -> Result<()> {
        let conn = self.conn.get_mut().unwrap();
        let current: i64 = conn
            .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
            .unwrap_or(0);

        if current < 2 {
            tracing::info!("migrating session_events schema v1 -> v2 (event_id dedup)");
            conn.execute_batch(include_str!("migrations/0002_event_id_dedup.sql"))
                .context("apply migration 0002")?;
        }

        conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            [2_i64],
        )?;
        Ok(())
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("db mutex poisoned")
    }
}

pub mod queries;
pub use queries::*;

pub fn default_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "claude-limits", "ClaudeLimits")
        .map(|p| p.data_local_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".claude-monitor"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn opens_fresh_db_and_applies_schema() {
        let dir = tempdir().unwrap();
        let db = Db::open(dir.path()).expect("open db");
        assert!(!db.recovered, "fresh open should not set recovered");
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

    /// Write a deliberately-truncated (non-SQLite) file as `data.db`, then call
    /// `Db::open`.  The recovery path must:
    ///   1. Rename the corrupt file to `data.db.corrupt-<timestamp>`
    ///   2. Create a fresh, schema-applied DB at `data.db`
    ///   3. Set `db.recovered = true`
    #[test]
    fn recovers_from_corrupt_db() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("data.db");

        // Write garbage — not a valid SQLite file.
        let mut f = std::fs::File::create(&db_path).unwrap();
        f.write_all(b"this is not a sqlite database\x00\x01\x02").unwrap();
        drop(f);

        let db = Db::open(dir.path()).expect("open should succeed via recovery");
        assert!(db.recovered, "recovered flag must be set");

        // The new DB must have the schema applied.
        let conn = db.conn();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(count >= 6, "recovered DB should have >=6 tables, got {count}");

        // The corrupt file must have been renamed (a .corrupt-<ts> sibling exists).
        let corrupt_files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(".corrupt-")
            })
            .collect();
        assert!(
            !corrupt_files.is_empty(),
            "corrupt file should be renamed to *.corrupt-<timestamp>"
        );

        // The fresh DB file must exist at the original path.
        assert!(db_path.exists(), "fresh data.db must exist after recovery");
    }
}
