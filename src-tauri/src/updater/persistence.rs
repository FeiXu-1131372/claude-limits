//! Tiny JSON file at `<app_data_dir>/updater.json` holding the last
//! successful check timestamp. Read on startup, written after every
//! successful check (whether or not an update was found).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

const FILE_NAME: &str = "updater.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
struct Persisted {
    last_checked_at: Option<DateTime<Utc>>,
}

pub fn read_last_checked_at(data_dir: &Path) -> Option<DateTime<Utc>> {
    let path = data_dir.join(FILE_NAME);
    let bytes = std::fs::read(&path).ok()?;
    let parsed: Persisted = serde_json::from_slice(&bytes).ok()?;
    parsed.last_checked_at
}

pub fn write_last_checked_at(data_dir: &Path, when: DateTime<Utc>) {
    let path = data_dir.join(FILE_NAME);
    let payload = Persisted { last_checked_at: Some(when) };
    let json = match serde_json::to_vec_pretty(&payload) {
        Ok(j) => j,
        Err(e) => { tracing::warn!("updater persistence: serialize failed: {e}"); return; }
    };
    if let Err(e) = std::fs::write(&path, json) {
        tracing::warn!("updater persistence: write failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    #[test]
    fn returns_none_when_file_missing() {
        let dir = TempDir::new().unwrap();
        assert_eq!(read_last_checked_at(dir.path()), None);
    }

    #[test]
    fn round_trips_timestamp() {
        let dir = TempDir::new().unwrap();
        let when = Utc.with_ymd_and_hms(2026, 4, 29, 12, 30, 0).unwrap();
        write_last_checked_at(dir.path(), when);
        assert_eq!(read_last_checked_at(dir.path()), Some(when));
    }

    #[test]
    fn returns_none_on_corrupt_file() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(FILE_NAME), b"not json").unwrap();
        assert_eq!(read_last_checked_at(dir.path()), None);
    }

    #[test]
    fn write_is_idempotent_overwrite() {
        let dir = TempDir::new().unwrap();
        let t1 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 4, 29, 0, 0, 0).unwrap();
        write_last_checked_at(dir.path(), t1);
        write_last_checked_at(dir.path(), t2);
        assert_eq!(read_last_checked_at(dir.path()), Some(t2));
    }
}
