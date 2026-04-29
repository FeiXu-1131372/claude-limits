# Claude Limits — Code Quality & Architecture Audit

**Date:** 2026-04-29
**Status:** Audit complete; remediation in progress
**Audience:** An engineer (or fresh AI agent) picking up the remediation work cold

---

## How to use this document

This is a self-contained guide. Read it top to bottom before opening any source file.

**You are picking up an audit, not starting one.** Four specialised reviewers (Rust backend, React frontend, architecture, security/tests) examined every source file in the project. Their findings are consolidated here with file:line citations and concrete fixes.

**Workflow for each finding:**
1. Read the entry — note the file path, the failure mode, and the proposed fix.
2. Open the file. Read enough surrounding context to understand the change. Do not trust the line numbers blindly — they are accurate as of `main` at commit `5359dd9` (2026-04-29) but may have drifted.
3. Verify the issue still exists (`git log -p <file>` if in doubt).
4. Implement the fix following the project's CLAUDE.md conventions.
5. Write or update tests where the **Verification** field calls for it.
6. Run `pnpm lint && pnpm test && (cd src-tauri && cargo test --all-features && cargo clippy --all-targets -- -D warnings)`.
7. Commit per the convention in CLAUDE.md (concise, "why" not "what", co-authored).

**You should already have read** `CLAUDE.md` (auto-loaded) and skimmed `docs/superpowers/specs/2026-04-24-claude-limits-design.md` (the original spec — note that the spec has drifted from the code in places; the **Spec drift** section below lists them).

**Useful skills to invoke:**
- `superpowers:test-driven-development` — for any new behaviour. Write the failing test first.
- `superpowers:verification-before-completion` — before claiming any item done. Run the actual command; don't rely on "it should work".
- `superpowers:brainstorming` — for the items marked **DECISION REQUIRED** (e.g., db_reset). Discuss with the human before implementing.
- `superpowers:debug-hypothesis` — if a fix doesn't behave as expected. Don't guess; observe → hypothesise → experiment.

**What NOT to do:**
- Do not refactor beyond the scope of the finding. A bug fix doesn't need surrounding cleanup.
- Do not introduce new abstractions, helpers, or "while I'm here" changes. CLAUDE.md is explicit about this.
- Do not add comments unless the WHY is non-obvious. No "// Fix for P0-1" or PR-history references in comments.
- Do not skip verification. Every "completed" claim must be backed by a command run.

---

## Project context (one screen)

**What it is:** Cross-platform (macOS + Windows) menu-bar utility tracking Claude subscription rate limits (5h, 7d, Opus/Sonnet splits). Tauri v2, Rust backend, React 19 + TS frontend, Tailwind v4, Zustand, Framer Motion, Recharts. SQLite via rusqlite. See CLAUDE.md for design intent.

**Repository layout:**
```
src-tauri/         Rust backend
  src/
    lib.rs              entry point — startup orchestration, watcher lifecycle
    main.rs             thin shim
    app_state.rs        AppState struct, Settings, CachedUsage
    commands.rs         Tauri command handlers (the IPC surface)
    poll_loop.rs        background polling task (auth → fetch → cache → emit)
    logging.rs          tracing init
    auth/               OAuth orchestrator, token store, exchange, claude_code_creds
    jsonl_parser/       record parsing, walker, watcher, pricing
    store/              sqlite db, queries, schema.sql
    usage_api/          HTTP client + types
    notifier/           threshold rules, dispatch
    tray.rs             tray menu + click handlers
    tray_icon/          icon rendering (digits, macos, windows, shared)
  tests/                Rust integration tests
  capabilities/         Tauri v2 capability files
  tauri.conf.json
  Cargo.toml
  pricing.json          bundled at build time

src/                React frontend
  App.tsx               root, view routing
  main.tsx
  popover/              CompactPopover, InstrumentRow, ResetCountdown, UsageBar
  report/               ExpandedReport + 6 tabs (Sessions, Models, Trends, Projects, Cache, Heatmap)
  settings/             SettingsPanel, AuthPanel, AuthConflictChooser
  components/ui/        Badge, Banner, Button, Card, EmptyState, IconButton,
                        ProgressBar, Select, Slider, Tabs, Toggle
  lib/                  events, format, icons, ipc, motion, store, types,
                        useTabData, window-chrome
  lib/generated/        bindings.ts (specta-generated; currently bypassed — see P1-17)
  styles/               globals.css, tokens.css

docs/                   spec, design system, this audit, etc.
.github/workflows/      test.yml, release.yml
```

**Architecture in one paragraph:**

Two data sources, never mixed in the backend: (1) Anthropic Usage API polled on a timer by `poll_loop`, writing to `AppState.cached_usage` and emitting `usage_updated`; (2) local JSONL files ingested via two paths — a one-shot startup backfill spawned from `lib.rs` setup and a long-lived `notify`-backed watcher — both writing to SQLite via `walker::ingest_file`, with the watcher emitting `session_ingested` through an unbounded mpsc channel. The frontend Zustand store (`src/lib/store.ts`) subscribes to all events in `events.ts` and routes views (`CompactPopover` / `ExpandedReport` / `AuthPanel` / `AuthConflictChooser`) from `App.tsx`. `cached_usage` is the in-memory source of truth for API data; SQLite holds the historical session data; settings *should* live in SQLite but currently don't (see P0-1).

**Build commands:**
```
pnpm install --frozen-lockfile
pnpm dev          # vite dev server
pnpm tauri dev    # full app in dev
pnpm test         # vitest (frontend)
pnpm lint         # tsc --noEmit
cd src-tauri && cargo test --all-features
cd src-tauri && cargo clippy --all-targets -- -D warnings
```

---

## Strengths — DO NOT REGRESS

These were called out by the audit as well-designed. Preserve their behaviour.

1. **Event-id dedup** — `{requestId}:{message.id}` with structural fallback in `jsonl_parser/record.rs`; tests in `store/queries.rs` cover same-event-different-offset.
2. **JSONL forward-compat** — `#[serde(flatten, default)]` on unknown fields means new Anthropic schema additions never break parsing. Cache split (`cache_5m` / `cache_1h`) with legacy fallback.
3. **Notifier dedup** — per-bucket × per-threshold × per-window keys in `notification_state`, 24h fallback for `ExtraUsage`. Four tests in `notifier/rules.rs` cover the relevant paths.
4. **Backoff & backpressure in poll loop** — exponential with 30-min cap; settings re-read every iteration; `tokio::select!` for interruptible sleep; `tokio::sync::Notify` for force-refresh.
5. **Design-token discipline** on the frontend — only one hardcoded pixel value found (`text-[28px]` in CacheTab); zero hardcoded hex colours in components.
6. **Zustand selector discipline** — every `useAppStore` call uses a field selector.
7. **Auth conflict detection** — comparing by `AccountId` (not email string equality), structured `auth_source_conflict` event with both emails.
8. **`useTabData` hook** — `cancelled` guard, `hasLoadedOnce` flicker prevention, justified eslint-disable.
9. **Tray renderer** — `tiny-skia` PNG bytes; OS-independent; pixel-decoded tests.
10. **Cross-platform `#[cfg]` discipline** — clean module split for `tray_icon` and `auth/claude_code_creds`.

---

## Spec drift — read before touching auth or schema

The spec at `docs/superpowers/specs/2026-04-24-claude-limits-design.md` has drifted from the code. Trust the code unless explicitly fixing the spec.

| Spec says | Code does | Action |
|---|---|---|
| OAuth `redirect_uri = https://platform.claude.com/oauth/code/callback` | Code uses `https://console.anthropic.com/oauth/code/callback` | Code is correct (tracks a real Claude Code issue). Update spec when convenient. |
| OAuth `token_endpoint = https://platform.claude.com/v1/oauth/token` | Code uses `https://console.anthropic.com/v1/oauth/token` | Code is correct. Update spec. |
| Authorize URL includes `?code=true` | Code explicitly omits it | Code is correct (see comment referencing anthropics/claude-code#29983). |
| Schema has 3 tables | Schema has 7: `schema_version`, `accounts`, `api_snapshots`, `session_events`, `jsonl_cursors`, `notification_state`, `settings` | Update spec. |
| `get_account_info()` listed as a command | Not registered | Either implement or remove from spec. |
| `open_expanded_window` listed as a command | Replaced by `resize_window` | Update spec. |
| `debug_force_threshold(bucket: Bucket, pct: u8)` | Takes `bucket: String, pct: u8` and is a no-op (only `tracing::info!`) | Implement properly or remove. |
| §1 Non-goals: "Heatmap tab deferred to v1.1", "Cache tab folded into Models" | `HeatmapTab` and `CacheTab` ship in `ExpandedReport.tsx` `TAB_CONFIG` | **DECISION REQUIRED** — accept by updating spec, or remove from `TAB_CONFIG`. |
| §5C: DB corruption → emit `db_reset` event | No `db_reset` emitter exists; corruption causes `.expect("open db")` panic | See P0-8. |

---

## Findings index

- **P0** (8 items) — block release / fix this week. Correctness on the happy path or one common operation away.
- **P1** (23 items) — fix in next sprint. Security, structural, correctness with workarounds.
- **P2** (~25 items) — backlog. Real but not urgent.
- **P3** (handful) — nitpicks / cleanup.

Each entry has: **file:line**, **why it matters**, **fix**, **verification**.

---

# P0 — Block release

## P0-1 — Settings never persist

**File:** `src-tauri/src/commands.rs:259-261`; `src-tauri/src/store/schema.sql` (`settings` table is unused)

**Why it matters:** `update_settings` writes only to `state.settings` (in-memory `RwLock<Settings>`). The SQLite `settings` table is never read or written. On every app restart, polling interval, thresholds, and notification toggles revert to `Settings::default()`. The user-configurable polling interval — a headline feature in the spec — silently doesn't work across restarts.

**Fix:**
1. In `src-tauri/src/store/queries.rs`, add `Db::load_settings() -> Result<Option<Settings>>` and `Db::save_settings(&Settings) -> Result<()>` (single-row upsert keyed on a constant `id = 1`).
2. In `src-tauri/src/lib.rs::run`, after `Db::open`, call `db.load_settings()` and seed `AppState.settings` with the result if `Some`. Otherwise keep the default.
3. In `src-tauri/src/commands.rs::update_settings`, after the in-memory write, call `state.db.save_settings(&new_settings)`. Validate before persist (see P1-18).

**Verification:**
- New unit test in `store/queries.rs`: roundtrip `save_settings` then `load_settings`.
- Manual test: `pnpm tauri dev`, change polling interval, kill the app, restart, confirm the value sticks.
- Run `cargo test --all-features`.

---

## P0-2 — No Content Security Policy

**File:** `src-tauri/tauri.conf.json:32`

**Why it matters:** `"csp": null` explicitly disables Tauri's default CSP. Any successful XSS path (compromised dependency, future regression in how the frontend renders backend strings, supply-chain) has unrestricted script execution in the WebView, which can read tokens from the IPC layer.

**Fix:**
```json
"security": {
  "csp": "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: https://ipc.localhost; img-src 'self' data: asset: https://asset.localhost; font-src 'self' data:"
}
```
- `'unsafe-inline'` for styles is required by Tailwind v4's runtime style injection. If the build mode emits all CSS as files, drop it.
- `'wasm-unsafe-eval'` is required by some Tauri internals. Test without it first.
- `connect-src` must include the `ipc:` scheme and `https://ipc.localhost` for Tauri v2 IPC.

**Verification:**
- `pnpm tauri dev`. Open DevTools (right-click → Inspect — works in dev). Confirm no CSP violations in Console for the popover, expanded report, settings panel, and auth panel screens.
- `pnpm tauri build` and verify the production app launches and renders all screens.
- If Tauri rejects the policy, consult Tauri v2's `tauri.conf.json` schema at `src-tauri/gen/schemas/desktop-schema.json`.

---

## P0-3 — Cursor-corruption race between backfill and watcher

**File:** `src-tauri/src/lib.rs:228-266` (the two `tauri::async_runtime::spawn` blocks); `src-tauri/src/jsonl_parser/walker.rs::ingest_file`

**Why it matters:** Startup spawns a backfill task (`for f in files { ingest_file(...); }`) and a long-lived watcher. Both call `ingest_file`, which performs `db.insert_events` and `db.set_cursor` as two separate SQLite operations. If both paths process the same file, the second call's cursor write can regress the first's progress, causing repeated re-ingestion. The `UNIQUE(event_id)` constraint deduplicates the events themselves, so it is wasted work, not double-counts — but it's unbounded wasted work on an active session.

**Fix (option A — recommended):** Wrap the events+cursor write in a single SQLite transaction inside `walker::ingest_file`:
```rust
// pseudocode
let tx = conn.transaction()?;
db_with_tx::insert_events(&tx, ...)?;
db_with_tx::set_cursor(&tx, ...)?;
tx.commit()?;
```
This requires either threading a `&Transaction` through the existing query helpers, or adding a single `db.ingest_atomic(&path, &events, &cursor)` method that does both writes in one tx.

**Fix (option B):** Per-file `Mutex` keyed on the canonicalised path, held across the entire `ingest_file` call. Simpler but does not survive process restart (which is fine — the cursor is durable).

**Verification:**
- Add a test in `tests/jsonl_walker.rs`: write a JSONL file, spawn two threads each calling `ingest_file` on the same file, verify final cursor and row count are correct.
- Existing tests must still pass.

---

## P0-4 — Tray icon `expect` panics the whole process

**File:** `src-tauri/src/tray.rs:22`

**Why it matters:** `Image::from_bytes(&bytes).expect("renderer produces valid png")` — every time the tray icon updates. If `tiny-skia`'s PNG encode ever rejects bytes (memory pressure, future Tauri API change, partial buffer), the entire Tauri runtime panics. With `panic = "abort"` in release profile, the app dies with no recovery path.

**Fix:**
```rust
match Image::from_bytes(&bytes) {
    Ok(img) => { let _ = tray.set_icon(Some(img)); }
    Err(e) => tracing::warn!("tray icon decode failed, keeping previous icon: {e}"),
}
```

**Verification:**
- `cargo build` and `cargo test`.
- No new test required — this is defensive against a path that should be unreachable.

---

## P0-5 — Frontend event-listener leak

**File:** `src/lib/store.ts:51-72` (the `init` action)

**Why it matters:** `subscribe()` in `src/lib/events.ts` returns `Promise<UnlistenFn[]>`. `init()` in the store discards this. Every call to `init()` registers a fresh set of six Tauri `listen()` handlers without removing the previous ones. In dev with React strict-mode double-invocation and HMR, handlers accumulate silently. In production, any code path that re-calls `init()` (e.g., a future "switch account" or sign-out flow that re-bootstraps) doubles the per-event mutation count.

**Fix:**
1. Store the unlisteners on the store: `unlisteners: UnlistenFn[]` private field, or a module-level array in `store.ts`.
2. In `init`, if `unlisteners.length > 0`, call them all and clear before resubscribing.
3. Add a `cleanup` action that does the same. Wire it into any future sign-out path that re-bootstraps.

**Verification:**
- Add a vitest in `src/lib/store.test.ts` (new file): mock `listen` with a `vi.fn` that returns a `vi.fn()` unlistener, call `init` twice, assert each unlistener was invoked exactly once.
- Run `pnpm test`.

---

## P0-6 — Rules-of-Hooks violation in CompactPopover

**File:** `src/popover/CompactPopover.tsx:67`

**Why it matters:** `useAppStore((s) => s.toggleViewMode)` is called *after* two early returns (line 49 `if (view === 'settings') return ...` and line 60 `if (!usage) return ...`). React 19 strict mode will throw a hooks-order error. In production mode this can cause silent state corruption when the render path changes between conditional branches.

**Fix:** Move the `toggleViewMode` selector to the top of the component, alongside the other `useAppStore` calls (around line 23), before any conditional return.

**Verification:**
- `pnpm lint` (tsc).
- `pnpm tauri dev`, open the popover, click expand button. No console errors.
- Add a unit test if you set up the frontend test infrastructure (P1-21).

---

## P0-7 — `--ease-spring` token referenced everywhere, defined nowhere

**Files referencing it:**
- `src/popover/UsageBar.tsx:116`
- `src/report/ModelsTab.tsx:117, 142`
- `src/report/CacheTab.tsx:70`
- `src/components/ui/ProgressBar.tsx:82`
- `src/components/ui/Slider.tsx:70`
- `src/components/ui/Toggle.tsx:54`

**File missing the definition:** `src/styles/tokens.css`

**Why it matters:** `ease-[var(--ease-spring)]` resolves to `var(--ease-spring)` which is undefined → CSS drops the `ease` clause and uses the browser default. Every animated bar, toggle, and progress indicator in the app uses the wrong easing in production right now. CLAUDE.md design principle 3 ("every animation curve comes from tokens") is violated; the "spring physics only" guidance is also violated for CSS transitions.

**Fix:** Add to `tokens.css` `@theme` block:
```css
--ease-spring: cubic-bezier(0.22, 1, 0.36, 1);
```
The cubic-bezier above is a typical "spring-like" curve (Material Standard / iOS easeOut). If the design wants a more pronounced bounce, use `cubic-bezier(0.34, 1.56, 0.64, 1)` (back-ease-out). Pick one with the human; do not invent a value silently.

**Verification:**
- `pnpm tauri dev`. Watch the usage bar fill animation — it should now have a gentle settle. Toggle a setting toggle. Drag a slider.
- Visual diff against the design references in CLAUDE.md (Raycast / Linear).

---

## P0-8 — `db_reset` event declared but never emitted; DB corruption causes panic

**Files:** `src/lib/events.ts` (declares `db_reset`); `src/lib/store.ts` (handles `db_reset`); no Rust emitter; `src-tauri/src/lib.rs:22` calls `Db::open(&data_dir).expect("open db")`

**Why it matters:** Spec §5C promises a recovery path. The frontend has the banner ready. Rust has neither a corruption check nor a recovery path. A user with a corrupt SQLite file (power loss mid-write, disk error) gets a startup panic and an app that won't open.

**Decision required (discuss with the human):**
- **Option A — implement the spec.** On `Db::open` failure, back up the corrupt file to `db.sqlite.corrupt-<timestamp>`, recreate the schema, emit `db_reset` after the Tauri runtime is up, surface the banner. Add `PRAGMA integrity_check` on startup as a corruption probe.
- **Option B — remove the contract.** Delete `db_reset` from `events.ts` and the store handler. Document that DB corruption is unsupported.

Option A is the right answer if Claude Limits is intended for general distribution. Option B is acceptable if the user base is technical enough to recover manually.

**Verification (option A):**
- Manually corrupt the SQLite file (`echo garbage > db.sqlite`); launch the app; confirm recovery banner appears and the file is renamed.
- Add a test that opens a deliberately-truncated SQLite file and verifies the recovery path runs.

---

# P1 — Next sprint

Grouped by track. All have file:line citations and concrete fixes. Verification = the relevant test suite must pass and a new test should be added unless noted.

## Security & auth

### P1-1 — Tokens not zeroized on drop

**Files:** `src-tauri/src/auth/mod.rs:22` (`StoredToken`); `src-tauri/src/auth/oauth_paste_back.rs` (`PkcePair`)

**Fix:** Add `zeroize = { version = "1", features = ["zeroize_derive"] }` as a direct dependency. Derive `ZeroizeOnDrop` on `StoredToken` and `PkcePair`. The `access_token`, `refresh_token`, and PKCE `verifier` fields are heap-allocated `String`s; `Zeroize` handles them.

### P1-2 — Token error messages may leak token bodies

**Files:** `src-tauri/src/auth/exchange.rs:53-54, 72-73`; `src-tauri/src/auth/account_identity.rs:51-53`

**Why:** `Err(anyhow!("token exchange failed: {status}: {text}"))` includes the raw response body. If Anthropic ever echoes back form fields in errors (some authorisation servers do), `refresh_token` could appear in error strings sent to the frontend.

**Fix:** In user-facing/IPC-bound error messages, include only the HTTP status. Log the full body at `tracing::debug!` level with explicit `[REDACTED]` markers around any potential auth fields.

### P1-3 — Windows credential file TOCTOU

**File:** `src-tauri/src/auth/token_store.rs:53-95` (`save_fallback` and `restrict_permissions` on Windows)

**Why:** `fs::write(&p, payload)` runs before `restrict_permissions(&p)` calls `icacls`. Between those two calls the file exists with default ACLs (readable by other users in the same session on shared Windows machines).

**Fix:** Write to a sibling temp file with the restricted ACL applied first, then `fs::rename` atomically into place. Or use the `windows-sys` ACL APIs (already a dependency) to apply ACLs synchronously without a child process.

### P1-4 — `shell:default` capability too broad

**File:** `src-tauri/capabilities/default.json:21`

**Fix:** Replace `"shell:default"` with `"shell:allow-open"` only. Add a scope restricting to `https:` URLs (consult Tauri v2 plugin-shell capability schema for exact syntax — `src-tauri/gen/schemas/capabilities.json`).

### P1-5 — `dialog:default` capability unused

**File:** `src-tauri/capabilities/default.json:19`

**Fix:** Grep frontend for `dialog`. If unused (likely), delete the entry. If used, scope to the specific dialog primitives the code actually calls.

### P1-6 — `pending_oauth` PKCE pair never expires

**Files:** `src-tauri/src/app_state.rs:76`; `src-tauri/src/commands.rs:175-181` (`start_oauth_flow`, `submit_oauth_code`)

**Fix:** Replace `pending_oauth: RwLock<Option<PkcePair>>` with `RwLock<Option<(PkcePair, Instant)>>`. In `submit_oauth_code`, check elapsed time; reject + zeroize if > 10 minutes. Zeroize on replacement when a new flow starts.

### P1-7 — Source-file paths leak to frontend

**Files:** `src-tauri/src/jsonl_parser/walker.rs:64`; `src-tauri/src/store/queries.rs` (`session_events.source_file` column); `src-tauri/src/commands.rs::get_session_history` returns these paths

**Why:** Absolute paths like `/Users/alice/.claude/projects/secret-project/session.jsonl` are sent to the frontend and could be captured by a compromised dependency.

**Fix:** Store the relative path from the Claude projects root in `source_file` (use `walker::claude_projects_root()` as the base). Migration: drop and re-ingest is acceptable per the existing dedup design.

### P1-8 — Log directory not ACL-restricted

**File:** `src-tauri/src/logging.rs:6`

**Fix:** After `create_dir_all`, apply the same `restrict_permissions` pattern used for credentials. Tokens shouldn't appear in logs, but anything emitted under `tracing::error!` with auth context could end up there; defence in depth.

## Concurrency / blocking I/O

### P1-9 — Blocking `security` subprocess in async context (macOS)

**File:** `src-tauri/src/auth/claude_code_creds/macos.rs:38, 63, 99`

**Fix:** Wrap each `Command::new("security").output()` call in `tokio::task::spawn_blocking(|| { ... }).await?`. These are called from async contexts (`get_access_token`, `has_creds`).

### P1-10 — Blocking `icacls` subprocess in async context (Windows)

**File:** `src-tauri/src/auth/token_store.rs:83-95`

**Fix:** Same as P1-9, or replace with direct `windows-sys` ACL API calls (the dependency is already pulled in).

### P1-11 — `STALE_EMITTED` global not reset on sign-out

**File:** `src-tauri/src/poll_loop.rs:14`

**Why:** Static `AtomicBool`. Once set true during stale period, never emits `stale_data` again until process restart.

**Fix:** Move the flag into `AppState` so it's account-scoped, *or* add `STALE_EMITTED.store(false, Ordering::SeqCst)` to `commands::sign_out` after `cached_usage` is cleared.

## Frontend correctness

### P1-12 — Tab slide direction always forward

**File:** `src/report/ExpandedReport.tsx:132` (`custom={1}` hardcoded)

**Fix:** Track the previous tab index in a `useRef`. Pass `custom={newIndex > prevIndex ? 1 : -1}` so backward navigation animates left-to-right.

### P1-13 — `AuthConflictChooser.pick()` swallows IPC failures silently

**File:** `src/settings/AuthConflictChooser.tsx:14-17`

**Fix:** Wrap the `await ipc.pickAuthSource(source)` in `try/catch`. On error, set local error state and render below the buttons. Do not call `dismiss('conflict')` on failure.

### P1-14 — `AuthPanel.startOauth()` transitions step before awaiting `openUrl`

**File:** `src/settings/AuthPanel.tsx:40-49`

**Fix:** Move `setStep('paste')` to *after* the `await openUrl(...)` resolves. On error, leave `step === 'choose'` and surface the URL for manual open.

### P1-15 — `SettingsPanel` derives state via `useEffect`

**File:** `src/settings/SettingsPanel.tsx:21, 25`

**Fix:** Initialise once with `useState(() => settings)`. Provide an explicit "Reset" button if reverting to store state is needed. Drop the `useEffect(() => setLocal(settings), [settings])`.

### P1-16 — Donut chart mutates render-scoped variable

**File:** `src/report/ModelsTab.tsx:85-96`

**Fix:** Compute segment offsets inside the existing `useMemo` that builds `segments`, storing `offset` on each segment object. Remove the `let accumulatedOffset = 0` mutation in the render body.

## Type contract & state model

### P1-17 — Generated specta bindings are bypassed

**Files:** `src/lib/types.ts` (manually maintained, the production source); `src/lib/generated/bindings.ts` (specta-generated, unused); `src/lib/ipc.ts` (calls `invoke<T>` typed against `types.ts`)

**Why:** The two type files have already diverged: `burn_rate` optional-vs-required, `fetched_at` optional-vs-required, `event_id` missing from `types.ts` entirely. Any future Rust schema change will silently slip past the type system.

**Fix:**
1. Delete `src/lib/types.ts`.
2. Re-export the needed types from `src/lib/generated/bindings.ts` (consider a barrel file to keep imports stable).
3. Refactor `src/lib/ipc.ts` to call `commands.*` from `bindings.ts` instead of raw `invoke`.
4. Fix any TypeScript errors that surface — they are real bugs the manual types were hiding.

This is a meaningful refactor (~2-3 hours) but unlocks the entire reason for having specta in the build.

### P1-18 — `update_settings` accepts unchecked input

**File:** `src-tauri/src/commands.rs:259-261`

**Fix:** Validate before persisting:
- `polling_interval_secs >= 60` (matches the floor in `poll_loop.rs`).
- Every `thresholds[i] <= 100`.
- Return `Err(...)` with a specific message if validation fails. Frontend's `ipc.updateSettings` already returns `Promise<void>` — surfacing the error to the user requires a small SettingsPanel touch.

### P1-19 — `preferred_source` not persisted

**File:** `src-tauri/src/auth/orchestrator.rs` (`Mutex<Option<AuthSource>>` field)

**Why:** After conflict resolution, the next launch re-detects both credential sources and re-prompts.

**Fix:** Persist via the `settings` table (depends on P0-1) as a new field on `Settings`, *or* as a separate single-row table. Read at orchestrator construction; write in `pick_auth_source`.

### P1-20 — Non-UTF-8 path encoding

**File:** `src-tauri/src/jsonl_parser/walker.rs:64` (`path.display().to_string()` for the cursor key)

**Fix:** Use `path.to_string_lossy().into_owned()` consistently *or* `path.to_str().ok_or_else(|| anyhow!("non-UTF-8 path: {:?}", path))?` and propagate the error. Pick the latter if you want surface-level diagnostics on weird paths.

## Testing structural gaps

### P1-21 — Frontend has zero tests despite full vitest setup

**Files:** `vite.config.ts` (vitest config block); `src/test-setup.ts`

**Why:** `pnpm test` passes vacuously. The CI workflow runs it. The entire UI layer is untested.

**Fix (start small):** Add tests for the lowest-hanging fruit first to establish patterns:
1. `src/lib/format.test.ts` — pure functions like `formatRelativeTime`, percentage formatting.
2. `src/lib/store.test.ts` — event-handler reducers (mock `listen` from `@tauri-apps/api/event`).
3. `src/popover/UsageBar.test.tsx` — render with various utilization values, assert the threshold colour class.

**Pattern note:** mock `@tauri-apps/api` and `@tauri-apps/api/event` in `test-setup.ts` so that components don't need real Tauri at test time.

### P1-22 — `auth_orchestrator` integration test always skips on dev machines

**File:** `src-tauri/tests/auth_orchestrator.rs:9-11`

**Why:** Bails out on any machine with Claude Code installed. The conflict-resolution path, refresh-if-needed path, and preferred-source routing are untested.

**Fix:** Refactor `AuthOrchestrator::new` to accept injectable `TokenExchange` and `IdentityFetcher` collaborators (traits or closures). Default constructor stays as-is. Test constructor takes mocks. Then write tests for each `match` arm in `get_access_token`.

### P1-23 — `token_store` keyring-fallback has no tests

**File:** `src-tauri/src/auth/token_store.rs`

**Fix:** Add a roundtrip test for `save_fallback` / `load_fallback` using a temp dir. Verify the file is written, the permissions/ACLs are applied, the round-trip preserves the token, and a corrupted fallback file returns `Ok(None)` (not an error).

---

# P2 — Backlog

Real issues but not urgent. Address opportunistically when touching the file. File:line and one-line fixes only — full justification is in the audit transcript (commit history of this doc, or ask the human).

## Backend
- `src-tauri/src/jsonl_parser/walker.rs:60` — cast `Duration::as_nanos()` (u128) to `i64` saturating; current cast silently overflows ~year 2262.
- `src-tauri/src/poll_loop.rs` first-failure UX — `usage_updated` handler in `store.ts` clears `stale: false` when a placeholder `CachedUsage` arrives. Move stale signaling into `CachedUsage` itself or check `last_error`.
- `src-tauri/src/auth/orchestrator.rs:64` — apply `refresh_if_needed` to ClaudeCode tokens too; currently expired CLI tokens go straight to 401.
- `src-tauri/src/store/mod.rs:58-60` — `Db::conn().expect("db mutex poisoned")` cascades a single panic into permanent crash. Use `unwrap_or_else(|e| e.into_inner())`.
- `src-tauri/src/tray_icon/windows.rs:18` — `DIGIT_HEIGHT_PX = 17.0` on a 32×32 canvas is too tall (mac uses same value on a 44px cell). Set ~`9.0`.
- `src-tauri/src/auth/orchestrator.rs:79-98` — conflict path re-fetches `/userinfo` for both tokens every poll cycle. Cache `AccountInfo` per token in the orchestrator with a short TTL.
- `src-tauri/src/jsonl_parser/walker.rs:27-38` — `metadata().file_type().is_symlink()` is always false (`metadata` follows symlinks). Use `symlink_metadata`.
- `src-tauri/src/jsonl_parser/pricing.rs:58-60` — `.contains()` for prefix match could spuriously match. Use `starts_with` or token-aware match.
- `src-tauri/src/lib.rs::setup` — JSONL watcher start failure logs `tracing::error!` but doesn't surface to UI. Spec promises a banner. Emit an event.
- Three `reqwest::Client` instances (`UsageClient`, `TokenExchange`, `IdentityFetcher`). Share one.
- `src-tauri/Cargo.toml` `rusqlite` `chrono` feature is unused.
- No auto-update mechanism. Wire `tauri-plugin-updater` even if unsigned.
- `AppState` mixes auth-session state (`pending_oauth`), pipeline buffer (`recent_five_hour`), and infrastructure handles. Move `pending_oauth` into `AuthOrchestrator`; move `recent_five_hour` private to `poll_loop`.

## Frontend
- `src/popover/CompactPopover.tsx:146-150` — `LoadingShell` 1Hz polling has no cancellation guard. Add a `cancelled` ref or `AbortController`.
- `src/components/ui/Slider.tsx:26-29, 56` — keeps `internalValue` state even when `controlledValue` is provided; redundant render per drag.
- `src/report/TrendsTab.tsx:128` — 30d range labels misalign under bars (filter produces 6 labels but layout is `justify-between`).
- `src/components/ui/Tabs.tsx:62-65` — unmounts inactive panels (`null` return). Use `hidden` attribute to preserve scroll/focus state.
- `src/popover/ResetCountdown.tsx:19-22` — 30s tick rate; minute display can be ~30s stale. Use 10s tick when `< 5 min` remaining.
- `src/settings/SettingsPanel.tsx:67-71` — `accountStatus.source` hardcoded `'OAuth'` for any non-null usage. Expose actual source from backend in `CachedUsage`.
- `src/components/ui/Banner.tsx:37-44` — dismiss `<button>` missing `type="button"`.
- `src/settings/AuthPanel.tsx:114, 134` — auth-choice `<button>` elements missing `type="button"`.
- `src/report/TrendsTab.tsx:63` — range selector `<button>` elements missing `type="button"`.
- `src/report/HeatmapTab.tsx:41 vs 74-77` — `getMonthPositions` uses Sunday-first day index; grid uses Monday-first. Off-by-one column.
- `src/report/HeatmapTab.tsx:48` — O(n²) `cells.indexOf(cell)`. Use `forEach((cell, idx) => …)`.
- `src/components/ui/Slider.tsx:73-74` — `[&::-webkit-slider-thumb]:focus-visible:*` unsupported in WebKit. Apply focus styling to the input itself.
- `src/report/CacheTab.tsx:74` — hardcoded `text-[28px]`. Use `text-[length:var(--text-display)]`.
- `src/lib/motion.ts:69-139` — five exported variants (`stalePulse`, `thresholdFlash`, `barFill`, `cardStagger`/`cardChild`, `numberTick`) and `popoverMount` are unused. Either wire them up where the comments suggest or delete.
- `src/styles/globals.css:136-143` — `prefers-reduced-motion` reduces durations to 100ms; should be ~0ms for vestibular accessibility.

## Tests
- No tests for: `poll_loop::poll_once` (auth failure paths, transient placeholder synthesis), `commands.rs` (PKCE flow, aggregations, resize_window validation), `app_state::is_stale`, `tray_icon`, `jsonl_parser/watcher` (filesystem-event triggered ingestion), `notifier` for all five bucket types, `store::Db` corrupted-file open path.
- `src-tauri/tests/usage_api_client.rs` — no timeout test; no malformed-JSON-body-on-200 test.
- No integration / IPC end-to-end tests. The generated `bindings.ts` is the natural seam for an end-to-end check.

---

# P3 — Nitpicks

Address only when touching the file for another reason.

- `src-tauri/src/logging.rs:14-18` — file and stderr layers share filter; every event written twice.
- `src-tauri/src/commands.rs:69, 99, 128, 151` — four breakdown commands re-fetch raw events independently. Minor inefficiency.
- `whoami` crate could be replaced with `std::env::var("USERNAME")` (Windows-only path).
- `rand 0.8` will eventually need bumping to 0.9.
- specta RC versions pinned (correct), but no automation to flag the eventual stable release.

---

# Sprint sequencing

Roughly one focused engineer-week per sprint. Tracks within a sprint are independent; you can parallelise.

## Sprint 1 — Stop the bleeding (P0)
**DoD:** All P0 items merged. CI green. Manual smoke test of popover + expand + settings + sign-out + restart.
1. P0-2 — Set CSP. Ship first; one-line config change.
2. P0-1 — Wire settings persistence. Add migration test.
3. P0-7 — Define `--ease-spring`. Visual diff.
4. P0-5 — Track + clean up event listeners.
5. P0-6 — Move hooks above early returns.
6. P0-4 — Replace `expect` in tray.rs.
7. P0-3 — Cursor write atomicity. Add concurrent-ingest regression test.
8. P0-8 — **Decision required**: implement db_reset OR remove from contract. Don't leave half-wired.

## Sprint 2 — Security hardening
**DoD:** Tokens zeroized; capabilities scoped; no blocking syscalls in async; logs ACL'd.
- P1-1, P1-2, P1-3, P1-4, P1-5, P1-6, P1-7, P1-8, P1-9, P1-10.

## Sprint 3 — Type contract + tests
**DoD:** `types.ts` deleted; `ipc.ts` consumes `bindings.ts`; orchestrator has injectable collaborators with tests; CI runs at least one frontend test that asserts something.
- P1-17, P1-18, P1-19, P1-21, P1-22, P1-23.

## Sprint 4 — Frontend correctness + arch cleanup
**DoD:** All listed frontend bugs fixed; `STALE_EMITTED` scoped correctly; path encoding robust; `AppState` slimmed.
- P1-11, P1-12, P1-13, P1-14, P1-15, P1-16, P1-20, plus AppState refactor.

## After Sprint 4
Drain P2 opportunistically. P3 only when touching the file.

---

# Verification checklist (run before any PR)

```bash
pnpm install --frozen-lockfile
pnpm lint                                         # tsc --noEmit
pnpm test                                         # vitest
cd src-tauri
cargo test --all-features --no-fail-fast          # Rust unit + integration
cargo clippy --all-targets -- -D warnings         # Rust lints
cd ..
```

Run on both platforms before merging cross-cutting changes (`tauri.conf.json`, `capabilities/`, anything in `auth/claude_code_creds`, anything in `tray_icon/`):
- macOS host
- Windows host

The CI matrix at `.github/workflows/test.yml` covers all three (Linux runs the OS-agnostic Rust tests only — `#[cfg]` excludes platform-specific code), but local verification before pushing saves a full cycle.

---

# Engineering scorecard

A snapshot of the codebase across 15 dimensions. Each is scored **0–10** with a one-line rationale. Re-run this scorecard after Sprint 4 to measure progress; targets reflect what's achievable with the P0/P1 work landed.

| # | Dimension | Now (2026-04-29) | Target post-refactor | Rationale (now) |
|---|---|---:|---:|---|
| 1 | **Correctness** (does it do the right thing on the happy path?) | 6 | 9 | Several real bugs found (P0-1 settings, P0-3 ingest race, P0-5 listener leak, P0-6 hooks order). Dedup model and forward-compat parsing are correct. |
| 2 | **Architecture & modularity** | 7 | 9 | Clean Rust module split. `AppState` mixes concerns. Settings split across 3 places. `types.ts` parallel to generated bindings. |
| 3 | **Type safety / contract integrity** | 6 | 9 | Rust types strong. Frontend bypasses specta-generated bindings; manual `types.ts` already diverges in 3 places. |
| 4 | **Test coverage** | 5 | 8 | Rust ~70% module coverage with mostly thorough depth. Frontend: **zero tests** despite full vitest setup. Several error paths untested. |
| 5 | **Security posture** | 5 | 9 | `csp: null`, no token zeroization, blocking syscalls in async, Windows credential TOCTOU. Has rustls, keyring, single-instance. |
| 6 | **Performance & resource use** | 7 | 8 | Tight binary (LTO, opt-z), modest poll loop. Unbounded mpsc channel, three separate `reqwest::Client` instances. |
| 7 | **Maintainability / readability** | 8 | 9 | Comments are about *why* not *what*. Clean naming. Some dead code (5 unused motion variants, dead `settings` table, dead specta export path). |
| 8 | **Cross-platform parity** | 7 | 8 | `#[cfg]` discipline is clean. Real platform-specific bugs (Windows tray digit overflow, macOS `security` blocking, Windows credential TOCTOU). |
| 9 | **Error handling** | 6 | 8 | Backoff and transient-placeholder are good. Watcher start failure is silent. Keyring fallback is silent. Several `expect`/`unwrap` in user-reachable paths. |
| 10 | **Build & tooling** | 8 | 8 | Modern stack (Tauri v2, Vite 6, pnpm 10, Rust stable). CI runs all 3 OSes. specta RC pinning is correct. |
| 11 | **Design-system fidelity** | 8 | 9 | Strong token discipline — one hardcoded `text-[28px]`, zero hex colours in components. But `--ease-spring` referenced in 7 places, defined nowhere → all CSS spring transitions broken. |
| 12 | **Documentation accuracy** | 6 | 9 | Spec has drifted in 8 places (OAuth URLs, schema, command list, deferred tabs that ship). CLAUDE.md is current. This audit doc is current. |
| 13 | **Dependency hygiene** | 7 | 8 | RC versions pinned (correct). One redundant feature (`rusqlite` `chrono`). Three independent `reqwest::Client` instances. |
| 14 | **Resilience / recovery** | 4 | 8 | `db_reset` declared but never emitted. Settings reset on every restart. JSONL watcher start failure is silent. Single SQLite mutex poison cascades crashes. |
| 15 | **Accessibility** | 7 | 8 | Solid baseline (aria-labels on icon buttons, `role=progressbar/tablist/alert`). Slider thumb focus invisible (WebKit selector unsupported). `prefers-reduced-motion` reduces to 100ms instead of ~0ms. |
| **Weighted average** | | **6.4** | **8.4** | (Equal-weighted mean of the 15 scores.) |

## Scoring rubric (use the same when re-scoring)

| Score | Meaning |
|---|---|
| 0–2 | Broken or non-existent. Active blocker. |
| 3–4 | Present but with serious gaps. Would not pass review at a quality-conscious shop. |
| 5–6 | Adequate. Ships, but visible structural issues. |
| 7 | Solid. Passes review with notes. |
| 8 | Good. Meets professional-product expectations. |
| 9 | Excellent. Best-in-class for this domain. |
| 10 | Perfect / aspirational. Reserve for true exemplars. |

## How to re-score

After Sprint 4, copy this table into a new section dated with the run, change the **Now** column header to that date, and re-evaluate each row. **Do not move the original numbers** — they're the historical baseline. A finding-by-finding diff is more useful than a single number, but the headline number ("we went from 6.4 to 8.X") is what stakeholders remember.

The targets above assume P0 + P1 land. If only P0 lands, target ≈ **7.2**. If only Sprint 1 lands, target ≈ **6.9**.

---

# Open decisions for the human

Discuss before implementing:

1. **P0-8 db_reset** — implement spec §5C or remove from contract?
2. **HeatmapTab + CacheTab** — spec §1 deferred them to v1.1, but they ship. Accept (update spec) or remove from `TAB_CONFIG`?
3. **P0-7 ease-spring curve** — Material easeOut (`cubic-bezier(0.22, 1, 0.36, 1)`) or back-ease-out (`cubic-bezier(0.34, 1.56, 0.64, 1)`)?
4. **P1-17 — types.ts deletion** — sized at ~2-3 hours including fixing the bugs that surface. Confirm scope.
5. **Auto-update (P2)** — wire `tauri-plugin-updater` in v1 or defer to v1.1? Without it, security fixes don't reach users.

---

*End of audit guide. If this document is over a month old, re-verify against `git log` before relying on the line numbers — they were accurate at commit `5359dd9` on 2026-04-29.*
