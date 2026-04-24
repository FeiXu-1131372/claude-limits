# Claude Usage Monitor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform (macOS + Windows) Tauri v2 menu-bar app that monitors Claude subscription rate-limits (5h + 7d buckets, Opus/Sonnet splits, extra-usage credits) and provides per-session analytics from local Claude Code JSONL logs.

**Architecture:** Rust backend (5 modules: auth, usage_api, jsonl_parser, store, notifier) behind a typed IPC boundary to a React 19 frontend. Design tokens + UI kit already scaffolded by the designer agent under `src/`. Backend is green-field.

**Tech Stack:**
- **Backend:** Rust stable, `rusqlite`, `reqwest`, `notify`, `keyring`, `oauth2`, `tokio`, `tauri` v2, `specta` for TS binding generation
- **Frontend:** React 19 + TypeScript, Tailwind CSS v4, Zustand, Recharts, Framer Motion, Lucide icons
- **Tooling:** pnpm, Vite, Cargo, GitHub Actions (CI matrix: Ubuntu + macOS + Windows)

**Source spec:** `docs/superpowers/specs/2026-04-24-claude-usage-monitor-design.md`
**Existing assets (preserve!):**
- `src/styles/tokens.css`, `globals.css`
- `src/lib/motion.ts`, `icons.ts`, `store.ts`
- `src/components/ui/` — Button, IconButton, Card, ProgressBar, Tabs, Toggle, Slider, Select, Banner, Badge, EmptyState
- `concepts/` — HTML mockups
- `CLAUDE.md`, `docs/`

**Branch policy:** Per `~/.claude/CLAUDE.md`, commit to current branch. Do NOT create feature branches or worktrees without asking.

**Commit discipline:** One commit per task step that ships code. Messages follow conventional commits (`feat:`, `fix:`, `test:`, `chore:`, `docs:`).

---

## Phase 0 — Project Bootstrap

### Task 0.1: Initialize pnpm + TypeScript + Vite config

**Files:**
- Create: `package.json`, `pnpm-workspace.yaml`, `tsconfig.json`, `tsconfig.node.json`, `vite.config.ts`, `index.html`, `src/main.tsx`, `.gitignore`, `.npmrc`

- [ ] **Step 1: Initialize git + pnpm and commit baseline**

```bash
cd "/Users/feixu/Developer/open Source/claude-usage-monitor"
git init
git add -A
git commit -m "chore: snapshot designer output before bootstrap"
```

Expected: "Initialized empty Git repository" then a commit with the designer's existing files.

- [ ] **Step 2: Create `.gitignore`**

```gitignore
node_modules/
target/
dist/
.DS_Store
*.local
.env
.env.*
!.env.example
src-tauri/target/
src-tauri/gen/
src/lib/generated/
```

- [ ] **Step 3: Create `package.json`**

```json
{
  "name": "claude-usage-monitor",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:watch": "vitest",
    "tauri": "tauri",
    "lint": "tsc --noEmit"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-autostart": "^2.0.0",
    "@tauri-apps/plugin-dialog": "^2.0.0",
    "@tauri-apps/plugin-notification": "^2.0.0",
    "@tauri-apps/plugin-opener": "^2.0.0",
    "@tauri-apps/plugin-os": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0",
    "@tauri-apps/plugin-single-instance": "^2.0.0",
    "@tauri-apps/plugin-window-state": "^2.0.0",
    "framer-motion": "^11.11.0",
    "lucide-react": "^0.460.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "recharts": "^2.13.0",
    "zustand": "^5.0.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.1.0",
    "@testing-library/jest-dom": "^6.6.0",
    "@testing-library/react": "^16.1.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "jsdom": "^25.0.0",
    "tailwindcss": "^4.0.0",
    "@tailwindcss/vite": "^4.0.0",
    "typescript": "^5.7.0",
    "vite": "^6.0.0",
    "vitest": "^2.1.0"
  }
}
```

- [ ] **Step 4: Create `tsconfig.json` and `tsconfig.node.json`**

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowImportingTsExtensions": false,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "baseUrl": ".",
    "paths": { "@/*": ["src/*"] }
  },
  "include": ["src", "vite.config.ts"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

`tsconfig.node.json`:
```json
{
  "compilerOptions": {
    "composite": true,
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "skipLibCheck": true,
    "strict": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 5: Create `vite.config.ts`**

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(__dirname, "src") } },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
    globals: true,
  },
});
```

- [ ] **Step 6: Create `index.html` and `src/main.tsx`**

`index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Claude Usage Monitor</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

`src/main.tsx`:
```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/globals.css";
import "./styles/tokens.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 7: Create `src/App.tsx` placeholder + `src/test-setup.ts`**

`src/App.tsx`:
```tsx
export default function App() {
  return <div style={{ padding: 24 }}>Claude Usage Monitor — bootstrap OK</div>;
}
```

`src/test-setup.ts`:
```ts
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 8: Install deps and verify build**

```bash
pnpm install
pnpm build
```

Expected: `pnpm build` succeeds, produces `dist/` directory.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "chore: scaffold pnpm + vite + react 19 + tailwind v4 config"
```

---

### Task 0.2: Initialize Tauri v2 backend

**Files:**
- Create: `src-tauri/` directory via `cargo tauri init`

- [ ] **Step 1: Initialize Tauri (non-interactive)**

```bash
cd "/Users/feixu/Developer/open Source/claude-usage-monitor"
pnpm tauri init --ci
```

Expected: `src-tauri/` directory created with `Cargo.toml`, `tauri.conf.json`, `src/main.rs`, `src/lib.rs`, `build.rs`, `icons/`, `capabilities/`. App name, frontend dist, and dev URL are all overwritten by Step 4's `tauri.conf.json` below — so we don't rely on the init CLI flags being stable across Tauri v2 minor versions.

- [ ] **Step 2: Replace `src-tauri/Cargo.toml` with pinned deps**

```toml
[package]
name = "claude-usage-monitor"
version = "0.1.0"
description = "Cross-platform Claude subscription usage monitor"
authors = ["Claude Usage Monitor contributors"]
license = "MIT"
edition = "2021"
rust-version = "1.77"

[lib]
name = "claude_usage_monitor_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-autostart = "2"
tauri-plugin-dialog = "2"
tauri-plugin-notification = "2"
tauri-plugin-opener = "2"
tauri-plugin-os = "2"
tauri-plugin-shell = "2"
tauri-plugin-single-instance = "2"
tauri-plugin-window-state = "2"
# Vibrancy / Mica / translucent-solid fallback — avoids needing macos-private-api
tauri-plugin-window-vibrancy = "0.5"

serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "gzip"] }
rusqlite = { version = "0.32", features = ["bundled", "chrono"] }
notify = "7"
notify-debouncer-full = "0.4"
# keyring — explicit backend features per platform
keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }
base64 = "0.22"
sha2 = "0.10"
rand = "0.8"
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
url = "2"
directories = "5"
# fs4 is the maintained replacement for fs2 (cross-platform file locking)
fs4 = "0.10"
parking_lot = "0.12"
# specta + tauri-specta: pin exact pre-release versions. Wired in Task 7.3.
# If the RC tree has drifted at plan-execution time, bump all three in lockstep.
specta = "=2.0.0-rc.22"
specta-typescript = "=0.0.9"
tauri-specta = { version = "=2.0.0-rc.21", features = ["derive", "typescript"] }

[target.'cfg(target_os = "macos")'.dependencies]
# Keychain access via `security` CLI is shell-out, no extra crates required.

[target.'cfg(target_os = "windows")'.dependencies]
whoami = "1"
windows-sys = { version = "0.59", features = ["Win32_Security_Authorization", "Win32_Foundation"] }

[dev-dependencies]
tempfile = "3"
mockito = "1"
pretty_assertions = "1"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

- [ ] **Step 3: Verify Rust build**

```bash
cd src-tauri
cargo build
cd ..
```

Expected: compilation succeeds (may take several minutes on first run).

- [ ] **Step 4: Add `src-tauri/tauri.conf.json` overrides for tray + window**

Replace the `app.windows` section so the main window starts hidden (we'll show the popover via tray):
```json
{
  "productName": "Claude Usage Monitor",
  "identifier": "com.claude-usage-monitor.app",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "popover",
        "title": "",
        "width": 360,
        "height": 420,
        "resizable": false,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "visible": false,
        "skipTaskbar": true
      }
    ],
    "trayIcon": {
      "iconPath": "icons/tray/idle-template.png",
      "iconAsTemplate": true,
      "menuOnLeftClick": false
    },
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: initialize tauri v2 backend with pinned deps"
```

---

### Task 0.3: Configure Tailwind v4 + verify design tokens load

**Files:**
- Create: `src/styles/tailwind.css` (entry) — **preserve existing** `tokens.css` and `globals.css`
- Modify: `src/main.tsx` to import Tailwind

- [ ] **Step 1: Inspect existing `globals.css` and `tokens.css`**

```bash
cat src/styles/globals.css
cat src/styles/tokens.css | head -80
```

Read to understand what CSS variables the designer defined (brand colors, glass blur, typography scale, motion curves).

- [ ] **Step 2: Add Tailwind v4 entry**

If `globals.css` does not already contain `@import "tailwindcss";` at the top, insert it as the first line. Use Edit tool:

```css
@import "tailwindcss";
/* existing content follows */
```

- [ ] **Step 3: Run dev server and visually verify tokens load**

```bash
pnpm dev
```

Expected: browser opens at http://localhost:1420 showing "Claude Usage Monitor — bootstrap OK". Inspect the page — the CSS custom properties from `tokens.css` should be visible in devtools under `:root`.

Kill the dev server with Ctrl+C once verified.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: wire tailwind v4 and verify design tokens load"
```

---

### Task 0.4: Logging scaffold (tracing → file)

**Files:**
- Create: `src-tauri/src/logging.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/logging.rs`**

```rust
use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init(log_dir: PathBuf) -> tracing_appender::non_blocking::WorkerGuard {
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "claude-usage-monitor.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,claude_usage_monitor_lib=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();

    tracing::info!("Logging initialized at {:?}", log_dir);
    guard
}

pub fn log_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "claude-usage-monitor", "ClaudeUsageMonitor")
        .map(|p| p.data_local_dir().join("logs"))
        .unwrap_or_else(|| PathBuf::from(".claude-monitor/logs"))
}
```

- [ ] **Step 2: Wire logging into `src-tauri/src/lib.rs`**

Replace the file with:
```rust
mod logging;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _log_guard = logging::init(logging::log_dir());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Build and verify no regressions**

```bash
cd src-tauri && cargo build && cd ..
```

Expected: compiles; tracing appears under `[dependencies]` transitive tree.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(logging): add tracing with daily-rotated file appender"
```

---

## Phase 1 — Storage Foundation

### Task 1.1: SQLite schema + `Db` handle with file lock

**Files:**
- Create: `src-tauri/src/store/mod.rs`, `src-tauri/src/store/schema.sql`
- Modify: `src-tauri/src/lib.rs` (add `mod store;`)
- Test: `src-tauri/src/store/mod.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Write `src-tauri/src/store/schema.sql`**

```sql
-- v1 schema
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    display_name TEXT,
    last_seen_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS api_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    fetched_at INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts(id)
);
CREATE INDEX IF NOT EXISTS idx_snapshots_account_time
    ON api_snapshots(account_id, fetched_at DESC);

CREATE TABLE IF NOT EXISTS session_events (
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
    UNIQUE (source_file, source_line)
);
CREATE INDEX IF NOT EXISTS idx_events_ts ON session_events(ts DESC);
CREATE INDEX IF NOT EXISTS idx_events_project ON session_events(project);
CREATE INDEX IF NOT EXISTS idx_events_model ON session_events(model);

CREATE TABLE IF NOT EXISTS jsonl_cursors (
    file_path TEXT PRIMARY KEY,
    last_mtime_ns INTEGER NOT NULL,
    byte_offset INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS notification_state (
    account_id TEXT NOT NULL,
    bucket TEXT NOT NULL,
    threshold INTEGER NOT NULL,
    last_fired_at INTEGER NOT NULL,
    PRIMARY KEY (account_id, bucket, threshold)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

- [ ] **Step 2: Write `src-tauri/src/store/mod.rs` skeleton with lock**

```rust
use anyhow::{Context, Result};
use fs4::fs_std::FileExt;
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
            .try_lock_exclusive()
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

pub fn default_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "claude-usage-monitor", "ClaudeUsageMonitor")
        .map(|p| p.data_local_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".claude-monitor"))
}
```

- [ ] **Step 3: Write failing test for `Db::open` + second-instance detection**

Append to `src-tauri/src/store/mod.rs`:
```rust
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
```

- [ ] **Step 4: Wire `mod store;` into `src-tauri/src/lib.rs`**

Add near the top of `lib.rs`:
```rust
mod store;
```

- [ ] **Step 5: Run tests and confirm pass**

```bash
cd src-tauri && cargo test -p claude-usage-monitor store:: -- --nocapture && cd ..
```

Expected: both tests pass.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(store): sqlite schema + Db handle with exclusive file lock"
```

---

### Task 1.2: Typed query helpers

**Files:**
- Create: `src-tauri/src/store/queries.rs`
- Modify: `src-tauri/src/store/mod.rs` (add `pub mod queries;`)
- Test: `src-tauri/src/store/queries.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Define types shared with queries**

Append to `src-tauri/src/store/mod.rs`:
```rust
pub mod queries;

pub use queries::*;
```

- [ ] **Step 2: Write `src-tauri/src/store/queries.rs`**

```rust
use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAccount {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSessionEvent {
    pub ts: DateTime<Utc>,
    pub project: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_5m_tokens: u64,
    pub cache_creation_1h_tokens: u64,
    pub cost_usd: f64,
    pub source_file: String,
    pub source_line: i64,
}

impl Db {
    pub fn upsert_account(&self, acc: &StoredAccount) -> Result<()> {
        let now = Utc::now().timestamp();
        self.conn().execute(
            "INSERT INTO accounts (id, email, display_name, last_seen_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET email=excluded.email,
                                            display_name=excluded.display_name,
                                            last_seen_at=excluded.last_seen_at",
            params![acc.id, acc.email, acc.display_name, now],
        )?;
        Ok(())
    }

    pub fn insert_snapshot(&self, account_id: &str, fetched_at: DateTime<Utc>, payload_json: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO api_snapshots (account_id, fetched_at, payload_json) VALUES (?1, ?2, ?3)",
            params![account_id, fetched_at.timestamp(), payload_json],
        )?;
        Ok(())
    }

    pub fn latest_snapshot(&self, account_id: &str) -> Result<Option<(DateTime<Utc>, String)>> {
        let conn = self.conn();
        let row = conn
            .query_row(
                "SELECT fetched_at, payload_json FROM api_snapshots
                 WHERE account_id = ?1 ORDER BY fetched_at DESC LIMIT 1",
                params![account_id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        Ok(row.map(|(ts, p)| (DateTime::from_timestamp(ts, 0).unwrap(), p)))
    }

    pub fn insert_events(&self, events: &[StoredSessionEvent]) -> Result<usize> {
        if events.is_empty() { return Ok(0); }
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let mut inserted = 0;
        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO session_events
                (ts, project, model, input_tokens, output_tokens, cache_read_tokens,
                 cache_creation_5m_tokens, cache_creation_1h_tokens, cost_usd, source_file, source_line)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for e in events {
                let n = stmt.execute(params![
                    e.ts.timestamp(), e.project, e.model,
                    e.input_tokens as i64, e.output_tokens as i64, e.cache_read_tokens as i64,
                    e.cache_creation_5m_tokens as i64, e.cache_creation_1h_tokens as i64,
                    e.cost_usd, e.source_file, e.source_line
                ])?;
                inserted += n;
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    pub fn events_between(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<StoredSessionEvent>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT ts, project, model, input_tokens, output_tokens, cache_read_tokens,
                    cache_creation_5m_tokens, cache_creation_1h_tokens, cost_usd, source_file, source_line
             FROM session_events WHERE ts BETWEEN ?1 AND ?2 ORDER BY ts DESC",
        )?;
        let rows = stmt.query_map(params![from.timestamp(), to.timestamp()], |r| {
            Ok(StoredSessionEvent {
                ts: DateTime::from_timestamp(r.get(0)?, 0).unwrap(),
                project: r.get(1)?,
                model: r.get(2)?,
                input_tokens: r.get::<_, i64>(3)? as u64,
                output_tokens: r.get::<_, i64>(4)? as u64,
                cache_read_tokens: r.get::<_, i64>(5)? as u64,
                cache_creation_5m_tokens: r.get::<_, i64>(6)? as u64,
                cache_creation_1h_tokens: r.get::<_, i64>(7)? as u64,
                cost_usd: r.get(8)?,
                source_file: r.get(9)?,
                source_line: r.get(10)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn prune_events_older_than(&self, cutoff: DateTime<Utc>) -> Result<usize> {
        let rows = self
            .conn()
            .execute("DELETE FROM session_events WHERE ts < ?1", params![cutoff.timestamp()])?;
        Ok(rows)
    }

    pub fn get_cursor(&self, file: &str) -> Result<Option<(i64, i64)>> {
        let conn = self.conn();
        let row = conn
            .query_row(
                "SELECT last_mtime_ns, byte_offset FROM jsonl_cursors WHERE file_path = ?1",
                params![file],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
            )
            .optional()?;
        Ok(row)
    }

    pub fn set_cursor(&self, file: &str, mtime_ns: i64, offset: i64) -> Result<()> {
        self.conn().execute(
            "INSERT INTO jsonl_cursors (file_path, last_mtime_ns, byte_offset) VALUES (?1, ?2, ?3)
             ON CONFLICT(file_path) DO UPDATE SET last_mtime_ns=excluded.last_mtime_ns, byte_offset=excluded.byte_offset",
            params![file, mtime_ns, offset],
        )?;
        Ok(())
    }

    pub fn notification_last_fired(&self, account_id: &str, bucket: &str, threshold: i64) -> Result<Option<DateTime<Utc>>> {
        let conn = self.conn();
        let row = conn
            .query_row(
                "SELECT last_fired_at FROM notification_state
                 WHERE account_id = ?1 AND bucket = ?2 AND threshold = ?3",
                params![account_id, bucket, threshold],
                |r| r.get::<_, i64>(0),
            )
            .optional()?;
        Ok(row.map(|ts| DateTime::from_timestamp(ts, 0).unwrap()))
    }

    pub fn record_notification_fired(&self, account_id: &str, bucket: &str, threshold: i64, at: DateTime<Utc>) -> Result<()> {
        self.conn().execute(
            "INSERT INTO notification_state (account_id, bucket, threshold, last_fired_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(account_id, bucket, threshold) DO UPDATE SET last_fired_at=excluded.last_fired_at",
            params![account_id, bucket, threshold, at.timestamp()],
        )?;
        Ok(())
    }
}

use rusqlite::OptionalExtension;
```

- [ ] **Step 3: Write failing tests**

Append to `src-tauri/src/store/queries.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Db;
    use tempfile::tempdir;

    fn fresh_db() -> (tempfile::TempDir, Db) {
        let dir = tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        db.upsert_account(&StoredAccount {
            id: "acc1".into(),
            email: "a@example.com".into(),
            display_name: None,
        }).unwrap();
        (dir, db)
    }

    #[test]
    fn snapshot_roundtrip() {
        let (_dir, db) = fresh_db();
        let now = Utc::now();
        db.insert_snapshot("acc1", now, r#"{"five_hour":null}"#).unwrap();
        let (ts, payload) = db.latest_snapshot("acc1").unwrap().expect("snapshot");
        assert_eq!(ts.timestamp(), now.timestamp());
        assert!(payload.contains("five_hour"));
    }

    #[test]
    fn events_insert_and_dedupe() {
        let (_dir, db) = fresh_db();
        let e = StoredSessionEvent {
            ts: Utc::now(),
            project: "p".into(), model: "sonnet-4-6".into(),
            input_tokens: 10, output_tokens: 5, cache_read_tokens: 0,
            cache_creation_5m_tokens: 0, cache_creation_1h_tokens: 0,
            cost_usd: 0.001, source_file: "f.jsonl".into(), source_line: 1,
        };
        assert_eq!(db.insert_events(&[e.clone()]).unwrap(), 1);
        assert_eq!(db.insert_events(&[e.clone()]).unwrap(), 0, "dedupe");
    }

    #[test]
    fn cursor_roundtrip() {
        let (_dir, db) = fresh_db();
        assert!(db.get_cursor("f.jsonl").unwrap().is_none());
        db.set_cursor("f.jsonl", 123, 456).unwrap();
        assert_eq!(db.get_cursor("f.jsonl").unwrap(), Some((123, 456)));
    }

    #[test]
    fn notification_state_roundtrip() {
        let (_dir, db) = fresh_db();
        assert!(db.notification_last_fired("acc1", "five_hour", 75).unwrap().is_none());
        let now = Utc::now();
        db.record_notification_fired("acc1", "five_hour", 75, now).unwrap();
        let got = db.notification_last_fired("acc1", "five_hour", 75).unwrap().unwrap();
        assert_eq!(got.timestamp(), now.timestamp());
    }

    #[test]
    fn prune_removes_old_events() {
        let (_dir, db) = fresh_db();
        let old = Utc::now() - chrono::Duration::days(200);
        let recent = Utc::now();
        let mk = |ts, line| StoredSessionEvent {
            ts, project: "p".into(), model: "sonnet-4-6".into(),
            input_tokens: 0, output_tokens: 0, cache_read_tokens: 0,
            cache_creation_5m_tokens: 0, cache_creation_1h_tokens: 0,
            cost_usd: 0.0, source_file: "f.jsonl".into(), source_line: line,
        };
        db.insert_events(&[mk(old, 1), mk(recent, 2)]).unwrap();
        let cutoff = Utc::now() - chrono::Duration::days(90);
        assert_eq!(db.prune_events_older_than(cutoff).unwrap(), 1);
    }
}
```

- [ ] **Step 4: Run and verify**

```bash
cd src-tauri && cargo test store::queries:: && cd ..
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(store): typed query helpers for snapshots, events, cursors, notifications"
```

---

## Phase 2 — Usage API

### Task 2.1: Wire-faithful types + committed fixtures + serde round-trip tests

**Files:**
- Create: `src-tauri/src/usage_api/mod.rs`, `src-tauri/src/usage_api/types.rs`
- Create: `src-tauri/tests/fixtures/api_responses/standard_account.json`, `extra_usage_enabled.json`, `newer_schema_with_extra_fields.json`
- Modify: `src-tauri/src/lib.rs` (add `mod usage_api;`)

- [ ] **Step 1: Write `src-tauri/src/usage_api/types.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Utilization {
    pub utilization: f64,                // 0..100
    pub resets_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtraUsage {
    pub is_enabled: bool,
    #[serde(default)] pub monthly_limit_cents: u64,
    #[serde(default)] pub used_credits_cents: u64,
    #[serde(default)] pub utilization: f64,
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageSnapshot {
    pub five_hour: Option<Utilization>,
    pub seven_day: Option<Utilization>,
    pub seven_day_sonnet: Option<Utilization>,
    pub seven_day_opus: Option<Utilization>,
    pub extra_usage: Option<ExtraUsage>,

    #[serde(default = "Utc::now", skip_serializing)]
    pub fetched_at: DateTime<Utc>,

    #[serde(flatten, default)]
    pub unknown: HashMap<String, serde_json::Value>,
}
```

- [ ] **Step 2: Write `src-tauri/src/usage_api/mod.rs` skeleton**

```rust
pub mod types;

pub use types::{ExtraUsage, UsageSnapshot, Utilization};
```

- [ ] **Step 3: Write `src-tauri/src/lib.rs` update**

Add `mod usage_api;` after `mod store;`.

- [ ] **Step 4: Create `src-tauri/tests/fixtures/api_responses/standard_account.json`**

```json
{
  "five_hour": { "utilization": 42.5, "resets_at": "2026-04-24T18:00:00Z" },
  "seven_day": { "utilization": 63.1, "resets_at": "2026-04-30T09:00:00Z" },
  "seven_day_sonnet": { "utilization": 58.0, "resets_at": "2026-04-30T09:00:00Z" },
  "seven_day_opus": { "utilization": 12.3, "resets_at": "2026-04-30T09:00:00Z" },
  "extra_usage": null
}
```

- [ ] **Step 5: Create `src-tauri/tests/fixtures/api_responses/extra_usage_enabled.json`**

```json
{
  "five_hour": { "utilization": 71.2, "resets_at": "2026-04-24T20:00:00Z" },
  "seven_day": { "utilization": 85.4, "resets_at": "2026-04-30T09:00:00Z" },
  "seven_day_sonnet": { "utilization": 80.0, "resets_at": "2026-04-30T09:00:00Z" },
  "seven_day_opus": { "utilization": 40.1, "resets_at": "2026-04-30T09:00:00Z" },
  "extra_usage": {
    "is_enabled": true,
    "monthly_limit_cents": 5000,
    "used_credits_cents": 1275,
    "utilization": 25.5,
    "resets_at": "2026-05-01T00:00:00Z"
  }
}
```

- [ ] **Step 6: Create `src-tauri/tests/fixtures/api_responses/newer_schema_with_extra_fields.json`**

```json
{
  "five_hour": { "utilization": 10.0, "resets_at": "2026-04-24T18:00:00Z", "pinned": false },
  "seven_day": { "utilization": 20.0, "resets_at": "2026-04-30T09:00:00Z" },
  "seven_day_sonnet": null,
  "seven_day_opus": null,
  "extra_usage": null,
  "future_field_we_do_not_know": { "nested": "value" },
  "organization_plan": "team-pro"
}
```

- [ ] **Step 7: Write round-trip tests as `src-tauri/tests/usage_api_types.rs`**

```rust
use claude_usage_monitor_lib::usage_api::types::UsageSnapshot;

#[test]
fn standard_account_round_trips() {
    let raw = include_str!("fixtures/api_responses/standard_account.json");
    let snap: UsageSnapshot = serde_json::from_str(raw).expect("parse");
    assert!(snap.five_hour.is_some());
    assert!(snap.extra_usage.is_none());
    let back = serde_json::to_string(&snap).unwrap();
    let reparsed: UsageSnapshot = serde_json::from_str(&back).unwrap();
    assert_eq!(snap.five_hour, reparsed.five_hour);
}

#[test]
fn extra_usage_enabled_parses() {
    let raw = include_str!("fixtures/api_responses/extra_usage_enabled.json");
    let snap: UsageSnapshot = serde_json::from_str(raw).expect("parse");
    let eu = snap.extra_usage.expect("extra_usage");
    assert!(eu.is_enabled);
    assert_eq!(eu.monthly_limit_cents, 5000);
    assert_eq!(eu.used_credits_cents, 1275);
    assert!(eu.resets_at.is_some());
}

#[test]
fn unknown_fields_are_preserved_not_errors() {
    let raw = include_str!("fixtures/api_responses/newer_schema_with_extra_fields.json");
    let snap: UsageSnapshot = serde_json::from_str(raw).expect("parse forward-compat");
    assert!(snap.unknown.contains_key("future_field_we_do_not_know"));
    assert!(snap.unknown.contains_key("organization_plan"));
}
```

- [ ] **Step 8: Expose module from lib to tests**

In `src-tauri/src/lib.rs` make `usage_api` public: `pub mod usage_api;`.

- [ ] **Step 9: Run tests and verify**

```bash
cd src-tauri && cargo test --test usage_api_types && cd ..
```

Expected: 3 passed.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat(usage_api): wire-faithful types with committed fixture round-trip tests"
```

---

### Task 2.2: HTTP client with headers, backoff, timeouts

**Files:**
- Create: `src-tauri/src/usage_api/client.rs`
- Modify: `src-tauri/src/usage_api/mod.rs`

- [ ] **Step 1: Write `src-tauri/src/usage_api/client.rs`**

```rust
use super::types::UsageSnapshot;
use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::{Client, StatusCode};
use std::time::Duration;

pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
pub const ANTHROPIC_BETA: &str = "oauth-2025-04-20";

#[derive(Debug)]
pub enum FetchOutcome {
    Ok(UsageSnapshot),
    Unauthorized,         // 401 → caller refreshes token
    RateLimited,          // 429 → caller backs off
    Transient(String),    // 5xx / timeout / network
}

pub struct UsageClient {
    base_url: String,
    inner: Client,
    app_version: String,
}

impl UsageClient {
    pub fn new(app_version: String) -> Result<Self> {
        let inner = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self { base_url: USAGE_URL.to_string(), inner, app_version })
    }

    #[cfg(test)]
    pub fn with_base_url(base_url: String, app_version: String) -> Result<Self> {
        let inner = Client::builder().timeout(Duration::from_secs(30)).build()?;
        Ok(Self { base_url, inner, app_version })
    }

    pub async fn fetch(&self, access_token: &str) -> FetchOutcome {
        let req = self
            .inner
            .get(&self.base_url)
            .bearer_auth(access_token)
            .header("anthropic-beta", ANTHROPIC_BETA)
            .header("User-Agent", format!("claude-usage-monitor/{}", self.app_version));

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) if e.is_timeout() => return FetchOutcome::Transient("timeout".into()),
            Err(e) => return FetchOutcome::Transient(e.to_string()),
        };

        match resp.status() {
            StatusCode::OK => match resp.json::<UsageSnapshot>().await {
                Ok(mut s) => {
                    s.fetched_at = Utc::now();
                    FetchOutcome::Ok(s)
                }
                Err(e) => FetchOutcome::Transient(format!("decode: {e}")),
            },
            StatusCode::UNAUTHORIZED => FetchOutcome::Unauthorized,
            StatusCode::TOO_MANY_REQUESTS => FetchOutcome::RateLimited,
            s if s.is_server_error() => FetchOutcome::Transient(format!("status: {s}")),
            other => FetchOutcome::Transient(format!("unexpected status: {other}")),
        }
    }
}

/// Exponential backoff ladder: 1m, 2m, 4m, 8m, 16m, 30m (cap).
pub fn next_backoff(previous: Duration) -> Duration {
    let doubled = previous.saturating_mul(2);
    let cap = Duration::from_secs(30 * 60);
    if doubled > cap { cap } else { doubled.max(Duration::from_secs(60)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_ladder() {
        let mut d = Duration::from_secs(60);
        d = next_backoff(d); assert_eq!(d, Duration::from_secs(120));
        d = next_backoff(d); assert_eq!(d, Duration::from_secs(240));
        d = next_backoff(d); assert_eq!(d, Duration::from_secs(480));
        d = next_backoff(d); assert_eq!(d, Duration::from_secs(960));
        d = next_backoff(d); assert_eq!(d, Duration::from_secs(1800));
        d = next_backoff(d); assert_eq!(d, Duration::from_secs(1800)); // cap
    }
}
```

- [ ] **Step 2: Export client from `src-tauri/src/usage_api/mod.rs`**

```rust
pub mod client;
pub mod types;

pub use client::{FetchOutcome, UsageClient, next_backoff};
pub use types::{ExtraUsage, UsageSnapshot, Utilization};
```

- [ ] **Step 3: Integration test against mock server**

Create `src-tauri/tests/usage_api_client.rs`:
```rust
use claude_usage_monitor_lib::usage_api::{FetchOutcome, UsageClient};
use mockito::Server;

#[tokio::test]
async fn handles_200_response() {
    let mut server = Server::new_async().await;
    let body = include_str!("fixtures/api_responses/standard_account.json");
    let _m = server
        .mock("GET", "/")
        .match_header("authorization", "Bearer tok")
        .match_header("anthropic-beta", "oauth-2025-04-20")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let c = UsageClient::with_base_url(server.url(), "0.0.0-test".into()).unwrap();
    match c.fetch("tok").await {
        FetchOutcome::Ok(snap) => assert!(snap.five_hour.is_some()),
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[tokio::test]
async fn handles_401() {
    let mut server = Server::new_async().await;
    let _m = server.mock("GET", "/").with_status(401).create_async().await;
    let c = UsageClient::with_base_url(server.url(), "0.0.0-test".into()).unwrap();
    assert!(matches!(c.fetch("tok").await, FetchOutcome::Unauthorized));
}

#[tokio::test]
async fn handles_429() {
    let mut server = Server::new_async().await;
    let _m = server.mock("GET", "/").with_status(429).create_async().await;
    let c = UsageClient::with_base_url(server.url(), "0.0.0-test".into()).unwrap();
    assert!(matches!(c.fetch("tok").await, FetchOutcome::RateLimited));
}

#[tokio::test]
async fn handles_5xx_as_transient() {
    let mut server = Server::new_async().await;
    let _m = server.mock("GET", "/").with_status(503).create_async().await;
    let c = UsageClient::with_base_url(server.url(), "0.0.0-test".into()).unwrap();
    assert!(matches!(c.fetch("tok").await, FetchOutcome::Transient(_)));
}
```

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test usage_api && cd ..
```

Expected: unit backoff test + 4 integration tests pass.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(usage_api): http client with required headers and backoff ladder"
```

---

## Phase 3 — Auth Module

### Task 3.1: PKCE generation + authorize URL builder

**Files:**
- Create: `src-tauri/src/auth/mod.rs`, `src-tauri/src/auth/oauth_paste_back.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod auth;`)

- [ ] **Step 1: Write `src-tauri/src/auth/mod.rs` skeleton**

```rust
pub mod oauth_paste_back;
pub mod token_store;
pub mod claude_code_creds;
pub mod account_identity;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthSource { OAuth, ClaudeCode }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AccountId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Write `src-tauri/src/auth/oauth_paste_back.rs`**

```rust
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use url::Url;

pub const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub const AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
pub const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
pub const REDIRECT_URI: &str = "https://platform.claude.com/oauth/code/callback";
pub const SCOPES: &str = "user:profile user:inference";

#[derive(Debug, Clone)]
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
    pub state: String,
}

// `Clone` is required by AppState::pending_oauth reads in commands::submit_oauth_code.

pub fn generate_pkce() -> PkcePair {
    let mut verifier_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut verifier_bytes);
    let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

    let challenge_bytes = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(challenge_bytes);

    let mut state_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut state_bytes);
    let state = URL_SAFE_NO_PAD.encode(state_bytes);

    PkcePair { verifier, challenge, state }
}

pub fn build_authorize_url(pkce: &PkcePair) -> Result<String> {
    let mut url = Url::parse(AUTHORIZE_URL)?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", CLIENT_ID)
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("scope", SCOPES)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &pkce.state)
        .append_pair("code", "true");
    Ok(url.into())
}

/// Parses "code#state" as rendered on Anthropic's callback page.
pub fn parse_pasted_code(pasted: &str, expected_state: &str) -> Result<String> {
    let trimmed = pasted.trim();
    let (code, state) = trimmed
        .split_once('#')
        .ok_or_else(|| anyhow!("Missing '#state' suffix"))?;
    if state != expected_state {
        return Err(anyhow!("State mismatch: possible replay or mis-paste"));
    }
    if code.is_empty() {
        return Err(anyhow!("Code is empty"));
    }
    Ok(code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_verifier_and_challenge_are_distinct() {
        let p = generate_pkce();
        assert_ne!(p.verifier, p.challenge);
        assert!(p.state.len() >= 16);
    }

    #[test]
    fn authorize_url_contains_expected_params() {
        let p = generate_pkce();
        let url = build_authorize_url(&p).unwrap();
        assert!(url.contains("client_id=9d1c250a-e61b-44d9-88ed-5944d1962f5e"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("code=true"));
        assert!(url.contains(&format!("state={}", p.state)));
    }

    #[test]
    fn parse_rejects_missing_hash() {
        let err = parse_pasted_code("abcd", "st1").unwrap_err();
        assert!(err.to_string().contains("Missing"));
    }

    #[test]
    fn parse_rejects_state_mismatch() {
        let err = parse_pasted_code("code#bad", "st1").unwrap_err();
        assert!(err.to_string().contains("State"));
    }

    #[test]
    fn parse_accepts_valid_pasted_code() {
        let code = parse_pasted_code("abc123#st1", "st1").unwrap();
        assert_eq!(code, "abc123");
    }
}
```

- [ ] **Step 3: Add empty stubs for the sibling modules so `mod.rs` compiles**

Create empty files:
`src-tauri/src/auth/token_store.rs`:
```rust
// Implemented in Task 3.2.
```

`src-tauri/src/auth/claude_code_creds.rs`:
```rust
// Dispatcher — implemented in Tasks 3.3 and 3.4.
```

`src-tauri/src/auth/account_identity.rs`:
```rust
// Implemented in Task 3.5.
```

- [ ] **Step 4: Wire `pub mod auth;` into `src-tauri/src/lib.rs`**

Add after `pub mod usage_api;`.

- [ ] **Step 5: Run tests**

```bash
cd src-tauri && cargo test auth::oauth_paste_back && cd ..
```

Expected: 5 passed.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(auth): PKCE generation, authorize URL, paste-back code parsing"
```

---

### Task 3.2: Token store (keyring primary, DACL file fallback) + token exchange

**Files:**
- Create: `src-tauri/src/auth/token_store.rs` (rewrite)
- Create: `src-tauri/src/auth/exchange.rs`
- Modify: `src-tauri/src/auth/mod.rs`

- [ ] **Step 1: Write `src-tauri/src/auth/token_store.rs`**

```rust
use super::StoredToken;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const KEYRING_SERVICE: &str = "claude-usage-monitor";
const KEYRING_USER: &str = "oauth_refresh";

#[derive(Serialize, Deserialize)]
struct FallbackPayload {
    token: StoredToken,
}

pub fn save(token: &StoredToken, fallback_dir: &Path) -> Result<()> {
    let payload = serde_json::to_string(token)?;
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => match entry.set_password(&payload) {
            Ok(_) => {
                let _ = fs::remove_file(fallback_path(fallback_dir));
                Ok(())
            }
            Err(e) => {
                tracing::warn!("keyring save failed ({e}); falling back to restricted file");
                save_fallback(token, fallback_dir)
            }
        },
        Err(e) => {
            tracing::warn!("keyring unavailable ({e}); using restricted file");
            save_fallback(token, fallback_dir)
        }
    }
}

pub fn load(fallback_dir: &Path) -> Result<Option<StoredToken>> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if let Ok(s) = entry.get_password() {
            if let Ok(t) = serde_json::from_str::<StoredToken>(&s) {
                return Ok(Some(t));
            }
        }
    }
    load_fallback(fallback_dir)
}

pub fn clear(fallback_dir: &Path) -> Result<()> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.delete_credential();
    }
    let p = fallback_path(fallback_dir);
    let _ = fs::remove_file(p);
    Ok(())
}

fn fallback_path(dir: &Path) -> PathBuf { dir.join("credentials.json") }

fn save_fallback(token: &StoredToken, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let p = fallback_path(dir);
    let payload = serde_json::to_string_pretty(token)?;
    fs::write(&p, payload).context("write fallback credential file")?;
    restrict_permissions(&p)?;
    Ok(())
}

fn load_fallback(dir: &Path) -> Result<Option<StoredToken>> {
    let p = fallback_path(dir);
    if !p.exists() { return Ok(None); }
    let s = fs::read_to_string(&p)?;
    Ok(serde_json::from_str(&s).ok())
}

#[cfg(unix)]
fn restrict_permissions(p: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(p)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(p, perms)?;
    Ok(())
}

#[cfg(windows)]
fn restrict_permissions(p: &Path) -> Result<()> {
    // Invoke icacls to break inheritance and grant current user full control only.
    // This mirrors what keyring's file-backend would set.
    use std::process::Command;
    let status = Command::new("icacls")
        .arg(p)
        .args(["/inheritance:r", "/grant:r", &format!("{}:F", whoami::username())])
        .status()
        .context("icacls failed to run")?;
    if !status.success() {
        anyhow::bail!("icacls returned non-zero");
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn restrict_permissions(_: &Path) -> Result<()> { Ok(()) }
```

Add `whoami = "1"` to `[target.'cfg(windows)'.dependencies]` in `src-tauri/Cargo.toml`.

- [ ] **Step 2: Write `src-tauri/src/auth/exchange.rs`**

```rust
use super::oauth_paste_back::{CLIENT_ID, REDIRECT_URI, TOKEN_URL};
use super::StoredToken;
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    #[allow(dead_code)] token_type: Option<String>,
}

pub struct TokenExchange {
    endpoint: String,
    client: reqwest::Client,
}

impl TokenExchange {
    pub fn new() -> Self {
        Self {
            endpoint: TOKEN_URL.to_string(),
            client: reqwest::Client::new(),
        }
    }

    #[cfg(test)]
    pub fn with_endpoint(endpoint: String) -> Self {
        Self { endpoint, client: reqwest::Client::new() }
    }

    pub async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> Result<StoredToken> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", CLIENT_ID),
            ("code_verifier", pkce_verifier),
        ];
        let resp = self.client.post(&self.endpoint).form(&params).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("token exchange failed: {status}: {text}"));
        }
        let tr: TokenResponse = resp.json().await?;
        Ok(StoredToken {
            access_token: tr.access_token,
            refresh_token: tr.refresh_token,
            expires_at: Utc::now() + Duration::seconds(tr.expires_in),
        })
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<StoredToken> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", CLIENT_ID),
        ];
        let resp = self.client.post(&self.endpoint).form(&params).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("refresh failed: {status}: {text}"));
        }
        let tr: TokenResponse = resp.json().await?;
        Ok(StoredToken {
            access_token: tr.access_token,
            refresh_token: tr.refresh_token.or_else(|| Some(refresh_token.to_string())),
            expires_at: Utc::now() + Duration::seconds(tr.expires_in),
        })
    }
}
```

- [ ] **Step 3: Wire exchange module**

In `src-tauri/src/auth/mod.rs` add `pub mod exchange;`.

- [ ] **Step 4: Write integration tests `src-tauri/tests/auth_exchange.rs`**

```rust
use claude_usage_monitor_lib::auth::exchange::TokenExchange;
use mockito::Server;

#[tokio::test]
async fn successful_code_exchange() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("grant_type".into(), "authorization_code".into()),
            mockito::Matcher::UrlEncoded("code".into(), "abc".into()),
            mockito::Matcher::UrlEncoded("code_verifier".into(), "verif".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"acc","refresh_token":"ref","expires_in":3600,"token_type":"Bearer"}"#)
        .create_async().await;

    let ex = TokenExchange::with_endpoint(server.url());
    let tok = ex.exchange_code("abc", "verif").await.unwrap();
    assert_eq!(tok.access_token, "acc");
    assert_eq!(tok.refresh_token.as_deref(), Some("ref"));
}

#[tokio::test]
async fn exchange_error_body_surfaces() {
    let mut server = Server::new_async().await;
    let _m = server.mock("POST", "/").with_status(400).with_body("bad_code").create_async().await;
    let ex = TokenExchange::with_endpoint(server.url());
    let err = ex.exchange_code("abc", "verif").await.unwrap_err();
    assert!(err.to_string().contains("bad_code"));
}

#[tokio::test]
async fn refresh_preserves_refresh_token_when_not_returned() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"new","expires_in":3600}"#)
        .create_async().await;
    let ex = TokenExchange::with_endpoint(server.url());
    let tok = ex.refresh("old-refresh").await.unwrap();
    assert_eq!(tok.refresh_token.as_deref(), Some("old-refresh"));
}
```

- [ ] **Step 5: Run and verify**

```bash
cd src-tauri && cargo test --test auth_exchange && cd ..
```

Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(auth): token store (keyring + DACL fallback) + OAuth token exchange"
```

---

### Task 3.3: Claude Code creds reader — macOS

**Files:**
- Delete stub file: `src-tauri/src/auth/claude_code_creds.rs` (from Task 3.1)
- Create: `src-tauri/src/auth/claude_code_creds/mod.rs`
- Create: `src-tauri/src/auth/claude_code_creds/macos.rs`

- [ ] **Step 1: Convert the single stub file into a module directory**

```bash
rm src-tauri/src/auth/claude_code_creds.rs
mkdir -p src-tauri/src/auth/claude_code_creds
```

- [ ] **Step 2: Write `src-tauri/src/auth/claude_code_creds/mod.rs` (dispatcher)**

Uses cfg-gated `return` statements — on any given OS, exactly one branch is compiled in, so the function has a single expression-free return path.

```rust
use super::StoredToken;
use anyhow::Result;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

pub fn load() -> Result<Option<StoredToken>> {
    #[cfg(target_os = "macos")]
    return macos::load();
    #[cfg(target_os = "windows")]
    return windows::load();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Ok(None);
}

pub fn has_creds() -> bool {
    #[cfg(target_os = "macos")]
    return macos::has_creds();
    #[cfg(target_os = "windows")]
    return windows::has_creds();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return false;
}
```

- [ ] **Step 3: Write `src-tauri/src/auth/claude_code_creds/macos.rs`**

```rust
#![cfg(target_os = "macos")]

use super::super::StoredToken;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::process::Command;

const SERVICE_PREFIX: &str = "Claude Code-credentials";

#[derive(Deserialize)]
struct RawCreds {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: OauthBlock,
}

#[derive(Deserialize)]
struct OauthBlock {
    #[serde(rename = "accessToken")] access_token: String,
    #[serde(rename = "refreshToken")] refresh_token: Option<String>,
    #[serde(rename = "expiresAt")] expires_at_ms: i64,
}

pub fn load() -> Result<Option<StoredToken>> {
    let services = discover_services()?;
    let mut candidates = Vec::new();
    for svc in services {
        if let Ok(Some(tok)) = read_one(&svc) {
            candidates.push(tok);
        }
    }
    // Prefer the credential with the longest remaining TTL.
    candidates.sort_by_key(|t| t.expires_at);
    Ok(candidates.pop())
}

fn discover_services() -> Result<Vec<String>> {
    let output = Command::new("security").arg("dump-keychain").output();
    let stdout = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Ok(vec![SERVICE_PREFIX.to_string()]),
    };
    let mut services = Vec::new();
    for line in stdout.lines() {
        if let Some(idx) = line.find("\"svce\"<blob>=\"") {
            let rest = &line[idx + 14..];
            if let Some(end) = rest.find('"') {
                let name = &rest[..end];
                if name.starts_with(SERVICE_PREFIX) && !services.contains(&name.to_string()) {
                    services.push(name.to_string());
                }
            }
        }
    }
    if services.is_empty() {
        services.push(SERVICE_PREFIX.to_string());
    }
    Ok(services)
}

fn read_one(service: &str) -> Result<Option<StoredToken>> {
    let out = Command::new("security")
        .args(["find-generic-password", "-s", service, "-w"])
        .output()
        .context("spawn security find-generic-password")?;
    if !out.status.success() { return Ok(None); }

    let mut bytes = out.stdout;
    if let Some(&last) = bytes.last() { if last == b'\n' { bytes.pop(); } }

    // Trim leading non-ASCII byte occasionally prepended by the keychain.
    if !bytes.is_empty() && bytes[0] > 0x7F { bytes.remove(0); }

    let text = String::from_utf8(bytes).context("keychain payload not utf-8")?;
    let raw: RawCreds = match serde_json::from_str(&text) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    let exp = Utc
        .timestamp_millis_opt(raw.claude_ai_oauth.expires_at_ms)
        .single()
        .ok_or_else(|| anyhow!("invalid expires_at_ms"))?;
    Ok(Some(StoredToken {
        access_token: raw.claude_ai_oauth.access_token,
        refresh_token: raw.claude_ai_oauth.refresh_token,
        expires_at: exp,
    }))
}

pub fn has_creds() -> bool {
    Command::new("security")
        .args(["find-generic-password", "-s", SERVICE_PREFIX])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn parse_sample_payload() {
        // Simulate the exact shape Claude Code writes to keychain.
        let sample = r#"{"claudeAiOauth":{"accessToken":"a","refreshToken":"r","expiresAt":1840000000000}}"#;
        let raw: RawCreds = serde_json::from_str(sample).unwrap();
        assert_eq!(raw.claude_ai_oauth.access_token, "a");
        assert_eq!(raw.claude_ai_oauth.refresh_token.as_deref(), Some("r"));
        let expected = Utc.timestamp_millis_opt(1_840_000_000_000).single().unwrap();
        assert!(expected > Utc::now() - Duration::days(365 * 100));
    }
}
```

- [ ] **Step 4: Run macOS-gated tests**

```bash
cd src-tauri && cargo test --lib auth::claude_code_creds:: && cd ..
```

Expected on macOS: test passes. On Ubuntu/Windows: the macOS module is compiled out and no tests run under that path.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(auth): macOS Claude Code credential reader with multi-service discovery"
```

---

### Task 3.4: Claude Code creds reader — Windows

**Files:**
- Create: `src-tauri/src/auth/claude_code_creds/windows.rs`

- [ ] **Step 1: Write `src-tauri/src/auth/claude_code_creds/windows.rs`**

```rust
#![cfg(target_os = "windows")]

use super::super::StoredToken;
use anyhow::{anyhow, Context, Result};
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct RawCreds {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: OauthBlock,
}

#[derive(Deserialize)]
struct OauthBlock {
    #[serde(rename = "accessToken")] access_token: String,
    #[serde(rename = "refreshToken")] refresh_token: Option<String>,
    #[serde(rename = "expiresAt")] expires_at_ms: i64,
}

fn credentials_path() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE")?;
    Some(PathBuf::from(home).join(".claude").join(".credentials.json"))
}

pub fn load() -> Result<Option<StoredToken>> {
    let p = match credentials_path() {
        Some(p) => p,
        None => return Ok(None),
    };
    if !p.exists() { return Ok(None); }
    let text = std::fs::read_to_string(&p).context("read .credentials.json")?;
    let raw: RawCreds = match serde_json::from_str(&text) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    let exp = Utc
        .timestamp_millis_opt(raw.claude_ai_oauth.expires_at_ms)
        .single()
        .ok_or_else(|| anyhow!("invalid expires_at_ms"))?;
    Ok(Some(StoredToken {
        access_token: raw.claude_ai_oauth.access_token,
        refresh_token: raw.claude_ai_oauth.refresh_token,
        expires_at: exp,
    }))
}

pub fn has_creds() -> bool {
    credentials_path().map(|p| p.exists()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_realistic_payload_from_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".credentials.json");
        fs::write(&path, r#"{"claudeAiOauth":{"accessToken":"a","refreshToken":"r","expiresAt":1840000000000}}"#).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let raw: RawCreds = serde_json::from_str(&text).unwrap();
        assert_eq!(raw.claude_ai_oauth.access_token, "a");
    }
}
```

- [ ] **Step 2: Run Windows-gated tests (no-op on other OSes)**

```bash
cd src-tauri && cargo test --lib auth::claude_code_creds:: && cd ..
```

Expected on Windows: test passes. On macOS/Linux: test is compiled out.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(auth): Windows Claude Code credential reader (plaintext file)"
```

---

### Task 3.5: Account identity + conflict resolution

**Files:**
- Create: `src-tauri/src/auth/account_identity.rs` (rewrite)
- Create: `src-tauri/src/auth/orchestrator.rs`
- Modify: `src-tauri/src/auth/mod.rs`

- [ ] **Step 1: Write `src-tauri/src/auth/account_identity.rs`**

```rust
use super::AccountId;
use anyhow::{anyhow, Result};
use serde::Deserialize;

pub const USERINFO_URL: &str = "https://api.anthropic.com/api/oauth/userinfo";

#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    #[serde(rename = "sub")] pub id: String,
    pub email: String,
    pub name: Option<String>,
}

pub struct IdentityFetcher {
    endpoint: String,
    client: reqwest::Client,
}

impl IdentityFetcher {
    pub fn new() -> Self {
        Self {
            endpoint: USERINFO_URL.to_string(),
            client: reqwest::Client::new(),
        }
    }

    #[cfg(test)]
    pub fn with_endpoint(endpoint: String) -> Self {
        Self { endpoint, client: reqwest::Client::new() }
    }

    pub async fn fetch(&self, access_token: &str) -> Result<UserInfo> {
        let resp = self.client
            .get(&self.endpoint)
            .bearer_auth(access_token)
            .header("anthropic-beta", crate::usage_api::client::ANTHROPIC_BETA)
            .send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("userinfo {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
        }
        Ok(resp.json().await?)
    }
}

impl From<&UserInfo> for AccountId {
    fn from(u: &UserInfo) -> Self { AccountId(u.id.clone()) }
}
```

- [ ] **Step 2: Write `src-tauri/src/auth/orchestrator.rs`**

Stitches the three sources together behind a single API.

```rust
use super::{
    account_identity::IdentityFetcher,
    claude_code_creds,
    exchange::TokenExchange,
    token_store, AccountId, AuthSource, StoredToken,
};
use chrono::{Duration, Utc};
use std::path::PathBuf;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("no auth source configured")]
    NoSource,
    #[error("two Claude accounts detected: {oauth_email} (OAuth) vs {cli_email} (Claude Code)")]
    Conflict { oauth_email: String, cli_email: String },
    #[error("no refresh token available")]
    NoRefreshToken,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type AuthResult<T> = std::result::Result<T, AuthError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountInfo {
    pub id: AccountId,
    pub email: String,
    pub display_name: Option<String>,
}

pub struct AuthOrchestrator {
    pub fallback_dir: PathBuf,
    pub exchange: TokenExchange,
    pub identity: IdentityFetcher,
    pub preferred_source: Mutex<Option<AuthSource>>,
}

impl AuthOrchestrator {
    pub fn new(fallback_dir: PathBuf) -> Self {
        Self {
            fallback_dir,
            exchange: TokenExchange::new(),
            identity: IdentityFetcher::new(),
            preferred_source: Mutex::new(None),
        }
    }

    pub async fn get_access_token(&self) -> AuthResult<(String, AuthSource, AccountInfo)> {
        let preferred = *self.preferred_source.lock().await;

        let token_oauth = token_store::load(&self.fallback_dir).map_err(AuthError::from)?;
        let token_cli = claude_code_creds::load().map_err(AuthError::from)?;

        match (token_oauth, token_cli, preferred) {
            (Some(t), None, _) => {
                let refreshed = self.refresh_if_needed(t).await?;
                self.finalize(refreshed, AuthSource::OAuth).await
            }
            (None, Some(t), _) => self.finalize(t, AuthSource::ClaudeCode).await,
            (None, None, _) => Err(AuthError::NoSource),
            (Some(a), Some(b), Some(pref)) => {
                let chosen = if pref == AuthSource::OAuth { (a, AuthSource::OAuth) } else { (b, AuthSource::ClaudeCode) };
                let refreshed = if chosen.1 == AuthSource::OAuth { self.refresh_if_needed(chosen.0).await? } else { chosen.0 };
                self.finalize(refreshed, chosen.1).await
            }
            (Some(oauth_tok), Some(cli_tok), None) => {
                let oauth_info = self.identity.fetch(&oauth_tok.access_token).await.map_err(AuthError::from)?;
                let cli_info = self.identity.fetch(&cli_tok.access_token).await.map_err(AuthError::from)?;
                if oauth_info.id == cli_info.id {
                    let refreshed = self.refresh_if_needed(oauth_tok).await?;
                    self.finalize(refreshed, AuthSource::OAuth).await
                } else {
                    Err(AuthError::Conflict {
                        oauth_email: oauth_info.email,
                        cli_email: cli_info.email,
                    })
                }
            }
        }
    }

    pub async fn set_preferred_source(&self, src: AuthSource) {
        *self.preferred_source.lock().await = Some(src);
    }

    async fn refresh_if_needed(&self, tok: StoredToken) -> AuthResult<StoredToken> {
        if tok.expires_at > Utc::now() + Duration::minutes(2) {
            return Ok(tok);
        }
        let refresh = tok.refresh_token.clone().ok_or(AuthError::NoRefreshToken)?;
        let new_tok = self.exchange.refresh(&refresh).await.map_err(AuthError::from)?;
        token_store::save(&new_tok, &self.fallback_dir).map_err(AuthError::from)?;
        Ok(new_tok)
    }

    async fn finalize(&self, tok: StoredToken, source: AuthSource) -> AuthResult<(String, AuthSource, AccountInfo)> {
        let info = self.identity.fetch(&tok.access_token).await.map_err(AuthError::from)?;
        let acc = AccountInfo {
            id: (&info).into(),
            email: info.email,
            display_name: info.name,
        };
        Ok((tok.access_token, source, acc))
    }
}
```

- [ ] **Step 3: Export from `src-tauri/src/auth/mod.rs`**

```rust
pub mod account_identity;
pub mod claude_code_creds;
pub mod exchange;
pub mod oauth_paste_back;
pub mod orchestrator;
pub mod token_store;

pub use orchestrator::{AccountInfo, AuthError, AuthOrchestrator, AuthResult};
```

Also ensure `usage_api::client::ANTHROPIC_BETA` is public.

- [ ] **Step 4: Integration test `src-tauri/tests/auth_orchestrator.rs`**

```rust
use claude_usage_monitor_lib::auth::{orchestrator::AuthOrchestrator, AuthError};
use tempfile::tempdir;

// Note: this test exercises only the no-auth-source path since token_store
// interacts with system keyring; richer orchestrator tests live in the Rust
// tests dir and use mocked IdentityFetcher via `with_endpoint`.

#[tokio::test]
async fn no_sources_errors_with_typed_variant() {
    let dir = tempdir().unwrap();
    let orc = AuthOrchestrator::new(dir.path().to_path_buf());
    match orc.get_access_token().await {
        Err(AuthError::NoSource) => {}
        other => panic!("expected AuthError::NoSource, got {other:?}"),
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cd src-tauri && cargo test auth:: && cd ..
```

Expected: all auth tests pass (~10 in unit + integration).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(auth): orchestrator resolves OAuth/CLI sources + userinfo conflict detection"
```

---

## Phase 4 — JSONL Parser

### Task 4.1: External pricing.json + prefix-matched lookup

**Files:**
- Create: `src-tauri/pricing.json`
- Create: `src-tauri/src/jsonl_parser/mod.rs`, `src-tauri/src/jsonl_parser/pricing.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod jsonl_parser;`)

- [ ] **Step 1: Write `src-tauri/pricing.json`**

```json
{
  "pricing": [
    { "prefix": "opus-4-7",   "input_per_mtok":  15.0, "output_per_mtok":  75.0, "cache_read_per_mtok":  1.50, "cache_5m_per_mtok":  18.75, "cache_1h_per_mtok":  30.00 },
    { "prefix": "opus-4-6",   "input_per_mtok":  15.0, "output_per_mtok":  75.0, "cache_read_per_mtok":  1.50, "cache_5m_per_mtok":  18.75, "cache_1h_per_mtok":  30.00 },
    { "prefix": "opus-4-5",   "input_per_mtok":  15.0, "output_per_mtok":  75.0, "cache_read_per_mtok":  1.50, "cache_5m_per_mtok":  18.75, "cache_1h_per_mtok":  30.00 },
    { "prefix": "opus-4-1",   "input_per_mtok":  15.0, "output_per_mtok":  75.0, "cache_read_per_mtok":  1.50, "cache_5m_per_mtok":  18.75, "cache_1h_per_mtok":  30.00 },
    { "prefix": "opus-4",     "input_per_mtok":  15.0, "output_per_mtok":  75.0, "cache_read_per_mtok":  1.50, "cache_5m_per_mtok":  18.75, "cache_1h_per_mtok":  30.00 },
    { "prefix": "sonnet-4-6", "input_per_mtok":   3.0, "output_per_mtok":  15.0, "cache_read_per_mtok":  0.30, "cache_5m_per_mtok":   3.75, "cache_1h_per_mtok":   6.00 },
    { "prefix": "sonnet-4-5", "input_per_mtok":   3.0, "output_per_mtok":  15.0, "cache_read_per_mtok":  0.30, "cache_5m_per_mtok":   3.75, "cache_1h_per_mtok":   6.00 },
    { "prefix": "sonnet-4",   "input_per_mtok":   3.0, "output_per_mtok":  15.0, "cache_read_per_mtok":  0.30, "cache_5m_per_mtok":   3.75, "cache_1h_per_mtok":   6.00 },
    { "prefix": "haiku-4-5",  "input_per_mtok":   1.0, "output_per_mtok":   5.0, "cache_read_per_mtok":  0.10, "cache_5m_per_mtok":   1.25, "cache_1h_per_mtok":   2.00 },
    { "prefix": "haiku-3-5",  "input_per_mtok":   0.8, "output_per_mtok":   4.0, "cache_read_per_mtok":  0.08, "cache_5m_per_mtok":   1.00, "cache_1h_per_mtok":   1.60 },
    { "prefix": "opus",       "input_per_mtok":  15.0, "output_per_mtok":  75.0, "cache_read_per_mtok":  1.50, "cache_5m_per_mtok":  18.75, "cache_1h_per_mtok":  30.00 },
    { "prefix": "sonnet",     "input_per_mtok":   3.0, "output_per_mtok":  15.0, "cache_read_per_mtok":  0.30, "cache_5m_per_mtok":   3.75, "cache_1h_per_mtok":   6.00 },
    { "prefix": "haiku",      "input_per_mtok":   1.0, "output_per_mtok":   5.0, "cache_read_per_mtok":  0.10, "cache_5m_per_mtok":   1.25, "cache_1h_per_mtok":   2.00 }
  ]
}
```

Note: prices are placeholders in USD/MTok; verify against Anthropic's published rates before first release.

- [ ] **Step 2: Write `src-tauri/src/jsonl_parser/pricing.rs`**

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PricingEntry {
    pub prefix: String,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_read_per_mtok: f64,
    pub cache_5m_per_mtok: f64,
    pub cache_1h_per_mtok: f64,
}

#[derive(Debug, Deserialize)]
struct PricingFile {
    pricing: Vec<PricingEntry>,
}

pub struct PricingTable {
    entries: Vec<PricingEntry>, // sorted by prefix.len() DESC for longest-match
}

impl PricingTable {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path).context("read pricing.json")?;
        Self::from_str(&raw)
    }

    pub fn from_str(raw: &str) -> Result<Self> {
        let f: PricingFile = serde_json::from_str(raw)?;
        let mut entries = f.pricing;
        entries.sort_by(|a, b| b.prefix.len().cmp(&a.prefix.len()));
        Ok(Self { entries })
    }

    pub fn bundled() -> Result<Self> {
        let raw = include_str!("../../pricing.json");
        Self::from_str(raw)
    }

    pub fn lookup(&self, model: &str) -> Option<&PricingEntry> {
        let needle = model.to_ascii_lowercase();
        self.entries.iter().find(|e| needle.contains(&e.prefix))
    }

    pub fn cost_for(
        &self,
        model: &str,
        input: u64,
        output: u64,
        cache_read: u64,
        cache_5m: u64,
        cache_1h: u64,
    ) -> f64 {
        let Some(e) = self.lookup(model) else { return 0.0 };
        let m = 1_000_000.0;
        (input as f64) / m * e.input_per_mtok
            + (output as f64) / m * e.output_per_mtok
            + (cache_read as f64) / m * e.cache_read_per_mtok
            + (cache_5m as f64) / m * e.cache_5m_per_mtok
            + (cache_1h as f64) / m * e.cache_1h_per_mtok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> PricingTable { PricingTable::bundled().unwrap() }

    #[test]
    fn longest_prefix_wins() {
        let tbl = t();
        let opus47 = tbl.lookup("claude-opus-4-7-20260115").unwrap();
        assert_eq!(opus47.prefix, "opus-4-7");
        let opus_generic = tbl.lookup("opus-7-5-future").unwrap();
        assert_eq!(opus_generic.prefix, "opus");
    }

    #[test]
    fn every_current_family_is_priced() {
        let tbl = t();
        for m in [
            "opus-4-7", "opus-4-6", "opus-4-5", "opus-4-1", "opus-4",
            "sonnet-4-6", "sonnet-4-5", "sonnet-4",
            "haiku-4-5", "haiku-3-5",
        ] { assert!(tbl.lookup(m).is_some(), "missing pricing for {m}"); }
    }

    #[test]
    fn unknown_model_is_zero_cost_not_panic() {
        let tbl = t();
        assert_eq!(tbl.cost_for("completely-unknown-model", 100, 200, 0, 0, 0), 0.0);
    }

    #[test]
    fn cost_math_matches_expected() {
        let tbl = t();
        // 1 MTok input on sonnet-4-6 should be ~$3.00
        let c = tbl.cost_for("sonnet-4-6", 1_000_000, 0, 0, 0, 0);
        assert!((c - 3.0).abs() < 0.001);
    }
}
```

- [ ] **Step 3: Write `src-tauri/src/jsonl_parser/mod.rs` skeleton**

```rust
pub mod pricing;
pub mod record;
pub mod walker;
pub mod watcher;

pub use pricing::{PricingEntry, PricingTable};
pub use record::SessionEvent;
```

Create stub files `record.rs`, `walker.rs`, `watcher.rs`:
```rust
// Implemented in later Phase 4 tasks.
```

- [ ] **Step 4: Add `pub mod jsonl_parser;` to `src-tauri/src/lib.rs`**

- [ ] **Step 5: Run tests**

```bash
cd src-tauri && cargo test jsonl_parser::pricing && cd ..
```

Expected: 4 passed.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(jsonl_parser): external pricing.json with prefix-matched lookup table"
```

---

### Task 4.2: SessionEvent record + forward-compat fixtures

**Files:**
- Create: `src-tauri/src/jsonl_parser/record.rs` (rewrite)
- Create: `src-tauri/tests/fixtures/jsonl/current_schema.jsonl`
- Create: `src-tauri/tests/fixtures/jsonl/older_schema.jsonl`
- Create: `src-tauri/tests/fixtures/jsonl/malformed_lines.jsonl`
- Create: `src-tauri/tests/fixtures/jsonl/partial_line_at_eof.jsonl`

- [ ] **Step 1: Write `src-tauri/src/jsonl_parser/record.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Forward-compatible shape. Required: `ts`, `project`, `model`. Numeric
/// fields default to zero when absent. Unknown fields are captured in
/// `unknown` so they don't cause parse failures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionEvent {
    pub ts: DateTime<Utc>,
    pub project: String,
    pub model: String,

    #[serde(default)] pub input_tokens: u64,
    #[serde(default)] pub output_tokens: u64,
    #[serde(default)] pub cache_read_tokens: u64,
    #[serde(default)] pub cache_creation_5m_tokens: u64,
    #[serde(default)] pub cache_creation_1h_tokens: u64,

    #[serde(default)] pub cost_usd: f64,

    #[serde(flatten, default)]
    pub unknown: HashMap<String, serde_json::Value>,
}
```

- [ ] **Step 2: Write `src-tauri/tests/fixtures/jsonl/current_schema.jsonl`**

```
{"ts":"2026-04-20T10:00:00Z","project":"demo-a","model":"claude-sonnet-4-6-20260301","input_tokens":1000,"output_tokens":500,"cache_read_tokens":200,"cache_creation_5m_tokens":100,"cache_creation_1h_tokens":0,"cost_usd":0.012}
{"ts":"2026-04-20T10:05:12Z","project":"demo-a","model":"claude-opus-4-7-20260115","input_tokens":500,"output_tokens":1500,"cache_read_tokens":0,"cache_creation_5m_tokens":0,"cache_creation_1h_tokens":0,"cost_usd":0.119}
{"ts":"2026-04-20T11:14:22Z","project":"demo-b","model":"claude-haiku-4-5-20260201","input_tokens":200,"output_tokens":100,"cache_read_tokens":0,"cache_creation_5m_tokens":0,"cache_creation_1h_tokens":0,"cost_usd":0.0007}
```

- [ ] **Step 3: Write `src-tauri/tests/fixtures/jsonl/older_schema.jsonl`**

Includes fields from an older JSONL version + now-removed ones the parser should still ignore.
```
{"ts":"2025-11-10T14:00:00Z","project":"legacy","model":"claude-opus-4-20250901","input_tokens":1234,"output_tokens":2345,"extra_old_field":"ignored","deprecated_flag":true}
{"ts":"2025-12-15T09:30:00Z","project":"legacy","model":"claude-sonnet-4-20251001","input_tokens":100,"output_tokens":200}
```

- [ ] **Step 4: Write `src-tauri/tests/fixtures/jsonl/malformed_lines.jsonl`**

```
{"ts":"2026-04-20T10:00:00Z","project":"a","model":"sonnet-4-6","input_tokens":10}
this is not json at all
{"ts":"2026-04-20T10:01:00Z","project":"a","model":"sonnet-4-6","input_tokens":20}
{incomplete json
{"ts":"2026-04-20T10:02:00Z","project":"a","model":"sonnet-4-6","input_tokens":30}
```

- [ ] **Step 5: Write `src-tauri/tests/fixtures/jsonl/partial_line_at_eof.jsonl`**

NB: the last line is intentionally missing its closing `}` and trailing newline.
```
{"ts":"2026-04-20T10:00:00Z","project":"a","model":"sonnet-4-6","input_tokens":10}
{"ts":"2026-04-20T10:01:00Z","project":"a","model":"sonnet-4-6","input_tokens":20
```

Use `printf` so no trailing newline is added:
```bash
printf '%s\n%s' \
  '{"ts":"2026-04-20T10:00:00Z","project":"a","model":"sonnet-4-6","input_tokens":10}' \
  '{"ts":"2026-04-20T10:01:00Z","project":"a","model":"sonnet-4-6","input_tokens":20' \
  > src-tauri/tests/fixtures/jsonl/partial_line_at_eof.jsonl
```

- [ ] **Step 6: Write unit tests as `src-tauri/tests/jsonl_record.rs`**

```rust
use claude_usage_monitor_lib::jsonl_parser::SessionEvent;

#[test]
fn current_schema_parses_every_line() {
    let raw = include_str!("fixtures/jsonl/current_schema.jsonl");
    let events: Vec<SessionEvent> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("parse"))
        .collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[1].model, "claude-opus-4-7-20260115");
    assert_eq!(events[0].cache_read_tokens, 200);
}

#[test]
fn older_schema_with_unknown_fields_still_parses() {
    let raw = include_str!("fixtures/jsonl/older_schema.jsonl");
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        let e: SessionEvent = serde_json::from_str(line).expect("parse older");
        assert!(!e.project.is_empty());
    }
}

#[test]
fn malformed_lines_are_individually_rejectable() {
    let raw = include_str!("fixtures/jsonl/malformed_lines.jsonl");
    let (ok, err): (Vec<_>, Vec<_>) = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .partition(|l| serde_json::from_str::<SessionEvent>(l).is_ok());
    assert_eq!(ok.len(), 3);
    assert_eq!(err.len(), 2);
}
```

- [ ] **Step 7: Run tests**

```bash
cd src-tauri && cargo test --test jsonl_record && cd ..
```

Expected: 3 passed.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(jsonl_parser): SessionEvent + forward-compat fixtures + parser tests"
```

---

### Task 4.3: Walker (one-level, symlinks skipped, truncation-safe)

**Files:**
- Create: `src-tauri/src/jsonl_parser/walker.rs` (rewrite)

- [ ] **Step 1: Write `src-tauri/src/jsonl_parser/walker.rs`**

```rust
use super::record::SessionEvent;
use super::pricing::PricingTable;
use crate::store::{Db, StoredSessionEvent};
use anyhow::{Context, Result};
use chrono::Utc;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

pub fn claude_projects_root() -> Option<PathBuf> {
    directories::UserDirs::new()
        .map(|u| u.home_dir().join(".claude").join("projects"))
}

pub fn discover_jsonl_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() { return Ok(files); }
    for entry in fs::read_dir(root).context("read projects dir")? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if !meta.is_dir() { continue; }
        if meta.file_type().is_symlink() { continue; }
        let project_dir = entry.path();
        for f in fs::read_dir(&project_dir)? {
            let f = f?;
            let fmeta = f.metadata()?;
            if !fmeta.is_file() { continue; }
            if fmeta.file_type().is_symlink() { continue; }
            let path = f.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") { continue; }
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

    let mut reader = BufReader::new(f);
    let mut buf = Vec::new();
    let mut stored = Vec::<StoredSessionEvent>::new();
    let mut consumed: i64 = offset;
    let mut line_num: i64 = 0;

    loop {
        buf.clear();
        let n = reader.read_until(b'\n', &mut buf)?;
        if n == 0 { break; }
        // If the last byte isn't a newline, we hit EOF on a partial line:
        // don't consume it — leave cursor where it was.
        if *buf.last().unwrap() != b'\n' { break; }
        consumed += n as i64;
        line_num += 1;
        let text = match std::str::from_utf8(&buf) { Ok(t) => t.trim(), Err(_) => continue };
        if text.is_empty() { continue; }
        match serde_json::from_str::<SessionEvent>(text) {
            Ok(ev) => {
                let cost = if ev.cost_usd > 0.0 {
                    ev.cost_usd
                } else {
                    pricing.cost_for(&ev.model, ev.input_tokens, ev.output_tokens,
                                      ev.cache_read_tokens, ev.cache_creation_5m_tokens,
                                      ev.cache_creation_1h_tokens)
                };
                // If the JSONL line has no `project` field, fall back to the
                // parent directory name (Claude Code's project slug).
                let project_name = if ev.project.is_empty() { project.clone() } else { ev.project.clone() };
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
                    source_line: line_num,
                });
            }
            Err(e) => tracing::warn!("malformed line in {} at {}: {}", key, line_num, e),
        }
    }
    let inserted = db.insert_events(&stored)?;
    db.set_cursor(&key, mtime_ns, consumed)?;
    Ok(inserted)
}
```

- [ ] **Step 2: Write integration test `src-tauri/tests/jsonl_walker.rs`**

```rust
use chrono::{Duration, Utc};
use claude_usage_monitor_lib::jsonl_parser::{walker, PricingTable};
use claude_usage_monitor_lib::store::{Db, StoredAccount};
use std::fs;
use tempfile::tempdir;

fn setup() -> (tempfile::TempDir, Db, PricingTable, std::path::PathBuf) {
    let d = tempdir().unwrap();
    let db_dir = d.path().join("db");
    let projects = d.path().join("projects");
    let proj = projects.join("demo");
    fs::create_dir_all(&proj).unwrap();
    let db = Db::open(&db_dir).unwrap();
    db.upsert_account(&StoredAccount { id: "acc".into(), email: "e".into(), display_name: None }).unwrap();
    (d, db, PricingTable::bundled().unwrap(), projects)
}

#[test]
fn ingests_current_schema_file() {
    let (_d, db, p, projects) = setup();
    let f = projects.join("demo").join("session.jsonl");
    fs::copy("tests/fixtures/jsonl/current_schema.jsonl", &f).unwrap();
    let n = walker::ingest_file(&db, &p, &f).unwrap();
    assert_eq!(n, 3);
}

#[test]
fn idempotent_on_same_file() {
    let (_d, db, p, projects) = setup();
    let f = projects.join("demo").join("session.jsonl");
    fs::copy("tests/fixtures/jsonl/current_schema.jsonl", &f).unwrap();
    let a = walker::ingest_file(&db, &p, &f).unwrap();
    let b = walker::ingest_file(&db, &p, &f).unwrap();
    assert_eq!(a, 3);
    assert_eq!(b, 0);
}

#[test]
fn partial_line_at_eof_is_not_consumed() {
    let (_d, db, p, projects) = setup();
    let f = projects.join("demo").join("session.jsonl");
    fs::copy("tests/fixtures/jsonl/partial_line_at_eof.jsonl", &f).unwrap();
    let n = walker::ingest_file(&db, &p, &f).unwrap();
    assert_eq!(n, 1, "only the first complete line is ingested");

    // Complete the partial line and re-ingest — the second line should appear.
    let mut contents = fs::read_to_string(&f).unwrap();
    contents.push_str(",\"output_tokens\":30}\n");
    fs::write(&f, contents).unwrap();
    let n = walker::ingest_file(&db, &p, &f).unwrap();
    assert_eq!(n, 1, "completed line ingested on next pass");
}

#[test]
fn truncation_resets_cursor_and_dedupes() {
    let (_d, db, p, projects) = setup();
    let f = projects.join("demo").join("session.jsonl");
    fs::copy("tests/fixtures/jsonl/current_schema.jsonl", &f).unwrap();
    assert_eq!(walker::ingest_file(&db, &p, &f).unwrap(), 3);

    // Truncate file to a single line with the SAME source_line=1 as before.
    let first_line = include_str!("fixtures/jsonl/current_schema.jsonl").lines().next().unwrap().to_string() + "\n";
    fs::write(&f, first_line).unwrap();

    // Cursor is now greater than new file length — walker must reset to 0 and reparse.
    // Row already exists with (source_file, source_line=1) so INSERT OR IGNORE inserts 0 new rows.
    let n = walker::ingest_file(&db, &p, &f).unwrap();
    assert_eq!(n, 0, "cursor reset + dedup should add no new rows");

    // Subsequent call with unchanged file must be idempotent (cursor now matches file size).
    let n2 = walker::ingest_file(&db, &p, &f).unwrap();
    assert_eq!(n2, 0);

    // Verify DB still contains exactly 3 events total.
    let count = db.events_between(
        chrono::Utc::now() - chrono::Duration::days(3650),
        chrono::Utc::now() + chrono::Duration::days(1),
    ).unwrap().len();
    assert_eq!(count, 3);
}

#[test]
fn malformed_lines_are_skipped_not_fatal() {
    let (_d, db, p, projects) = setup();
    let f = projects.join("demo").join("session.jsonl");
    fs::copy("tests/fixtures/jsonl/malformed_lines.jsonl", &f).unwrap();
    let n = walker::ingest_file(&db, &p, &f).unwrap();
    assert_eq!(n, 3, "only 3 of 5 lines are valid");
}

#[test]
fn discover_jsonl_skips_deep_nesting() {
    let (_d, _db, _p, projects) = setup();
    let deep = projects.join("demo").join("nested").join("deeper");
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("hidden.jsonl"), r#"{"ts":"2026-01-01T00:00:00Z","project":"x","model":"opus"}"#).unwrap();
    // A file directly in demo/ should still be discovered.
    fs::write(projects.join("demo").join("session.jsonl"), "").unwrap();
    let files = walker::discover_jsonl_files(&projects).unwrap();
    assert_eq!(files.len(), 1, "only the one-level file is discovered");
}
```

- [ ] **Step 3: Run tests**

```bash
cd src-tauri && cargo test --test jsonl_walker && cd ..
```

Expected: 6 passed.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(jsonl_parser): one-level walker with truncation + partial-line handling"
```

---

### Task 4.4: Live file watcher

**Files:**
- Create: `src-tauri/src/jsonl_parser/watcher.rs` (rewrite)

- [ ] **Step 1: Write `src-tauri/src/jsonl_parser/watcher.rs`**

```rust
use super::pricing::PricingTable;
use super::walker;
use crate::store::Db;
use anyhow::Result;
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct WatcherHandle {
    _debouncer: notify_debouncer_full::Debouncer<notify::RecommendedWatcher, notify_debouncer_full::FileIdMap>,
}

pub fn start(db: Arc<Db>, pricing: Arc<PricingTable>, root: PathBuf, tx: mpsc::UnboundedSender<usize>) -> Result<WatcherHandle> {
    let (notify_tx, mut notify_rx) = mpsc::unbounded_channel::<Vec<DebouncedEvent>>();
    let mut debouncer = new_debouncer(Duration::from_millis(500), None, move |res| {
        if let Ok(events) = res { let _ = notify_tx.send(events); }
    })?;
    debouncer.watcher().watch(&root, RecursiveMode::Recursive)?;

    let db_clone = db.clone();
    let pricing_clone = pricing.clone();
    tokio::spawn(async move {
        while let Some(events) = notify_rx.recv().await {
            let mut touched = std::collections::HashSet::<PathBuf>::new();
            for e in events {
                for p in e.paths {
                    if p.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        touched.insert(p);
                    }
                }
            }
            for p in touched {
                match walker::ingest_file(&db_clone, &pricing_clone, &p) {
                    Ok(n) if n > 0 => { let _ = tx.send(n); }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("ingest {} failed: {}", p.display(), e),
                }
            }
        }
    });

    Ok(WatcherHandle { _debouncer: debouncer })
}
```

- [ ] **Step 2: Smoke-test the watcher manually in dev (no automated test for this one)**

This will be covered by the manual release checklist. Skip automated test here.

- [ ] **Step 3: Verify compile**

```bash
cd src-tauri && cargo build && cd ..
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(jsonl_parser): debounced file watcher invoking walker on changes"
```

---

## Phase 5 — Notifier

### Task 5.1: Threshold rules with Option<resets_at> handling

**Files:**
- Create: `src-tauri/src/notifier/mod.rs`, `src-tauri/src/notifier/rules.rs`
- Modify: `src-tauri/src/lib.rs` (`pub mod notifier;`)

- [ ] **Step 1: Write `src-tauri/src/notifier/rules.rs`**

```rust
use crate::store::Db;
use crate::usage_api::{ExtraUsage, UsageSnapshot, Utilization};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bucket {
    FiveHour,
    SevenDay,
    SevenDayOpus,
    SevenDaySonnet,
    ExtraUsage,
}

impl Bucket {
    pub fn label(&self) -> &'static str {
        match self {
            Bucket::FiveHour => "five_hour",
            Bucket::SevenDay => "seven_day",
            Bucket::SevenDayOpus => "seven_day_opus",
            Bucket::SevenDaySonnet => "seven_day_sonnet",
            Bucket::ExtraUsage => "extra_usage",
        }
    }
    pub fn human(&self) -> &'static str {
        match self {
            Bucket::FiveHour => "5-hour",
            Bucket::SevenDay => "7-day",
            Bucket::SevenDayOpus => "7-day Opus",
            Bucket::SevenDaySonnet => "7-day Sonnet",
            Bucket::ExtraUsage => "pay-as-you-go",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Fired {
    pub bucket: Bucket,
    pub threshold: u8,
    pub title: String,
    pub body: String,
}

fn utilization_of(bucket: Bucket, s: &UsageSnapshot) -> (Option<f64>, Option<DateTime<Utc>>) {
    fn of(u: &Option<Utilization>) -> (Option<f64>, Option<DateTime<Utc>>) {
        u.as_ref().map(|v| (Some(v.utilization), Some(v.resets_at))).unwrap_or((None, None))
    }
    fn ofe(e: &Option<ExtraUsage>) -> (Option<f64>, Option<DateTime<Utc>>) {
        e.as_ref().map(|v| (Some(v.utilization), v.resets_at)).unwrap_or((None, None))
    }
    match bucket {
        Bucket::FiveHour => of(&s.five_hour),
        Bucket::SevenDay => of(&s.seven_day),
        Bucket::SevenDayOpus => of(&s.seven_day_opus),
        Bucket::SevenDaySonnet => of(&s.seven_day_sonnet),
        Bucket::ExtraUsage => ofe(&s.extra_usage),
    }
}

fn humanize_duration(d: Duration) -> String {
    let secs = d.num_seconds().max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 { format!("{h}h {m}m") } else { format!("{m}m") }
}

pub fn evaluate(
    db: &Db,
    account_id: &str,
    snapshot: &UsageSnapshot,
    thresholds: &[u8],
    now: DateTime<Utc>,
) -> Result<Vec<Fired>> {
    const BUCKETS: [Bucket; 5] = [
        Bucket::FiveHour, Bucket::SevenDay,
        Bucket::SevenDayOpus, Bucket::SevenDaySonnet, Bucket::ExtraUsage,
    ];
    let mut fired = Vec::new();
    for bucket in BUCKETS {
        let (Some(util), resets_at) = utilization_of(bucket, snapshot) else { continue };
        for &threshold in thresholds {
            if util < threshold as f64 { continue; }
            let last = db.notification_last_fired(account_id, bucket.label(), threshold as i64)?;
            let already = match resets_at {
                Some(reset) => {
                    // Fired within current window? last_fired is "since the latest reset"
                    // if last_fired > (reset - window_length). Simplify: if last_fired
                    // exists AND last_fired is BEFORE the next reset, we've already fired.
                    last.map(|l| l < reset && l > now - Duration::days(8)).unwrap_or(false)
                }
                None => last.map(|l| (now - l) < Duration::hours(24)).unwrap_or(false),
            };
            if already { continue; }

            let title = format!("Claude {} usage at {}%", bucket.human(), threshold);
            let body = match (bucket, resets_at) {
                (Bucket::ExtraUsage, None) => "Pay-as-you-go credits running low".to_string(),
                (_, Some(reset)) => format!("Resets in {}", humanize_duration(reset - now)),
                (_, None) => "Window reset time unknown".to_string(),
            };
            db.record_notification_fired(account_id, bucket.label(), threshold as i64, now)?;
            fired.push(Fired { bucket, threshold, title, body });
        }
    }
    Ok(fired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Db, StoredAccount};
    use crate::usage_api::Utilization;
    use tempfile::tempdir;

    fn fresh() -> (tempfile::TempDir, Db) {
        let d = tempdir().unwrap();
        let db = Db::open(d.path()).unwrap();
        db.upsert_account(&StoredAccount { id: "a".into(), email: "e".into(), display_name: None }).unwrap();
        (d, db)
    }

    fn snap_five_hour(util: f64, reset_in_hours: i64) -> UsageSnapshot {
        UsageSnapshot {
            five_hour: Some(Utilization { utilization: util, resets_at: Utc::now() + Duration::hours(reset_in_hours) }),
            seven_day: None, seven_day_sonnet: None, seven_day_opus: None, extra_usage: None,
            fetched_at: Utc::now(), unknown: Default::default(),
        }
    }

    #[test]
    fn fires_once_per_threshold_per_window() {
        let (_d, db) = fresh();
        let s = snap_five_hour(80.0, 3);
        let now = Utc::now();
        let f1 = evaluate(&db, "a", &s, &[75, 90], now).unwrap();
        assert_eq!(f1.len(), 1, "only 75% crosses at 80");
        let f2 = evaluate(&db, "a", &s, &[75, 90], now + Duration::minutes(5)).unwrap();
        assert!(f2.is_empty(), "no re-fire within window");
    }

    #[test]
    fn refires_after_window_reset() {
        let (_d, db) = fresh();
        let now = Utc::now();
        let early = snap_five_hour(80.0, 3);
        evaluate(&db, "a", &early, &[75], now).unwrap();
        // Simulate passage of time past the reset.
        let later_reset = Utc::now() + Duration::hours(8);
        let fresh_snap = UsageSnapshot {
            five_hour: Some(Utilization { utilization: 80.0, resets_at: later_reset }),
            seven_day: None, seven_day_sonnet: None, seven_day_opus: None, extra_usage: None,
            fetched_at: later_reset, unknown: Default::default(),
        };
        let fired = evaluate(&db, "a", &fresh_snap, &[75], later_reset + Duration::minutes(1)).unwrap();
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn extra_usage_without_reset_uses_24h_cooldown() {
        let (_d, db) = fresh();
        let snap = UsageSnapshot {
            five_hour: None, seven_day: None, seven_day_sonnet: None, seven_day_opus: None,
            extra_usage: Some(ExtraUsage {
                is_enabled: true, monthly_limit_cents: 5000, used_credits_cents: 3750,
                utilization: 75.0, resets_at: None,
            }),
            fetched_at: Utc::now(), unknown: Default::default(),
        };
        let now = Utc::now();
        let a = evaluate(&db, "a", &snap, &[75], now).unwrap();
        assert_eq!(a.len(), 1);
        assert!(a[0].body.contains("credits"));
        let b = evaluate(&db, "a", &snap, &[75], now + Duration::hours(12)).unwrap();
        assert!(b.is_empty(), "inside 24h cooldown");
        let c = evaluate(&db, "a", &snap, &[75], now + Duration::hours(25)).unwrap();
        assert_eq!(c.len(), 1, "past 24h cooldown");
    }

    #[test]
    fn below_threshold_does_not_fire() {
        let (_d, db) = fresh();
        let s = snap_five_hour(50.0, 3);
        assert!(evaluate(&db, "a", &s, &[75, 90], Utc::now()).unwrap().is_empty());
    }
}
```

- [ ] **Step 2: Write `src-tauri/src/notifier/mod.rs`**

```rust
pub mod rules;

pub use rules::{evaluate, Bucket, Fired};
```

- [ ] **Step 3: Wire `pub mod notifier;` into `lib.rs`**

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test notifier:: && cd ..
```

Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(notifier): threshold evaluator with reset-gating and extra-usage cooldown"
```

---

## Phase 6 — Tauri Commands, Events, and Polling Loop

### Task 6.1: Shared app state + Tauri command surface

**Files:**
- Create: `src-tauri/src/app_state.rs`, `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/app_state.rs`**

```rust
use crate::auth::{AuthOrchestrator, AuthSource};
use crate::auth::oauth_paste_back::PkcePair;
use crate::jsonl_parser::PricingTable;
use crate::store::Db;
use crate::usage_api::{UsageClient, UsageSnapshot};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub polling_interval_secs: u64,
    pub thresholds: Vec<u8>,
    pub theme: String,
    pub launch_at_login: bool,
    pub crash_reports: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            polling_interval_secs: 300,
            thresholds: vec![75, 90],
            theme: "system".into(),
            launch_at_login: false,
            crash_reports: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedUsage {
    pub snapshot: UsageSnapshot,
    pub account_id: String,
    pub account_email: String,
    pub last_error: Option<String>,
}

impl CachedUsage {
    pub fn is_stale(&self, now: DateTime<Utc>) -> bool {
        (now - self.snapshot.fetched_at) > chrono::Duration::minutes(15)
            || now < self.snapshot.fetched_at
            || self.last_error.is_some()
    }
}

pub struct AppState {
    pub db: Arc<Db>,
    pub auth: Arc<AuthOrchestrator>,
    pub usage: Arc<UsageClient>,
    pub pricing: Arc<PricingTable>,
    pub settings: RwLock<Settings>,
    pub cached_usage: RwLock<Option<CachedUsage>>,
    /// PKCE verifier+state held between `start_oauth_flow` and `submit_oauth_code`.
    /// `None` outside the flow; overwritten if the user restarts sign-in.
    pub pending_oauth: RwLock<Option<PkcePair>>,
    pub fallback_dir: std::path::PathBuf,
}

impl AppState {
    pub fn snapshot(&self) -> Option<CachedUsage> {
        self.cached_usage.read().clone()
    }
}
```

(`parking_lot` is already listed in the Cargo.toml from Task 0.2.)

- [ ] **Step 2: Write `src-tauri/src/commands.rs`**

```rust
use crate::app_state::{AppState, CachedUsage, Settings};
use crate::auth::AuthSource;
use crate::notifier::Bucket;
use crate::store::StoredSessionEvent;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, State};

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyBucket {
    pub date: String,       // YYYY-MM-DD local
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStats {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectStats {
    pub project: String,
    pub session_count: u64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_cache_read_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub estimated_savings_usd: f64,
    pub hit_ratio: f64,
}

fn err_to_string<E: std::fmt::Display>(e: E) -> String { e.to_string() }

#[command]
pub async fn get_current_usage(state: State<'_, Arc<AppState>>) -> Result<Option<CachedUsage>, String> {
    Ok(state.snapshot())
}

#[command]
pub async fn get_session_history(days: u32, state: State<'_, Arc<AppState>>) -> Result<Vec<StoredSessionEvent>, String> {
    let to = Utc::now();
    let from = to - Duration::days(days as i64);
    state.db.events_between(from, to).map_err(err_to_string)
}

#[command]
pub async fn get_daily_trends(days: u32, state: State<'_, Arc<AppState>>) -> Result<Vec<DailyBucket>, String> {
    let events = get_session_history(days, state).await?;
    use std::collections::BTreeMap;
    let mut by_day: BTreeMap<String, DailyBucket> = BTreeMap::new();
    for e in events {
        let date = e.ts.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string();
        let slot = by_day.entry(date.clone()).or_insert_with(|| DailyBucket {
            date, input_tokens: 0, output_tokens: 0, cost_usd: 0.0,
        });
        slot.input_tokens += e.input_tokens;
        slot.output_tokens += e.output_tokens;
        slot.cost_usd += e.cost_usd;
    }
    Ok(by_day.into_values().collect())
}

#[command]
pub async fn get_model_breakdown(days: u32, state: State<'_, Arc<AppState>>) -> Result<Vec<ModelStats>, String> {
    let events = get_session_history(days, state).await?;
    use std::collections::HashMap;
    let mut by_model: HashMap<String, ModelStats> = HashMap::new();
    for e in events {
        let entry = by_model.entry(e.model.clone()).or_insert(ModelStats {
            model: e.model.clone(), input_tokens: 0, output_tokens: 0,
            cache_read_tokens: 0, cache_creation_tokens: 0, cost_usd: 0.0,
        });
        entry.input_tokens += e.input_tokens;
        entry.output_tokens += e.output_tokens;
        entry.cache_read_tokens += e.cache_read_tokens;
        entry.cache_creation_tokens += e.cache_creation_5m_tokens + e.cache_creation_1h_tokens;
        entry.cost_usd += e.cost_usd;
    }
    Ok(by_model.into_values().collect())
}

#[command]
pub async fn get_project_breakdown(days: u32, state: State<'_, Arc<AppState>>) -> Result<Vec<ProjectStats>, String> {
    let events = get_session_history(days, state).await?;
    use std::collections::HashMap;
    let mut by_project: HashMap<String, ProjectStats> = HashMap::new();
    for e in events {
        let entry = by_project.entry(e.project.clone()).or_insert(ProjectStats {
            project: e.project.clone(), session_count: 0, total_cost_usd: 0.0,
        });
        entry.session_count += 1;
        entry.total_cost_usd += e.cost_usd;
    }
    Ok(by_project.into_values().collect())
}

#[command]
pub async fn get_cache_stats(days: u32, state: State<'_, Arc<AppState>>) -> Result<CacheStats, String> {
    let events = get_session_history(days, state).await?;
    let mut read = 0u64; let mut created = 0u64;
    for e in &events {
        read += e.cache_read_tokens;
        created += e.cache_creation_5m_tokens + e.cache_creation_1h_tokens;
    }
    let total = read + created;
    let hit_ratio = if total > 0 { (read as f64) / (total as f64) } else { 0.0 };
    // Savings approximation: cache-read cost is ~10% of input; count as savings the delta.
    let savings = (read as f64 / 1_000_000.0) * 2.7;
    Ok(CacheStats {
        total_cache_read_tokens: read,
        total_cache_creation_tokens: created,
        estimated_savings_usd: savings,
        hit_ratio,
    })
}

#[command]
pub async fn start_oauth_flow(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    use crate::auth::oauth_paste_back::{build_authorize_url, generate_pkce};
    let pkce = generate_pkce();
    let url = build_authorize_url(&pkce).map_err(err_to_string)?;
    *state.pending_oauth.write() = Some(pkce);
    Ok(url)
}

#[command]
pub async fn submit_oauth_code(
    pasted: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    use crate::auth::exchange::TokenExchange;
    use crate::auth::oauth_paste_back::parse_pasted_code;
    use crate::auth::token_store;

    let pkce = state.pending_oauth.read().clone()
        .ok_or_else(|| "No active sign-in — click 'Sign in with Claude' first".to_string())?;

    let code = parse_pasted_code(&pasted, &pkce.state).map_err(err_to_string)?;
    let exchange = TokenExchange::new();
    let token = exchange.exchange_code(&code, &pkce.verifier).await.map_err(err_to_string)?;
    token_store::save(&token, &state.fallback_dir).map_err(err_to_string)?;

    // Clear pending and set preferred source so the poll loop picks it up.
    *state.pending_oauth.write() = None;
    state.auth.set_preferred_source(AuthSource::OAuth).await;
    Ok(())
}

#[command]
pub async fn use_claude_code_creds(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.auth.set_preferred_source(AuthSource::ClaudeCode).await;
    Ok(())
}

#[command]
pub async fn pick_auth_source(source: AuthSource, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.auth.set_preferred_source(source).await;
    Ok(())
}

#[command]
pub async fn sign_out(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    use crate::auth::token_store;
    token_store::clear(&state.fallback_dir).map_err(err_to_string)?;
    *state.cached_usage.write() = None;
    *state.pending_oauth.write() = None;
    Ok(())
}

/// Whether the machine already has Claude Code credentials the app could reuse.
/// Used on first-run to decide whether to show the "Use Claude Code credentials" button.
#[command]
pub async fn has_claude_code_creds() -> Result<bool, String> {
    Ok(crate::auth::claude_code_creds::has_creds())
}

#[command]
pub async fn update_settings(s: Settings, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    *state.settings.write() = s;
    Ok(())
}

#[command]
pub async fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, String> {
    Ok(state.settings.read().clone())
}

#[cfg(debug_assertions)]
#[command]
pub async fn debug_force_threshold(bucket: String, pct: u8, _state: State<'_, Arc<AppState>>) -> Result<(), String> {
    tracing::info!("debug_force_threshold({bucket}, {pct})");
    Ok(())
}
```

- [ ] **Step 3: Register commands in `src-tauri/src/lib.rs`**

Replace the body of `run()`:
```rust
mod app_state;
mod auth;
mod commands;
mod jsonl_parser;
mod logging;
mod notifier;
mod store;
mod usage_api;

use app_state::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_dir = logging::log_dir();
    let _log_guard = logging::init(log_dir.clone());

    let data_dir = store::default_dir();
    let db = Arc::new(store::Db::open(&data_dir).expect("open db"));
    let pricing = Arc::new(jsonl_parser::PricingTable::bundled().expect("pricing"));
    let auth = Arc::new(auth::AuthOrchestrator::new(data_dir.clone()));
    let usage_client = Arc::new(
        usage_api::UsageClient::new(env!("CARGO_PKG_VERSION").to_string()).expect("client")
    );

    let app_state = Arc::new(AppState {
        db: db.clone(),
        auth,
        usage: usage_client,
        pricing: pricing.clone(),
        settings: parking_lot::RwLock::new(app_state::Settings::default()),
        cached_usage: parking_lot::RwLock::new(None),
        pending_oauth: parking_lot::RwLock::new(None),
        fallback_dir: data_dir.clone(),
    });

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("popover") { let _ = w.show(); let _ = w.set_focus(); }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_current_usage,
            commands::get_session_history,
            commands::get_daily_trends,
            commands::get_model_breakdown,
            commands::get_project_breakdown,
            commands::get_cache_stats,
            commands::start_oauth_flow,
            commands::submit_oauth_code,
            commands::use_claude_code_creds,
            commands::pick_auth_source,
            commands::sign_out,
            commands::has_claude_code_creds,
            commands::update_settings,
            commands::get_settings,
            #[cfg(debug_assertions)] commands::debug_force_threshold,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Verify compile**

```bash
cd src-tauri && cargo build && cd ..
```

Expected: clean build (warnings about unused vars are acceptable).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(commands): shared AppState + Tauri command surface"
```

---

### Task 6.2: Polling loop with immediate-first-fetch

**Files:**
- Create: `src-tauri/src/poll_loop.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/poll_loop.rs`**

```rust
use crate::app_state::{AppState, CachedUsage};
use crate::auth::AuthError;
use crate::notifier;
use crate::usage_api::{next_backoff, FetchOutcome};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

static STALE_EMITTED: AtomicBool = AtomicBool::new(false);

pub fn spawn(handle: AppHandle, state: Arc<AppState>) {
    tokio::spawn(async move {
        // Immediate first fetch — no 5-minute dead zone on launch.
        let _ = poll_once(&handle, &state).await;
        let mut backoff = Duration::from_secs(60);
        loop {
            let interval = {
                let s = state.settings.read();
                Duration::from_secs(s.polling_interval_secs.max(60))
            };
            tokio::time::sleep(interval).await;

            // Emit stale_data edge-triggered: fire once when a cached snapshot
            // crosses the staleness threshold, reset after any successful poll.
            if let Some(cached) = &*state.cached_usage.read() {
                if cached.is_stale(Utc::now()) {
                    if !STALE_EMITTED.swap(true, Ordering::Relaxed) {
                        let _ = handle.emit("stale_data", ());
                    }
                }
            }

            match poll_once(&handle, &state).await {
                PollResult::Ok => {
                    STALE_EMITTED.store(false, Ordering::Relaxed);
                    backoff = Duration::from_secs(60);
                }
                PollResult::Backoff => {
                    tokio::time::sleep(backoff).await;
                    backoff = next_backoff(backoff);
                }
                PollResult::Transient => { /* keep ticking normally */ }
            }
        }
    });
}

enum PollResult { Ok, Backoff, Transient }

async fn poll_once(handle: &AppHandle, state: &AppState) -> PollResult {
    let (token, _source, account) = match state.auth.get_access_token().await {
        Ok(t) => t,
        Err(AuthError::NoSource) => {
            // First-run or signed-out state — no error surface, frontend already
            // shows AuthPanel based on `CachedUsage` being None.
            return PollResult::Transient;
        }
        Err(AuthError::Conflict { oauth_email, cli_email }) => {
            let _ = handle.emit("auth_source_conflict", json!({
                "oauth_email": oauth_email,
                "cli_email":   cli_email,
            }));
            return PollResult::Transient;
        }
        Err(e) => {
            tracing::warn!("auth failure: {e}");
            let _ = handle.emit("auth_required", ());
            return PollResult::Transient;
        }
    };

    match state.usage.fetch(&token).await {
        FetchOutcome::Ok(snapshot) => {
            let cached = CachedUsage {
                snapshot: snapshot.clone(),
                account_id: account.id.0.clone(),
                account_email: account.email.clone(),
                last_error: None,
            };
            *state.cached_usage.write() = Some(cached.clone());
            let _ = handle.emit("usage_updated", &cached);

            let thresholds = state.settings.read().thresholds.clone();
            match notifier::evaluate(&state.db, &cached.account_id, &snapshot, &thresholds, Utc::now()) {
                Ok(fired) => {
                    for f in fired {
                        use tauri_plugin_notification::NotificationExt;
                        let _ = handle.notification().builder().title(f.title).body(f.body).show();
                    }
                }
                Err(e) => tracing::warn!("notifier evaluate failed: {e}"),
            }
            crate::tray::set_level(handle,
                snapshot.five_hour.as_ref().map(|u| u.utilization), false);
            PollResult::Ok
        }
        FetchOutcome::Unauthorized => {
            let _ = handle.emit("auth_required", ());
            PollResult::Transient
        }
        FetchOutcome::RateLimited => {
            crate::tray::set_level(handle, None, true);
            PollResult::Backoff
        }
        FetchOutcome::Transient(e) => {
            let current = state.cached_usage.read().clone();
            if let Some(mut c) = current {
                c.last_error = Some(e);
                *state.cached_usage.write() = Some(c.clone());
                let _ = handle.emit("usage_updated", &c);
            }
            PollResult::Transient
        }
    }
}
```

- [ ] **Step 2: Register in `lib.rs` using `setup`**

Inside `tauri::Builder::default()...` chain, before `.run(...)`:

```rust
.setup(|app| {
    let handle = app.handle().clone();
    let state = app.state::<Arc<AppState>>().inner().clone();
    poll_loop::spawn(handle, state);
    Ok(())
})
```

And add `mod poll_loop;` at the top.

- [ ] **Step 3: Verify build**

```bash
cd src-tauri && cargo build && cd ..
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(poll_loop): immediate-first-fetch polling with backoff and event emission"
```

---

### Task 6.3: JSONL watcher wired into startup + `session_ingested` event

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Start the watcher inside `setup`**

```rust
.setup(|app| {
    let handle = app.handle().clone();
    let state = app.state::<Arc<AppState>>().inner().clone();
    poll_loop::spawn(handle.clone(), state.clone());

    if let Some(root) = jsonl_parser::walker::claude_projects_root() {
        // Initial backfill: ingest anything we haven't seen yet.
        let bf_state = state.clone();
        tokio::spawn(async move {
            if let Ok(files) = jsonl_parser::walker::discover_jsonl_files(&root) {
                for f in files {
                    let _ = jsonl_parser::walker::ingest_file(&bf_state.db, &bf_state.pricing, &f);
                }
            }
        });

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<usize>();
        let handle_for_events = handle.clone();
        tokio::spawn(async move {
            while let Some(n) = rx.recv().await {
                let _ = handle_for_events.emit("session_ingested", n);
            }
        });
        let _ = jsonl_parser::watcher::start(state.db.clone(), state.pricing.clone(), root, tx);
    }

    Ok(())
})
```

- [ ] **Step 2: Verify build and manual smoke**

```bash
cd src-tauri && cargo build && cd ..
pnpm tauri dev
```

Expected: app launches. If a real `~/.claude/projects/` exists, file changes under it trigger the watcher (observable via `tail -f` on `~/Library/Application Support/.../logs/`).

Kill with Ctrl+C.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(runtime): startup backfill + live watcher wired into Tauri lifecycle"
```

---

## Phase 7 — Frontend Integration Layer

### Task 7.1: Typed IPC wrappers

**Files:**
- Create: `src/lib/ipc.ts`, `src/lib/events.ts`, `src/lib/types.ts`

- [ ] **Step 1: Write `src/lib/types.ts` (mirrors Rust types — maintained by hand until `tauri-specta` codegen ships in Task 7.3)**

```ts
export type AuthSource = "OAuth" | "ClaudeCode";

export interface Utilization {
  utilization: number;         // 0..100
  resets_at: string;           // ISO-8601 UTC
}

export interface ExtraUsage {
  is_enabled: boolean;
  monthly_limit_cents: number;
  used_credits_cents: number;
  utilization: number;
  resets_at: string | null;
}

export interface UsageSnapshot {
  five_hour: Utilization | null;
  seven_day: Utilization | null;
  seven_day_sonnet: Utilization | null;
  seven_day_opus: Utilization | null;
  extra_usage: ExtraUsage | null;
  fetched_at: string;
  unknown: Record<string, unknown>;
}

export interface CachedUsage {
  snapshot: UsageSnapshot;
  account_id: string;
  account_email: string;
  last_error: string | null;
}

export interface DailyBucket { date: string; input_tokens: number; output_tokens: number; cost_usd: number; }
export interface ModelStats { model: string; input_tokens: number; output_tokens: number; cache_read_tokens: number; cache_creation_tokens: number; cost_usd: number; }
export interface ProjectStats { project: string; session_count: number; total_cost_usd: number; }
export interface CacheStats { total_cache_read_tokens: number; total_cache_creation_tokens: number; estimated_savings_usd: number; hit_ratio: number; }

export interface SessionEvent {
  ts: string; project: string; model: string;
  input_tokens: number; output_tokens: number;
  cache_read_tokens: number;
  cache_creation_5m_tokens: number; cache_creation_1h_tokens: number;
  cost_usd: number;
  source_file: string; source_line: number;
}

export interface Settings {
  polling_interval_secs: number;
  thresholds: number[];
  theme: string;
  launch_at_login: boolean;
  crash_reports: boolean;
}
```

- [ ] **Step 2: Write `src/lib/ipc.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import type {
  AuthSource, CachedUsage, CacheStats, DailyBucket,
  ModelStats, ProjectStats, SessionEvent, Settings,
} from "./types";

export const ipc = {
  getCurrentUsage: () => invoke<CachedUsage | null>("get_current_usage"),
  getSessionHistory: (days: number) => invoke<SessionEvent[]>("get_session_history", { days }),
  getDailyTrends: (days: number) => invoke<DailyBucket[]>("get_daily_trends", { days }),
  getModelBreakdown: (days: number) => invoke<ModelStats[]>("get_model_breakdown", { days }),
  getProjectBreakdown: (days: number) => invoke<ProjectStats[]>("get_project_breakdown", { days }),
  getCacheStats: (days: number) => invoke<CacheStats>("get_cache_stats", { days }),

  startOauthFlow: () => invoke<string>("start_oauth_flow"),
  submitOauthCode: (pasted: string) => invoke<void>("submit_oauth_code", { pasted }),
  useClaudeCodeCreds: () => invoke<void>("use_claude_code_creds"),
  pickAuthSource: (source: AuthSource) => invoke<void>("pick_auth_source", { source }),
  signOut: () => invoke<void>("sign_out"),
  hasClaudeCodeCreds: () => invoke<boolean>("has_claude_code_creds"),

  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (s: Settings) => invoke<void>("update_settings", { s }),
};
```

- [ ] **Step 3: Write `src/lib/events.ts`**

```ts
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CachedUsage } from "./types";

export type AppEvent =
  | { type: "usage_updated"; payload: CachedUsage }
  | { type: "session_ingested"; payload: number }
  | { type: "auth_required" }
  | { type: "auth_source_conflict"; payload: { oauth_email: string; cli_email: string } }
  | { type: "stale_data" }
  | { type: "db_reset" };

export function subscribe(handler: (e: AppEvent) => void): Promise<UnlistenFn[]> {
  return Promise.all([
    listen<CachedUsage>("usage_updated", (e) => handler({ type: "usage_updated", payload: e.payload })),
    listen<number>("session_ingested", (e) => handler({ type: "session_ingested", payload: e.payload })),
    listen("auth_required", () => handler({ type: "auth_required" })),
    listen<{ oauth_email: string; cli_email: string }>("auth_source_conflict", (e) => handler({ type: "auth_source_conflict", payload: e.payload })),
    listen("stale_data", () => handler({ type: "stale_data" })),
    listen("db_reset", () => handler({ type: "db_reset" })),
  ]);
}
```

- [ ] **Step 4: Verify TypeScript build**

```bash
pnpm lint
```

Expected: no errors (the existing `src/lib/store.ts` may reference symbols we're about to add — Task 7.2 completes that).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(ipc): typed wrappers around Tauri invoke + event subscription"
```

---

### Task 7.2: Zustand store wiring

**Files:**
- Modify: `src/lib/store.ts` (augment — preserve existing design-system state)

- [ ] **Step 1: Read the current `src/lib/store.ts`**

```bash
cat src/lib/store.ts
```

Identify whether the designer wrote a UI-theme store or a data store; augment rather than replace.

- [ ] **Step 2: Add usage + settings slices**

Append (or integrate — depending on existing structure) to `src/lib/store.ts`:

```ts
import { create } from "zustand";
import { ipc } from "./ipc";
import { subscribe, type AppEvent } from "./events";
import type { CachedUsage, Settings } from "./types";

interface AppStore {
  usage: CachedUsage | null;
  settings: Settings | null;
  hasClaudeCodeCreds: boolean;
  authRequired: boolean;
  conflict: { oauth_email: string; cli_email: string } | null;
  stale: boolean;
  dbReset: boolean;

  init: () => Promise<void>;
  refreshSettings: () => Promise<void>;
  setSettings: (s: Settings) => Promise<void>;
  dismissBanner: (kind: "authRequired" | "stale" | "dbReset" | "conflict") => void;
}

export const useAppStore = create<AppStore>((set, get) => ({
  usage: null, settings: null, hasClaudeCodeCreds: false,
  authRequired: false, conflict: null, stale: false, dbReset: false,

  async init() {
    const [usage, settings, hasClaudeCodeCreds] = await Promise.all([
      ipc.getCurrentUsage(),
      ipc.getSettings(),
      ipc.hasClaudeCodeCreds().catch(() => false),
    ]);
    set({ usage, settings, hasClaudeCodeCreds });
    await subscribe((e: AppEvent) => {
      switch (e.type) {
        case "usage_updated": set({ usage: e.payload, authRequired: false, stale: false }); break;
        case "session_ingested": /* consumers re-fetch on demand */ break;
        case "auth_required": set({ authRequired: true }); break;
        case "auth_source_conflict": set({ conflict: e.payload }); break;
        case "stale_data": set({ stale: true }); break;
        case "db_reset": set({ dbReset: true }); break;
      }
    });
  },

  async refreshSettings() {
    const s = await ipc.getSettings();
    set({ settings: s });
  },

  async setSettings(s) {
    await ipc.updateSettings(s);
    set({ settings: s });
  },

  dismissBanner(kind) {
    switch (kind) {
      case "authRequired": set({ authRequired: false }); break;
      case "stale": set({ stale: false }); break;
      case "dbReset": set({ dbReset: false }); break;
      case "conflict": set({ conflict: null }); break;
    }
  },
}));
```

- [ ] **Step 3: Call `init` from `src/App.tsx`**

```tsx
import { useEffect } from "react";
import { useAppStore } from "./lib/store";
import "./styles/globals.css";
import "./styles/tokens.css";

export default function App() {
  const init = useAppStore(s => s.init);
  useEffect(() => { init(); }, [init]);
  return <div style={{ padding: 24 }}>Claude Usage Monitor — bootstrap OK</div>;
}
```

- [ ] **Step 4: Verify lint**

```bash
pnpm lint
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(store): zustand wiring for usage, settings, and banner state"
```

---

### Task 7.3: Machine-generated TypeScript bindings via tauri-specta

The hand-maintained `src/lib/types.ts` from Task 7.1 is a bootstrapping shim. This task switches to generated bindings so Rust and TS can never drift.

**Files:**
- Modify: `src-tauri/src/commands.rs` (add `#[derive(specta::Type)]`, `#[specta::specta]`)
- Modify: `src-tauri/src/app_state.rs`, `src-tauri/src/usage_api/types.rs`, `src-tauri/src/store/queries.rs`, `src-tauri/src/auth/orchestrator.rs`, `src-tauri/src/auth/mod.rs` (add `specta::Type` derives)
- Modify: `src-tauri/src/lib.rs` (register commands with tauri-specta, emit bindings)
- Create: `src-tauri/build_bindings.rs`
- Replace: `src/lib/types.ts` → `src/lib/generated/bindings.ts` (generated)
- Modify: `src/lib/ipc.ts` to import from the generated module

- [ ] **Step 1: Add `specta::Type` derives to every type crossing the IPC boundary**

For each of the following structs/enums, add `specta::Type` to the existing `#[derive(...)]` list:
- `app_state::Settings`, `app_state::CachedUsage`
- `auth::AuthSource`, `auth::AccountId`, `auth::orchestrator::AccountInfo`
- `usage_api::types::{Utilization, ExtraUsage, UsageSnapshot}`
- `store::queries::{StoredAccount, StoredSessionEvent}`
- `commands::{DailyBucket, ModelStats, ProjectStats, CacheStats}`
- `notifier::rules::Bucket`

Example (in `src-tauri/src/app_state.rs`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Settings { /* ... */ }
```

Note: `UsageSnapshot` contains `HashMap<String, serde_json::Value>` in its `unknown` field. Mark that field `#[specta(skip)]` and do the same for any `DateTime<Utc>` — specta's Chrono integration needs the `chrono` feature; alternatively, use `#[specta(type = String)]` to map it to TS `string`.

- [ ] **Step 2: Annotate commands with `#[specta::specta]`**

Every `#[tauri::command]` in `commands.rs` becomes `#[tauri::command]` + `#[specta::specta]`. Example:
```rust
#[tauri::command]
#[specta::specta]
pub async fn get_current_usage(state: State<'_, Arc<AppState>>) -> Result<Option<CachedUsage>, String> { /* ... */ }
```

- [ ] **Step 3: Register the command set with `tauri-specta` and emit bindings in `lib.rs`**

Replace the `generate_handler!` call with the tauri-specta builder. Keep the full command list — do not drop `debug_force_threshold`.

```rust
use tauri_specta::{collect_commands, Builder};

let specta_builder = Builder::<tauri::Wry>::new()
    .commands(collect_commands![
        commands::get_current_usage,
        commands::get_session_history,
        commands::get_daily_trends,
        commands::get_model_breakdown,
        commands::get_project_breakdown,
        commands::get_cache_stats,
        commands::start_oauth_flow,
        commands::submit_oauth_code,
        commands::use_claude_code_creds,
        commands::pick_auth_source,
        commands::sign_out,
        commands::has_claude_code_creds,
        commands::update_settings,
        commands::get_settings,
        #[cfg(debug_assertions)] commands::debug_force_threshold,
    ]);

#[cfg(debug_assertions)]
specta_builder
    .export(
        specta_typescript::Typescript::default().bigint(specta_typescript::BigIntExportBehavior::Number),
        "../src/lib/generated/bindings.ts",
    )
    .expect("failed to export bindings");

tauri::Builder::default()
    // … existing chain …
    .invoke_handler(specta_builder.invoke_handler())
    // … rest of chain …
```

- [ ] **Step 4: Delete `src/lib/types.ts` and rewrite `src/lib/ipc.ts` to import from generated bindings**

```bash
rm src/lib/types.ts
```

Update `src/lib/ipc.ts`:
```ts
import { commands as generated } from "./generated/bindings";
export { commands as ipc } from "./generated/bindings";
// Re-export generated types used across the frontend:
export type {
  AuthSource, CachedUsage, CacheStats, DailyBucket,
  ModelStats, ProjectStats, SessionEvent, Settings,
  Utilization, ExtraUsage, UsageSnapshot,
} from "./generated/bindings";
```

Update `src/lib/events.ts` to import `CachedUsage` from `./ipc` instead of `./types`. Anywhere in the frontend that imported from `./types` should switch to `./ipc`.

- [ ] **Step 5: First build emits bindings; verify**

```bash
cd src-tauri && cargo build && cd ..
ls -la src/lib/generated/bindings.ts
```

Expected: the file is generated with the Rust command/type surface.

- [ ] **Step 6: Add the generated path to `.gitignore` is NOT wanted** — commit the bindings file. The build emits, but we commit so downstream CI/tests don't need Rust to run frontend-only tests.

Remove `src/lib/generated/` from `.gitignore` (added in Task 0.1).

- [ ] **Step 7: Run tests end-to-end**

```bash
pnpm lint && pnpm test
cd src-tauri && cargo test && cd ..
```

Expected: green.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(ipc): machine-generated TS bindings via tauri-specta (Rust→TS type-safe)"
```

---

## Phase 8 — Compact Popover

### Task 8.1: CompactPopover + UsageBar integration with existing UI kit

**Files:**
- Create: `src/popover/CompactPopover.tsx`, `src/popover/UsageBar.tsx`, `src/popover/ResetCountdown.tsx`
- Test: `src/popover/__tests__/UsageBar.test.tsx`, `src/popover/__tests__/CompactPopover.test.tsx`

- [ ] **Step 1: Inspect existing `ProgressBar` component to reuse**

```bash
cat src/components/ui/ProgressBar.tsx
```

The designer's `ProgressBar` is already threshold-aware. `UsageBar` composes it with bucket semantics.

- [ ] **Step 2: Write `src/popover/ResetCountdown.tsx`**

```tsx
import { useEffect, useState } from "react";

function humanize(ms: number): string {
  if (ms <= 0) return "now";
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export function ResetCountdown({ resetsAt }: { resetsAt: string }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const i = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(i);
  }, []);
  const target = new Date(resetsAt).getTime();
  return <span className="font-mono text-xs opacity-70">Resets in {humanize(target - now)}</span>;
}
```

- [ ] **Step 3: Write `src/popover/UsageBar.tsx`**

```tsx
import { ProgressBar } from "@/components/ui/ProgressBar";
import { ResetCountdown } from "./ResetCountdown";
import type { Utilization } from "@/lib/types";

interface Props {
  label: string;
  data: Utilization | null;
  thresholds: number[];
}

export function UsageBar({ label, data, thresholds }: Props) {
  if (!data) {
    return (
      <div className="flex items-center justify-between py-2">
        <span className="text-sm opacity-60">{label}</span>
        <span className="font-mono text-xs opacity-40">n/a</span>
      </div>
    );
  }
  const pct = Math.round(data.utilization);
  return (
    <div className="py-2">
      <div className="mb-1 flex items-baseline justify-between">
        <span className="text-sm">{label}</span>
        <span className="font-mono text-sm tabular-nums">{pct}%</span>
      </div>
      <ProgressBar value={pct} thresholds={thresholds} />
      <div className="mt-1 flex justify-end">
        <ResetCountdown resetsAt={data.resets_at} />
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Write `src/popover/CompactPopover.tsx`**

```tsx
import { Banner } from "@/components/ui/Banner";
import { Card } from "@/components/ui/Card";
import { ProgressBar } from "@/components/ui/ProgressBar";
import { useAppStore } from "@/lib/store";
import { UsageBar } from "./UsageBar";

export function CompactPopover() {
  // Per-field selectors avoid re-rendering on unrelated store changes.
  const usage = useAppStore(s => s.usage);
  const thresholds = useAppStore(s => s.settings?.thresholds ?? [75, 90]);
  const authRequired = useAppStore(s => s.authRequired);
  const stale = useAppStore(s => s.stale);
  const conflict = useAppStore(s => s.conflict);
  const dismissBanner = useAppStore(s => s.dismissBanner);

  if (!usage) {
    return (
      <div className="flex h-full items-center justify-center p-6">
        <span className="opacity-60">Loading usage…</span>
      </div>
    );
  }
  const snap = usage.snapshot;
  const extra = snap.extra_usage;

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      {authRequired && <Banner tone="warn" onDismiss={() => dismissBanner("authRequired")}>Sign in to continue monitoring.</Banner>}
      {stale && <Banner tone="info" onDismiss={() => dismissBanner("stale")}>Data may be stale.</Banner>}
      {conflict && <Banner tone="warn">Two accounts detected — choose which to monitor in Settings.</Banner>}

      <Card variant="glass" className="p-4">
        <UsageBar label="5-hour" data={snap.five_hour} thresholds={thresholds} />
        <UsageBar label="7-day" data={snap.seven_day} thresholds={thresholds} />
        {(snap.seven_day_opus || snap.seven_day_sonnet) && (
          <div className="mt-2 grid grid-cols-2 gap-3 border-t border-white/5 pt-2">
            <UsageBar label="Opus" data={snap.seven_day_opus} thresholds={thresholds} />
            <UsageBar label="Sonnet" data={snap.seven_day_sonnet} thresholds={thresholds} />
          </div>
        )}
        {extra?.is_enabled && (
          <div className="mt-2 border-t border-white/5 pt-2">
            <ExtraUsageBar pct={extra.utilization} resetsAt={extra.resets_at} thresholds={thresholds} />
          </div>
        )}
      </Card>

      <div className="flex items-center justify-between text-xs opacity-60">
        <span className="truncate">{usage.account_email}</span>
        <button className="underline-offset-2 hover:underline">See details</button>
      </div>
    </div>
  );
}

/** Extra-usage credits: `resets_at` may be null for one-time top-ups.
 *  When null, render a different sub-label and omit the countdown. */
function ExtraUsageBar({ pct, resetsAt, thresholds }: { pct: number; resetsAt: string | null; thresholds: number[] }) {
  if (resetsAt) {
    return (
      <UsageBar
        label="Pay-as-you-go credits"
        data={{ utilization: pct, resets_at: resetsAt }}
        thresholds={thresholds}
      />
    );
  }
  const rounded = Math.round(pct);
  return (
    <div className="py-2">
      <div className="mb-1 flex items-baseline justify-between">
        <span className="text-sm">Pay-as-you-go credits</span>
        <span className="font-mono text-sm tabular-nums">{rounded}%</span>
      </div>
      <ProgressBar value={rounded} thresholds={thresholds} />
      <div className="mt-1 flex justify-end">
        <span className="font-mono text-xs opacity-70">No reset window</span>
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Write tests `src/popover/__tests__/UsageBar.test.tsx`**

```tsx
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { UsageBar } from "../UsageBar";

describe("UsageBar", () => {
  it("renders n/a when data is null", () => {
    render(<UsageBar label="5-hour" data={null} thresholds={[75, 90]} />);
    expect(screen.getByText("n/a")).toBeInTheDocument();
  });

  it("renders integer percentage", () => {
    render(<UsageBar label="5-hour" data={{ utilization: 42.7, resets_at: "2099-01-01T00:00:00Z" }} thresholds={[75, 90]} />);
    expect(screen.getByText("43%")).toBeInTheDocument();
  });
});
```

- [ ] **Step 6: Write tests `src/popover/__tests__/CompactPopover.test.tsx`**

```tsx
import { render, screen } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
import { useAppStore } from "@/lib/store";
import { CompactPopover } from "../CompactPopover";

function setUsage(opts: Partial<ReturnType<typeof makeSnap>> = {}) {
  const snap = makeSnap(opts);
  useAppStore.setState({
    usage: { snapshot: snap, account_id: "a", account_email: "a@b.com", last_error: null },
    settings: { polling_interval_secs: 300, thresholds: [75, 90], theme: "system", launch_at_login: false, crash_reports: false },
    authRequired: false, stale: false, conflict: null, dbReset: false,
  });
}

function makeSnap(opts: any) {
  return {
    five_hour: { utilization: 40, resets_at: "2099-01-01T00:00:00Z" },
    seven_day: { utilization: 60, resets_at: "2099-01-01T00:00:00Z" },
    seven_day_sonnet: null, seven_day_opus: null, extra_usage: null,
    fetched_at: new Date().toISOString(), unknown: {},
    ...opts,
  };
}

describe("CompactPopover", () => {
  beforeEach(() => setUsage());

  it("shows both primary bars", () => {
    render(<CompactPopover />);
    expect(screen.getByText("5-hour")).toBeInTheDocument();
    expect(screen.getByText("7-day")).toBeInTheDocument();
  });

  it("renders auth banner when authRequired", () => {
    useAppStore.setState({ authRequired: true });
    render(<CompactPopover />);
    expect(screen.getByText(/Sign in/i)).toBeInTheDocument();
  });

  it("shows extra-usage bar when enabled", () => {
    setUsage({ extra_usage: { is_enabled: true, monthly_limit_cents: 5000, used_credits_cents: 1000, utilization: 20, resets_at: null } });
    render(<CompactPopover />);
    expect(screen.getByText(/Pay-as-you-go/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 7: Run tests**

```bash
pnpm test
```

Expected: 5 passed.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(popover): compact popover with 5h/7d bars, Opus/Sonnet split, extra-usage"
```

---

## Phase 9 — Auth Screens

### Task 9.1: AuthPanel first-run + paste-back flow

**Files:**
- Create: `src/settings/AuthPanel.tsx`
- Test: `src/settings/__tests__/AuthPanel.test.tsx`

- [ ] **Step 1: Write `src/settings/AuthPanel.tsx`**

```tsx
import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { ipc } from "@/lib/ipc";

type Step = "choose" | "waiting" | "paste" | "submitting";

export function AuthPanel({ hasClaudeCodeCreds }: { hasClaudeCodeCreds: boolean }) {
  const [step, setStep] = useState<Step>("choose");
  const [code, setCode] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function startOauth() {
    setError(null);
    try {
      const url = await ipc.startOauthFlow();
      await openUrl(url);
      setStep("paste");
    } catch (e) {
      setError(String(e));
    }
  }

  async function submit() {
    setError(null);
    setStep("submitting");
    try {
      await ipc.submitOauthCode(code.trim());
      setStep("choose");
      setCode("");
    } catch (e) {
      setError(String(e));
      setStep("paste");
    }
  }

  async function useLocal() {
    setError(null);
    try { await ipc.useClaudeCodeCreds(); } catch (e) { setError(String(e)); }
  }

  return (
    <Card variant="glass" className="mx-auto mt-8 max-w-sm p-6">
      <h2 className="mb-4 text-lg font-medium">Sign in to Claude</h2>

      {step === "choose" && (
        <div className="flex flex-col gap-3">
          <Button variant="primary" onClick={startOauth}>Sign in with Claude</Button>
          {hasClaudeCodeCreds && (
            <Button variant="ghost" onClick={useLocal}>Use Claude Code credentials</Button>
          )}
        </div>
      )}

      {step === "paste" && (
        <div className="flex flex-col gap-3">
          <p className="text-sm opacity-80">
            Paste the code shown on the callback page:
          </p>
          <input
            autoFocus
            className="rounded-md border border-white/10 bg-white/5 px-3 py-2 font-mono text-sm"
            placeholder="code#state"
            value={code}
            onChange={e => setCode(e.target.value)}
          />
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={() => setStep("choose")}>Cancel</Button>
            <Button variant="primary" onClick={submit} disabled={!code.includes("#")}>Continue</Button>
          </div>
        </div>
      )}

      {step === "submitting" && <p className="text-sm opacity-70">Verifying…</p>}

      {error && <p className="mt-3 text-sm text-[color:var(--color-danger)]">{error}</p>}
    </Card>
  );
}
```

- [ ] **Step 2: Write tests `src/settings/__tests__/AuthPanel.test.tsx`**

```tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { AuthPanel } from "../AuthPanel";

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn().mockResolvedValue(undefined) }));
vi.mock("@/lib/ipc", () => ({
  ipc: {
    startOauthFlow: vi.fn().mockResolvedValue("https://claude.ai/oauth/authorize?x=1"),
    submitOauthCode: vi.fn().mockResolvedValue(undefined),
    useClaudeCodeCreds: vi.fn().mockResolvedValue(undefined),
  },
}));

describe("AuthPanel", () => {
  it("shows both buttons when Claude Code creds exist", () => {
    render(<AuthPanel hasClaudeCodeCreds={true} />);
    expect(screen.getByText("Sign in with Claude")).toBeInTheDocument();
    expect(screen.getByText("Use Claude Code credentials")).toBeInTheDocument();
  });

  it("hides the local-creds button when none exist", () => {
    render(<AuthPanel hasClaudeCodeCreds={false} />);
    expect(screen.queryByText("Use Claude Code credentials")).not.toBeInTheDocument();
  });

  it("disables Continue until code contains #", async () => {
    render(<AuthPanel hasClaudeCodeCreds={false} />);
    fireEvent.click(screen.getByText("Sign in with Claude"));
    const input = await screen.findByPlaceholderText("code#state");
    fireEvent.change(input, { target: { value: "abc123" } });
    expect(screen.getByText("Continue")).toBeDisabled();
    fireEvent.change(input, { target: { value: "abc123#state" } });
    expect(screen.getByText("Continue")).not.toBeDisabled();
  });
});
```

- [ ] **Step 3: Run**

```bash
pnpm test
```

Expected: 3 passed.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(auth-ui): first-run AuthPanel with paste-back flow"
```

---

### Task 9.2: Conflict chooser

**Files:**
- Create: `src/settings/AuthConflictChooser.tsx`

- [ ] **Step 1: Write `src/settings/AuthConflictChooser.tsx`**

```tsx
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { ipc } from "@/lib/ipc";
import { useAppStore } from "@/lib/store";

export function AuthConflictChooser() {
  const conflict = useAppStore(s => s.conflict);
  const dismiss = useAppStore(s => s.dismissBanner);
  if (!conflict) return null;

  async function pick(source: "OAuth" | "ClaudeCode") {
    await ipc.pickAuthSource(source);
    dismiss("conflict");
  }

  return (
    <Card variant="glass" className="mx-auto mt-8 max-w-sm p-6">
      <h2 className="mb-2 text-lg font-medium">Two Claude accounts detected</h2>
      <p className="mb-4 text-sm opacity-80">Which one should this app monitor?</p>
      <div className="flex flex-col gap-2">
        <Button variant="primary" onClick={() => pick("OAuth")}>
          {conflict.oauth_email} <span className="opacity-60">(signed in to this app)</span>
        </Button>
        <Button variant="ghost" onClick={() => pick("ClaudeCode")}>
          {conflict.cli_email} <span className="opacity-60">(Claude Code)</span>
        </Button>
      </div>
    </Card>
  );
}
```

- [ ] **Step 2: Verify build**

```bash
pnpm lint
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(auth-ui): conflict chooser for mixed-account scenarios"
```

---

## Phase 10 — Expanded Report

### Task 10.1: Tab shell + Sessions tab

**Files:**
- Create: `src/report/ExpandedReport.tsx`, `src/report/SessionsTab.tsx`

- [ ] **Step 1: Write `src/report/ExpandedReport.tsx`**

```tsx
import { useState } from "react";
import { Tabs } from "@/components/ui/Tabs";
import { SessionsTab } from "./SessionsTab";
import { ModelsTab } from "./ModelsTab";
import { TrendsTab } from "./TrendsTab";
import { ProjectsTab } from "./ProjectsTab";

const TABS = ["Sessions", "Models", "Trends", "Projects"] as const;
type TabKey = typeof TABS[number];

export function ExpandedReport() {
  const [active, setActive] = useState<TabKey>("Sessions");
  return (
    <div className="flex h-screen flex-col bg-[color:var(--color-bg)] text-[color:var(--color-text)]">
      <Tabs value={active} onValueChange={v => setActive(v as TabKey)} items={TABS.map(t => ({ value: t, label: t }))} />
      <div className="flex-1 overflow-auto p-6">
        {active === "Sessions" && <SessionsTab />}
        {active === "Models" && <ModelsTab />}
        {active === "Trends" && <TrendsTab />}
        {active === "Projects" && <ProjectsTab />}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Write `src/report/SessionsTab.tsx`**

```tsx
import { useEffect, useState } from "react";
import { ipc } from "@/lib/ipc";
import { EmptyState } from "@/components/ui/EmptyState";
import type { SessionEvent } from "@/lib/types";

export function SessionsTab() {
  const [events, setEvents] = useState<SessionEvent[] | null>(null);
  useEffect(() => { ipc.getSessionHistory(7).then(setEvents).catch(() => setEvents([])); }, []);
  if (events === null) return <p className="opacity-60">Loading…</p>;
  if (events.length === 0) return <EmptyState title="No sessions yet" description="Start a Claude Code session to see per-session data here." />;
  return (
    <table className="w-full text-sm">
      <thead><tr className="text-left opacity-60">
        <th className="py-1">When</th><th>Project</th><th>Model</th><th className="text-right">Input</th><th className="text-right">Output</th><th className="text-right">Cost</th>
      </tr></thead>
      <tbody>
        {events.map((e, i) => (
          <tr key={`${e.source_file}-${e.source_line}-${i}`} className="border-t border-white/5">
            <td className="py-1 font-mono text-xs opacity-80">{new Date(e.ts).toLocaleString()}</td>
            <td className="truncate">{e.project}</td>
            <td className="truncate font-mono text-xs">{e.model}</td>
            <td className="text-right tabular-nums">{e.input_tokens.toLocaleString()}</td>
            <td className="text-right tabular-nums">{e.output_tokens.toLocaleString()}</td>
            <td className="text-right tabular-nums">${e.cost_usd.toFixed(3)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
```

- [ ] **Step 3: Create stub files for the other tabs (real bodies in Task 10.2+)**

`src/report/ModelsTab.tsx`:
```tsx
export function ModelsTab() { return <p className="opacity-60">Models — coming in Task 10.2</p>; }
```

`src/report/TrendsTab.tsx`:
```tsx
export function TrendsTab() { return <p className="opacity-60">Trends — coming in Task 10.3</p>; }
```

`src/report/ProjectsTab.tsx`:
```tsx
export function ProjectsTab() { return <p className="opacity-60">Projects — coming in Task 10.3</p>; }
```

- [ ] **Step 4: Verify build**

```bash
pnpm lint
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(report): expanded report shell + Sessions tab"
```

---

### Task 10.2: Models tab (donut + Cache section)

**Files:**
- Create: `src/report/ModelsTab.tsx` (rewrite)

- [ ] **Step 1: Write `src/report/ModelsTab.tsx`**

```tsx
import { useEffect, useState } from "react";
import { PieChart, Pie, Cell, Legend, ResponsiveContainer, Tooltip } from "recharts";
import { ipc } from "@/lib/ipc";
import { Card } from "@/components/ui/Card";
import type { ModelStats, CacheStats } from "@/lib/types";

const COLORS = [
  "var(--color-accent)", "var(--color-warn)", "var(--color-danger)",
  "var(--color-success)", "var(--color-text-muted)",
];

export function ModelsTab() {
  const [models, setModels] = useState<ModelStats[] | null>(null);
  const [cache, setCache] = useState<CacheStats | null>(null);
  useEffect(() => {
    Promise.all([ipc.getModelBreakdown(30), ipc.getCacheStats(30)])
      .then(([m, c]) => { setModels(m); setCache(c); })
      .catch(() => { setModels([]); });
  }, []);
  if (models === null || cache === null) return <p className="opacity-60">Loading…</p>;
  const total = models.reduce((a, b) => a + b.cost_usd, 0);
  const data = models.map(m => ({ name: shortName(m.model), value: m.cost_usd }));

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
      <Card variant="glass" className="p-4">
        <h3 className="mb-2 text-sm font-medium opacity-80">Cost by model (30d) — ${total.toFixed(2)}</h3>
        <div className="h-64">
          <ResponsiveContainer>
            <PieChart>
              <Pie data={data} dataKey="value" innerRadius={60} outerRadius={100}>
                {data.map((_, i) => <Cell key={i} fill={COLORS[i % COLORS.length]} />)}
              </Pie>
              <Tooltip formatter={(v: number) => `$${v.toFixed(2)}`} />
              <Legend />
            </PieChart>
          </ResponsiveContainer>
        </div>
      </Card>
      <Card variant="glass" className="p-4">
        <h3 className="mb-2 text-sm font-medium opacity-80">Cache efficiency (30d)</h3>
        <dl className="grid grid-cols-2 gap-3 text-sm">
          <dt className="opacity-60">Hit ratio</dt>
          <dd className="text-right font-mono tabular-nums">{(cache.hit_ratio * 100).toFixed(1)}%</dd>
          <dt className="opacity-60">Cache reads</dt>
          <dd className="text-right font-mono tabular-nums">{cache.total_cache_read_tokens.toLocaleString()}</dd>
          <dt className="opacity-60">Cache writes</dt>
          <dd className="text-right font-mono tabular-nums">{cache.total_cache_creation_tokens.toLocaleString()}</dd>
          <dt className="opacity-60">Estimated savings</dt>
          <dd className="text-right font-mono tabular-nums">${cache.estimated_savings_usd.toFixed(2)}</dd>
        </dl>
      </Card>
    </div>
  );
}

function shortName(model: string): string {
  const m = model.match(/(opus|sonnet|haiku)-(\d+(?:-\d+)?)/i);
  return m ? `${m[1]} ${m[2]}` : model;
}
```

- [ ] **Step 2: Verify build**

```bash
pnpm lint
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(report): Models tab with donut chart and cache section"
```

---

### Task 10.3: Trends tab (30-day strip) + Projects tab

**Files:**
- Create: `src/report/TrendsTab.tsx` (rewrite), `src/report/ProjectsTab.tsx` (rewrite)

- [ ] **Step 1: Write `src/report/TrendsTab.tsx`**

```tsx
import { useEffect, useState } from "react";
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from "recharts";
import { ipc } from "@/lib/ipc";
import { Card } from "@/components/ui/Card";
import type { DailyBucket } from "@/lib/types";

export function TrendsTab() {
  const [data, setData] = useState<DailyBucket[] | null>(null);
  useEffect(() => { ipc.getDailyTrends(30).then(setData).catch(() => setData([])); }, []);
  if (data === null) return <p className="opacity-60">Loading…</p>;
  return (
    <Card variant="glass" className="p-4">
      <h3 className="mb-2 text-sm font-medium opacity-80">Cost trend (30 days)</h3>
      <div className="h-72">
        <ResponsiveContainer>
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
            <XAxis dataKey="date" tick={{ fontSize: 11 }} />
            <YAxis tickFormatter={v => `$${v.toFixed(2)}`} tick={{ fontSize: 11 }} />
            <Tooltip formatter={(v: number) => `$${v.toFixed(3)}`} />
            <Line type="monotone" dataKey="cost_usd" stroke="var(--color-accent)" strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </Card>
  );
}
```

- [ ] **Step 2: Write `src/report/ProjectsTab.tsx`**

```tsx
import { useEffect, useState } from "react";
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from "recharts";
import { ipc } from "@/lib/ipc";
import { Card } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";
import type { ProjectStats } from "@/lib/types";

export function ProjectsTab() {
  const [data, setData] = useState<ProjectStats[] | null>(null);
  useEffect(() => { ipc.getProjectBreakdown(30).then(setData).catch(() => setData([])); }, []);
  if (data === null) return <p className="opacity-60">Loading…</p>;
  if (data.length === 0) return <EmptyState title="No projects yet" description="Per-project breakdown will appear here." />;
  const sorted = [...data].sort((a, b) => b.total_cost_usd - a.total_cost_usd).slice(0, 15);
  return (
    <Card variant="glass" className="p-4">
      <h3 className="mb-2 text-sm font-medium opacity-80">Cost by project (top 15, 30d)</h3>
      <div className="h-80">
        <ResponsiveContainer>
          <BarChart data={sorted} layout="vertical" margin={{ left: 100 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
            <XAxis type="number" tickFormatter={v => `$${v.toFixed(2)}`} tick={{ fontSize: 11 }} />
            <YAxis type="category" dataKey="project" tick={{ fontSize: 11 }} width={100} />
            <Tooltip formatter={(v: number) => `$${v.toFixed(2)}`} />
            <Bar dataKey="total_cost_usd" fill="var(--color-accent)" />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </Card>
  );
}
```

- [ ] **Step 3: Verify build**

```bash
pnpm lint
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(report): Trends (30d) and Projects tabs"
```

---

## Phase 11 — Settings Screen

### Task 11.1: SettingsPanel with polling, thresholds, theme, autostart, crash-report toggle

**Files:**
- Create: `src/settings/SettingsPanel.tsx`
- Test: `src/settings/__tests__/SettingsPanel.test.tsx`

- [ ] **Step 1: Write `src/settings/SettingsPanel.tsx`**

```tsx
import { useEffect, useState } from "react";
import { Card } from "@/components/ui/Card";
import { Slider } from "@/components/ui/Slider";
import { Toggle } from "@/components/ui/Toggle";
import { Select } from "@/components/ui/Select";
import { Button } from "@/components/ui/Button";
import { useAppStore } from "@/lib/store";
import { ipc } from "@/lib/ipc";
import { enable as enableAutostart, disable as disableAutostart } from "@tauri-apps/plugin-autostart";
import type { Settings } from "@/lib/types";

export function SettingsPanel() {
  const settings = useAppStore(s => s.settings);
  const setSettings = useAppStore(s => s.setSettings);
  const [local, setLocal] = useState<Settings | null>(settings);
  useEffect(() => setLocal(settings), [settings]);

  if (!local) return <p className="opacity-60">Loading…</p>;

  const clamp = (n: number, min: number, max: number) => Math.min(max, Math.max(min, n));
  const pollingMinutes = Math.round(local.polling_interval_secs / 60);

  function update<K extends keyof Settings>(key: K, value: Settings[K]) {
    setLocal({ ...local!, [key]: value });
  }

  async function save() {
    const next = { ...local, polling_interval_secs: clamp(local.polling_interval_secs, 60, 1800) };
    await setSettings(next);
    try {
      if (next.launch_at_login) await enableAutostart();
      else await disableAutostart();
    } catch (e) { console.warn("autostart toggle failed", e); }
  }

  async function signOut() { await ipc.signOut(); }

  return (
    <div className="mx-auto max-w-md space-y-4 p-6">
      <Card variant="glass" className="p-4">
        <label className="flex items-center justify-between text-sm">
          <span>Polling interval</span>
          <span className="font-mono tabular-nums">{pollingMinutes}m</span>
        </label>
        <Slider
          min={1} max={30} step={1} value={pollingMinutes}
          onChange={v => update("polling_interval_secs", v * 60)}
        />
      </Card>

      <Card variant="glass" className="p-4">
        <h3 className="mb-2 text-sm font-medium">Alert thresholds</h3>
        {local.thresholds.map((t, i) => (
          <div key={i} className="mb-2">
            <label className="flex items-center justify-between text-sm">
              <span>Threshold {i + 1}</span>
              <span className="font-mono tabular-nums">{t}%</span>
            </label>
            <Slider
              min={25} max={95} step={5} value={t}
              onChange={v => {
                const ts = [...local.thresholds]; ts[i] = v; update("thresholds", ts);
              }}
            />
          </div>
        ))}
      </Card>

      <Card variant="glass" className="p-4">
        <div className="flex items-center justify-between py-1">
          <span className="text-sm">Launch at login</span>
          <Toggle checked={local.launch_at_login} onCheckedChange={v => update("launch_at_login", v)} />
        </div>
        <div className="flex items-center justify-between py-1">
          <span className="text-sm">Send anonymous crash reports</span>
          <Toggle checked={local.crash_reports} onCheckedChange={v => update("crash_reports", v)} />
        </div>
        <div className="flex items-center justify-between py-1">
          <span className="text-sm">Theme</span>
          <Select
            value={local.theme}
            onChange={v => update("theme", v)}
            items={[{ value: "system", label: "System" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]}
          />
        </div>
      </Card>

      <div className="flex justify-between">
        <Button variant="ghost" onClick={signOut}>Sign out</Button>
        <Button variant="primary" onClick={save}>Save</Button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Test `src/settings/__tests__/SettingsPanel.test.tsx`**

```tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { useAppStore } from "@/lib/store";
import { SettingsPanel } from "../SettingsPanel";

vi.mock("@tauri-apps/plugin-autostart", () => ({ enable: vi.fn(), disable: vi.fn() }));
vi.mock("@/lib/ipc", () => ({ ipc: { signOut: vi.fn() } }));

describe("SettingsPanel", () => {
  beforeEach(() => {
    useAppStore.setState({
      settings: { polling_interval_secs: 300, thresholds: [75, 90], theme: "system", launch_at_login: false, crash_reports: false },
      setSettings: vi.fn() as any,
    });
  });

  it("displays current polling interval in minutes", () => {
    render(<SettingsPanel />);
    expect(screen.getByText("5m")).toBeInTheDocument();
  });

  it("shows two threshold sliders when two thresholds are configured", () => {
    render(<SettingsPanel />);
    expect(screen.getByText("Threshold 1")).toBeInTheDocument();
    expect(screen.getByText("Threshold 2")).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: Run**

```bash
pnpm test
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(settings): configuration panel with polling, thresholds, autostart, crash-report stub"
```

---

## Phase 12 — Tray Integration & Single-Instance

### Task 12.1: Tray icon with color-coded badge

**Files:**
- Create: `src-tauri/src/tray.rs`, tray icon assets under `src-tauri/icons/tray/`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Generate simple placeholder tray icons**

Create 4 PNGs at 32×32 under `src-tauri/icons/tray/`:
- `idle-template.png` — monochrome (for macOS template image)
- `warn.png` — amber
- `danger.png` — red
- `paused.png` — monochrome with dot

You may use a script or any image editor. The shipping versions come later from the designer agent; placeholders are fine for functional wiring.

- [ ] **Step 2: Write `src-tauri/src/tray.rs`**

```rust
use tauri::image::Image;
use tauri::{AppHandle, Manager};

pub fn set_level(app: &AppHandle, pct: Option<f64>, paused: bool) {
    let (bytes, template) = pick(pct, paused);
    let tray = app.tray_by_id("main").or_else(|| app.tray_by_id("default"));
    if let Some(tray) = tray {
        let _ = tray.set_icon(Some(Image::from_bytes(bytes).expect("icon bytes")));
        let _ = tray.set_icon_as_template(template);
        if let Some(pct) = pct {
            let _ = tray.set_tooltip(Some(format!("Claude {}%", pct.round() as i64)));
        }
    }
}

fn pick(pct: Option<f64>, paused: bool) -> (&'static [u8], bool) {
    if paused { return (include_bytes!("../icons/tray/paused.png"), true); }
    match pct {
        Some(p) if p >= 90.0 => (include_bytes!("../icons/tray/danger.png"), false),
        Some(p) if p >= 75.0 => (include_bytes!("../icons/tray/warn.png"), false),
        _ => (include_bytes!("../icons/tray/idle-template.png"), true),
    }
}
```

- [ ] **Step 3: Create the tray in `setup`**

In `src-tauri/src/lib.rs`, replace/extend `.setup(...)` to construct the tray:
```rust
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

.setup(|app| {
    let handle = app.handle().clone();
    let state = app.state::<Arc<AppState>>().inner().clone();

    let show = MenuItem::with_id(app, "show", "Show popover", true, None::<&str>)?;
    let expand = MenuItem::with_id(app, "expand", "Open expanded report", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = MenuBuilder::new(app).items(&[&show, &expand, &quit]).build()?;

    TrayIconBuilder::with_id("main")
        .tooltip("Claude Usage Monitor")
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray/idle-template.png"))?)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => { if let Some(w) = app.get_webview_window("popover") { let _ = w.show(); } }
            "expand" => { /* Task 12.2 wires this */ }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("popover") {
                    if w.is_visible().unwrap_or(false) { let _ = w.hide(); } else { let _ = w.show(); let _ = w.set_focus(); }
                }
            }
        })
        .build(app)?;

    poll_loop::spawn(handle.clone(), state.clone());
    // … existing jsonl watcher wiring from Task 6.3 …
    Ok(())
})
```

- [ ] **Step 4: Declare the tray module in `lib.rs`**

Add `mod tray;` alongside the other `mod` declarations at the top of `src-tauri/src/lib.rs`.

(Note: `poll_loop.rs` already calls `crate::tray::set_level` from Task 6.2 — no additional wiring needed here.)

- [ ] **Step 5: Build and smoke-test**

```bash
cd src-tauri && cargo build && cd ..
pnpm tauri dev
```

Expected: tray icon appears. Left-click toggles popover visibility. Right-click shows menu.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(tray): color-coded tray icon with popover toggle and context menu"
```

---

### Task 12.2: Expanded window + single-instance behavior

**Files:**
- Modify: `src-tauri/tauri.conf.json`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/App.tsx`

- [ ] **Step 1: Add a second window config to `tauri.conf.json`**

```json
"windows": [
  { "label": "popover", "title": "", "width": 360, "height": 420, "resizable": false, "decorations": false, "transparent": true, "alwaysOnTop": true, "visible": false, "skipTaskbar": true },
  { "label": "report",  "title": "Claude Usage Monitor", "width": 960, "height": 640, "minWidth": 800, "minHeight": 560, "resizable": true, "visible": false }
]
```

- [ ] **Step 2: Add `open_expanded_window` command**

Append to `src-tauri/src/commands.rs`:
```rust
#[command]
pub async fn open_expanded_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("report") {
        let _ = w.show(); let _ = w.set_focus();
    }
    Ok(())
}
```
Register it in the `generate_handler!` list in `lib.rs`.

- [ ] **Step 3: Route rendering by window label in `src/App.tsx`**

```tsx
import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore } from "./lib/store";
import { CompactPopover } from "./popover/CompactPopover";
import { ExpandedReport } from "./report/ExpandedReport";
import { SettingsPanel } from "./settings/SettingsPanel";
import { AuthPanel } from "./settings/AuthPanel";
import { AuthConflictChooser } from "./settings/AuthConflictChooser";
import "./styles/globals.css";
import "./styles/tokens.css";

export default function App() {
  const init = useAppStore(s => s.init);
  const usage = useAppStore(s => s.usage);
  const conflict = useAppStore(s => s.conflict);
  const hasClaudeCodeCreds = useAppStore(s => s.hasClaudeCodeCreds);
  useEffect(() => { init(); }, [init]);

  const label = getCurrentWindow().label;

  if (conflict) return <AuthConflictChooser />;
  if (!usage) return <AuthPanel hasClaudeCodeCreds={hasClaudeCodeCreds} />;

  if (label === "popover") return <CompactPopover />;
  if (label === "report") return <ExpandedReport />;
  return <SettingsPanel />;
}
```

- [ ] **Step 4: Wire "See details" in `CompactPopover`**

In `src/popover/CompactPopover.tsx`, change the `<button>See details</button>` to:
```tsx
import { invoke } from "@tauri-apps/api/core";
...
<button className="underline-offset-2 hover:underline" onClick={() => invoke("open_expanded_window")}>See details</button>
```

- [ ] **Step 5: Verify single-instance behavior**

```bash
pnpm tauri dev
# Then in another terminal:
./src-tauri/target/debug/claude-usage-monitor
```

Expected: second invocation raises the already-running popover rather than starting a new instance.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(windows): expanded report window + See-details wiring + single-instance"
```

---

## Phase 13 — CI and Release Pipelines

### Task 13.1: GitHub Actions `test.yml` (matrix)

**Files:**
- Create: `.github/workflows/test.yml`

- [ ] **Step 1: Write `.github/workflows/test.yml`**

```yaml
name: test
on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install pnpm
        uses: pnpm/action-setup@v4
        with: { version: 9 }

      - name: Setup Node
        uses: actions/setup-node@v4
        with: { node-version: 20, cache: pnpm }

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with: { components: clippy }

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with: { workspaces: src-tauri }

      - name: Install Ubuntu deps
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Install JS deps
        run: pnpm install --frozen-lockfile

      - name: Frontend typecheck + tests
        run: |
          pnpm lint
          pnpm test

      - name: Rust tests
        working-directory: src-tauri
        run: cargo test --all-features --no-fail-fast

      - name: Rust clippy
        working-directory: src-tauri
        run: cargo clippy --all-targets -- -D warnings
```

- [ ] **Step 2: Verify locally by pushing to a test branch (optional)** — or just commit; CI will run on push.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore(ci): test matrix on ubuntu + macos + windows"
```

---

### Task 13.2: GitHub Actions `release.yml` (unsigned artifacts)

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write `.github/workflows/release.yml`**

```yaml
name: release
on:
  push:
    tags: ['v*']

jobs:
  release:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-latest
            target: universal-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install pnpm
        uses: pnpm/action-setup@v4
        with: { version: 9 }

      - name: Setup Node
        uses: actions/setup-node@v4
        with: { node-version: 20, cache: pnpm }

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install macOS universal targets
        if: matrix.os == 'macos-latest'
        run: rustup target add aarch64-apple-darwin x86_64-apple-darwin

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with: { workspaces: src-tauri }

      - name: Install JS deps
        run: pnpm install --frozen-lockfile

      - name: Build Tauri app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: ${{ github.ref_name }}
          releaseBody: 'Unsigned build — see README for first-launch instructions on each OS.'
          args: --target ${{ matrix.target }}
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "chore(ci): unsigned release workflow (macOS universal + Windows x86_64)"
```

---

## Phase 14 — README + Release Checklist

### Task 14.1: README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write `README.md`**

```markdown
# Claude Usage Monitor

Cross-platform menu-bar utility for monitoring Claude subscription rate-limits on macOS and Windows.

## Features
- 5-hour and 7-day usage buckets (with Opus / Sonnet splits)
- Extra-usage credits view (if enabled on your account)
- Per-session analytics from local Claude Code logs
- OAuth 2.0 + PKCE authentication (paste-back flow)
- Optional: reuse existing Claude Code credentials
- Threshold alerts at user-configured percentages

## First launch
Downloads are **unsigned**. On first launch:

- **macOS:** `xattr -d com.apple.quarantine "/Applications/Claude Usage Monitor.app"` or right-click → Open from Finder.
- **Windows:** SmartScreen → "More info" → "Run anyway".

WebView2 is required on Windows 10 (Windows 11 ships it). If missing, the installer auto-bootstraps it.

## Development
```bash
pnpm install
pnpm tauri dev
```

## License
MIT
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "docs: README with first-launch guidance"
```

---

### Task 14.2: Release checklist

**Files:**
- Create: `docs/release-checklist.md`

- [ ] **Step 1: Write `docs/release-checklist.md`**

```markdown
# Release Checklist

Before tagging a release, complete every item on both macOS and Windows.

## macOS (14+)
- [ ] Fresh install (download `.dmg`, drag to Applications, remove quarantine)
- [ ] OAuth paste-back: click "Sign in with Claude", complete in browser, paste `code#state`, verify usage loads
- [ ] Use Claude Code credentials shortcut: sign out, click "Use Claude Code credentials", verify usage loads
- [ ] `debug_force_threshold(five_hour, 75)` fires a notification once
- [ ] Re-run `debug_force_threshold(five_hour, 75)` before reset → no notification
- [ ] Open expanded report; all 4 tabs render
- [ ] Disconnect network → stale indicator appears within 15m; notifications do not fire
- [ ] System clock moved backward 2h → `CachedUsage` marks stale; countdown does not go negative

## Windows (11)
- [ ] Fresh install (`.msi`), SmartScreen "Run anyway"
- [ ] Repeat every macOS step that uses auth + tabs + debug threshold
- [ ] Verify DACL on `credentials.json` fallback (icacls shows user-only access)

## Windows (10)
- [ ] WebView2 auto-bootstrap succeeds
- [ ] Popover renders with translucent-solid fallback (no Mica)
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "docs: add release checklist"
```

---

## Self-Review

Reviewed against the full spec and the plan-review findings. Coverage map:

| Spec § | Task(s) |
|---|---|
| §1 requirements table | Whole plan — each requirement resolves to a specific task |
| §2 OAuth concrete values (client_id, redirect_uri, token_endpoint, scopes) | 3.1 (PKCE + URL), 3.2 (exchange + refresh), 6.1 (command bodies fully wired, including pending-PKCE state) |
| §2 required headers (Authorization, anthropic-beta, UA) | 2.2 (usage client), 3.5 (userinfo fetcher) |
| §2.5 parity matrix | 3.3 (macOS keychain multi-service), 3.4 (Windows plaintext file), 12.1 (tray template + ICO), 11.1 (autostart), 3.2 (keyring + ACL-restricted fallback on Windows) |
| §3 file layout | Matches tasks 1.1-1.2, 2.1-2.2, 3.1-3.5, 4.1-4.4, 5.1, 6.1-6.3, 7.1-7.3, 8.1, 9.1-9.2, 10.1-10.3, 11.1, 12.1-12.2 |
| §4 types | 2.1 (UsageSnapshot/Utilization/ExtraUsage — wire-faithful), 4.2 (SessionEvent — forward-compat), 6.1 (CachedUsage, Settings, AccountInfo) |
| §4 IPC | 6.1 — all commands have real bodies. Generated bindings in 7.3. |
| §4 events | `usage_updated`, `auth_required`, `auth_source_conflict`, `stale_data`, `session_ingested` all emitted from 6.2 + 6.3. `db_reset` deferred (see below). |
| §5 Scenario A paste-back | 3.1, 3.2, 6.1, 9.1, 9.2 |
| §5 Scenario B polling loop | 6.2 (immediate-first + edge-triggered stale_data + typed-error branching) |
| §5 Scenario C JSONL | 4.3 (walker — one-level, truncation-safe, partial-line-safe), 4.4 (watcher) |
| §5 Scenario D popover/report | 8.1, 10.1-10.3, 12.2 (expanded window + See-details wiring) |
| §5 Scenario E alerts | 5.1 (Option<resets_at> handling for extra_usage), 6.2 (notification fire) |
| §6 error handling | Distributed across 2.2, 3.2, 3.5, 4.3, 5.1, 6.2, 11.1 |
| §7 testing — Tier 1 | Unit tests in each module task |
| §7 testing — Tier 2 | 2.2, 3.2, 3.5, 4.3 integration tests |
| §7 testing — Tier 3 | 8.1, 9.1, 11.1 frontend tests |
| §7 CI matrix | 13.1 (Ubuntu + macOS + Windows, not only release) |
| §9 decisions — all 11 | Reflected in settings task 11.1 and defaults in 6.1 |
| §11 risks accepted | Covered in README and spec; no implementation work |

### Blocking-gap fixes from plan review

All three blocking gaps from `docs/spec-review.md` (v2) are closed:

1. **OAuth pending-state wiring** — `AppState` has `pending_oauth: RwLock<Option<PkcePair>>` and `fallback_dir: PathBuf`. `start_oauth_flow`, `submit_oauth_code`, and `sign_out` in Task 6.1 now have complete bodies (no Task 6.4 stub).
2. **`has_claude_code_creds` exposed to frontend** — new command in Task 6.1, TS binding in Task 7.1, fetched on init in Task 7.2 (`store.hasClaudeCodeCreds`), passed to `AuthPanel` in Task 12.2.
3. **tauri-specta codegen** — Task 7.3 produces `src/lib/generated/bindings.ts` and replaces the hand-maintained `types.ts`.

### Compilation-bug fixes from plan review

4. **Task 3.3 import path** — module is now at `src/auth/claude_code_creds/macos.rs` with `use super::super::StoredToken;` written correctly on first pass (no rename dance).
5. **Dispatcher compile** — uses cfg-gated `return` statements instead of cfg-gated blocks-as-statements.
6. **Task 6.1 placeholder removed** — `start_oauth_flow` has real body that writes the PKCE verifier to `state.pending_oauth`.
7. **Task 8.1 extra-usage `null` resets_at** — separate `ExtraUsageBar` component branches on `resets_at`, rendering "No reset window" when null. Matches the notifier's "Pay-as-you-go credits running low" message.

### Tooling fixes from plan review

8. **`pnpm tauri init`** — simplified to `--ci` only; `tauri.conf.json` Step 4 overwrite handles everything else.
9. **Cargo dependencies:**
   - `tauri` features: dropped `macos-private-api` (use `tauri-plugin-window-vibrancy` instead)
   - `specta` + `tauri-specta` + `specta-typescript` — pinned to exact RC versions
   - `keyring` — explicit `apple-native`, `windows-native`, `sync-secret-service` features
   - `fs2` → `fs4` (maintained replacement)
   - Added `parking_lot = "0.12"` to the main deps block
   - Added `whoami = "1"` to Windows target deps

### Smaller-issue fixes from plan review

10. **Task 4.3 walker** — `ProjectFallback` trait removed; correct code written directly; project-name fallback uses `ev.project.is_empty()` check.
11. **Task 4.3 truncation test** — real assertion: verifies dedup keeps row count at 3 and cursor is idempotent on subsequent calls.
12. **Task 3.5 orchestrator** — typed `AuthError::Conflict { oauth_email, cli_email }` variant replaces fragile string-based `anyhow!` signalling.
13. **Task 6.2 stale_data emission** — baked into the poll loop body (edge-triggered via `STALE_EMITTED` atomic, resets after any `PollResult::Ok`).
14. **Task 8.1 Zustand selectors** — each field subscribed separately via `s => s.field` to avoid whole-store re-renders.
15. **Self-review accuracy** — this list supersedes the prior "except one instance" claim; all TBDs have been eliminated.

### Type consistency

- `AuthSource` consistent: Rust enum (`OAuth | ClaudeCode`), TS string literal union (`"OAuth" | "ClaudeCode"`). Machine-generated by tauri-specta after Task 7.3.
- `CachedUsage` identical shape across Rust (Task 6.1) and generated TS (Task 7.3).
- `SessionEvent` TS mirrors Rust `StoredSessionEvent` (command return shape).
- Command names match across `tauri_specta::collect_commands![...]` (Rust) and generated `commands` object (frontend).

### Remaining known deferrals

- **`db_reset` event emission** — SQLite corruption recovery + event emission. Not wired in v1; the plan relies on the process failing to open the DB at all (current `Db::open` returns `Err`). Graceful recovery with a `.corrupt-<ts>` backup + re-backfill is a follow-up after first public release.
- **Extra-usage test coverage** — Task 5.1 tests the notifier's `Option<resets_at>` path; Task 8.1 tests the UsageBar N/A path but not the ExtraUsageBar "No reset window" path. Add a third `CompactPopover.test.tsx` case during implementation if tight on coverage.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-24-claude-usage-monitor.md`. Two execution options:

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task with two-stage review. Fast iteration, clean isolation. Use `superpowers:subagent-driven-development`.

**2. Inline Execution** — execute tasks sequentially in this session with checkpoints. Use `superpowers:executing-plans`.

Which approach?
