use super::record::SessionEvent;
use super::pricing::PricingTable;
use crate::store::{Db, StoredSessionEvent};
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{BufRead, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

pub fn claude_projects_root() -> Option<PathBuf> {
    directories::UserDirs::new()
        .map(|u| u.home_dir().join(".claude").join("projects"))
}

pub fn discover_jsonl_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(root).context("read projects dir")? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if !meta.is_dir() {
            continue;
        }
        if meta.file_type().is_symlink() {
            continue;
        }
        let project_dir = entry.path();
        for f in fs::read_dir(&project_dir)? {
            let f = f?;
            let fmeta = f.metadata()?;
            if !fmeta.is_file() {
                continue;
            }
            if fmeta.file_type().is_symlink() {
                continue;
            }
            let path = f.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if fmeta.len() > MAX_FILE_BYTES {
                tracing::warn!("skipping oversized file (>100MB): {}", path.display());
                continue;
            }
            files.push(path);
        }
    }
    Ok(files)
}

pub fn ingest_file(db: &Db, pricing: &PricingTable, path: &Path) -> Result<usize> {
    let meta = fs::metadata(path)?;
    let file_len = meta.len() as i64;
    let mtime_ns = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0);

    let key = path.display().to_string();
    let (prev_mtime, mut offset) = db.get_cursor(&key)?.unwrap_or((0, 0));

    if file_len < offset {
        tracing::info!("truncation detected, resetting cursor for {}", key);
        offset = 0;
    } else if prev_mtime == mtime_ns && file_len == offset {
        return Ok(0);
    }

    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(offset as u64))?;

    let project = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut reader = std::io::BufReader::new(f);
    let mut buf = Vec::new();
    let mut stored = Vec::<StoredSessionEvent>::new();
    let mut consumed: i64 = offset;

    loop {
        buf.clear();
        let line_start = consumed;
        let n = reader.read_until(b'\n', &mut buf)?;
        if n == 0 {
            break;
        }
        if *buf.last().unwrap() != b'\n' {
            break;
        }
        consumed += n as i64;
        let text = match std::str::from_utf8(&buf) {
            Ok(t) => t.trim(),
            Err(_) => continue,
        };
        if text.is_empty() {
            continue;
        }
        match serde_json::from_str::<SessionEvent>(text) {
            Ok(ev) => {
                let cost = if ev.cost_usd > 0.0 {
                    ev.cost_usd
                } else {
                    pricing.cost_for(
                        &ev.model,
                        ev.input_tokens,
                        ev.output_tokens,
                        ev.cache_read_tokens,
                        ev.cache_creation_5m_tokens,
                        ev.cache_creation_1h_tokens,
                    )
                };
                let project_name = if ev.project.is_empty() {
                    project.clone()
                } else {
                    ev.project.clone()
                };
                stored.push(StoredSessionEvent {
                    ts: ev.ts,
                    project: project_name,
                    model: ev.model,
                    input_tokens: ev.input_tokens,
                    output_tokens: ev.output_tokens,
                    cache_read_tokens: ev.cache_read_tokens,
                    cache_creation_5m_tokens: ev.cache_creation_5m_tokens,
                    cache_creation_1h_tokens: ev.cache_creation_1h_tokens,
                    cost_usd: cost,
                    source_file: key.clone(),
                    source_line: line_start,
                });
            }
            Err(e) => tracing::warn!("malformed line in {} at offset {}: {}", key, line_start, e),
        }
    }
    let inserted = db.insert_events(&stored)?;
    db.set_cursor(&key, mtime_ns, consumed)?;
    Ok(inserted)
}
