-- v1 → v2: switch session_events dedup from (source_file, source_line) to
-- a content-stable event_id (Claude's "{requestId}:{message.id}"). The old
-- key let the same usage block sneak in multiple times whenever Claude Code
-- wrote it to different offsets in the same file (retries / partial
-- rewinds), inflating reported cost. We can't recover real event_ids for
-- already-stored rows, so we drop them and let the walker re-ingest from
-- scratch (cursors reset to byte_offset=0). The .jsonl files on disk are
-- the source of truth, so this is non-destructive in practice.
DROP TABLE IF EXISTS session_events;
CREATE TABLE session_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts INTEGER NOT NULL,
    project TEXT NOT NULL,
    model TEXT NOT NULL,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cache_read_tokens INTEGER NOT NULL DEFAULT 0,
    cache_creation_5m_tokens INTEGER NOT NULL DEFAULT 0,
    cache_creation_1h_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0,
    source_file TEXT NOT NULL,
    source_line INTEGER NOT NULL,
    event_id TEXT NOT NULL,
    UNIQUE (event_id)
);
CREATE INDEX idx_events_ts ON session_events(ts DESC);
CREATE INDEX idx_events_project ON session_events(project);
CREATE INDEX idx_events_model ON session_events(model);

DELETE FROM jsonl_cursors;
