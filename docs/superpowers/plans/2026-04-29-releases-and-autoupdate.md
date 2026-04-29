# Releases & Auto-Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add in-app auto-update to the Claude Limits menubar app (macOS + Windows), using Tauri's updater plugin against GitHub Releases, with $0 infrastructure cost and a calm popover-only UX.

**Architecture:** Tauri `tauri-plugin-updater@2` polls a static `latest.json` hosted on GitHub Releases. Bundles are signed with an ed25519 updater keypair (separate from paid OS code signing — OS-level signing stays out of scope). The Rust side runs a background scheduler (on launch + every 6h), emits events for each state transition, and exposes two commands. The React side subscribes to events, stores state in a small Zustand slice, renders an `UpdateBanner` only when an update is staged, and adds a "Check for updates" link in the popover footer + tray menu.

**Tech Stack:**
- Rust: `tauri-plugin-updater@2`, `mockito` (tests), `tempfile` (tests), `tokio`
- TypeScript/React: Zustand, Framer Motion (existing), Lucide React (existing), Vitest + RTL
- CI: GitHub Actions (existing `release.yml`), `tauri-action`
- Distribution: GitHub Releases (existing — no new infra)

**Reference spec:** `docs/superpowers/specs/2026-04-29-releases-and-autoupdate-design.md`

**Repo:** `FeiXu-1131372/claude-limits` (used in URLs throughout this plan)

---

## File Map

### New files
- `src-tauri/src/updater/mod.rs` — entire Rust updater module (scheduler, events, commands)
- `src-tauri/src/updater/version.rs` — semver compare helper (TDD)
- `src-tauri/src/updater/persistence.rs` — `last_checked_at` JSON file IO (TDD)
- `src-tauri/src/updater/scheduler.rs` — interval delay calculation (TDD)
- `src/state/updateStore.ts` — Zustand slice for update state
- `src/state/updateStore.test.ts` — store reducer tests
- `src/lib/updateEvents.ts` — Tauri event listener
- `src/components/UpdateBanner.tsx` — popover banner (visible only when `ready` / `failed-install`)
- `src/components/UpdateBanner.test.tsx` — banner render tests
- `scripts/release.mjs` — version bump + commit + tag helper
- `scripts/generate-latest-json.mjs` — composes `latest.json` from CI artifacts

### Modified files
- `src-tauri/Cargo.toml` — add `tauri-plugin-updater` dep
- `src-tauri/src/lib.rs` — register updater plugin, register two commands, kick off scheduler in `setup()`, add "Check for Updates" tray menu item
- `src-tauri/tauri.conf.json` — add `plugins.updater` block
- `src-tauri/capabilities/default.json` — add updater permissions
- `src/App.tsx` — mount the event listener once on startup
- `src/popover/CompactPopover.tsx` — render `<UpdateBanner />` at top + version line at bottom
- `vite.config.ts` — inject `__APP_VERSION__` from `package.json`
- `package.json` — add `release` npm script
- `.github/workflows/release.yml` — add signing secrets, updater bundles, manifest job
- `README.md` — document update behavior + manual one-time first upgrade

---

## Sequencing notes

- **Phase 1** (Task 1) is one-time manual setup. Cannot be parallelized; everything else blocks on it because the public key must be in `tauri.conf.json` before any build can include the updater plugin.
- **Phase 2** (Tasks 2–10) is the Rust backend, with TDD-able tasks (4, 5, 6) interleaved with wiring tasks.
- **Phase 3** (Tasks 11–16) is the frontend — independent of Phase 2 once the Tauri events contract is fixed in Task 7. Phases 2 and 3 can be parallelized after Task 7.
- **Phase 4** (Tasks 17–19) is the release pipeline. Only relevant when shipping the first updater-enabled release.
- **Phase 5** (Task 20) is documentation, last.

Each task is meant to result in a passing test/build and a commit.

---

## Phase 1 — One-time setup

### Task 1: Generate updater keypair and configure secrets

**Files:**
- Local-only: `~/.tauri/claude-limits.key` (PRIVATE — never committed)
- Modify: `src-tauri/tauri.conf.json` (will be done in Task 2 once we have the public key in hand)
- GitHub Actions secrets (web UI)

This task has no code. It's a manual setup that produces three artifacts:
1. The ed25519 public key string (embedded in the app via `tauri.conf.json`)
2. The private key file contents (stored as a GH Actions secret)
3. The password for the private key (stored as a separate GH Actions secret)

- [ ] **Step 1: Install Tauri CLI if not already**

Run: `pnpm tauri --version`
Expected: prints `tauri-cli x.y.z`. If "command not found", install: `pnpm add -D @tauri-apps/cli@^2.1.0` (already in `package.json` per existing setup).

- [ ] **Step 2: Generate keypair locally**

Run:
```bash
mkdir -p ~/.tauri
pnpm tauri signer generate -w ~/.tauri/claude-limits.key
```

When prompted for a password, choose a strong one. Save it somewhere safe (1Password / your password manager).

Expected output:
```
Your keypair was generated successfully
Private: /Users/feixu/.tauri/claude-limits.key (Keep it secret!)
Public: dW50cnVzdGVkIGNvbW1lbnQ6...   ← long base64 string

To sign your updates, sign with:
  TAURI_SIGNING_PRIVATE_KEY=...
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD=...
```

- [ ] **Step 3: Save the public key string for Task 2**

Run: `cat ~/.tauri/claude-limits.key.pub`
Copy the output. You will paste it into `tauri.conf.json` `plugins.updater.pubkey` in the next task.

- [ ] **Step 4: Add private key to GitHub Actions secrets**

Open: `https://github.com/FeiXu-1131372/claude-limits/settings/secrets/actions`

Create two repository secrets:
1. Name: `TAURI_SIGNING_PRIVATE_KEY` — Value: the **entire contents** of `~/.tauri/claude-limits.key` (run `cat ~/.tauri/claude-limits.key` and paste).
2. Name: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — Value: the password you chose in Step 2.

- [ ] **Step 5: Verify secrets exist**

In the GitHub web UI, both secret names should now appear in the Actions secrets list. (The values are write-only and can't be displayed, only overwritten.)

- [ ] **Step 6: No commit needed**

Nothing to commit yet. The public key gets committed in Task 2 as part of `tauri.conf.json`.

---

## Phase 2 — Rust backend

### Task 2: Add `tauri-plugin-updater` dependency and config

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Add the Cargo dependency**

In `src-tauri/Cargo.toml`, in the `[dependencies]` block (after the existing `tauri-plugin-shell = "2"` line), add:

```toml
tauri-plugin-updater = "2"
```

- [ ] **Step 2: Add the updater plugin block to `tauri.conf.json`**

Current `src-tauri/tauri.conf.json` has no `plugins` key. Add it as a sibling of `app` and `bundle`:

```json
{
  "productName": "Claude Limits",
  "identifier": "com.claude-limits.app",
  "build": { ... unchanged ... },
  "app": { ... unchanged ... },
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/FeiXu-1131372/claude-limits/releases/latest/download/latest.json"
      ],
      "pubkey": "PASTE_PUBLIC_KEY_FROM_TASK_1_STEP_3",
      "windows": { "installMode": "passive" }
    }
  },
  "bundle": { ... unchanged ... }
}
```

Replace `PASTE_PUBLIC_KEY_FROM_TASK_1_STEP_3` with the actual base64 string copied in Task 1.

- [ ] **Step 3: Add updater permissions to capabilities**

In `src-tauri/capabilities/default.json`, in the `permissions` array, add three entries (preserve existing ones):

```json
"permissions": [
  ... existing entries ...,
  "updater:default",
  "updater:allow-check",
  "updater:allow-download-and-install"
]
```

- [ ] **Step 4: Verify the Cargo build still works**

Run: `cd src-tauri && cargo check --all-features`
Expected: clean compile (the dependency is added but not yet used; no warnings about unused crate yet because there's no `use` of it).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "chore(updater): add tauri-plugin-updater dependency and config"
```

---

### Task 3: Create updater module skeleton

**Files:**
- Create: `src-tauri/src/updater/mod.rs`
- Modify: `src-tauri/src/lib.rs:7-11` (declare module)

- [ ] **Step 1: Create the module file with public surface**

Create `src-tauri/src/updater/mod.rs`:

```rust
//! Auto-update orchestration: scheduler, events, install.
//!
//! The actual download + signature verification + install is handled by
//! `tauri-plugin-updater`; this module owns the policy (when to check,
//! what to emit, how to persist last-checked time) and the two Tauri
//! commands the frontend invokes.

mod persistence;
mod scheduler;
mod version;

use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum UpdatePhase {
    Check,
    Download,
    Verify,
    Install,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateOutcome {
    UpToDate,
    Ready { version: String },
    Failed { phase: UpdatePhase, message: String },
}
```

- [ ] **Step 2: Declare the module in `lib.rs`**

In `src-tauri/src/lib.rs`, add `mod updater;` to the existing module list (around lines 1-11). The block should now look like:

```rust
mod app_state;
pub mod auth;
mod commands;
pub mod jsonl_parser;
mod logging;
pub mod notifier;
mod poll_loop;
pub mod store;
mod tray;
mod tray_icon;
mod updater;
pub mod usage_api;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: clean compile. May warn about unused `UpdateOutcome` variants — ignore for now, those will be used in later tasks. If warnings block via `-D warnings` in CI clippy, prefix the enum with `#[allow(dead_code)]` for now.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/updater/mod.rs src-tauri/src/lib.rs
git commit -m "feat(updater): add module skeleton + UpdateOutcome enum"
```

---

### Task 4: Version comparison helper (TDD)

**Files:**
- Create: `src-tauri/src/updater/version.rs`

We need a single function: given the running app's version (from `CARGO_PKG_VERSION`) and the manifest's version string, return `true` if the manifest version is strictly newer. Tauri's plugin does its own check, but we want an explicit predicate so we can unit-test the boundary cases the plugin upstream may not.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/updater/version.rs`:

```rust
//! Strict semver "is newer" comparison for the updater.
//!
//! We deliberately re-implement (rather than depend on the `semver` crate)
//! because the manifest only ever contains MAJOR.MINOR.PATCH for this app
//! — no prerelease, no build metadata — and pulling in 50KB of code for
//! one comparison would be silly.

pub fn is_newer(running: &str, candidate: &str) -> Result<bool, String> {
    let r = parse(running)?;
    let c = parse(candidate)?;
    Ok(c > r)
}

fn parse(s: &str) -> Result<(u32, u32, u32), String> {
    let parts: Vec<&str> = s.trim_start_matches('v').split('.').collect();
    if parts.len() != 3 {
        return Err(format!("expected MAJOR.MINOR.PATCH, got {s:?}"));
    }
    let mut out = [0u32; 3];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p
            .parse::<u32>()
            .map_err(|e| format!("invalid version part {p:?}: {e}"))?;
    }
    Ok((out[0], out[1], out[2]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn detects_newer_patch() {
        assert_eq!(is_newer("0.1.0", "0.1.1"), Ok(true));
    }

    #[test]
    fn detects_newer_minor() {
        assert_eq!(is_newer("0.1.5", "0.2.0"), Ok(true));
    }

    #[test]
    fn detects_newer_major() {
        assert_eq!(is_newer("0.9.9", "1.0.0"), Ok(true));
    }

    #[test]
    fn rejects_same_version() {
        assert_eq!(is_newer("0.2.0", "0.2.0"), Ok(false));
    }

    #[test]
    fn rejects_older() {
        assert_eq!(is_newer("0.2.0", "0.1.9"), Ok(false));
    }

    #[test]
    fn handles_double_digit_minor() {
        // 0.10.0 > 0.9.0 — naive string compare would fail this
        assert_eq!(is_newer("0.9.0", "0.10.0"), Ok(true));
    }

    #[test]
    fn strips_leading_v() {
        assert_eq!(is_newer("v0.1.0", "v0.2.0"), Ok(true));
    }

    #[test]
    fn rejects_malformed_running() {
        assert!(is_newer("not-a-version", "0.2.0").is_err());
    }

    #[test]
    fn rejects_malformed_candidate() {
        assert!(is_newer("0.1.0", "0.2").is_err());
    }
}
```

- [ ] **Step 2: Run the tests (they should pass — implementation written alongside)**

Strictly we wrote impl + tests together for brevity. Run them now:

Run: `cd src-tauri && cargo test --package claude-limits updater::version`
Expected: 9 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/updater/version.rs
git commit -m "feat(updater): version comparison helper with semver-correct double-digit handling"
```

---

### Task 5: Persistence helper for `last_checked_at` (TDD)

**Files:**
- Create: `src-tauri/src/updater/persistence.rs`

The scheduler needs to remember when it last checked, so a quick relaunch doesn't trigger an immediate re-check. We persist a single ISO-8601 timestamp to a small JSON file in the app data dir. Failures to read or write are logged but never propagated — this is best-effort.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/updater/persistence.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests**

Run: `cd src-tauri && cargo test --package claude-limits updater::persistence`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/updater/persistence.rs
git commit -m "feat(updater): last_checked_at persistence helper"
```

---

### Task 6: Scheduler delay calculation (TDD)

**Files:**
- Create: `src-tauri/src/updater/scheduler.rs`

The scheduler runs a check on launch + every 6h while running. Encapsulate the "how long should I sleep next?" math in a pure function so we can test the boundary conditions. The actual `tokio` task lives in `mod.rs` and just calls this helper.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/updater/scheduler.rs`:

```rust
//! Schedule math for the updater background task. Pure function +
//! tests; the tokio loop that calls it lives in `super::run_scheduler`.

use chrono::{DateTime, Duration, Utc};

pub const CHECK_INTERVAL_HOURS: i64 = 6;

/// How long until the next check should run, given when we last checked
/// (or `None` if never). Returns `Duration::zero()` when overdue.
pub fn delay_until_next_check(
    now: DateTime<Utc>,
    last_checked_at: Option<DateTime<Utc>>,
) -> Duration {
    let interval = Duration::hours(CHECK_INTERVAL_HOURS);
    match last_checked_at {
        None => Duration::zero(), // never checked → check immediately
        Some(prev) => {
            let elapsed = now - prev;
            if elapsed >= interval {
                Duration::zero()
            } else {
                interval - elapsed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use pretty_assertions::assert_eq;

    fn t(h: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 29, 0, 0, 0).unwrap() + Duration::hours(h)
    }

    #[test]
    fn never_checked_means_check_now() {
        assert_eq!(delay_until_next_check(t(0), None), Duration::zero());
    }

    #[test]
    fn just_checked_waits_full_interval() {
        let prev = t(0);
        let now = t(0);
        assert_eq!(delay_until_next_check(now, Some(prev)), Duration::hours(6));
    }

    #[test]
    fn three_hours_ago_waits_three_more() {
        let prev = t(0);
        let now = t(3);
        assert_eq!(delay_until_next_check(now, Some(prev)), Duration::hours(3));
    }

    #[test]
    fn overdue_means_check_now() {
        let prev = t(0);
        let now = t(10);
        assert_eq!(delay_until_next_check(now, Some(prev)), Duration::zero());
    }

    #[test]
    fn exactly_at_interval_means_check_now() {
        let prev = t(0);
        let now = t(6);
        assert_eq!(delay_until_next_check(now, Some(prev)), Duration::zero());
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cd src-tauri && cargo test --package claude-limits updater::scheduler`
Expected: 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/updater/scheduler.rs
git commit -m "feat(updater): scheduler delay calculation helper"
```

---

### Task 7: Updater orchestration in `mod.rs`

**Files:**
- Modify: `src-tauri/src/updater/mod.rs`

Glue the helpers together: a single `check_and_emit` function that asks the Tauri updater plugin for an update, downloads + installs if found, and emits events at every state transition. Plus the public `run_scheduler` function that loops forever calling `check_and_emit` at the right cadence.

- [ ] **Step 1: Add the orchestration code**

Replace the current contents of `src-tauri/src/updater/mod.rs` with:

```rust
//! Auto-update orchestration: scheduler, events, install.
//!
//! The actual download + signature verification + install is handled by
//! `tauri-plugin-updater`; this module owns the policy (when to check,
//! what to emit, how to persist last-checked time) and the two Tauri
//! commands the frontend invokes.

mod persistence;
mod scheduler;
mod version;

use chrono::Utc;
use serde::Serialize;
use specta::Type;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum UpdatePhase {
    Check,
    Download,
    Verify,
    Install,
}

/// In-flight guard so concurrent triggers (timer + manual click) collapse
/// into a single check.
#[derive(Default)]
pub struct UpdaterGuard {
    pub busy: Mutex<bool>,
}

const EV_CHECKING: &str = "update://checking";
const EV_UP_TO_DATE: &str = "update://up-to-date";
const EV_AVAILABLE: &str = "update://available";
const EV_PROGRESS: &str = "update://progress";
const EV_READY: &str = "update://ready";
const EV_FAILED: &str = "update://failed";

pub fn data_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
}

/// Single check cycle. Emits events as it progresses. Never panics.
pub async fn check_and_emit(app: &AppHandle) {
    // Reentrancy guard. If a check is already running, do nothing.
    let guard = app.state::<Arc<UpdaterGuard>>();
    {
        let mut busy = guard.busy.lock().await;
        if *busy {
            tracing::debug!("update check already in flight; skipping");
            return;
        }
        *busy = true;
    }
    let _release = ReleaseOnDrop { app: app.clone() };

    let _ = app.emit(EV_CHECKING, ());

    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            emit_failed(app, UpdatePhase::Check, format!("updater unavailable: {e}"));
            return;
        }
    };

    let maybe_update = match updater.check().await {
        Ok(u) => u,
        Err(e) => {
            emit_failed(app, UpdatePhase::Check, format!("check failed: {e}"));
            return;
        }
    };

    // Persist successful check time (whether or not an update exists).
    persistence::write_last_checked_at(&data_dir(app), Utc::now());

    let Some(update) = maybe_update else {
        let _ = app.emit(EV_UP_TO_DATE, serde_json::json!({ "checkedAt": Utc::now() }));
        return;
    };

    let new_version = update.version.clone();
    let _ = app.emit(
        EV_AVAILABLE,
        serde_json::json!({
            "version": new_version,
            "notes": update.body.clone().unwrap_or_default(),
            "pubDate": update.date.map(|d| d.to_string()).unwrap_or_default(),
        }),
    );

    // Throttled progress emit: at most 5 per second.
    let mut last_progress_emit = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(1))
        .unwrap_or_else(std::time::Instant::now);
    let mut downloaded: u64 = 0;
    let app_for_progress = app.clone();

    let download_result = update
        .download(
            move |chunk_len, content_len| {
                downloaded += chunk_len as u64;
                if last_progress_emit.elapsed() >= std::time::Duration::from_millis(200) {
                    last_progress_emit = std::time::Instant::now();
                    let _ = app_for_progress.emit(
                        EV_PROGRESS,
                        serde_json::json!({
                            "downloaded": downloaded,
                            "total": content_len.unwrap_or(0),
                        }),
                    );
                }
            },
            || {},
        )
        .await;

    let bytes = match download_result {
        Ok(b) => b,
        Err(e) => {
            // tauri-plugin-updater's `download` performs signature verification
            // internally; a sig failure surfaces here. Distinguish by string match
            // — coarse but the plugin doesn't expose a typed error variant.
            let msg = e.to_string();
            let phase = if msg.contains("signature") || msg.contains("verify") {
                UpdatePhase::Verify
            } else {
                UpdatePhase::Download
            };
            emit_failed(app, phase, msg);
            return;
        }
    };

    if let Err(e) = update.install(bytes) {
        emit_failed(app, UpdatePhase::Install, format!("install staging failed: {e}"));
        return;
    }

    let _ = app.emit(EV_READY, serde_json::json!({ "version": new_version }));
}

/// Triggers `update.install()` to actually run the staged installer.
/// Tauri's plugin restarts the app for us.
pub async fn install_now(app: &AppHandle) -> Result<(), String> {
    // We re-fetch + re-download because Tauri's API doesn't expose a
    // "install the previously-staged bytes" entry point. The download
    // is small (~10MB) and the bytes are already on a CDN; this is fine.
    // (If users complain about a delay between clicking and restart,
    // we can revisit by holding the bytes in memory between check_and_emit
    // and install_now.)
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Err("no update available".into());
    };
    let bytes = update
        .download(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    update.install(bytes).map_err(|e| e.to_string())?;
    // Tauri's installer relaunches the app on success; control rarely returns here.
    app.restart();
}

/// Background task: on launch (after `delay_until_next_check`) and every
/// 6h thereafter, run a single check cycle.
pub fn run_scheduler(app: AppHandle) {
    #[cfg(debug_assertions)]
    {
        tracing::info!("updater scheduler disabled in dev build");
        let _ = app;
        return;
    }

    #[cfg(not(debug_assertions))]
    tauri::async_runtime::spawn(async move {
        loop {
            let last = persistence::read_last_checked_at(&data_dir(&app));
            let delay = scheduler::delay_until_next_check(Utc::now(), last);
            let std_delay = delay
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(0));
            if !std_delay.is_zero() {
                tokio::time::sleep(std_delay).await;
            }
            check_and_emit(&app).await;
            // Sleep the full interval before next cycle.
            tokio::time::sleep(std::time::Duration::from_secs(
                (scheduler::CHECK_INTERVAL_HOURS as u64) * 3600,
            ))
            .await;
        }
    });
}

fn emit_failed(app: &AppHandle, phase: UpdatePhase, message: String) {
    tracing::warn!(?phase, %message, "update cycle failed");
    let _ = app.emit(
        EV_FAILED,
        serde_json::json!({
            "phase": match phase {
                UpdatePhase::Check => "check",
                UpdatePhase::Download => "download",
                UpdatePhase::Verify => "verify",
                UpdatePhase::Install => "install",
            },
            "message": message,
        }),
    );
}

struct ReleaseOnDrop {
    app: AppHandle,
}
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        let app = self.app.clone();
        tauri::async_runtime::spawn(async move {
            let guard = app.state::<Arc<UpdaterGuard>>();
            *guard.busy.lock().await = false;
        });
    }
}

// Re-exported for use by `lib.rs` and `commands.rs`.
pub use version::is_newer;
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check --all-features`
Expected: clean compile. Some warnings about `is_newer` being unused are OK — it's pub for future use.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/updater/mod.rs
git commit -m "feat(updater): scheduler + check_and_emit + install_now orchestration"
```

---

### Task 8: Tauri commands

**Files:**
- Modify: `src-tauri/src/commands.rs` (add two commands at the bottom)

- [ ] **Step 1: Add the commands**

Open `src-tauri/src/commands.rs` (whichever line is the last `#[tauri::command]` — append after it):

```rust
#[tauri::command]
#[specta::specta]
pub async fn check_for_updates_now(app: tauri::AppHandle) -> Result<(), String> {
    crate::updater::check_and_emit(&app).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    crate::updater::install_now(&app).await
}
```

(Keep them on the production-mode list; they should be available in dev too, but since `run_scheduler` is a no-op in dev and `check_and_emit` will fail fast there, this is harmless.)

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: clean compile.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(updater): expose check_for_updates_now + install_update commands"
```

---

### Task 9: Wire updater into `lib.rs` setup + register commands + state + plugin

**Files:**
- Modify: `src-tauri/src/lib.rs`

We need to:
1. Register the `tauri-plugin-updater` plugin
2. Manage an `Arc<UpdaterGuard>` in app state
3. Add the two new commands to both specta `commands![]` lists
4. Call `updater::run_scheduler` from `setup()`

- [ ] **Step 1: Add the plugin registration**

In `src-tauri/src/lib.rs`, after `.plugin(tauri_plugin_dialog::init())` (around line 109), add:

```rust
.plugin(tauri_plugin_updater::Builder::new().build())
```

- [ ] **Step 2: Manage the UpdaterGuard**

Find the existing `.manage(app_state)` line (~line 97). Replace it with two `.manage(...)` calls:

```rust
.manage(app_state)
.manage(std::sync::Arc::new(crate::updater::UpdaterGuard::default()))
```

- [ ] **Step 3: Add commands to BOTH specta lists**

Both the `#[cfg(not(debug_assertions))]` (line ~46) and `#[cfg(debug_assertions)]` (line ~67) `tauri_specta::collect_commands![...]` calls need the two new commands appended (with trailing comma):

```rust
commands::check_for_updates_now,
commands::install_update,
```

- [ ] **Step 4: Kick off the scheduler in setup**

Inside the `setup(|app| { ... })` block, after the existing `poll_loop::spawn(handle.clone(), state.clone());` line (~line 225), add:

```rust
crate::updater::run_scheduler(handle.clone());
```

- [ ] **Step 5: Verify everything compiles**

Run: `cd src-tauri && cargo build --release` (full build to make sure the production path compiles cleanly).
Expected: clean build. Watch for warnings about unused imports — fix any.

- [ ] **Step 6: Verify specta exports the new commands**

Run: `cd src-tauri && cargo check` (this re-runs the specta export side-effect in dev builds).
Then: `grep -E "checkForUpdatesNow|installUpdate" ../src/lib/generated/bindings.ts`
Expected: both function names appear.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src/lib/generated/bindings.ts
git commit -m "feat(updater): wire plugin + state + commands + scheduler in lib.rs"
```

---

### Task 10: Add "Check for Updates" tray menu item

**Files:**
- Modify: `src-tauri/src/lib.rs` (the inline tray menu construction at lines ~180-223)

- [ ] **Step 1: Add the menu item between "show" and "quit"**

In `src-tauri/src/lib.rs`, find:

```rust
let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;
```

Replace with:

```rust
let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
let check_updates = MenuItem::with_id(
    app,
    "check_updates",
    "Check for Updates…",
    true,
    None::<&str>,
)?;
let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
let menu = MenuBuilder::new(app)
    .items(&[&show, &check_updates, &quit])
    .build()?;
```

- [ ] **Step 2: Handle the menu event**

Find the `tray.on_menu_event(...)` block. Add a `"check_updates"` arm:

```rust
tray.on_menu_event(|app, event| match event.id.as_ref() {
    "show" => {
        if let Some(w) = app.get_webview_window("popover") {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
    "check_updates" => {
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            crate::updater::check_and_emit(&app_clone).await;
        });
    }
    "quit" => app.exit(0),
    _ => {}
});
```

- [ ] **Step 3: Verify build**

Run: `cd src-tauri && cargo build`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(updater): add Check for Updates… tray menu item"
```

---

## Phase 3 — Frontend

### Task 11: Inject `__APP_VERSION__` via Vite

**Files:**
- Modify: `vite.config.ts`

- [ ] **Step 1: Read current `vite.config.ts`**

Run: `cat vite.config.ts`
You'll see a `defineConfig({ ... })` call. We need to add a `define` block.

- [ ] **Step 2: Inject the version constant**

At the top of `vite.config.ts`, add:

```ts
import pkg from './package.json' with { type: 'json' };
```

Inside the `defineConfig({ ... })` object, add (alongside `plugins`, `server`, etc.):

```ts
define: {
  __APP_VERSION__: JSON.stringify(pkg.version),
},
```

- [ ] **Step 3: Add the global type declaration**

Create or append to `src/global.d.ts` (create it if missing):

```ts
declare const __APP_VERSION__: string;
```

If the file already exists with other declarations, append the line.

- [ ] **Step 4: Verify the build still works**

Run: `pnpm lint && pnpm build`
Expected: both succeed.

- [ ] **Step 5: Commit**

```bash
git add vite.config.ts src/global.d.ts
git commit -m "build: inject __APP_VERSION__ from package.json"
```

---

### Task 12: Zustand `updateStore` + tests (TDD)

**Files:**
- Create: `src/state/updateStore.ts`
- Create: `src/state/updateStore.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/state/updateStore.test.ts`:

```ts
import { describe, it, expect, beforeEach } from 'vitest';
import { useUpdateStore } from './updateStore';

describe('updateStore', () => {
  beforeEach(() => {
    useUpdateStore.setState({
      status: 'idle',
      version: null,
      progress: 0,
      error: null,
      lastCheckedAt: null,
    });
  });

  it('starts idle', () => {
    expect(useUpdateStore.getState().status).toBe('idle');
  });

  it('transitions to checking', () => {
    useUpdateStore.getState().setStatus('checking');
    expect(useUpdateStore.getState().status).toBe('checking');
  });

  it('records up-to-date with timestamp', () => {
    useUpdateStore.getState().setUpToDate('2026-04-29T12:00:00Z');
    const s = useUpdateStore.getState();
    expect(s.status).toBe('up-to-date');
    expect(s.lastCheckedAt).toBe('2026-04-29T12:00:00Z');
  });

  it('records available with version', () => {
    useUpdateStore.getState().setAvailable('0.2.0');
    const s = useUpdateStore.getState();
    expect(s.status).toBe('available');
    expect(s.version).toBe('0.2.0');
  });

  it('updates progress while downloading', () => {
    useUpdateStore.getState().setProgress(0.42);
    const s = useUpdateStore.getState();
    expect(s.status).toBe('downloading');
    expect(s.progress).toBeCloseTo(0.42);
  });

  it('records ready with version and clears progress', () => {
    useUpdateStore.getState().setProgress(0.99);
    useUpdateStore.getState().setReady('0.2.0');
    const s = useUpdateStore.getState();
    expect(s.status).toBe('ready');
    expect(s.version).toBe('0.2.0');
    expect(s.progress).toBe(1);
  });

  it('records failed with phase + message and preserves version', () => {
    useUpdateStore.getState().setReady('0.2.0');
    useUpdateStore.getState().setFailed('install', 'file in use');
    const s = useUpdateStore.getState();
    expect(s.status).toBe('failed');
    expect(s.error).toEqual({ phase: 'install', message: 'file in use' });
    expect(s.version).toBe('0.2.0'); // retained so retry banner can show it
  });

  it('reset returns to idle and clears error', () => {
    useUpdateStore.getState().setFailed('check', 'no network');
    useUpdateStore.getState().reset();
    const s = useUpdateStore.getState();
    expect(s.status).toBe('idle');
    expect(s.error).toBeNull();
  });
});
```

- [ ] **Step 2: Run the test (it should fail — module doesn't exist)**

Run: `pnpm test src/state/updateStore.test.ts`
Expected: FAIL with module-not-found error.

- [ ] **Step 3: Implement the store**

Create `src/state/updateStore.ts`:

```ts
import { create } from 'zustand';

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'up-to-date'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'failed';

export type UpdatePhase = 'check' | 'download' | 'verify' | 'install';

export interface UpdateError {
  phase: UpdatePhase;
  message: string;
}

interface UpdateState {
  status: UpdateStatus;
  version: string | null;
  progress: number;
  error: UpdateError | null;
  lastCheckedAt: string | null;

  setStatus: (s: UpdateStatus) => void;
  setUpToDate: (checkedAt: string) => void;
  setAvailable: (version: string) => void;
  setProgress: (progress: number) => void;
  setReady: (version: string) => void;
  setFailed: (phase: UpdatePhase, message: string) => void;
  reset: () => void;
}

export const useUpdateStore = create<UpdateState>((set) => ({
  status: 'idle',
  version: null,
  progress: 0,
  error: null,
  lastCheckedAt: null,

  setStatus: (status) => set({ status }),
  setUpToDate: (lastCheckedAt) => set({ status: 'up-to-date', lastCheckedAt, error: null }),
  setAvailable: (version) => set({ status: 'available', version, error: null }),
  setProgress: (progress) => set({ status: 'downloading', progress }),
  setReady: (version) => set({ status: 'ready', version, progress: 1, error: null }),
  setFailed: (phase, message) => set({ status: 'failed', error: { phase, message } }),
  reset: () =>
    set({ status: 'idle', version: null, progress: 0, error: null }),
}));
```

- [ ] **Step 4: Run the tests**

Run: `pnpm test src/state/updateStore.test.ts`
Expected: 8 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/state/updateStore.ts src/state/updateStore.test.ts
git commit -m "feat(updater): Zustand store for update state with tests"
```

---

### Task 13: Tauri event listener (`updateEvents.ts`)

**Files:**
- Create: `src/lib/updateEvents.ts`

- [ ] **Step 1: Write the listener**

Create `src/lib/updateEvents.ts`:

```ts
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useUpdateStore, type UpdatePhase } from '../state/updateStore';

interface AvailablePayload { version: string; notes?: string; pubDate?: string }
interface UpToDatePayload { checkedAt: string }
interface ProgressPayload { downloaded: number; total: number }
interface ReadyPayload { version: string }
interface FailedPayload { phase: UpdatePhase; message: string }

/**
 * Attach all updater event listeners. Call once at app startup.
 * Returns a teardown function that unregisters every listener.
 */
export async function attachUpdateListeners(): Promise<UnlistenFn> {
  const store = useUpdateStore.getState();

  const unlisteners = await Promise.all([
    listen('update://checking', () => store.setStatus('checking')),
    listen<UpToDatePayload>('update://up-to-date', (e) => store.setUpToDate(e.payload.checkedAt)),
    listen<AvailablePayload>('update://available', (e) => store.setAvailable(e.payload.version)),
    listen<ProgressPayload>('update://progress', (e) => {
      const total = e.payload.total || 1;
      store.setProgress(Math.min(1, e.payload.downloaded / total));
    }),
    listen<ReadyPayload>('update://ready', (e) => store.setReady(e.payload.version)),
    listen<FailedPayload>('update://failed', (e) =>
      store.setFailed(e.payload.phase, e.payload.message),
    ),
  ]);

  return () => unlisteners.forEach((u) => u());
}
```

- [ ] **Step 2: Verify type-check**

Run: `pnpm lint`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/lib/updateEvents.ts
git commit -m "feat(updater): subscribe to Tauri update events and dispatch into store"
```

---

### Task 14: Mount listener in `App.tsx`

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Add the mount-time effect**

In `src/App.tsx`, after the existing `useEffect(() => { init().finally(...) }, [init]);` block (around line 17-19), add a second effect:

```tsx
useEffect(() => {
  let teardown: (() => void) | null = null;
  attachUpdateListeners().then((unlisten) => { teardown = unlisten; });
  return () => { teardown?.(); };
}, []);
```

And add the import at the top:

```tsx
import { attachUpdateListeners } from './lib/updateEvents';
```

- [ ] **Step 2: Verify type-check + tests still pass**

Run: `pnpm lint && pnpm test`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/App.tsx
git commit -m "feat(updater): attach event listeners on app mount"
```

---

### Task 15: `UpdateBanner` component + tests (TDD)

**Files:**
- Create: `src/components/UpdateBanner.tsx`
- Create: `src/components/UpdateBanner.test.tsx`

- [ ] **Step 1: Write the failing tests**

Create `src/components/UpdateBanner.test.tsx`:

```tsx
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { UpdateBanner } from './UpdateBanner';
import { useUpdateStore } from '../state/updateStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe('UpdateBanner', () => {
  beforeEach(() => {
    useUpdateStore.setState({
      status: 'idle',
      version: null,
      progress: 0,
      error: null,
      lastCheckedAt: null,
    });
  });

  it('renders nothing when idle', () => {
    const { container } = render(<UpdateBanner />);
    expect(container.firstChild).toBeNull();
  });

  it('renders nothing when checking / available / downloading', () => {
    for (const status of ['checking', 'available', 'downloading'] as const) {
      useUpdateStore.setState({ status, version: '0.2.0' });
      const { container, unmount } = render(<UpdateBanner />);
      expect(container.firstChild).toBeNull();
      unmount();
    }
  });

  it('renders the install banner when ready', () => {
    useUpdateStore.setState({ status: 'ready', version: '0.2.0' });
    render(<UpdateBanner />);
    expect(screen.getByText(/Update ready · v0\.2\.0/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Install & restart/ })).toBeInTheDocument();
  });

  it('renders the retry banner when install failed', () => {
    useUpdateStore.setState({
      status: 'failed',
      version: '0.2.0',
      error: { phase: 'install', message: 'file in use' },
    });
    render(<UpdateBanner />);
    expect(screen.getByText(/Install failed/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Retry/ })).toBeInTheDocument();
  });

  it('renders nothing for failed checks (only install failures surface)', () => {
    useUpdateStore.setState({
      status: 'failed',
      error: { phase: 'check', message: 'no network' },
    });
    const { container } = render(<UpdateBanner />);
    expect(container.firstChild).toBeNull();
  });

  it('invokes install_update when user clicks Install', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    useUpdateStore.setState({ status: 'ready', version: '0.2.0' });
    render(<UpdateBanner />);
    await userEvent.click(screen.getByRole('button', { name: /Install & restart/ }));
    expect(invoke).toHaveBeenCalledWith('install_update');
  });
});
```

Note: this assumes `@testing-library/user-event` is available. If not, install it:
```bash
pnpm add -D @testing-library/user-event
```

- [ ] **Step 2: Run the tests (they should fail — component doesn't exist)**

Run: `pnpm test src/components/UpdateBanner.test.tsx`
Expected: FAIL with module-not-found.

- [ ] **Step 3: Implement the component**

Create `src/components/UpdateBanner.tsx`:

```tsx
import { motion } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { ArrowUpCircle } from 'lucide-react';
import { useUpdateStore } from '../state/updateStore';

export function UpdateBanner() {
  const status = useUpdateStore((s) => s.status);
  const version = useUpdateStore((s) => s.version);
  const error = useUpdateStore((s) => s.error);

  const showInstall = status === 'ready';
  const showRetry = status === 'failed' && error?.phase === 'install';

  if (!showInstall && !showRetry) return null;

  const handleClick = () => {
    invoke('install_update').catch(() => {
      // Errors will arrive via the update://failed event; nothing to do here.
    });
  };

  return (
    <motion.div
      initial={{ y: -36, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ type: 'spring', stiffness: 280, damping: 28 }}
      className="flex items-center gap-2 px-3 py-2 border-b border-[color:var(--color-border-subtle)] bg-[color:color-mix(in_oklab,var(--color-accent-warm)_6%,transparent)]"
      role="status"
    >
      <ArrowUpCircle size={14} className="text-[color:var(--color-accent-cool)]" aria-hidden />
      <span className="flex-1 text-xs text-[color:var(--color-text-default)] tracking-tight">
        {showInstall ? `Update ready · v${version}` : 'Install failed'}
      </span>
      <button
        type="button"
        onClick={handleClick}
        className="text-xs text-[color:var(--color-accent-cool)] px-2 py-1 rounded-[var(--radius-sm)] hover:bg-[color:color-mix(in_oklab,var(--color-accent-cool)_8%,transparent)] transition-colors"
      >
        {showInstall ? 'Install & restart' : 'Retry'}
      </button>
    </motion.div>
  );
}
```

The CSS-variable references (`--color-accent-warm`, `--color-accent-cool`, etc.) must match what's already defined in `src/styles/tokens.css`. If they don't exist by those exact names, use whatever the project uses for "teal accent" and "terracotta tint" — check `tokens.css` first and adjust.

- [ ] **Step 4: Run the tests**

Run: `pnpm test src/components/UpdateBanner.test.tsx`
Expected: 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/UpdateBanner.tsx src/components/UpdateBanner.test.tsx package.json pnpm-lock.yaml
git commit -m "feat(updater): UpdateBanner component with ready + install-retry states"
```

---

### Task 16: Integrate banner + version line into `CompactPopover`

**Files:**
- Modify: `src/popover/CompactPopover.tsx`

The popover needs:
1. `<UpdateBanner />` mounted at the very top (above everything)
2. A version footer line: `Claude Limits v{__APP_VERSION__}  ·  Check for updates`
3. The "Check for updates" link wired to `invoke('check_for_updates_now')` with the in-place text-swap states

- [ ] **Step 1: Add imports**

In `src/popover/CompactPopover.tsx`, add at the top of the imports block:

```tsx
import { UpdateBanner } from '../components/UpdateBanner';
import { useUpdateStore } from '../state/updateStore';
import { invoke } from '@tauri-apps/api/core';
```

- [ ] **Step 2: Add a helper component for the footer line**

At the bottom of `CompactPopover.tsx` (after the existing `Shell` / `Header` definitions if any), add:

```tsx
function VersionFooter() {
  const status = useUpdateStore((s) => s.status);
  const [transient, setTransient] = useState<null | 'checking' | 'up-to-date' | 'failed'>(null);

  useEffect(() => {
    if (status === 'checking') setTransient('checking');
    else if (status === 'up-to-date') {
      setTransient('up-to-date');
      const t = setTimeout(() => setTransient(null), 3000);
      return () => clearTimeout(t);
    } else if (status === 'failed') {
      setTransient('failed');
      const t = setTimeout(() => setTransient(null), 3000);
      return () => clearTimeout(t);
    } else {
      setTransient(null);
    }
  }, [status]);

  const label = (() => {
    if (transient === 'checking') return 'Checking…';
    if (transient === 'up-to-date') return 'Up to date';
    if (transient === 'failed') return "Couldn't check";
    return 'Check for updates';
  })();

  const onClick = () => {
    if (transient === 'checking') return;
    invoke('check_for_updates_now').catch(() => {/* error arrives via event */});
  };

  return (
    <div className="text-center text-[11px] text-[color:var(--color-text-muted)] py-2 select-none">
      Claude Limits v{__APP_VERSION__}{' · '}
      <button
        type="button"
        onClick={onClick}
        disabled={transient === 'checking'}
        className="underline-offset-2 hover:underline hover:text-[color:var(--color-accent-cool)] transition-colors disabled:opacity-60"
      >
        {label}
      </button>
    </div>
  );
}
```

- [ ] **Step 3: Mount the banner + footer in the popover render tree**

In the `home` view's JSX (the default return path of `CompactPopover`), wrap the existing content so the banner is the first child and the footer is the last:

```tsx
return (
  <Shell>
    <UpdateBanner />
    {/* ... existing Header + body content unchanged ... */}
    <VersionFooter />
  </Shell>
);
```

If the existing layout uses a flex-column with one growing region, place `<UpdateBanner />` immediately inside `<Shell>` before the header, and `<VersionFooter />` as the last sibling. Make sure neither breaks the existing scroll/overflow behavior — the banner is fixed-height (36px) and the footer is fixed-height (~24px); the middle region must remain `flex-1 overflow-y-auto`.

The settings sub-view (lines ~50-58) does NOT need the banner/footer — it's a transient drill-down.

- [ ] **Step 4: Verify type-check, tests, and visual smoke**

Run: `pnpm lint && pnpm test`
Expected: clean.

Then visual smoke:
```bash
pnpm tauri dev
```

- Popover opens.
- Bottom shows "Claude Limits v0.1.0 · Check for updates" (greyed in dev — `cfg!(debug_assertions)` no-ops the scheduler, but the manual click still attempts to invoke the command, which will print a warning in the dev console; that's fine for the smoke test).
- Click "Check for updates" → text swaps to "Checking…" briefly → returns to original (in dev there's no real endpoint so it'll fail and swap to "Couldn't check" for 3s — that's expected).
- The `<UpdateBanner />` does NOT appear (status never reaches `ready`).

- [ ] **Step 5: Commit**

```bash
git add src/popover/CompactPopover.tsx
git commit -m "feat(updater): mount UpdateBanner + version footer in popover"
```

---

## Phase 4 — Release pipeline

### Task 17: `scripts/release.mjs` (version bump helper)

**Files:**
- Create: `scripts/release.mjs`
- Modify: `package.json` (add npm script)

- [ ] **Step 1: Write the script**

Create `scripts/release.mjs`:

```js
#!/usr/bin/env node
// Bump app version across package.json and src-tauri/Cargo.toml,
// commit, and create a git tag. Push is left to the user.
//
// Usage:  node scripts/release.mjs <new-version>
// Example: node scripts/release.mjs 0.2.0

import { readFileSync, writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const newVersion = process.argv[2];

if (!newVersion || !/^\d+\.\d+\.\d+$/.test(newVersion)) {
  console.error('Usage: node scripts/release.mjs <MAJOR.MINOR.PATCH>');
  process.exit(1);
}

// Refuse to release with a dirty tree.
const status = execSync('git status --porcelain', { cwd: repoRoot }).toString().trim();
if (status) {
  console.error('Refusing to release: working tree not clean.');
  console.error(status);
  process.exit(1);
}

// 1. Update package.json
const pkgPath = resolve(repoRoot, 'package.json');
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
const oldVersion = pkg.version;
pkg.version = newVersion;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');

// 2. Update src-tauri/Cargo.toml — surgical replace, NOT a parser, to preserve formatting.
const cargoPath = resolve(repoRoot, 'src-tauri', 'Cargo.toml');
const cargo = readFileSync(cargoPath, 'utf8');
const cargoLine = /^version\s*=\s*"[^"]+"/m;
if (!cargoLine.test(cargo)) {
  console.error('Could not find version line in src-tauri/Cargo.toml');
  process.exit(1);
}
writeFileSync(cargoPath, cargo.replace(cargoLine, `version = "${newVersion}"`));

console.log(`Bumped ${oldVersion} → ${newVersion}`);

// 3. Commit + tag.
execSync('git add package.json src-tauri/Cargo.toml', { cwd: repoRoot, stdio: 'inherit' });
execSync(`git commit -m "release: v${newVersion}"`, { cwd: repoRoot, stdio: 'inherit' });
execSync(`git tag v${newVersion}`, { cwd: repoRoot, stdio: 'inherit' });

console.log(`\nCreated commit + tag v${newVersion}.`);
console.log('Next: git push && git push --tags');
```

- [ ] **Step 2: Add npm script**

In `package.json`, in the `scripts` block, add:

```json
"release": "node scripts/release.mjs"
```

- [ ] **Step 3: Smoke test (dry — don't actually run if working tree is dirty)**

Run: `node scripts/release.mjs` (with no argument)
Expected: "Usage: node scripts/release.mjs <MAJOR.MINOR.PATCH>" and exit 1.

Run: `node scripts/release.mjs not-a-version`
Expected: same usage error.

(Don't actually run `node scripts/release.mjs 0.2.0` yet — that's done at release time.)

- [ ] **Step 4: Commit**

```bash
git add scripts/release.mjs package.json
git commit -m "build: add release.mjs version-bump script"
```

---

### Task 18: `scripts/generate-latest-json.mjs`

**Files:**
- Create: `scripts/generate-latest-json.mjs`

This script runs in CI after the matrix builds finish. It reads `.sig` files from a directory (artifacts downloaded by `gh release download`), composes `latest.json`, and prints it to stdout. The CI job pipes stdout to a file and uploads it.

- [ ] **Step 1: Write the script**

Create `scripts/generate-latest-json.mjs`:

```js
#!/usr/bin/env node
// Compose latest.json from a directory of release artifacts. Written for
// the GitHub Actions release.yml — runs after the matrix build downloads
// every platform's artifacts into one folder.
//
// Usage:
//   node scripts/generate-latest-json.mjs --tag v0.2.0 --dir ./artifacts > latest.json
//
// Required artifacts in <dir>:
//   claude-limits_<ver>_universal.app.tar.gz       + .sig
//   claude-limits_<ver>_x64-setup.nsis.zip         + .sig

import { readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const args = Object.fromEntries(
  process.argv.slice(2).reduce((acc, cur, i, arr) => {
    if (cur.startsWith('--')) acc.push([cur.slice(2), arr[i + 1]]);
    return acc;
  }, []),
);

const tag = args.tag;
const dir = args.dir;
const repo = args.repo ?? 'FeiXu-1131372/claude-limits';

if (!tag || !dir) {
  console.error('Usage: --tag v0.2.0 --dir ./artifacts [--repo owner/name]');
  process.exit(1);
}

const version = tag.replace(/^v/, '');
const baseUrl = `https://github.com/${repo}/releases/download/${tag}`;
const files = readdirSync(resolve(dir));

function findArtifact(suffix) {
  const match = files.find((f) => f.endsWith(suffix));
  if (!match) {
    console.error(`Missing artifact: *${suffix}`);
    process.exit(1);
  }
  return match;
}

function readSig(artifactName) {
  const sigName = `${artifactName}.sig`;
  const sigPath = resolve(dir, sigName);
  try {
    return readFileSync(sigPath, 'utf8').trim();
  } catch {
    console.error(`Missing signature file: ${sigName}`);
    process.exit(1);
  }
}

const macArtifact = findArtifact('.app.tar.gz');
const winArtifact = findArtifact('.nsis.zip');

const manifest = {
  version,
  notes: `See release notes at https://github.com/${repo}/releases/tag/${tag}`,
  pub_date: new Date().toISOString(),
  platforms: {
    'darwin-x86_64': {
      signature: readSig(macArtifact),
      url: `${baseUrl}/${macArtifact}`,
    },
    'darwin-aarch64': {
      signature: readSig(macArtifact),
      url: `${baseUrl}/${macArtifact}`,
    },
    'windows-x86_64': {
      signature: readSig(winArtifact),
      url: `${baseUrl}/${winArtifact}`,
    },
  },
};

process.stdout.write(JSON.stringify(manifest, null, 2) + '\n');
```

- [ ] **Step 2: Smoke test the failure path**

Run: `node scripts/generate-latest-json.mjs`
Expected: usage error, exit 1.

Run: `mkdir -p /tmp/empty-artifacts && node scripts/generate-latest-json.mjs --tag v0.2.0 --dir /tmp/empty-artifacts`
Expected: "Missing artifact: *.app.tar.gz", exit 1.

- [ ] **Step 3: Commit**

```bash
git add scripts/generate-latest-json.mjs
git commit -m "build: add latest.json composer for CI"
```

---

### Task 19: Update `.github/workflows/release.yml`

**Files:**
- Modify: `.github/workflows/release.yml`

Three changes: (1) inject signing secrets into the matrix build, (2) emit updater-bundle artifacts, (3) add a third job that composes and uploads `latest.json`.

- [ ] **Step 1: Replace the entire workflow file**

Open `.github/workflows/release.yml`. Replace the entire `release` job's `Build Tauri app` step, and add a new `compose-manifest` job. The full new file:

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
            bundles: app,dmg,updater
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            bundles: nsis,updater
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
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: ${{ github.ref_name }}
          releaseBody: 'Unsigned build — see README for first-launch instructions on each OS.'
          args: --target ${{ matrix.target }} --bundles ${{ matrix.bundles }}

  compose-manifest:
    needs: [release]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with: { node-version: 20 }

      - name: Download release artifacts
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          mkdir -p artifacts
          gh release download "${{ github.ref_name }}" \
            --repo "${{ github.repository }}" \
            --dir artifacts \
            --pattern '*.app.tar.gz*' \
            --pattern '*.nsis.zip*'

      - name: Compose latest.json
        run: |
          node scripts/generate-latest-json.mjs \
            --tag "${{ github.ref_name }}" \
            --dir artifacts \
            --repo "${{ github.repository }}" > artifacts/latest.json
          cat artifacts/latest.json

      - name: Upload latest.json to release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh release upload "${{ github.ref_name }}" \
            artifacts/latest.json \
            --repo "${{ github.repository }}" \
            --clobber
```

- [ ] **Step 2: Validate YAML syntax locally**

Run (if `yq` is installed):
```bash
yq eval '.' .github/workflows/release.yml > /dev/null
```

Expected: no error. If `yq` isn't installed, just visually inspect the indentation.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): emit updater bundles + compose latest.json manifest"
```

---

## Phase 5 — Documentation

### Task 20: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Locate the right place to add the Updates section**

Run: `grep -n "^## " README.md` — find an appropriate place (typically after "Install" / "First launch on Mac/Windows", before "Development").

- [ ] **Step 2: Add the Updates section**

Insert this section in the README, formatted to match the surrounding style:

```markdown
## Updates

Claude Limits checks for new versions automatically — on launch and every 6 hours
while running. When a new version is downloaded and ready, a small banner appears
at the top of the popover with an "Install & restart" button. Click it to upgrade.

You can also trigger a check manually from the popover footer ("Check for updates")
or the tray menu ("Check for Updates…").

**Important:** Auto-update was added in **v0.2.0**. If you're upgrading from v0.1.x,
you'll need to download and install v0.2.0 manually from the
[releases page](https://github.com/FeiXu-1131372/claude-limits/releases) — the
v0.1.x build has no updater wired up. Every release after v0.2.0 will auto-update.

The app is unsigned (no paid Apple Developer ID / Windows EV cert), so the *first
install* on a new machine still goes through the OS-specific first-launch flow
described above. After the first install, all updates are silent — Gatekeeper /
SmartScreen don't re-check signed-by-the-same-developer apps on update.

Update integrity: every release artifact is signed with our ed25519 updater key,
and the app refuses any update whose signature doesn't match the public key
embedded at build time.
```

- [ ] **Step 3: Verify rendering**

If you have a markdown preview: open `README.md` and verify the section reads cleanly. Otherwise: `cat README.md | head -120`.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs(readme): document auto-update behavior + manual one-time upgrade"
```

---

## Final Verification

After all 20 tasks are complete, do a clean end-to-end check before tagging the first updater-enabled release.

- [ ] **Step 1: Full lint + test sweep**

Run all checks:
```bash
pnpm lint
pnpm test
cd src-tauri && cargo test --all-features --no-fail-fast && cargo clippy --all-targets -- -D warnings
```

Expected: all green.

- [ ] **Step 2: Production build smoke**

Run: `pnpm tauri build`
Expected: clean build, produces signed `.app.tar.gz` + `.nsis.zip` (with `.sig` files alongside) — verify by listing `src-tauri/target/release/bundle/`.

- [ ] **Step 3: Tag and ship the first release**

When you're ready to actually release:
```bash
node scripts/release.mjs 0.2.0
git push && git push --tags
```

Then watch the GitHub Actions run at `https://github.com/FeiXu-1131372/claude-limits/actions`. Both jobs (`release` matrix + `compose-manifest`) must succeed. Verify on the release page that `latest.json` is among the assets.

- [ ] **Step 4: Manual update smoke (cross-platform)**

This is the only end-to-end test that proves the feature works. Cannot be automated.

For each of macOS + Windows:
1. Install the previous version (v0.1.0 if available, otherwise install v0.2.0 once and pretend it's "previous" by tagging a v0.2.1 with a trivial change).
2. Launch the app.
3. Wait for the auto-check (or click "Check for updates" in the tray menu).
4. The "Update ready · vX.Y.Z" banner should appear in the popover within seconds.
5. Click "Install & restart" — the app should exit, the installer should run silently, and the new version should relaunch.
6. Verify the popover footer shows the new version number.

Document any deviations as new tickets.

---

## Decisions encoded in this plan (mirrors spec §2)

| # | Decision | Where it lives |
|---|---|---|
| D1 | Skip paid OS signing for v1; ship updater-signed unsigned builds. | Task 1 (no Apple/MS certs), Task 19 (only updater secrets in CI), Task 20 (README note) |
| D2 | Auto-download in background; user clicks to install. | Task 7 (`check_and_emit` downloads + stages), Task 15 (banner only after `ready`) |
| D3 | Check on launch + every 6 hours while running. | Task 6 (`CHECK_INTERVAL_HOURS = 6`), Task 7 (`run_scheduler` loop) |
| D4 | Popover-only "Update ready" banner. No tray dot, no system notification. | Task 15 (banner only), Task 16 (mounted only in popover) |
| D5 | Host `latest.json` and artifacts on GitHub Releases. | Task 2 (endpoint URL), Task 18 (manifest composer) |
| D6 | Tauri's static-JSON updater endpoint. | Task 2 (config) |
| D7 | Single channel (stable). | Task 2 (single endpoint) |
| D8 | Updater disabled in dev builds. | Task 7 (`#[cfg(debug_assertions)]` in `run_scheduler`) |

---

## Self-review notes (filled in by plan author)

- **Spec coverage:** All sections of `2026-04-29-releases-and-autoupdate-design.md` are covered. Spec §3.1 architecture → Tasks 2, 7, 9. §3.2 manifest schema → Task 18. §3.3 trust model → Tasks 1, 2, 19. §4 state machine → Tasks 7, 12. §4.1 events → Tasks 7, 13. §4.2 failure handling → Task 7 (`emit_failed`). §4.3 persistence → Task 5. §5.1 Rust components → Tasks 2–10. §5.2 React components → Tasks 11–16. §5.3 CI → Tasks 17–19. §5.4 versioning → Task 17. §5.5 README → Task 20. §6 UI specifics → Tasks 15, 16. §7 testing → Tasks 4, 5, 6, 12, 15 (unit) + Final Verification §4 (manual smoke). §8 future work is intentionally out of scope.
- **Placeholder scan:** No "TBD"s. The only intentional placeholder is `PASTE_PUBLIC_KEY_FROM_TASK_1_STEP_3` in Task 2 step 2, which is explicit about how to fill it in.
- **Type consistency:** `UpdatePhase` is `'check' | 'download' | 'verify' | 'install'` everywhere (Rust enum serialized lowercase via `rename_all`, TS union mirroring it). `UpdateStatus` is consistent across store + tests + banner. Event names `update://...` consistent across Rust emit + TS listen. Command names `check_for_updates_now` + `install_update` consistent across Rust definitions + TS invokes.
- **Token usage in `UpdateBanner`:** Task 15 step 3 calls out that the CSS variable names need to be checked against `src/styles/tokens.css` and adjusted if the names differ. This is the one place a future implementer needs to look at existing project state rather than just typing what's in the plan.
