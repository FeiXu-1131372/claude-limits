# Multi-Account Swap — Design Specification

**Date:** 2026-05-05
**Status:** Design pending user review
**Builds on:** `docs/superpowers/specs/2026-04-24-claude-limits-design.md` (v1 single-account spec)
**Reference implementation:** [`claude-swap`](https://github.com/realiti4/claude-swap) (Python CLI; the most recently updated of the surveyed multi-account tools)

---

## 1. Overview

Add multi-account management to claude-limits. Users can register N Claude
accounts in the tray app, see each account's 5-hour and 7-day usage with reset
timers in a dedicated sub-screen, and one-click "swap" to make any registered
account the active one in Claude Code (CLI + VS Code extension).

The app continues to be a Windows + macOS tray utility. No Linux variant; no
WSL variant in v1.

### Goals

| Goal | Decision |
|---|---|
| Manage multiple accounts in claude-limits' own store | Yes — per-account refresh tokens + identity in `accounts.json` |
| Display per-account usage (5h %, 7d %, reset times) | Yes — fanned-out parallel polling, all accounts on the same configured interval |
| One-click swap that propagates to Claude Code CLI + VS Code extension | Yes — write target's credential blob to CC's primary store + splice `oauthAccount` into `~/.claude.json` |
| Add accounts via two routes | Both: (a) import the live Claude Code login, (b) sign in via OAuth paste-back inside the app |
| UI placement | Active account stays in the compact popover; new "Accounts" sub-screen lists all of them (slides in like Settings) |
| Existing single-account users | Silent migration on first launch — existing OAuth + Claude Code creds become Slot 1 (and Slot 2 if both present and distinct) |

### Non-goals (v1)

- Linux / WSL support (out of scope today; Windows + macOS only)
- Killing or restarting Claude Code processes on swap (we detect them and inform; we don't act)
- Aggregating usage across accounts in the tray icon (driven by active account only)
- Per-account notification thresholds in v1 (notifications fire only for the active account; v1.1 candidate)
- Cross-machine account sync / export-import like cswap's `--export`/`--import` (revisit if requested)
- Org-account aggregation views in the expanded report (v1 only surfaces the per-account org tag and "shares quota with…" hint)

---

## 2. Architecture

### 2.1 — High-level shape

```
┌──────────────────────────────────────────────────────────────────┐
│  Tray icon ─ click ─►  Popover (active account)                  │
│                            ↓                                      │
│                      Accounts sub-screen (all managed accounts)   │
│                            ↓                                      │
│                       IPC (Tauri commands + per-slot events)      │
│  ┌──────────────────── Rust backend ─────────────────────────┐   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐     │   │
│  │  │ Accounts │ │   Auth   │ │  Usage   │ │  Store   │     │   │
│  │  │  store   │ │ orchestr.│ │  poller  │ │ (SQLite) │     │   │
│  │  │(JSON+lock│ │ (per-slot│ │(fan-out) │ │  per-acc │     │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘     │   │
│  │       │            │            │                         │   │
│  │       ↓            ↓            ↓                         │   │
│  │   accounts.json   CC primary    Notifier (per-account     │   │
│  │   (our store)     store         threshold state)          │   │
│  │                   - macOS: Keychain                       │   │
│  │                   - Windows: ~/.claude/.credentials.json  │   │
│  │                   + ~/.claude.json (oauthAccount slice)   │   │
│  └────────────────────────────────────────────────────────────┘   │
│                            ↓                                      │
│           api.anthropic.com /api/oauth/usage  (per-slot)          │
│           platform.claude.com /v1/oauth/token (refresh inactive)  │
└──────────────────────────────────────────────────────────────────┘
```

### 2.2 — Tactical decisions baked into the design

**T1 — Active-account is derived, not stored.** The "active" account is whichever
managed account's `accountUuid` matches what's currently in
`~/.claude/.credentials.json` (or macOS Keychain) plus the `oauthAccount` slice
of `~/.claude.json`. We never write an `activeAccountNumber` field. Reason: the
user can also swap externally (`cswap`, `claude /login`, manual edit). Deriving
means we never drift. Cost: one identity comparison per poll tick (cheap).

**T2 — Token-refresh ownership is split between the app and Claude Code.** For
the **active** account: read CC's live credentials each poll, never refresh
from our side. For **inactive** accounts: refresh from our store as needed and
persist the new refresh token back immediately. This is mandatory, not optional —
Anthropic's OAuth refresh tokens are single-use rotating tokens
([Issue #24317](https://github.com/anthropics/claude-code/issues/24317),
[#27933](https://github.com/anthropics/claude-code/issues/27933)). If we
refreshed the active account while CC also refreshed it, one process gets
`invalid_grant` and the user is forced to re-authenticate.

**T3 — Per-account storage is a single JSON file.** `accounts.json` lives at
Tauri's app-data-dir, sibling of today's `credentials.json`. The legacy
`credentials.json` is consumed by the migration on first launch and then
deleted. Single file is fine: payloads are small (a few hundred bytes per
account), we always read the whole thing, and atomic temp-file+rename plus
file lock protects multi-process and partial-write hazards.

### 2.3 — Cross-platform code-path sharing

| Surface | Single code path | Per-platform code path |
|---|---|---|
| `accounts.json` (our store) | ✅ | — |
| `~/.claude.json` (oauthAccount slice — net-new IO surface in this spec) | ✅ | — |
| Live CC credentials read/write | — | macOS: Keychain via `security`. Windows: file at `%USERPROFILE%\.claude\.credentials.json` |
| Process detection (`sysinfo`) | ✅ | — |

The per-platform split for live CC credentials is forced by Claude Code itself.
On macOS it reads Keychain first and only falls back to the file when Keychain
is unavailable, so writing the file alone wouldn't take effect. Two thin
modules (~20–30 lines each) behind one `claude_code_creds::write(blob)` trait.

**Net-new IO surface:** the existing `claude_code_creds::load()`
(`auth/claude_code_creds/macos.rs:11-24` and `windows.rs:7-21`) only parses
`claudeAiOauth.{accessToken, refreshToken, expiresAt}` into `StoredToken` —
the rest of the blob (`subscriptionType`, `rateLimitTier`, `scopes`, etc.)
is dropped today, and `~/.claude.json` is never opened by any module. The
multi-account spec adds:
- `claude_code_creds::load_full_blob()` returning the entire `claudeAiOauth`
  JSON value (alongside the existing `load()` for the cheap-token path)
- A new `auth::paths` module that opens `~/.claude.json` (per §2.4
  resolution rules) and returns the `oauthAccount` slice as a JSON value
- Symmetric `write` / `write_oauth_account_slice` helpers used by
  `swap_to(slot)`

The first-launch migration (Scenario A) and add-from-CC (Scenario C) depend
on `~/.claude.json` being readable. If the file is absent (fresh CC install
that hasn't completed onboarding yet) the migration treats CC as
"no managed source" and skips the import — correct fail-soft behavior, no
crash.

### 2.4 — Path resolution helpers

`auth::paths::claude_global_config()` resolves `~/.claude.json`'s actual
location, mirroring `cswap.paths.get_global_config_path` (which mirrors
Claude Code's own resolution):

1. If `$CLAUDE_CONFIG_DIR` is set → `$CLAUDE_CONFIG_DIR/.claude.json`
2. Else if `<config_home>/.config.json` exists → that (legacy fallback —
   filename really is `.config.json`, not `.claude.json`; this is
   claude-code's older path that some installs still have)
3. Else `<homedir>/.claude.json`

Where `<config_home> = $CLAUDE_CONFIG_DIR ?? <homedir>/.claude`. Used both for
reading active oauthAccount during reconcile and writing during swap.

### 2.5 — macOS keychain swap target

macOS already has a multi-service discovery path
(`claude_code_creds/macos.rs:38-65`) that enumerates every service starting
with `"Claude Code-credentials"` and picks the one with the latest
`expiresAt`. For **swap writes**, we do **not** mirror this — we always write
to the canonical service `"Claude Code-credentials"`, matching cswap's
behavior:

```
security add-generic-password -U \
  -s "Claude Code-credentials" \
  -a "$USER" \
  -w -                                # blob via stdin, not arg
```

`-U` updates if exists, creates if not. Stdin (`-w -`) avoids leaking the
credential blob into the process command-line.

To prevent the existing reader from picking a stale non-canonical service
after a swap, the reader changes precedence: **canonical first; enumerate
only if canonical is absent**. This keeps backward compat with installs that
have multiple `Claude Code-credentials*` services from older CC versions
while making swap deterministic. We don't delete the extra services — they
might belong to other tools.

---

## 3. Data Model

### 3.1 — `accounts.json` schema

```rust
// File at <app-data-dir>/accounts.json
pub struct AccountsStore {
    pub schema_version: u32,                    // 1; bumped only on shape change
    pub accounts: BTreeMap<u32, ManagedAccount>, // keyed by stable slot id
}

pub struct ManagedAccount {
    pub slot: u32,                              // stable across renames; assigned at add time
    // Identity fields (extracted at add time for cheap rendering + matching)
    pub email: String,
    pub account_uuid: String,                   // primary identity key
    pub organization_uuid: Option<String>,      // None for personal accounts
    pub organization_name: Option<String>,
    pub subscription_type: Option<String>,      // "pro" | "max" | "team" | etc — display only
    pub source: AddSource,                      // OAuth | ImportedFromClaudeCode
    // Opaque blobs we splice back on swap — preserve unknown fields verbatim
    pub claude_code_oauth_blob: serde_json::Value,  // full claudeAiOauth contents
    pub oauth_account_blob: serde_json::Value,       // full oauthAccount slice
    // Lifecycle
    pub token_expires_at: DateTime<Utc>,        // mirrors blob.expiresAt for cheap freshness checks
    pub added_at: DateTime<Utc>,
    pub last_seen_active: Option<DateTime<Utc>>, // updated each tick when this slot is the live CC one
}

pub enum AddSource { OAuth, ImportedFromClaudeCode }
```

Both blobs are stored as opaque `serde_json::Value` because Anthropic adds
fields periodically (`subscriptionType`, `rateLimitTier`, `organizationRole`,
`workspaceRole`, `displayName`, `hasExtraUsageEnabled` are observed today but
not exhaustive). The swap operation writes back the entire stored blob — never
a subset — so unknown fields survive round-trip.

### 3.2 — `AppState` additions

```rust
pub struct AppState {
    // ...existing fields preserved...
    pub accounts: RwLock<AccountsStore>,
    pub cached_usage_by_slot: RwLock<HashMap<u32, CachedUsage>>,
    pub active_slot: RwLock<Option<u32>>,        // derived; recomputed each poll
    // The legacy `cached_usage: RwLock<Option<CachedUsage>>` becomes a
    // computed getter that returns cached_usage_by_slot[active_slot]
    // so existing tray + popover code keeps working unchanged.
}
```

### 3.3 — SQLite schema (no shape change)

`notification_state` already has the right shape — `account_id TEXT NOT NULL`
column with composite PK `(account_id, bucket, threshold)` (see
`store/schema.sql:60-66`). What changes is the **value domain**: today the
orchestrator writes the placeholder strings `"unknown-OAuth"` /
`"unknown-ClaudeCode"` (`auth/orchestrator.rs:211`); tomorrow it writes a real
`accountUuid` per slot.

A versioned migration (`store/migrations/0003_truncate_notification_placeholders.sql`)
**truncates** the table and bumps `schema_version` 2 → 3. Threshold-crossing
memory is ephemeral, and the placeholder rows would never match any real uuid
again. Cost: at most one re-fired notification per already-crossed threshold
on the next poll. Cheaper than carrying dead state.

`Db::migrate()` in `store/mod.rs:99-116` currently handles `< 2` only — the
`< 3` block is appended.

---

## 4. Module Layout

### 4.1 — Rust backend additions

```
src-tauri/src/
├── auth/
│   ├── mod.rs                       # re-exports AccountManager + helpers
│   ├── accounts/
│   │   ├── mod.rs                   # AccountManager: add, remove, swap, refresh_inactive
│   │   ├── store.rs                 # AccountsStore I/O: read, atomic write, file lock
│   │   ├── identity.rs              # identity extraction from CC blobs + /oauth/userinfo
│   │   └── migration.rs             # v0 (legacy single-credentials.json) → v1 import
│   ├── orchestrator.rs              # rewritten: read_live_claude_code, token_for_slot
│   ├── claude_code_creds/
│   │   ├── mod.rs                   # gains write() alongside load()
│   │   ├── macos.rs                 # adds: write via security CLI (no command-line leakage)
│   │   └── windows.rs               # adds: atomic temp+rename write to .credentials.json
│   ├── paths.rs                     # NEW: claude_global_config() resolution helper
│   ├── exchange.rs                  # unchanged (token endpoint client)
│   ├── oauth_paste_back.rs          # unchanged (PKCE + paste parsing)
│   └── token_store.rs               # DELETE after migration ships
├── poll_loop.rs                     # rewritten: per-tick fan-out, per-slot cache
├── process_detection/
│   └── mod.rs                       # NEW: sysinfo-based detection of running CC + VS Code
├── notifier/
│   └── rules.rs                     # account-id-scoped state; per-slot threshold memory
├── commands.rs                      # additions: list_accounts, add_account_*, remove_account, swap_to_account, detect_running_claude_code
└── app_state.rs                     # additions per §3.2
```

### 4.2 — React frontend additions

```
src/
├── popover/
│   └── CompactPopover.tsx           # header gains active-account label (clickable → accounts)
├── accounts/                        # NEW directory
│   ├── AccountsPanel.tsx            # sub-screen shell + scrollable list
│   ├── AccountRow.tsx               # one row: bars, reset timers, kebab menu
│   ├── AddAccountChooser.tsx        # the path-A / path-B picker
│   ├── SwapConfirmModal.tsx         # inline confirm with running-CC details
│   └── UnmanagedActiveBanner.tsx    # replaces v1's AuthSourceConflict banner
├── settings/
│   ├── AuthPanel.tsx                # paste-back UI — reused for "add another account"
│   ├── AuthConflictChooser.tsx      # DELETE
│   └── SettingsPanel.tsx            # account section becomes "Manage accounts →" link
├── lib/
│   ├── ipc.ts                       # additions for per-slot commands + events
│   └── store.ts                     # zustand: accounts[], activeSlot, perSlotCache
└── App.tsx                          # adds 'accounts' route alongside home/settings
```

---

## 5. Interfaces

### 5.1 — Tauri commands (IPC surface)

Existing single-account commands change semantics rather than name where possible, to keep the IPC surface small. Removed: `use_claude_code_creds`, `pick_auth_source`, `sign_out` (replaced by per-account variants).

| Command | Status | Behavior |
|---|---|---|
| `start_oauth_flow()` | unchanged | Generates PKCE pair, returns authorize URL |
| `submit_oauth_code(pasted)` | **breaking signature change** | Today: `Result<(), String>` (writes the token to legacy `credentials.json`). New: `Result<u32, String>` (hands the token to `AccountManager::add_from_oauth`, returns the assigned slot). Frontend `ipc.ts` wrapper + every caller updates accordingly. |
| `get_current_usage()` | unchanged | Returns active slot's `CachedUsage` (computed via `cached_usage_by_slot[active_slot]`) |
| `force_refresh()` | unchanged | Wakes the poll loop |
| `get_settings` / `update_settings` | unchanged | — |
| `has_claude_code_creds()` | unchanged | — |

New commands:

```rust
#[command] async fn list_accounts() -> Vec<AccountListEntry>;
#[command] async fn add_account_from_claude_code() -> Result<u32, String>;
#[command] async fn remove_account(slot: u32) -> Result<(), String>;
#[command] async fn swap_to_account(slot: u32) -> Result<SwapReport, String>;
#[command] async fn detect_running_claude_code() -> RunningClaudeCode;
#[command] async fn refresh_account(slot: u32) -> Result<(), String>;  // kebab "Refresh now"
#[command] async fn reauthenticate_account(slot: u32) -> Result<String, String>; // returns authorize URL scoped to slot

pub struct AccountListEntry {
    pub slot: u32,
    pub email: String,
    pub org_name: Option<String>,
    pub org_uuid: Option<String>,
    pub subscription_type: Option<String>,
    pub source: AddSource,
    pub is_active: bool,
    pub cached_usage: Option<CachedUsage>,
    pub last_error: Option<String>,
}

pub struct SwapReport {
    pub new_active_slot: u32,
    pub running: RunningClaudeCode,
}

pub struct RunningClaudeCode {
    pub cli_processes: u32,
    pub vscode_with_extension: Vec<String>,  // workspace folder paths
}
```

### 5.2 — Events (backend → frontend)

| Event | Payload | When |
|---|---|---|
| `usage_updated` | `{ slot: u32, cached: CachedUsage }` | After each per-slot fetch (existing event becomes scoped) |
| `accounts_changed` | `{ accounts: AccountListEntry[] }` | After add / remove / swap / external swap detection |
| `auth_required_for_slot` | `{ slot: u32, email: String }` | 401 on a specific slot's poll |
| `unmanaged_active_account` | `{ email: String, account_uuid: String }` | Live CC account isn't in our store; sticky-dismissed per uuid |
| `swap_completed` | `SwapReport` | Triggers the "Switched to …" toast |
| `migrated_accounts` | `{ slots: Vec<u32> }` | One-shot on first launch after v1 → multi upgrade |
| `requires_setup` | `{}` | Zero managed accounts AND no live CC creds detected. Frontend routes to `AuthPanel` (the empty-state add-account screen). Replaces the v1 `auth_required` route trigger in `App.tsx:55`. |

Removed: `auth_required` (split into `auth_required_for_slot` for per-row UX, and `requires_setup` for the empty-state route), `auth_source_conflict` (replaced by `unmanaged_active_account`).

### 5.3 — Type generation

Continues to use `specta` to emit TypeScript bindings to
`src/lib/generated/bindings.ts`. All new structs derive `specta::Type`.

---

## 6. Data Flow

### Scenario A — First launch after upgrade (silent migration)

```
App starts → AccountManager::load(dir):
  ├─ accounts.json exists?  → use it as-is, done
  └─ accounts.json absent (existing single-account user):
       1. Read legacy credentials.json (existing OAuth token)
       2. Read live Claude Code creds (existing claude_code_creds::load())
       3. For each present source, fetch identity via /api/oauth/userinfo
          (skip when source already carries accountUuid in the captured blob)
       4. Insert into accounts.json as Slot 1 (and Slot 2 if both present + distinct)
       5. Delete the legacy credentials.json
       6. Emit migrated_accounts(slots) → UI shows a one-line toast:
          "Imported your existing account(s) into multi-account view."
```

Failure on identity fetch: keep `credentials.json` in place, show a banner
"Couldn't import — retry on next launch." No partial state on disk.

### Scenario B — Polling loop (fan-out)

Per tick:

1. **Reconcile `active_slot`** by reading live CC creds + oauthAccount slice
   and matching `account_uuid` to a managed slot.

   **Cost guard (macOS):** the keychain read in
   `claude_code_creds::load()` calls `security dump-keychain` (~tens of ms,
   blocks if keychain is locked). Cache the resolved active slot keyed on
   `(claudeAiOauth.expiresAt, ~/.claude.json mtime)`. Re-resolve only when
   either changes. Default 5-min poll interval makes this a non-issue, but
   shorter intervals (down to 1 min) and an SSH-locked-keychain edge case
   make the cache worthwhile.

2. If live CC has an account that matches no managed slot → emit
   `unmanaged_active_account`. If our store is empty AND CC has no live
   creds → emit `requires_setup`.
3. Fan out: `futures::future::join_all` over all slots' usage fetches in
   parallel. The existing `UsageClient` is already `Arc<>`-shared
   (`lib.rs:42-44`), so fan-out reuses the same client without contention.
   Each task uses `auth.token_for_slot(slot)`.
4. For each result: update `cached_usage_by_slot[slot]`, emit per-slot
   `usage_updated`.
5. Drive tray badge + notifier from the active slot only. Inactive slots'
   usage is visible in the Accounts list but doesn't affect tray or fire
   notifications in v1.

**Per-slot backoff:** the v1 single-shared `backoff: Duration` in
`poll_loop.rs:51-53` becomes per-slot state living inside
`cached_usage_by_slot[slot]` (or a sibling `backoff_by_slot: HashMap<u32,
Duration>`). A 429 on slot N applies its own backoff timer; slots M and P
keep polling on the configured interval. The shared `next_backoff()` math
stays the same; only the storage key changes.

### Scenario C — Add account from Claude Code

```
Click "Use Claude Code's current login":
  1. Read live claudeAiOauth + oauthAccount blobs
  2. Extract account_uuid; check accounts.find_by_account_uuid(uuid)
     ├─ Found: refresh in-place (overwrite stored blobs, return existing slot)
     └─ Not found:
        3. Fetch identity via /oauth/userinfo (defensive)
        4. Allocate next slot number
        5. Persist into accounts.json under file lock
  6. Emit accounts_changed
  7. Next poll fetches usage for new slot
```

### Scenario D — Add account via OAuth paste-back

Reuses `start_oauth_flow` + paste-back paste field. On `submit_oauth_code`:

- Synthesize `claudeAiOauth` blob from the token-endpoint response
  (`accessToken`, `refreshToken`, `expiresAt`, `scopes`).
- Synthesize `oauthAccount` blob from `/api/oauth/userinfo`
  (`accountUuid`, `emailAddress`, `organizationUuid`, `organizationName`,
  whatever else userinfo returns).
- Pass to `AccountManager::add_from_oauth` → same dedup-by-uuid logic.

OAuth-added accounts initially lack `subscriptionType` / `rateLimitTier` (those
come from `/v1/oauth/token` exchanges into Claude Code, not from userinfo).
First time the user swaps to such an account, Claude Code's first request
populates them into the live `.credentials.json`; the next poll's reconcile
updates our stored blob.

### Scenario E — Swap to inactive account

```
User clicks an inactive row:
  1. detect_running_claude_code() → RunningClaudeCode
  2. If running > 0: show inline confirm (running detail surfaced)
  3. AccountManager::swap_to(target_slot) under file lock:
     a. Snapshot live CC creds + ~/.claude.json oauthAccount (rollback memory)
     b. Write target.claude_code_oauth_blob to CC's primary store
        (Keychain on macOS, file on Windows) — atomic per platform
     c. Read ~/.claude.json (or legacy fallback), splice in
        target.oauth_account_blob, atomic temp+rename
     d. On any failure: restore from snapshot in reverse, return SwapError
  4. force_refresh.notify_one() → poll loop wakes
  5. Toast: "Switched to <email>. Running Claude Code sessions will switch on
     their next token refresh (~5 min)." [+ running-process list if any]
```

### Scenario F — External swap (cswap or `claude /login`)

The poll loop's reconcile step detects the new live account on the next tick.
If it matches a managed slot → silently update `active_slot`, emit
`accounts_changed`. If it doesn't → emit `unmanaged_active_account`.

This is the payoff of T1 (derived active): external mutations Just Work
without explicit synchronization.

### Scenario G — Per-slot 401

Slot N returns 401 → `cached_usage_by_slot[N].last_error = "auth_required"` +
emit `auth_required_for_slot`. UI row shows inline "token expired —
re-authenticate". Other slots continue polling. If the failed slot is the
**active** one, additionally surface the existing top-level `authRequired`
banner so the user notices at-a-glance.

### Scenario H — Remove account

```
Kebab → "Remove account…" → inline confirm (red destructive):
  "Remove <email> from claude-limits? Claude Code's login is unaffected."

On confirm:
  1. AccountManager::remove(slot) — deletes from accounts.json under lock
  2. cached_usage_by_slot.remove(slot)
  3. Emit accounts_changed
```

We never touch Claude Code's own creds during remove. The active CC account is
unaffected; if the removed slot was active, our app shows the
`unmanaged_active_account` banner offering to re-add it.

---

## 7. UI

### 7.1 — Compact popover (active account)

Today's layout preserved. Two small additions to the chrome strip:

```
┌──────────────────────────────────────────────┐
│ CLAUDE • alice@x.com [Acme]   ⟳ ⚙ ⤢ ✕      │
├──────────────────────────────────────────────┤
│ [auth/stale/conflict banners]                │
│                                              │
│ 5h ▰▰▰▰▰▱▱▱▱▱  62%  resets in 1h 23m        │
│ 7d ▰▰▰▱▱▱▱▱▱▱  31%  resets in 4d 12h        │
│ extra-usage … (only if enabled)              │
│                                              │
│ Updated 2m ago        v0.4.1 · Check updates │
└──────────────────────────────────────────────┘
```

- Active-account label in the header. Click → opens Accounts sub-screen.
  Truncates at ~22 chars; tooltip shows full email + org.
- No new icon button — the clickable label is the affordance, matching macOS
  Control Center / Raycast conventions.

### 7.2 — Accounts sub-screen

Slides in like Settings. Scrollable list of rows + footer "Add account" button:

```
┌──────────────────────────────────────────────┐
│ ← Back                ACCOUNTS           ✕   │
├──────────────────────────────────────────────┤
│ ●  alice@x.com           [Acme · Max]   ⋯   │  ← active row
│    5h ▰▰▰▰▰▰▱▱▱▱  62%  resets in 1h 23m     │
│    7d ▰▰▰▱▱▱▱▱▱▱  31%  resets in 4d 12h     │
│                                              │
│    bob@x.com             [Acme · Max]   ⋯   │  ← inactive; click to swap
│    5h ▰▰▱▱▱▱▱▱▱▱  19%  resets in 1h 23m     │
│    7d ▰▰▰▰▰▱▱▱▱▱  48%  resets in 4d 12h     │
│    └ shares quota with alice@x.com (Acme)    │
│                                              │
│    me@personal.dev       [personal · Pro] ⋯ │
│    5h ▰▱▱▱▱▱▱▱▱▱   8%  resets in 0h 41m     │
│    7d ▰▱▱▱▱▱▱▱▱▱   3%  resets in 6d 02h     │
│                                              │
│    headless@bot.local    [setup-token]   ⋯  │
│    └ usage unavailable                       │
│                                              │
│    + Add account                             │
└──────────────────────────────────────────────┘
```

- Active dot (left, **static** accent fill) on the active row only. Plain
  filled circle, no pulse — keeps the list quiet per CLAUDE.md's "no
  gratuitous animations" rule. (The compact-popover header retains the
  existing `StatusDot` pulse for the live-poll indicator; that's a different
  affordance.)
- `[org · subscription]` chip uses muted-text style; `personal` for
  no-organization accounts.
- Kebab `⋯` menu per row: `Refresh now`, `Re-authenticate`, `Remove account…`
  (destructive items behind the kebab to avoid accidental clicks).
- Bars use the existing `UsageBar` component; no Opus/Sonnet sub-bars in the
  list (those stay on the active-account compact view).
- Click row body of an inactive account → triggers swap (with confirm if a CC
  process is detected).
- Rows render in slot-number order (insertion order, stable across renames).
  The `└ shares quota with …` hint appears on second-and-later rows in the
  same `organization_uuid` group, naming the first row in that group.
- `└ usage unavailable` / `└ token expired — re-authenticate` replaces bars
  when `cached_usage[slot].last_error` is set.
- Window resizes to `min(content_height, 560px)` so 2-3 accounts fit without
  scroll, more grows scrollable.

### 7.3 — Add-account chooser

```
ADD ACCOUNT
─────────────────────
[ Use Claude Code's current login (alice@x.com) ]
   ↑ shown when CC has a live login that's not already managed

[ Sign in with a different Claude account ]
   ↑ always shown; opens existing OAuth paste-back flow
```

**Existing AuthPanel reuse:** today `AuthPanel.tsx:68` calls
`ipc.useClaudeCodeCreds()`, which is being removed. The "Use Claude Code
credentials" tile becomes "Use Claude Code's current login" and calls the
new `add_account_from_claude_code()` command (matching path-A here). The
"Sign in with Claude" tile is unchanged in label and continues to invoke
`start_oauth_flow()` → paste-back → `submit_oauth_code()`, which now returns
a slot id instead of `()` (per §5.1). The same panel serves three roles:
empty-state setup (when `requires_setup` event fires), per-account
re-authentication (Scenario G), and the add-account chooser opened from the
Accounts sub-screen footer.

### 7.4 — Swap confirm (when CC processes detected)

```
Switch to bob@x.com?
Claude Code is running:
  • CLI · /home/alice/projects/foo  (1 session)
  • VS Code · /home/alice/projects/bar
Sessions will pick up the new account on their next token
refresh (~5 min). Restart for an immediate switch.
                              [ Cancel ] [ Switch ]
```

Inline within the popover, not an OS modal.

### 7.5 — Unmanaged-active banner

```
⚠ Claude Code is logged in as charlie@new.dev — not managed.
  [ Add to accounts ]   [ Dismiss ]
```

Sticky-dismissed per `accountUuid`; cleared on add or remove.

### 7.6 — Tray icon — unchanged

Driven by active account only. Tooltip gains a multi-line variant when N>1:
`Active: alice@x.com — 5h 62%, 7d 31%\n+2 other accounts`.

---

## 8. Error Handling

Inherits the v1 spec's error matrix. Multi-account additions:

| Failure | Response |
|---|---|
| `accounts.json` corrupt on startup | Back up to `accounts.json.corrupt-<timestamp>`, fresh empty store, run Scenario A migration if legacy sources present, banner: "Account store was reset — re-add accounts from the list." |
| `accounts.json` write fails mid-add | File lock holds; temp file unlinked; in-memory state rolled back; toast: "Couldn't save account — disk full or permission?" |
| `accounts.json` lock contention | Tauri single-instance prevents two app processes; lock is insurance. Hold timeout >5s → log warning + abort with "Another operation in progress — retry." |
| Swap step b fails (Keychain/file write) | Roll back: nothing written to `~/.claude.json` yet; return `SwapError::CredentialWriteFailed`. |
| Swap step c fails (`~/.claude.json` write) | Roll back step b: restore previous creds to Keychain/file. Return `SwapError::ConfigWriteFailed`. If restore also fails: log loudly, surface "Critical: Claude Code credentials may be inconsistent — please run `claude /login` to re-authenticate." |
| Inactive refresh succeeds, persist to `accounts.json` fails | Log + emit `auth_required_for_slot`. Do not carry the in-memory token — next refresh from disk will fail with `invalid_grant`, which we already handle. |
| `/oauth/userinfo` returns 404 for a CC-origin token | Use `accountUuid` from the captured `oauthAccount` blob (always present for CC-imported accounts). userinfo only needed for OAuth-added accounts. |
| `sysinfo` process detection fails | Treat as "no processes detected" — no-confirm swap path. Log; don't block on a non-critical signal. |
| Swap target's stored blob is missing/malformed (manual edit of `accounts.json`) | Refuse with `SwapError::IncompleteAccount`. Toast: "This account's stored credentials look corrupt — try re-adding it." |

### Explicit non-behaviors

- **No "smart" merge of `oauthAccount` slice during swap.** Replace the whole
  slice with the stored blob, atomically.
- **No automatic re-add of removed accounts.** Explicit re-add only.
- **No background refresh storm on startup.** First poll fan-out is sufficient;
  no preemptive refresh-on-startup outside that.
- **No credential touching from "Remove all accounts."** Destructive of our
  store only; doesn't sign Claude Code out.

---

## 9. Testing Strategy

### 9.1 — Rust unit tests (additions on top of v1)

| Module | What to test |
|---|---|
| `auth::accounts::store` | Multi-slot round-trip; corrupt-file backup-and-reset; lock prevents concurrent writes; schema_version migration; blob preservation (unknown fields survive). |
| `auth::accounts::manager` | `add_from_claude_code` dedup-by-accountUuid; `add_from_oauth` synthesizes blobs from `/oauth/userinfo` fixtures; `remove(slot)` idempotent; `swap_to` two-step rollback on step-c failure. |
| `auth::orchestrator` | `token_for_slot(active)` reads live CC, never refreshes; `token_for_slot(inactive)` refreshes if expiring within 2min, persists back; path resolution precedence. |
| `auth::claude_code_creds::macos` (gated) | `write` via `security add-generic-password -U` with stdin (no command-line leakage); non-zero exit treated as failure. |
| `auth::claude_code_creds::windows` (gated) | `write` via temp+rename; ACL preserved; partial-write recovery. |
| `auth::accounts::migration` | Legacy `credentials.json` → Slot 1; legacy + CC same uuid → one slot; legacy + CC different uuids → two slots; identity-fetch failure leaves legacy file intact. |
| `poll_loop` | Fan-out parallelism (3 slots in parallel via `join_all`); one slot's 401 doesn't stop others; per-slot burn-rate buffers don't bleed; `unmanaged_active_account` emitted exactly once per uuid. |
| `notifier::rules` | `notification_state` keyed by `account_id`; threshold crossing on slot A doesn't suppress slot B; `debug_force_threshold(slot, …)` per-slot. |
| `process_detection` | Detects `claude` / `claude.exe`; detects VS Code with claude-code extension; returns empty + Ok on permission-denied (graceful). |

### 9.2 — Rust integration tests

- Add via paste-back → swap to → external `cswap` swap → reconcile detects external swap → swap-back works
- 3 managed accounts polled in parallel against mocked Anthropic returning 200/200/429 — only the 429 slot enters backoff; others' caches updated
- Two managed accounts share `organization_uuid` → both fetches return identical numbers → metadata exposes shared-quota relationship
- Refresh-rotation invariant: spawn two tokio tasks both calling `refresh_inactive(slot)` simultaneously → only one network call (file lock serializes), the other reads freshly-persisted token
- **Active-slot refresh prohibition:** integration test specifically asserts
  that polling the active slot never issues a refresh request to
  `/v1/oauth/token`, even when the live CC token is within the 2-min refresh
  window. Guards T2 against regression.

### 9.3 — Tests to delete or rewrite (existing v1)

The following tests in `auth/orchestrator.rs` exercise the removed conflict
path (`get_access_token` lines 117-157) and must be deleted or rewritten
to target the new derive-active mechanism:
- Any test that constructs both `Some(oauth_tok)` and `Some(cli_tok)` and
  asserts `AuthError::Conflict` — replaced by the `unmanaged_active_account`
  event tests in `poll_loop` (§9.1).
- Any test that calls `pick_auth_source` / `set_preferred_source` —
  replaced by `swap_to(slot)` tests.
- `submit_oauth_code` tests asserting `Result<(), _>` shape — updated to
  assert `Result<u32, _>` and verify the slot was created.

### 9.4 — Frontend component tests (Vitest + RTL)

- `AccountsList` row rendering; active-dot positioning; org-share hint logic; per-row error state vs bars
- `AddAccountChooser` shows path A only when `has_claude_code_creds() && uuid not already managed`
- `SwapConfirmModal` renders correctly with empty / non-empty `RunningClaudeCode`
- `UnmanagedActiveBanner` sticky-dismiss persists across re-renders for same uuid
- IPC contract: `list_accounts()` shape matches generated TS types; per-slot events route to per-row state correctly

### 9.5 — Manual release checklist (additions to `docs/release-checklist.md`)

- [ ] Fresh install → `claude /login` as A → claude-limits launches → A appears as active in Accounts list
- [ ] Add B via "Use Claude Code's current login" path (after `claude /login` as B)
- [ ] Add C via OAuth paste-back path (without changing CC's login)
- [ ] All three show usage in Accounts sub-screen with correct numbers
- [ ] Click row B → swap → verify CC primary store + `~/.claude.json` reflect B; restart `claude` and confirm B is active
- [ ] Repeat with VS Code extension running — toast shows running-process hint, restart extension and confirm B
- [ ] Run `cswap --switch-to A` externally → claude-limits' active dot moves to A within one poll interval; no false `unmanaged_active_account` banner
- [ ] `claude /login` as new D externally → `unmanaged_active_account` banner appears; click "Add to accounts" → D appears, banner clears
- [ ] Remove C → CC's login (A or B) untouched
- [ ] Single-account upgrade: install previous version with one OAuth account → upgrade to multi-account → existing account appears as Slot 1, no manual action
- [ ] Org-shared accounts: add two in same org → bars show identical numbers, "shares quota with…" hint appears

### 9.6 — Explicit non-tests

- No tests against Anthropic's live OAuth in CI (mocked); manual checklist covers it once per release.
- No GUI automation of the swap flow.
- No load tests for >10 accounts (out of scope; design assumes typical N=2-5).

---

## 10. Migration & Compatibility

### 10.1 — From v1 (single-account) to multi

Silent migration on first launch (Scenario A). Existing OAuth + Claude Code
creds become Slot 1 (and Slot 2 if both present + distinct accountUuid).
Legacy `credentials.json` deleted after successful migration. One-shot toast
on the popover.

### 10.2 — Settings shape

`Settings.preferred_auth_source: Option<AuthSource>` becomes vestigial — not
read by the new orchestrator. Kept in the struct for one release to avoid
breaking the SQLite settings round-trip on downgrade; removed in v2.

### 10.3 — Schema versioning

`AccountsStore.schema_version` starts at 1. Future additive changes (new
optional fields on `ManagedAccount`) don't bump the version — serde tolerates
absent fields. Bump only on shape changes that need a deliberate migration.

---

## 11. Risks Accepted

- **Refresh-token rotation race with Claude Code.** Mitigated by T2 (we never
  refresh active slot). Documented invariant. If a bug ever causes us to
  refresh the active slot, the user's CC will hit `invalid_grant` on its next
  refresh. Tests enforce the invariant; integration test specifically verifies
  active-slot calls don't issue refresh requests.
- **Manual edits to `accounts.json` can corrupt blobs.** Schema validation on
  load catches structural breakage. Field-level corruption (e.g. user edits
  `account_uuid` to garbage) surfaces as `SwapError::IncompleteAccount` or
  reconcile failures, not silent misbehavior.
- **macOS Keychain unavailable (SSH session).** Documented limitation:
  multi-account swap requires running claude-limits from a desktop session on
  macOS. Same limitation as `cswap` and Anthropic's own OAuth flow.
- **Org-shared quota looks like a bug.** Two managed accounts in the same
  organization show identical numbers because Anthropic pools quota per
  `organizationUuid` ([Issue #41886](https://github.com/anthropics/claude-code/issues/41886)).
  Mitigated by the `└ shares quota with …` hint.
- **VS Code extension token cache.** Even after our swap propagates to disk,
  the extension may continue using the in-memory token from before. Restart
  picks up the new account immediately. Toast tells the user this.
- **Subscription metadata for OAuth-added accounts is initially missing.**
  `subscriptionType` and `rateLimitTier` only land in the credential blob
  after Claude Code first uses the account. Until then, the row chip shows
  `[personal]` or `[<orgname>]` without the trailing `· Pro` / `· Max`.
  Cosmetic only; doesn't affect functionality.
- **Process detection is best-effort.** `sysinfo` may miss exotic launchers
  or sandboxed processes. We treat detection failure as "nothing detected" —
  the swap proceeds with the no-confirm path, and the user gets the standard
  toast.

---

## 12. Out-of-Scope (Explicitly)

The following are deliberately deferred — in priority order if revisited:

1. **Per-account notification thresholds.** v1 fires notifications only for
   the active account. Per-slot threshold config + per-slot threshold
   notifications is a v1.1 candidate.
2. **Linux / WSL support.** No platform support work in this spec.
3. **Account export / import** (cswap's `--export`/`--import`). Useful for
   moving accounts between machines; revisit if requested.
4. **Killing or restarting Claude Code processes during swap.** We detect and
   inform; we don't act.
5. **Org-account aggregation views.** Per-org rollups in the expanded report
   (e.g. "Acme org total: 5h 81%"). Org grouping in v1 is limited to the
   per-row hint.
6. **Setup-token (`sk-ant-oat01-…`) registration as an add-account path.** Use
   case is headless servers; not the primary tray-app audience. Mention in
   docs as a v1.1 candidate.
7. **Multi-instance synchronization across machines.** Out of scope; user
   manages each machine's accounts list locally.

---

## 13. Sources Consulted

- [Authentication - Claude Code Docs](https://code.claude.com/docs/en/authentication)
- [Use Claude Code in VS Code - Claude Code Docs](https://code.claude.com/docs/en/vs-code)
- [claude-swap on PyPI](https://pypi.org/project/claude-swap/) and [GitHub](https://github.com/realiti4/claude-swap)
- [cc-account-switcher (bash)](https://github.com/ming86/cc-account-switcher)
- [Frequent re-authentication required (OAuth refresh-token race) · Issue #24317](https://github.com/anthropics/claude-code/issues/24317)
- [OAuth token refresh race condition with multiple concurrent CLI processes · Issue #27933](https://github.com/anthropics/claude-code/issues/27933)
- [/login does not switch accounts when already logged in · Issue #23906](https://github.com/anthropics/claude-code/issues/23906)
- [Rate limit quota is shared per organizationUuid · Issue #41886](https://github.com/anthropics/claude-code/issues/41886)
- [OTEL organization.id vs oauthAccount.organizationUuid · Issue #4339](https://github.com/anthropics/claude-code/issues/4339)
- [SSH sessions require re-login despite valid credentials.json · Issue #29816](https://github.com/anthropics/claude-code/issues/29816)
- [VS Code Remote SSH: extension requires re-login from SSH · Issue #44089](https://github.com/anthropics/claude-code/issues/44089)
- [v2.0.14 macOS Keychain issue · Issue #9403](https://github.com/anthropics/claude-code/issues/9403)
- [Keychain lookup fails for Claude Code credentials · openclaw Issue #1714](https://github.com/openclaw/openclaw/issues/1714)
- [Mac deletes the .credentials.json file that Linux uses · Issue #10039](https://github.com/anthropics/claude-code/issues/10039)
- [Claude Code Configuration Files: Complete Guide](https://inventivehq.com/knowledge-base/claude/where-configuration-files-are-stored)
