# Claude Usage Monitor — Design Specification

**Date:** 2026-04-24
**Revision:** 2 (post spec-review — see `docs/spec-review.md`)
**Status:** Design approved, pending implementation plan
**Project directory:** `claude-usage-monitor/` (working name — rename before public release)

---

## 1. Overview

A cross-platform (macOS + Windows) menu-bar utility that monitors a Claude subscription's rate-limit usage (5-hour and 7-day buckets, with Opus / Sonnet splits on the 7-day window) and provides per-session analytics from local Claude Code logs. Designed as an open-source alternative to the 7 existing tools surveyed, differentiated by:

- **True feature parity across Windows + macOS** (UI fidelity may differ on Windows 10 where Mica / acrylic backdrops are unavailable — see §2.5)
- **Minimal footprint** (Tauri v2 → ~10 MB bundle, ~50–80 MB RAM — vs. Electron's 150+ MB)
- **Clean OAuth 2.0 + PKCE authentication** (paste-back flow, matching Anthropic's registered redirect)
- **Glassy modern UI** (Raycast / Linear / Arc aesthetic)

### Core requirements

| Requirement | Decision |
|---|---|
| Monitoring scope | Claude subscription limits (5h + 7d buckets, 7d Opus/Sonnet splits, extra-usage credits) **and** per-session breakdown from local JSONL |
| Form factor | Menu bar / system tray icon with popover |
| Primary view | Compact: 5h bar + 7d bar + reset timers + (if enabled) extra-usage bar |
| Secondary view | Expanded report with 4 tabs (Sessions, Models, Trends, Projects) |
| Authentication | OAuth PKCE primary (paste-back flow); read existing Claude Code credentials as optional shortcut |
| Platforms | Windows 10/11 + macOS 14+ — true feature parity |
| Aesthetic | Glassy / modern (Raycast-style), translucent popover, soft gradients, monospace accents |
| Notifications | Threshold alerts only (user-configurable %s) |
| Polling | User-configurable 1m–30m (default 5m) |
| Distribution | Public MIT on GitHub, manual releases, unsigned builds |

### Non-goals (v1)

- Code signing / notarization
- Winget / Homebrew distribution
- Team / organization aggregation
- Webhook integrations (Discord / Slack / Telegram)
- Non-Anthropic providers (Copilot, Codex, etc.)
- Mobile / web companion
- Theme customization beyond light/dark
- **Heatmap tab** (deferred to v1.1 — would be ~75% synthesized over a 90-day backfill on first launch; replaced in v1 by a 30-day strip inside the Trends tab)
- **Cache tab** as standalone — folded into Models tab as a collapsible section
- **Crash reporting** actual implementation (v1 ships the opt-in toggle defaulting off; wiring deferred to v1.1)

---

## 2. Architecture

### High-level topology

```
┌─────────────────────────────────────────────────────────────┐
│  Tray Icon  ─ click ─►  Popover Window (webview)            │
│   ↓ (badge % updates from backend)                           │
│   └──────────── IPC (Tauri commands + events) ──┐           │
│                                                  ↓           │
│  ┌─────────────────── Rust Backend ──────────────────────┐  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │  │
│  │  │  Auth    │  │ Usage    │  │  JSONL   │  │ Store  │ │  │
│  │  │ (OAuth + │  │  Poller  │  │  Parser  │  │(SQLite)│ │  │
│  │  │  creds)  │  │          │  │ (watcher)│  │        │ │  │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘  └───┬────┘ │  │
│  │       └─── Notification Engine ──┘            │       │  │
│  └───────────────────────┼────────────────────────┼──────┘  │
│                          ↓                        ↓          │
│              OS Notifications              ~/.local/...     │
└──────────────────────────────────────────────────────────────┘
                           ↓
          api.anthropic.com (OAuth + usage)
          ~/.claude/projects/<slug>/*.jsonl (local, one level)
```

### Tech stack

| Layer | Choice | Reason |
|---|---|---|
| Desktop framework | **Tauri v2** | ~10 MB bundle, ~50–80 MB RAM, WebView2/WKWebView based. Proven by ai-token-monitor. |
| Backend language | **Rust** (stable) | Performance for JSONL parsing, strong type system for IPC contracts. |
| Frontend framework | **React 19 + TypeScript** | Mature, fits glassy-UI aesthetic with Framer Motion. |
| Styling | **Tailwind CSS v4** | Fast iteration, works with CSS custom-property token system. |
| State management | **Zustand** | Lightweight, sufficient for this scope. |
| Charts | **Recharts** | React-native, avoids bundling D3 directly. |
| Animations | **Framer Motion** | Spring physics for the premium feel. |
| Database | **SQLite via `rusqlite`** | Local-only, zero external deps. |
| HTTP client | **`reqwest`** (Rust) | 429/timeout ergonomics, proven in ai-token-monitor. |
| File watcher | **`notify` crate** | Cross-platform, proven. |
| App credential storage | **`keyring` crate** (primary) + restricted-ACL file (fallback) | Keychain on macOS, Credential Manager on Windows. Fallback file: 0o600 on macOS/Unix; on Windows, the file is ACL-restricted to the current user (DACL via `SetNamedSecurityInfoW` / `icacls /inheritance:r /grant:r "%USERNAME%:F"`) since NTFS doesn't honor POSIX mode bits. |
| Build / packaging | **Tauri CLI 2** | Produces `.dmg` on macOS hosts and `.exe` + `.msi` on Windows hosts. **CI matrix required** — Tauri v2 does NOT cross-compile macOS from non-macOS hosts (requires Apple tooling). |

### OAuth configuration (concrete values)

- `client_id = 9d1c250a-e61b-44d9-88ed-5944d1962f5e` (Claude Code's public client — see "Risks accepted" §11)
- `authorize_url = https://claude.ai/oauth/authorize`
- `redirect_uri = https://platform.claude.com/oauth/code/callback`
- `token_endpoint = https://platform.claude.com/v1/oauth/token`
- `scopes = user:profile user:inference`
- `authorize` query includes `?code=true` → Anthropic's hosted "show the code" page displays `code#state` for the user to paste back into the app

**Flow is paste-back, not custom-scheme deep-link.** Claude's OAuth server rejects any redirect_uri other than the registered one; this is the only viable pattern for third-party apps today.

### Usage-API request headers (required)

- `Authorization: Bearer <access_token>`
- `anthropic-beta: oauth-2025-04-20` (without this header the endpoint returns 4xx)
- `User-Agent: claude-usage-monitor/<version>` (cosmetic, but polite)

### Component responsibilities

**Tauri shell**
Tray icon + popover window (transparent, vibrancy/Mica-enabled per §2.5), secondary expanded window, auto-launch at login, single-instance guard.

**Rust backend — 5 isolated modules:**

- `auth` — PKCE generation, paste-back code exchange, refresh-token rotation, Claude Code credential reader (OS-conditional), account-identity resolution via `/api/oauth/userinfo`. Single public surface: `get_access_token() -> Result<(String, AuthSource, AccountId)>`.
- `usage_api` — Calls `api.anthropic.com/api/oauth/usage` with required headers, parses the 5-field response (`five_hour`, `seven_day`, `seven_day_sonnet`, `seven_day_opus`, `extra_usage`). Handles 429 backoff, 401 refresh-and-retry.
- `jsonl_parser` — Walks `~/.claude/projects/<slug>/*.jsonl` (one level deep, symlinks skipped), extracts session events with forward-compatible serde (unknown fields ignored), writes to store. Uses `notify` crate; handles file truncation (cursor > file-size ⇒ reset cursor to 0).
- `store` — SQLite, three tables: `api_snapshots` (30-day retention), `session_events` (90-day retention), `notification_state` (threshold-crossing memory). Single file lock for multi-instance safety.
- `notifier` — Evaluates threshold rules against latest snapshot, fires OS notifications via Tauri's notification plugin. Dev-only `debug_force_threshold(bucket, pct)` command for deterministic testing.

**React frontend** — View layer only; never touches filesystem or network. Two screens: `CompactPopover` (tray) and `ExpandedReport` (separate window, 4 tabs). State via Zustand, IPC through typed `invoke()` wrappers generated from Rust types.

### 2.5 Cross-platform parity matrix

| Surface | macOS (14+) | Windows 11 | Windows 10 | Library / notes |
|---|---|---|---|---|
| Popover backdrop | NSVisualEffectView vibrancy (system materials) | Mica | Translucent solid fallback (no blur) | `tauri-plugin-window-vibrancy`; Win10 uses CSS tinted fill |
| Tray icon | Template image (auto-tints for menu-bar) + color overlay badge when >75% | Full-color ICO, color-coded directly | Same as Win11 | Badges drawn at runtime; template path used for base icon on macOS |
| Auto-launch at login | `tauri-plugin-autostart` → SMAppService (may prompt re-approval for unsigned apps after Gatekeeper revalidation) | `tauri-plugin-autostart` → `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` | Same as Win11 | Known limit: SMAppService may silently fail on non-notarized apps; fall back to `LaunchAgents` plist if available |
| App's own credential storage | Keychain via `keyring` crate | Credential Manager via `keyring` crate | Same as Win11 | Fallback file at `<config>/credentials.json`: 0o600 on macOS; DACL-restricted to current user on Windows (`SetNamedSecurityInfoW` or `icacls /inheritance:r /grant:r` equivalent) |
| Reading Claude Code credentials | Keychain: enumerate `Claude Code-credentials*` service names via `security dump-keychain`; trim occasional leading non-ASCII byte before JSON parse | File: `%USERPROFILE%\.claude\.credentials.json` (plaintext JSON) | Same as Win11 | WSL Claude Code: v1 non-goal — `wsl.exe -d <distro> -- cat ~/.claude/.credentials.json` is documented as v2 feature |
| Notifications | `tauri-plugin-notification` (User Notifications framework) | `tauri-plugin-notification` (Toast XML) | Same | Requires first-launch permission prompt on macOS |
| Deep-link | **Not used** (paste-back flow avoids this entirely) | — | — | — |
| Updater | Manual (GitHub releases) | Manual | Manual | No Sparkle / winget in v1; out-of-scope |
| Single-instance guard | `tauri-plugin-single-instance` + SQLite file lock | Same | Same | Second launch raises existing popover |

**Soft guarantees documented in UI:** Win10 users see a footer in Settings → About: "Backdrop effects limited on Windows 10 — consider upgrading for full visual fidelity." No hidden degradation.

---

## 3. File Layout

### Rust backend (`src-tauri/src/`)

```
src-tauri/src/
├── main.rs                          # Tauri app bootstrap, tray, single-instance, command registration
├── auth/
│   ├── mod.rs                       # pub fn get_access_token() -> (token, source, account_id)
│   ├── oauth_paste_back.rs          # PKCE generation, browser open, code parsing, token exchange
│   ├── claude_code_creds.rs         # Dispatcher
│   ├── claude_code_creds_macos.rs   # Keychain discovery + stray-byte trim
│   ├── claude_code_creds_windows.rs # Plaintext file read from %USERPROFILE%\.claude\.credentials.json
│   ├── token_store.rs               # keyring crate + 0o600 file fallback; docs encryption-at-rest
│   └── account_identity.rs          # /api/oauth/userinfo fetch for conflict resolution
├── usage_api/
│   ├── mod.rs                       # pub async fn fetch_usage(token) -> UsageSnapshot
│   ├── client.rs                    # reqwest with required headers, 429 backoff, 30s timeout
│   └── types.rs                     # UsageSnapshot, Utilization, ExtraUsage — serde round-trip tested
├── jsonl_parser/
│   ├── mod.rs                       # pub fn start_watcher() + pub fn backfill(days)
│   ├── walker.rs                    # One-level glob: ~/.claude/projects/<slug>/*.jsonl; symlinks skipped; cursor reset on truncation
│   ├── record.rs                    # SessionEvent; serde(default) + unknown-fields-ignored
│   └── pricing.rs                   # Prefix-matched string-keyed table loaded from pricing.json
├── store/
│   ├── mod.rs                       # Single Db handle; SQLite file lock for multi-instance
│   ├── schema.sql                   # Migrations (api_snapshots, session_events, notification_state)
│   └── queries.rs                   # Typed queries
├── notifier/
│   ├── mod.rs                       # Threshold evaluator + debug_force_threshold
│   └── rules.rs                     # Fires once per crossing; clears on resets_at
└── commands.rs                      # Tauri #[command] functions

src-tauri/pricing.json                # Externalized model→cost table (shipped + runtime-override path)
src-tauri/tests/fixtures/
├── jsonl/
│   ├── current_schema.jsonl
│   ├── older_schema.jsonl
│   ├── malformed_lines.jsonl
│   └── partial_line_at_eof.jsonl
└── api_responses/
    ├── standard_account.json
    ├── extra_usage_enabled.json
    └── newer_schema_with_extra_fields.json  # forward-compat test
```

### React frontend (`src/`)

```
src/
├── App.tsx                           # Routes: CompactPopover | ExpandedReport | Settings | AuthPanel
├── popover/
│   ├── CompactPopover.tsx            # 5h bar, 7d bar (+ Opus/Sonnet sub-bars), extra-usage bar (if enabled), reset timers
│   └── UsageBar.tsx                  # Reusable glassy progress bar with threshold color transitions
├── report/
│   ├── ExpandedReport.tsx            # Tab shell (4 tabs)
│   ├── SessionsTab.tsx               # Virtualized list
│   ├── ModelsTab.tsx                 # Donut chart + collapsible Cache Efficiency section
│   ├── TrendsTab.tsx                 # Recharts line chart with 30-day strip (replaces Heatmap v1)
│   └── ProjectsTab.tsx               # Per-project bar chart
├── settings/
│   ├── SettingsPanel.tsx             # Thresholds, polling interval, theme, launch-at-login, crash-reports toggle, account switcher
│   └── AuthPanel.tsx                 # First-run chooser + OAuth paste-back UI (code input + validation)
├── lib/
│   ├── ipc.ts                        # Typed wrappers around invoke()
│   ├── store.ts                      # Zustand store
│   ├── icons.ts                      # Semantic-role → Lucide icon mapping
│   └── theme.ts                      # Glass tokens
└── styles/
    ├── globals.css
    └── tokens.css                    # Single source of design tokens (colors, spacing, radii, motion)
```

---

## 4. Interfaces (Contracts That Matter)

### Rust types (wire-faithful)

```rust
// auth/mod.rs
pub enum AuthSource { OAuth, ClaudeCode }
pub struct AccountId(pub String);  // derived from /oauth/userinfo email/id

pub async fn get_access_token() -> Result<(String, AuthSource, AccountId)>;
pub async fn start_oauth_flow() -> Result<String>;            // returns authorize URL (PKCE + URL build can fail)
pub async fn submit_oauth_code(pasted: String) -> Result<()>; // parses "code#state"
pub fn has_claude_code_creds() -> bool;
pub async fn resolve_conflict(preferred: AuthSource) -> Result<()>;

// AccountInfo — surfaced to UI for the conflict chooser and Settings → About
pub struct AccountInfo {
    pub id: AccountId,
    pub email: String,
    pub display_name: Option<String>,
}

// usage_api/types.rs — matches wire shape exactly
pub struct UsageSnapshot {
    pub five_hour: Option<Utilization>,
    pub seven_day: Option<Utilization>,
    pub seven_day_sonnet: Option<Utilization>,
    pub seven_day_opus: Option<Utilization>,
    pub extra_usage: Option<ExtraUsage>,
    #[serde(skip_deserializing)]
    pub fetched_at: DateTime<Utc>,       // set by client on receipt
    #[serde(flatten, default)]
    pub unknown: HashMap<String, serde_json::Value>,  // forward-compat
}

pub struct Utilization {
    pub utilization: f64,   // 0..100 (per claude-usage-bar convention)
    pub resets_at: DateTime<Utc>,
}

pub struct ExtraUsage {
    pub is_enabled: bool,
    pub monthly_limit_cents: u64,
    pub used_credits_cents: u64,
    pub utilization: f64,
    pub resets_at: Option<DateTime<Utc>>,
}

// Staleness is derived, not wire-level
pub struct CachedUsage {
    pub snapshot: UsageSnapshot,
    pub account_id: AccountId,
    pub last_error: Option<PollError>,
}
impl CachedUsage {
    pub fn is_stale(&self, now: DateTime<Utc>) -> bool {
        // stale if older than 15m, or clock-skew (now < fetched_at), or last_error present
        (now - self.snapshot.fetched_at) > Duration::minutes(15)
            || now < self.snapshot.fetched_at
            || self.last_error.is_some()
    }
}

// jsonl_parser/record.rs — forward-compatible
// Required fields: ts, project, model. If any are missing, the line is treated
// as malformed and skipped. Numeric/derived fields default to zero when absent.
#[derive(Deserialize)]
pub struct SessionEvent {
    pub ts: DateTime<Utc>,                 // stored UTC; converted to local only at display
    pub project: String,
    pub model: String,                     // raw string; priced via prefix match
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

### IPC surface

```rust
#[tauri::command] async fn get_current_usage() -> CachedUsage;
#[tauri::command] async fn get_session_history(range: DateRange) -> Vec<SessionEvent>;
#[tauri::command] async fn get_daily_trends(days: u32) -> Vec<DailyBucket>;
#[tauri::command] async fn get_model_breakdown(days: u32) -> ModelBreakdown;
#[tauri::command] async fn get_project_breakdown(days: u32) -> Vec<ProjectStats>;
#[tauri::command] async fn get_cache_stats(days: u32) -> CacheStats;   // consumed by Models tab
#[tauri::command] async fn start_oauth_flow() -> Result<String>;         // returns authorize URL
#[tauri::command] async fn submit_oauth_code(pasted: String) -> Result<()>;
#[tauri::command] async fn use_claude_code_creds();
#[tauri::command] async fn sign_out();
#[tauri::command] async fn update_settings(s: Settings);
#[tauri::command] async fn get_account_info() -> AccountInfo;
#[tauri::command] async fn pick_auth_source(source: AuthSource);        // conflict resolution
#[tauri::command] async fn open_expanded_window();
#[cfg(debug_assertions)]
#[tauri::command] async fn debug_force_threshold(bucket: Bucket, pct: u8);
```

### Events (backend → frontend)

- `usage_updated` — fires after each successful poll with new `CachedUsage`
- `session_ingested` — fires when JSONL watcher picks up new rows (debounced 500ms)
- `auth_required` — refresh token expired; UI shows reconnect banner
- `auth_source_conflict` — OAuth token + Claude Code creds resolve to different AccountIds; UI shows chooser
- `stale_data` — snapshot age > 15m
- `db_reset` — SQLite corrupted on startup, fresh DB created

### Type generation

Shared types use `ts-rs` or `specta` to emit TypeScript definitions at build time. Frontend imports from `src/lib/generated/bindings.ts`. No type drift possible.

---

## 5. Data Flow

### Scenario A: First-run authentication (paste-back flow)

```
User launches app
  └─► Tray icon appears with "?" badge
  └─► First-run popover shows two buttons:
        [ Sign in with Claude ]   [ Use Claude Code credentials ]
                                  (shown only if has_claude_code_creds() == true)

  (OAuth paste-back path)
  1. Backend generates PKCE verifier + challenge
  2. Backend returns authorize URL to frontend:
     https://claude.ai/oauth/authorize
       ?response_type=code&client_id=9d1c250a-e61b-44d9-88ed-5944d1962f5e
       &redirect_uri=https://platform.claude.com/oauth/code/callback
       &scope=user:profile user:inference
       &code_challenge=<S256>&code_challenge_method=S256
       &state=<random>&code=true
  3. Frontend opens URL in system browser; AuthPanel now shows a text field: "Paste the code shown on the callback page"
  4. User logs in on claude.ai, approves scopes → Anthropic's callback page renders "code#state" as a copy-able string
  5. User copies it, pastes into AuthPanel, clicks Continue
  6. Frontend calls submit_oauth_code(pasted)
  7. Backend parses "code#state", verifies state matches, exchanges code at token_endpoint
  8. Refresh token → keyring (file fallback on failure); access token → memory; AccountId resolved via /api/oauth/userinfo
  9. First fetch_usage() kicks off → popover transitions to normal view

  (Local creds path)
  1. macOS: claude_code_creds_macos::load()
     a. Enumerate service names via `security dump-keychain` filtered by "Claude Code-credentials"
     b. For each match, `security find-generic-password -s <name> -w`
     c. If the first byte is non-ASCII, skip it; parse remainder as JSON
     d. Pick the credential with the longest remaining TTL
     Windows: claude_code_creds_windows::load()
     a. Read %USERPROFILE%\.claude\.credentials.json directly
     b. Parse JSON; extract claudeAiOauth.accessToken + refreshToken + expiresAt
  2. Resolve AccountId via /api/oauth/userinfo
  3. Same continuation from OAuth step 8

  (Both sources present — AuthSource conflict)
  - After a successful OAuth sign-in, if has_claude_code_creds() also returns true:
    a. Fetch AccountId from both sources
    b. If equal → silently prefer OAuth (user's own token, revocable from the app)
    c. If different → emit auth_source_conflict event with both email addresses;
       AuthPanel shows chooser: "Two Claude accounts detected. Which should this app monitor?"
  - Choice is persisted; next launch uses the chosen source without re-prompting.
```

### Scenario B: Polling loop (user-configurable, default 5m, range 1–30m)

```
Backend spawns a tokio task on startup. First fetch is immediate
(no 5-minute dead zone on launch); subsequent fetches wait the
configured interval.

  // Immediate first fetch
  poll_once()

  loop {
    sleep(configured_interval)          // 1m to 30m, default 5m
    poll_once()
  }

  fn poll_once() {
    (token, source, account) = auth::get_access_token()
    response = client.get("https://api.anthropic.com/api/oauth/usage")
      .header("Authorization", format!("Bearer {token}"))
      .header("anthropic-beta", "oauth-2025-04-20")
      .header("User-Agent", "claude-usage-monitor/<ver>")
      .timeout(30s)
      .send()

    match response:
      200 → parse UsageSnapshot (serde, unknown fields preserved)
            store.insert_snapshot(account, snapshot)
            emit "usage_updated" with CachedUsage { snapshot, account, last_error: None }
            notifier::evaluate(snapshot)
            tray::update_badge(snapshot.five_hour.utilization)
      401 → auth::refresh_and_retry() once
              on failure: clear token, emit "auth_required"
      429 → exponential backoff: 1m → 2m → 4m → 8m → 16m → 30m cap
            tray gets subtle "paused" dot overlay
      5xx / timeout / network err →
            emit "usage_updated" with CachedUsage { ..., last_error: Some(e) }
            no notifications fired (don't spam offline users)
  }
```

Tray badge color coding uses template image + colored overlay dot on macOS (preserves auto-tint), full-color ICO on Windows. Tooltip shows both 5h and 7d utilization plus extra-usage if enabled.

### Scenario C: JSONL parsing

```
On startup:
  backfill(days=90) runs once:
    └─► walker uses glob: ~/.claude/projects/<slug>/*.jsonl (ONE level, no **)
    └─► Skip symlinks; max depth 2; skip files larger than 100 MB (safety)
    └─► For each file: check stored (mtime, offset) cursor
         - If file size < cursor.offset → cursor reset to 0, full re-parse
         - If mtime unchanged and size >= cursor.offset → skip
         - Else: read from offset to EOF
    └─► Parse each line with serde(default) — unknown fields ignored, not errors
         - Line that fails JSON parse → skip, log "malformed line at file:N", counter++
         - If >10% of lines in a file fail → warn log with filename
    └─► Batch insert into store.session_events (1k rows per tx)
    └─► Update cursor

Live watcher (`notify` crate):
  on file-modified event (debounced 500ms):
    └─► Re-read from stored offset to EOF
    └─► Handle partial-line-at-EOF:
         - If last byte before EOF is not '\n', buffer it; don't commit cursor past it
         - Next event picks up the completed line
  on file-created event:
    └─► Add to watch set, parse from offset 0

Frontend listens for "session_ingested" (debounced 500ms) and refreshes
active tab's data — user sees new sessions appear without manual refresh.
```

**Timezone policy:** All timestamps stored as UTC in SQLite. Conversion to local only at display in React components, using the user's OS time zone (`Intl.DateTimeFormat().resolvedOptions().timeZone`). Daily buckets are computed in local time so "today" matches the user's calendar.

### Scenario D: Popover + expanded report

```
User clicks tray icon:
  └─► Tauri shows popover (app running, no startup cost)
  └─► CompactPopover mounts, calls ipc.getCurrentUsage()
        (returns instantly from in-memory last CachedUsage)
  └─► Subscribes to "usage_updated" + "stale_data" + "auth_required" events
  └─► Displays:
        - 5h progress bar + countdown to resets_at
        - 7d progress bar + countdown + tiny Opus/Sonnet sub-bars below
        - Extra-usage bar (only if extra_usage.is_enabled)
        - Banner (if auth_required / stale_data / db_reset)

User clicks "See details":
  └─► Opens second window (960×640, resizable, min 800×560)
  └─► ExpandedReport mounts with 4 tabs, default = Sessions
  └─► Lazy-fetch on tab first-view:
        Sessions: ipc.getSessionHistory({ days: 7 })
        Models:   ipc.getModelBreakdown({ days: 30 }) + ipc.getCacheStats({ days: 30 })
        Trends:   ipc.getDailyTrends(30)   // 30-day strip replaces Heatmap v1
        Projects: ipc.getProjectBreakdown({ days: 30 })
  └─► All queries hit local SQLite, zero network calls
```

### Scenario E: Threshold alert firing

```
notifier::evaluate(snapshot) after each successful poll:

  state = load notification_state for this account
  for bucket in [five_hour, seven_day, seven_day_opus, seven_day_sonnet, extra_usage]:
    if bucket is None: continue
    for threshold in settings.thresholds:    // default [75, 90]
      if bucket.utilization < threshold: continue

      // Threshold-crossing gate: for time-windowed buckets (five_hour, seven_day,
      // seven_day_opus, seven_day_sonnet) use resets_at to clear state automatically.
      // For extra_usage (credit bucket), resets_at is Option — monthly credits may have
      // a reset date, one-time top-ups don't. Fall back to a 24h re-fire cooldown.
      fired_within_this_window = match bucket.resets_at {
        Some(reset) => state.last_fired[bucket][threshold] >= bucket.previous_reset(reset),
        None        => (now - state.last_fired[bucket][threshold]) < Duration::hours(24),
      }
      if fired_within_this_window: continue

      body = match bucket.resets_at {
        Some(reset) => format!("Resets in {}", humanize(reset - now)),
        None        => "Pay-as-you-go credits running low".to_string(),
      }
      tauri_plugin_notification::send(
        title: format!("Claude {bucket_label} usage at {threshold}%"),
        body,
      )
      state.last_fired[bucket][threshold] = now
      store.save(state)
```

The `< resets_at` check auto-clears state on each reset, so next day's crossing fires fresh. `debug_force_threshold(bucket, pct)` injects a synthetic crossing for deterministic testing.

---

## 6. Error Handling

**Guiding principle:** Fail visibly, never silently. Every failure surfaces in the UI or as an actionable notification. No empty `catch(_){}`.

### Auth failures

| Failure | Response |
|---|---|
| **No auth source chosen / user dismisses first-run** | Popover stays in first-run state indefinitely (shows the two auth buttons). No polling starts, no errors fired, no background activity except JSONL parsing if `~/.claude/projects/` exists (user can still view past sessions without API auth). Tray badge shows a neutral "—" instead of a percentage. |
| OAuth authorize-page closed without pasting | No-op. Sign-in button remains; user retries. |
| Pasted code malformed / state mismatch | AuthPanel shows inline error: "Code invalid or expired — try signing in again". No token stored. |
| Token exchange HTTP 4xx | AuthPanel shows exact status + message; logs to `~/.claude-monitor/logs/`. |
| Refresh token revoked / expired | `fetch_usage` returns 401 → auth module clears stored token → emits `auth_required`. Popover banner: "Sign in to continue monitoring." No polling until user reauths. |
| Claude Code creds present initially, then gone (`claude login` expired/logged out) | Banner: "Claude Code credentials no longer available — sign in with Claude instead." |
| Keychain/Credential Manager access denied by user | Fall back to restricted-ACL file at `<config>/credentials.json` (0o600 on macOS/Unix; user-only DACL on Windows — see §2.5). Show system notification once: "Could not access secure storage — credentials saved to a protected file." |
| **Auth sources disagree (conflict)** | Fetch AccountId from both via `/api/oauth/userinfo`. If different, emit `auth_source_conflict` with both emails. AuthPanel chooser is shown; choice persisted. Polling pauses until user chooses. |
| macOS keychain prompts every launch | Use stable `app_identifier` constant; document in README that binary-path changes (e.g., moving between /Applications and ~/Downloads) re-trigger prompt. |

### Network failures

| Failure | Response |
|---|---|
| 429 rate limit | Exponential backoff 1m → 30m cap. Tray "paused" overlay. Resumes after one successful call. |
| 5xx | Retry once after 30s. If still failing, mark `last_error` on CachedUsage. "Last updated Xm ago" (amber >15m, red >1h). |
| Network unreachable | Same stale-data behavior. No notifications fired. |
| Request timeout (>30s) | Same as 5xx. |
| `/api/oauth/userinfo` fails during conflict resolution | Pause polling; show banner "Could not verify account — retry"; don't guess. |

### JSONL parser failures

| Failure | Response |
|---|---|
| `~/.claude/projects/` absent | Log info. Skip backfill + watcher. UI empty states: "No Claude Code sessions found. Install Claude Code to see per-session analytics." |
| Malformed JSONL line | Skip line, increment per-file counter, log with filename + line number. >10% failure rate → warn log. |
| **File shorter than stored offset (truncation/rotation)** | Reset cursor to 0, full re-parse. Log the rotation event. |
| **Partial line at EOF** | Buffer unterminated trailing bytes; don't advance cursor past them. Next watcher tick picks up completed line. |
| **Unknown fields in a line** | Serde `#[serde(flatten, default)]` captures them; line parses normally. Not counted as malformed. |
| File permission denied | Skip, log, continue. Surfaces in Settings → Diagnostics: "N files skipped". |
| Disk I/O error | Restart watcher after 10s backoff. 3 consecutive failures → disable JSONL with banner. |
| File size > 100 MB | Skip with warning (protects against runaway logs / symlinked binaries). |

### Store / SQLite failures

| Failure | Response |
|---|---|
| Disk full on insert | System notification once per session. Stop writing, keep reading cached data. |
| DB file corrupt on startup | Back up to `data.db.corrupt-<timestamp>`, fresh DB, trigger 90-day re-backfill. Banner: "Database was reset — rebuilding history." Emit `db_reset` event. |
| Migration failure | Back up old DB, fall back to fresh. Same banner. |
| **Second instance detected** | Tauri single-instance plugin raises the existing popover to front and exits the new process. SQLite file lock is insurance. |
| **System clock moved backward (`now < fetched_at`)** | `CachedUsage::is_stale` returns true (forces re-poll). Reset countdown uses `max(Duration::zero(), resets_at - now)`. Log the skew. |

### Platform quirks

| Failure | Response |
|---|---|
| macOS Gatekeeper blocks launch (unsigned) | README: `xattr -d com.apple.quarantine "Claude Usage Monitor.app"` |
| Windows: WebView2 runtime missing on Win10 | Tauri auto-bootstraps the runtime installer. Fallback: error dialog with MS download link. |
| Windows SmartScreen blocks launch | README: "More info → Run anyway." |
| tauri-plugin-autostart fails silently on unsigned macOS app | Fallback: write `~/Library/LaunchAgents/com.claude-monitor.plist` manually; detect failure and show Settings notice. |

### Error surface tiers

1. **In-UI banner** — auth problems, stale data, DB reset, auth-source conflict.
2. **System notification** — threshold alerts, keychain fallback triggered, disk full.
3. **Logs only** — parse warnings, transient network errors, watcher restarts, clock skew. Viewable via Settings → Diagnostics → "Open logs folder".

### Explicit non-behaviors

- **No silent fallbacks** — if OAuth fails, we don't pretend local creds worked.
- **No invisible retry loops.**
- **No empty-catch blocks** — every `Result` handled explicitly; `unwrap()` only in startup code where failure aborts the app.
- **No unsolicited telemetry** — logs stay local. Settings toggle "Send anonymous crash reports" defaults **off**; actual wiring deferred to v1.1 (the toggle ships now so v1.1 needs no settings migration).

---

## 7. Testing Strategy

Lean but load-bearing on what actually breaks in production.

### Tier 1: Rust unit tests

| Module | What to test |
|---|---|
| `usage_api::types` | **Serde round-trip tests against committed real-world fixtures** (`standard_account.json`, `extra_usage_enabled.json`, `newer_schema_with_extra_fields.json`). Unknown fields must not break parsing. |
| `jsonl_parser::walker` | Cursor advances; cursor resets on truncation; partial-line-at-EOF buffered; malformed lines skipped; symlinks skipped; depth cap; 90-day limit. |
| `jsonl_parser::pricing` | Prefix match for every currently-shipping model family: `opus-4-7`, `opus-4-6`, `opus-4-5`, `opus-4-1`, `opus-4`, `sonnet-4-6`, `sonnet-4-5`, `sonnet-4`, `haiku-4-5`, `haiku-3-5`. Test fails if any fixture model isn't priced. Cache-tier math (5m vs 1h). |
| `auth::oauth_paste_back` | PKCE verifier/challenge match spec; `code#state` parsing; state-mismatch rejected; token-exchange payload correct. |
| `auth::claude_code_creds_macos` | **Gated `#[cfg(target_os = "macos")]`, run on macOS CI.** Multi-service discovery; stray-byte trim; longest-TTL selection. |
| `auth::claude_code_creds_windows` | **Gated `#[cfg(target_os = "windows")]`, run on Windows CI.** File-not-found; malformed JSON; valid credentials extraction. |
| `usage_api::client` | 429 backoff math; 401 triggers refresh; required headers on every request; stale flag. |
| `notifier::rules` | Threshold fires once; re-fires after `resets_at`; respects clock-skew (doesn't fire if `now < resets_at` was flipped); `debug_force_threshold` works. |
| `store::queries` | Migrations apply; insert/retrieve roundtrip; retention prune; file lock prevents double-write. |

Target: **~75% coverage on those 8 modules**. No chasing on glue code.

Fixtures committed to `src-tauri/tests/fixtures/`.

### Tier 2: Rust integration tests (`src-tauri/tests/`)

- OAuth paste-back → fetch_usage → store → notifier (mocked Anthropic HTTP, including 200 / 401 / 429 paths)
- Token expires mid-poll → refresh → retry succeeds
- Both auth sources present + different accounts → `auth_source_conflict` event → user picks → only chosen source polled thereafter
- JSONL watcher: partial line at EOF → next tick completes it
- Corrupt DB on startup → backup → rebuild → `db_reset` event

### Tier 3: Frontend component tests (Vitest + RTL)

Only components with real logic:
- `UsageBar` — colors at 50/75/90%, fill width, countdown formatting, clock-skew guard
- `CompactPopover` — renders each banner type (stale/auth/conflict/db_reset)
- `AuthPanel` — paste-back validation: rejects empty, rejects missing `#`, rejects state mismatch
- `SettingsPanel` — polling slider clamps to 1–30m; threshold slider range
- `ipc.ts` — typed errors propagate

Mock Tauri's `invoke` with a stub.

### Tier 4: Manual release checklist

Documented in `docs/release-checklist.md`. Uses `debug_force_threshold` so threshold items are deterministic:

- [ ] Fresh install macOS → OAuth paste-back flow end-to-end with real Claude account
- [ ] Same on Windows 11 (WebView2 already present)
- [ ] Same on Windows 10 (verify WebView2 auto-bootstrap)
- [ ] Sign out → sign back in with local Claude Code creds → verify fetch
- [ ] Two-accounts test: sign in OAuth with account A while Claude Code creds are for account B → conflict chooser works
- [ ] `debug_force_threshold(five_hour, 75)` fires notification once
- [ ] Restart app; `debug_force_threshold(five_hour, 75)` should NOT fire again until `resets_at` passes
- [ ] Open expanded report against real 90-day history, spot-check totals against claude-usage-bar / ai-token-monitor
- [ ] Disconnect network → stale indicator appears within 15m; notifications don't fire
- [ ] Manually edit system clock backward 2h → `CachedUsage` marks stale; countdown doesn't go negative

### CI (`.github/workflows/test.yml`)

Matrix: `ubuntu-latest`, `macos-latest`, `windows-latest`.
- `ubuntu`: runs the OS-agnostic tests only (fast feedback)
- `macos`: additionally runs `claude_code_creds_macos` tests
- `windows`: additionally runs `claude_code_creds_windows` tests
- Frontend tests run on all three.

`release.yml`: on tag push, same matrix, produces unsigned `.dmg` / `.exe` / `.msi`, uploads to GitHub Releases. No Sparkle, no notarization, no Winget.

### Explicit non-tests

- No end-to-end Tauri WebDriver tests
- No visual regression
- No keychain mocking beyond fixture files
- No JSONL load tests in CI (benchmark once at 500 MB during dev, publish number, move on)

---

## 8. Comparative Positioning

Summary of the 7-project survey that informed this design:

| Project | Stack | Cross-platform | Auth | Data source |
|---|---|---|---|---|
| ai-token-monitor | Tauri v2 + Rust + React 19 | ✅ | Keychain creds + GitHub OAuth | Local JSONL |
| Claude-Code-Usage-Monitor | Native Win32 + Rust | ❌ Win-only | Reads `.credentials.json` | API OAuth endpoint |
| claude-usage | SwiftBar + Python | ❌ macOS-only | Chrome cookie extraction | `claude.ai` web API |
| **claude-usage-bar** | **Native SwiftUI** | ❌ macOS-only | **OAuth 2.0 + PKCE paste-back** ⭐ | `api.anthropic.com/oauth/usage` |
| Claude-Usage-Tracker | Native SwiftUI | ❌ macOS-only | Session key / CLI OAuth / console key | `claude.ai` web API |
| claude-usage-widget | Electron | ✅ | Hidden BrowserWindow + cookie | `claude.ai` via DOM scraping |
| ClaudeBar | Native SwiftUI (Swift 6.2) | ❌ macOS-only | Reads CLI creds + parses `claude /usage` | CLI probe + local JSONL |

### What we're borrowing

- **OAuth paste-back flow** — from claude-usage-bar (only project doing auth correctly against Anthropic's actual OAuth server)
- **Tauri v2 architecture + JSONL parser pattern + file watcher + external `pricing.json`** — from ai-token-monitor
- **Multi-service keychain discovery + stray-byte trim** — from ai-token-monitor (`oauth_usage.rs:149–177, 253–263`)
- **Cross-platform packaging config** (DMG/NSIS) — from claude-usage-widget
- **WSL credential path handling (as v2 target)** — from Claude-Code-Usage-Monitor

### What we're fixing vs. the field

- **No Electron footprint penalty**
- **True cross-platform from day one** — not a macOS-first afterthought
- **OAuth paste-back first** — not a session-cookie hack that breaks when Anthropic tightens security
- **Forward-compatible JSONL parsing** — unknown fields accepted, truncation handled
- **Correct API shape** — no per-model HashMap fiction; Opus/Sonnet splits + extra_usage handled

### Why Tauri v2 over alternatives

Considered and rejected:
- **Native per-platform (SwiftUI + WinUI3):** would give better OS fidelity but doubles the codebase. Three of the 7 competitors took this path and none of them ship cross-platform.
- **Electron:** footprint is disqualifying for a menu-bar utility (150+ MB vs. 10 MB).
- **Wry/Tao direct (no Tauri):** shaves 2–4 MB but loses Tauri's plugin ecosystem (single-instance, notifications, autostart, window-vibrancy, updater). Not worth the engineering overhead for a team of one.

Tauri v2 provides: single React codebase, single typed IPC surface (`specta`/`ts-rs`-generated TS from Rust types), shared SQLite schema, proven cross-platform builds, and a working reference implementation (ai-token-monitor).

---

## 9. Resolved Decisions

| Topic | Decision |
|---|---|
| Monitoring scope | API buckets + per-session JSONL |
| Form factor | Menu bar tray + popover |
| Authentication | OAuth paste-back primary, Claude Code creds shortcut; conflict → `/oauth/userinfo` chooser; OAuth wins on same-account |
| Platform priority | True feature parity Win/macOS (UI fidelity differences on Win10 documented) |
| Aesthetic | Glassy/modern, Raycast/Linear/Arc; designed via Owl-Listener/designer-skills (see `designer-handoff-prompt.md`) |
| Expanded report tabs | 4: Sessions, Models (with Cache section), Trends (30-day strip), Projects. **Heatmap deferred to v1.1.** |
| Notifications | Threshold alerts only; thresholds user-configurable; `debug_force_threshold` for testing |
| Polling interval | 1–30m, default 5m |
| JSONL backfill cap | Last 90 days, max depth 2, symlinks skipped |
| Crash reporting | Opt-in toggle in Settings (default off); v1.1 wires the backend |
| Distribution | Public MIT, manual unsigned releases, CI matrix (macOS + Windows + Ubuntu) |

---

## 10. Next Steps

1. User reviews this revised spec.
2. Invoke `superpowers:writing-plans` skill to produce implementation plan at `docs/superpowers/plans/`.
3. In parallel: designer agent executes `docs/designer-handoff-prompt.md` to produce UI system.
4. Implementation proceeds module-by-module against the plan with review checkpoints.

---

## 11. Risks Accepted

Called out explicitly so nothing gets "ambushed" later:

- **Client-ID reuse.** The app uses Claude Code's public `client_id` (`9d1c250a-e61b-44d9-88ed-5944d1962f5e`) because Anthropic has no third-party developer registration portal. Two other surveyed OSS projects do the same. Anthropic could break this path by tightening client verification; no mitigation exists until Anthropic publishes one.
- **Unsigned distribution friction.** macOS Gatekeeper and Windows SmartScreen both require user action on first launch. README mitigations are documented; Homebrew / Winget are deferred.
- **No telemetry in v1.** Bugs on platforms the maintainer doesn't use daily may go unreported. Opt-in crash reporting toggle ships in v1 (backend wired in v1.1).
- **Pricing is point-in-time.** New Claude model launches require a `pricing.json` update + release. Runtime-override path allows users to self-patch.
- **Cache-tier cost accuracy depends on JSONL field names.** If Claude Code's schema stops distinguishing `cache_creation_5m_tokens` vs. `cache_creation_1h_tokens`, cache-cost numbers degrade silently.
- **Single-person manual release checklist.** Requires both macOS and Windows hardware (or a Windows VM). Not CI-automatable.
- **Account-email display.** Settings shows the email returned by `/api/oauth/userinfo` so users can identify the monitored account when two are involved.

---

## Appendix — Checked and Found Fine

(From spec-review audit; recording here so future readers know what was already validated.)

- React 19 + Tailwind v4 + Zustand + Recharts + Framer Motion — production-stable stack
- `notify` crate cross-platform — proven across multiple surveyed competitors
- `rusqlite` for a single-digit-MB DB, 90-day retention — trivially fine at this scale
- `reqwest` with backoff — ai-token-monitor proves it works at this exact use case
- 5-module boundary decomposition — clean and testable
- Non-goals (§1) — well-chosen and held firm
