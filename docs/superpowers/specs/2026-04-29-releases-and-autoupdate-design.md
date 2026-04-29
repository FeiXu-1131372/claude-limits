# Releases & Auto-Update — Design

**Date:** 2026-04-29
**Status:** Approved (pre-implementation)
**Scope:** Add in-app auto-update to the Claude Limits menubar utility, on a $0 budget, without changing the existing unsigned-OS-build distribution model.

## 1. Goals & non-goals

### Goals

1. Released app detects new versions automatically and installs them with a single user click.
2. Cross-platform parity: macOS (universal) and Windows (x86_64) get the same update behavior.
3. No paid signing certificates required. Updater integrity is enforced cryptographically via Tauri's own signature scheme.
4. No backend infrastructure to maintain. Update artifacts and manifest live on GitHub Releases.
5. Zero impact on app cold-start: update checks never block the UI.
6. Calm, on-brand UX: no nag dialogs, no surprise restarts, no notification noise.

### Non-goals (deliberately out of scope for v1)

- Apple Developer ID signing / notarization (~$99/yr) — unsigned-build distribution stays as-is. Users still see the existing macOS first-launch dance documented in the README. This is the conscious cost-tradeoff.
- Windows code-signing / EV certificates.
- Mandatory updates / forced installs.
- Rollback to a prior version (users can manually download an older release from GitHub if needed; near-zero demand for a menubar utility).
- Release channels other than `stable` (no beta / nightly / canary).
- "Skip this version" UI.
- Telemetry on update success rates.
- Delta / patch updates (full bundles only — they're ~10MB).
- In-app changelog rendering. Release notes live on GitHub Releases; the popover banner just says "Update ready · vX.Y.Z".

## 2. Decisions log

| # | Decision | Reason |
|---|---|---|
| D1 | Skip paid OS signing for v1; ship updater-signed unsigned builds. | Free. README already documents first-launch on each OS. Upgrade path to paid signing is identical CI swap later. |
| D2 | Auto-download in background; user clicks to install. | Long-running menubar app — silent background download is cheap. Restart stays user-controlled. |
| D3 | Check on launch + every 6 hours while running. | Aggressive enough to deliver bugfixes within a workday; quiet enough not to hammer GitHub. |
| D4 | Popover-only "Update ready" banner. No tray dot, no system notification. | Brand is "calm, precise, premium" — minimal signal beats discoverability for v1. |
| D5 | Host `latest.json` and artifacts on GitHub Releases at the stable `latest/download/...` URL. | Free, CDN-backed, durable. Same bucket as today's release artifacts. |
| D6 | Tauri's static-JSON updater endpoint, not a custom server. | Zero infra. Tauri's built-in updater plugin handles the whole download/verify/install flow. |
| D7 | Single-channel (stable only). | Premature to fork channels with no users yet. Adding a channel later is additive (separate manifest URL). |
| D8 | Updater disabled in dev builds (`cfg!(debug_assertions)`). | Prevents accidental updates while developing. |

## 3. Architecture

### 3.1 System diagram

```
GitHub Releases (existing, free)              Tauri Updater plugin              Claude Limits app
─────────────────────────────────             ─────────────────────              ──────────────────
v0.2.0 release                                1. GET latest.json            ┌─ Rust side
├── claude-limits_0.2.0_universal.dmg         2. Compare versions           │  ├─ scheduler: launch + every 6h
├── claude-limits_0.2.0_universal.app.tar.gz  3. Download .tar.gz/.zip      │  ├─ download to temp
├── claude-limits_0.2.0_universal.app.tar.gz.sig 4. Verify ed25519 sig      │  ├─ verify signature
├── claude-limits_0.2.0_x64-setup.exe         5. Stage for install          │  └─ emit events to frontend
├── claude-limits_0.2.0_x64-setup.nsis.zip                                  │
├── claude-limits_0.2.0_x64-setup.nsis.zip.sig                              └─ React side
└── latest.json  ← points to artifacts above                                   ├─ updateStore (Zustand)
                                                                               └─ UpdateBanner
```

Three free, separate concerns working together:

1. **Tauri's updater plugin** (`tauri-plugin-updater@2`) — polls the JSON endpoint, downloads, verifies signatures, installs. Built into Tauri.
2. **`latest.json` manifest** — small JSON file, hosted on GitHub Releases at `https://github.com/FeiXu-1131372/claude-limits/releases/latest/download/latest.json` (this URL always 302s to the latest tag's asset, making it stable across releases).
3. **Updater keypair** — ed25519. Generated once locally with `tauri signer generate`. Public key embedded in `tauri.conf.json`. Private key stored as a GitHub Actions secret. CI signs every release artifact.

### 3.2 `latest.json` schema

Format expected by `tauri-plugin-updater@2`:

```json
{
  "version": "0.2.0",
  "notes": "See release notes at https://github.com/FeiXu-1131372/claude-limits/releases/tag/v0.2.0",
  "pub_date": "2026-04-29T12:00:00Z",
  "platforms": {
    "darwin-x86_64": {
      "signature": "<base64 ed25519 sig>",
      "url": "https://github.com/FeiXu-1131372/claude-limits/releases/download/v0.2.0/claude-limits_0.2.0_universal.app.tar.gz"
    },
    "darwin-aarch64": {
      "signature": "<base64 ed25519 sig>",
      "url": "https://github.com/FeiXu-1131372/claude-limits/releases/download/v0.2.0/claude-limits_0.2.0_universal.app.tar.gz"
    },
    "windows-x86_64": {
      "signature": "<base64 ed25519 sig>",
      "url": "https://github.com/FeiXu-1131372/claude-limits/releases/download/v0.2.0/claude-limits_0.2.0_x64-setup.nsis.zip"
    }
  }
}
```

Both macOS architectures point at the same universal `.app.tar.gz` (the existing release pipeline already produces a universal binary).

### 3.3 Trust model

| Threat | Mitigation |
|---|---|
| Malicious release pushed to GitHub | Updater rejects any artifact whose signature doesn't match the embedded ed25519 public key. Attacker would also need the GH Actions signing secret. |
| MITM on download | Updater enforces HTTPS; signature catches any tampering even if HTTPS is broken. |
| Manifest tampered | Manifest itself is HTTPS-fetched from GitHub and references signed artifacts; an attacker who replaces only the manifest can't produce valid signatures for substituted binaries. |
| Compromised signing secret | Documented incident response: rotate keypair, ship a new release with the new public key, accept that pre-rotation users have to manually upgrade once. |
| Downgrade attack | Updater only installs if `manifest.version > app.version` (semver compare). |

This is the same trust model used by Tauri's reference setup. Reasonable for a free OSS distribution with no paid OS signing.

## 4. Update state machine

```
        ┌──────────┐
        │   Idle   │ ◄────────────────────────┐
        └────┬─────┘                          │
             │ launch / 6h timer / manual     │
             ▼                                │
        ┌──────────┐                          │
        │ Checking │                          │
        └────┬─────┘                          │
             │                                │
        ┌────┴────────────────┐               │
        ▼                     ▼               │
  ┌──────────┐         ┌─────────────┐        │
  │ UpToDate │         │ Available   │        │
  └────┬─────┘         │ (vX.Y.Z)    │        │
       │               └──────┬──────┘        │
       └──────► back ◄────┐   │ auto          │
                          │   ▼               │
                          │ ┌──────────────┐  │
                          │ │ Downloading  │  │
                          │ │ (0–100%)     │  │
                          │ └──────┬───────┘  │
                          │        │          │
                          │   ┌────┴────┐     │
                          │   ▼         ▼     │
                          │ ┌──────┐ ┌──────┐ │
                          │ │Ready │ │Failed│─┘
                          │ └──┬───┘ └──────┘
                          │    │ user clicks "Install & restart"
                          │    ▼
                          │ ┌────────────┐
                          │ │ Installing │ → app exits, installer runs, app relaunches
                          │ └────────────┘
                          │
                          └── Failed (network/sig/IO) → log, retry next cycle
```

### 4.1 Events emitted by the Rust side

The Rust updater module emits events on the global Tauri app handle. The frontend subscribes once at mount.

| Event | Payload | When |
|---|---|---|
| `update://checking` | `{}` | Check cycle started |
| `update://up-to-date` | `{ checkedAt: ISO-8601 }` | Manifest fetched, no newer version |
| `update://available` | `{ version, notes, pubDate }` | Newer version found, before download |
| `update://progress` | `{ downloaded, total }` | During download (throttled to ~5/sec) |
| `update://ready` | `{ version }` | Download done, signature verified, staged |
| `update://failed` | `{ phase, message }` | Any failure; `phase` ∈ `check \| download \| verify \| install` |

`update://progress` is emitted but unused in v1 UI (banner only appears after `ready`). It's there so we can add a progress bar later without changing the Rust contract.

### 4.2 Failure handling

| Failure | Behavior |
|---|---|
| No network on check | Silent failure, log at `WARN`, retry next 6h cycle. Never blocks startup. |
| `latest.json` malformed / 404 | Silent failure, log at `WARN`, retry next cycle. |
| Signature verification fails | Log at `ERROR` (this would be a serious incident), discard the download, do **not** retry until the next scheduled check. **Never** fall back to "install anyway." |
| Download interrupted | No partial-download resumption — restart from scratch on next cycle. Artifacts are ~10MB. |
| Install fails (file-in-use / disk-full / permissions) | Frontend gets `update://failed` with phase `install`, popover banner swaps to a retry affordance. Staged bundle is retained until next successful install. |

### 4.3 Persistence

Only one piece of state persists across app restarts: `last_update_check_at` (ISO-8601 timestamp). Stored as a small JSON file in the existing app data dir (`directories::ProjectDirs`), **not** in the SQLite database. The 6h timer reads this on startup so we don't double-check after a quick relaunch.

## 5. Components & file changes

### 5.1 Rust side

#### New: `src-tauri/src/updater/mod.rs`

Single module, ~250 LOC target. Public surface:

```rust
pub fn init() -> tauri::plugin::TauriPlugin<tauri::Wry> { /* tauri_plugin_updater::Builder::new().build() */ }

pub fn start_scheduler(app: &AppHandle);
//   spawns a tokio task that:
//     1. reads last_update_check_at
//     2. waits until 6h from then (or immediately if overdue)
//     3. calls check_and_download
//     4. loops

pub async fn check_and_download(app: &AppHandle) -> Result<UpdateOutcome>;
//   single-cycle: fetch manifest, compare versions, download, verify, stage
//   emits events at each transition

pub async fn install_update(app: &AppHandle) -> Result<()>;
//   invokes the staged installer; app will exit and relaunch

pub enum UpdateOutcome { UpToDate, Ready(Version), Failed(Phase, String) }
```

`#[cfg(debug_assertions)]` compile-time guards in `start_scheduler` and the two commands turn the whole thing into no-ops in dev builds.

#### New Tauri commands (registered in `lib.rs`)

```rust
#[tauri::command]
async fn check_for_updates_now(app: AppHandle) -> Result<(), String>;
//   fires off check_and_download in a background task; returns immediately.

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String>;
//   delegates to updater::install_update.
```

Both commands are typed via `tauri-specta` so the frontend gets generated TS bindings.

#### Edits

- `src-tauri/src/lib.rs` — register `tauri_plugin_updater::Builder::new().build()`, register both commands, call `updater::start_scheduler` from `setup()`.
- `src-tauri/src/tray.rs` — add a "Check for Updates…" menu item just above "Quit". On click, dispatch `check_for_updates_now`. Greyed/relabeled to "Checking…" while a check is in flight (driven by an internal Mutex<bool>).
- `src-tauri/Cargo.toml` — add `tauri-plugin-updater = "2"`.
- `src-tauri/tauri.conf.json` — add `plugins.updater` block:

  ```json
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/FeiXu-1131372/claude-usage-monitor/releases/latest/download/latest.json"
      ],
      "pubkey": "<output of tauri signer generate>",
      "windows": { "installMode": "passive" }
    }
  }
  ```

  - `installMode: passive` → silent install with progress UI but no user prompts.

- `src-tauri/capabilities/default.json` — add `"updater:default"`, `"updater:allow-check"`, `"updater:allow-download-and-install"`.

### 5.2 React side

#### New: `src/state/updateStore.ts`

Small Zustand slice mirroring the state machine:

```ts
type UpdateStatus =
  | 'idle' | 'checking' | 'up-to-date' | 'available'
  | 'downloading' | 'ready' | 'failed';

interface UpdateState {
  status: UpdateStatus;
  version: string | null;        // when status ∈ {available, downloading, ready}
  progress: number;              // 0–1, when status === 'downloading'
  error: { phase: string; message: string } | null;
  lastCheckedAt: string | null;  // ISO-8601
  setStatus: (s: UpdateStatus) => void;
  setAvailable: (version: string) => void;
  setProgress: (p: number) => void;
  setReady: (version: string) => void;
  setFailed: (phase: string, message: string) => void;
  setUpToDate: (checkedAt: string) => void;
}
```

#### New: `src/lib/updateEvents.ts`

```ts
export function attachUpdateListeners(): () => void {
  // listen() to all six update://... events
  // dispatch into useUpdateStore
  // return unlisten function
}
```

Called once from `App.tsx` in a mount-time `useEffect`.

#### New: `src/components/UpdateBanner.tsx`

Renders nothing unless `status === 'ready'` or `status === 'failed'` with phase `install`.

```tsx
// Pseudocode
if (status === 'ready') {
  return (
    <motion.div initial={{ y: -36, opacity: 0 }} animate={{ y: 0, opacity: 1 }}>
      <ArrowUpCircle size={14} className="text-accent" />
      <span>Update ready · v{version}</span>
      <button onClick={installUpdate}>Install & restart</button>
    </motion.div>
  );
}
if (status === 'failed' && error.phase === 'install') {
  // same banner shape, message "Install failed", button "Retry"
}
```

Tokens used (all already in the design system):
- Surface tint: terracotta @ 6% opacity
- Icon + button text: teal accent
- Body text: neutral-200
- Spring: same `spring.gentle` curve used elsewhere (220ms)

#### Edits

- `src/components/Popover.tsx`:
  - Mount `<UpdateBanner />` at the very top of the popover, above the existing content.
  - Add a centered version line at the bottom: `Claude Limits v{__APP_VERSION__}  ·  Check for updates`.
    - "Check for updates" is a button-styled text link.
    - Click states (in-place text swap, no layout shift): idle → "Checking…" → "Up to date" (3s) → idle. If result is `available`/`ready`, the banner appears, version line returns to idle.
  - `__APP_VERSION__` injected via Vite's `define` from `package.json`.
- `src/App.tsx` — mount-time `useEffect` calls `attachUpdateListeners()`, returns its cleanup.

### 5.3 CI / release pipeline

Three changes to `.github/workflows/release.yml`:

1. **Inject signing secrets into the build job**:

   ```yaml
   env:
     TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
     TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
   ```

   `tauri-action` reads these and produces `.sig` files alongside each artifact.

2. **Add `updater` bundle target** so the build produces the update-friendly archive formats:

   ```yaml
   args: --target ${{ matrix.target }} --bundles app,dmg,updater  # macOS
   args: --target ${{ matrix.target }} --bundles nsis,updater     # Windows
   ```

3. **Add a third job, `compose-manifest`, that runs after the matrix completes**:

   - `needs: [release]`
   - Downloads all release artifacts via `gh release download`
   - Reads `.sig` file contents
   - Runs `scripts/generate-latest-json.mjs` to compose `latest.json`
   - Uploads it via `gh release upload <tag> latest.json`

#### New: `scripts/generate-latest-json.mjs`

Pure Node script, no deps beyond Node 20 stdlib. Inputs: tag name, artifact directory. Output: `latest.json` matching the schema in §3.2. Emits an error and fails the job if any expected `.sig` file is missing.

#### One-time setup steps (manual, documented in README)

1. Run `pnpm tauri signer generate -w ~/.tauri/claude-limits.key` locally.
2. Add the public key to `tauri.conf.json` `plugins.updater.pubkey`. Commit it.
3. Add the private key (entire file contents) and password to GitHub Actions repo secrets:
   - `TAURI_SIGNING_PRIVATE_KEY`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

These steps happen once per project. Subsequent releases are fully automated by tagging.

### 5.4 Versioning workflow

Single source of truth: `package.json`. A small release script syncs:
- `package.json` `version`
- `src-tauri/Cargo.toml` `[package].version`

Recommended flow (a single command does everything):
```
node scripts/release.mjs 0.2.0
# 1. Updates package.json version (in-place edit, no git tag yet).
# 2. Updates src-tauri/Cargo.toml [package].version.
# 3. git add package.json src-tauri/Cargo.toml
# 4. git commit -m "release: v0.2.0"
# 5. git tag v0.2.0
# Then the user manually pushes:
git push && git push --tags     # tag push triggers release.yml
```

The script is straightforward Node — no deps, ~40 LOC. Tag is created *after* the commit so it points at the correct SHA. We deliberately avoid `pnpm version` because it tags before we've synced `Cargo.toml`, and amending the commit afterwards would invalidate the tag.

### 5.5 README updates

- Add "Updates" section: explains the 6h check cadence, the popover banner, and the manual "Check for updates" affordance.
- Add explicit note: "Updates from v0.2.0 onward are automatic. Upgrading from v0.1.0 → v0.2.0 must be done manually (no updater was wired up in 0.1.0)."
- Keep the existing first-launch sections for both OSes (still relevant — auto-update doesn't bypass Gatekeeper / SmartScreen on the *first* install).

### 5.6 Summary of file changes

```
NEW    src-tauri/src/updater/mod.rs
EDIT   src-tauri/src/lib.rs                    (register plugin + commands + scheduler)
EDIT   src-tauri/src/tray.rs                   (add "Check for updates" menu item)
EDIT   src-tauri/Cargo.toml                    (add tauri-plugin-updater)
EDIT   src-tauri/tauri.conf.json               (plugins.updater config)
EDIT   src-tauri/capabilities/default.json     (updater permissions)

NEW    src/state/updateStore.ts
NEW    src/components/UpdateBanner.tsx
NEW    src/lib/updateEvents.ts
EDIT   src/components/Popover.tsx              (mount banner + version line)
EDIT   src/App.tsx                             (start event listener on mount)
EDIT   vite.config.ts                          (inject __APP_VERSION__ from package.json)

EDIT   .github/workflows/release.yml           (signing secrets + updater bundles + manifest job)
NEW    scripts/generate-latest-json.mjs        (composes manifest in CI)
NEW    scripts/release.mjs                     (bumps version across files + commits + tags)
EDIT   package.json                            (release script entry)
EDIT   README.md                               (document update behavior + version note)
```

## 6. UI specifics

### 6.1 Popover banner

```
┌─────────────────────────────────────────────────┐
│  ↑  Update ready · v0.2.0   [Install & restart] │
└─────────────────────────────────────────────────┘
```

- Height: 36px
- Background: terracotta @ 6% opacity, rounded top corners only (matches popover container)
- Icon: Lucide `ArrowUpCircle`, 14px, teal accent
- Body: "Update ready · vX.Y.Z" — system font, 12px, neutral-200, tracking-tight
- Button: ghost-pill, teal accent text, no background; hover → 4% teal background; active → 8%
- Slide-in animation: from y=-36, opacity 0 → y=0, opacity 1, spring `gentle` (220ms)
- No dismiss / "later" / "skip" affordance — install is the only action.

### 6.2 Version line (popover footer)

```
                Claude Limits v0.2.0  ·  Check for updates
```

- 11px, neutral-500, centered
- "Check for updates" text-link styled (underline on hover, teal-tinted on hover)
- States (in-place swap, no layout shift):
  - Idle → "Check for updates"
  - Checking → "Checking…" with subtle pulsing dot to the left
  - Up to date → "Up to date" for 3 seconds, then back to idle
  - Available → banner appears; line returns to idle
  - Failed → "Couldn't check" for 3 seconds, then back to idle

### 6.3 Tray menu item

Right-click tray menu, just above "Quit":

- Idle: `Check for Updates…`
- In-flight: greyed out + label `Checking…`

Both surfaces (popover footer + tray menu) call the same `check_for_updates_now` command.

### 6.4 Edge cases

- **Dev mode**: updater no-op via `cfg!(debug_assertions)`. Banner never appears, version line shows but "Check for updates" is greyed out with tooltip "Disabled in dev builds".
- **Pre-v0.2.0 → v0.2.0**: manual download required (existing v0.1.0 has no updater). Documented in README. Every release after v0.2.0 auto-updates.
- **Currently-running app at install time**: Tauri's `installMode: "passive"` on Windows handles file-in-use. macOS `.app.tar.gz` swap on relaunch. No extra logic.
- **Offline at scheduled check**: silent failure, retry next cycle.
- **Clock skew / future `pub_date`**: ignored — version comparison is semver-based, not date-based.

## 7. Testing strategy

### 7.1 Unit tests (Rust)

- `updater::manifest::parse` — accepts valid manifests, rejects malformed JSON, rejects schemas missing required fields.
- `updater::version::is_newer(a, b)` — semver comparison edge cases (0.2.0 vs 0.10.0, prerelease ignored, etc.).
- `updater::scheduler::next_check_delay` — given a `last_checked_at`, returns the correct duration.

### 7.2 Integration tests (Rust)

- Spin up a local HTTP server (using `mockito`, already in dev-deps) that serves a fake `latest.json` and a fake signed bundle.
- Run `check_and_download` end-to-end against the local server.
- Assert correct events emitted in the correct order.
- Assert signature mismatch causes verification failure and no install.

### 7.3 Frontend tests (Vitest)

- `updateStore` reducers — state transitions match the state machine.
- `UpdateBanner` component — renders correctly for each status, button click invokes the right command (mock Tauri's `invoke`).

### 7.4 Manual smoke test (per release)

Documented checklist in `docs/release-checklist.md` (created as part of this work):

1. Build and tag a `v0.99.0-test` release pointing at a private repo branch.
2. Install the previous version on each OS (mac + Windows).
3. Wait for auto-check or click "Check for updates".
4. Verify banner appears, click "Install & restart", verify app relaunches at new version.
5. Repeat with airplane mode on (verify silent failure).
6. Repeat with mismatched signature (verify reject + log).

## 8. Open questions / future work

- Add a progress bar to the banner during download? Currently the banner only appears after `ready`. Cheap to add later — `update://progress` events already emitted.
- Add release notes inline in the banner via a tooltip / expandable section? Currently we link to the GitHub release page implicitly via "Update ready". Defer until users ask.
- Add a beta channel? Trivial to add — second manifest URL, settings toggle. Defer until there's user demand.
- Telemetry on update success/failure rates? Would need a backend and consent UX. Defer indefinitely.

## 9. Appendix: why this is robust on $0

| Concern | How it's handled |
|---|---|
| Cost | $0 — GitHub Releases is free, ed25519 keys are free, Tauri plugin is free. |
| Infrastructure to maintain | None — no update server, no DB, no auth. Static JSON file on a CDN. |
| Trust | ed25519 signature on every artifact, public key embedded in the app, private key in GH Actions secret. Comparable to most desktop OSS apps. |
| Reliability | GitHub Releases SLA + CDN. The same channel that already serves your `.dmg` / `.msi` today. |
| Upgrade path to paid signing | Drop in Apple Developer ID + Windows EV cert into the same `release.yml`. Updater architecture is unchanged. |
| Lock-in | None — the updater contract is just an HTTPS GET to a static JSON file. Swappable with any host (S3, Cloudflare, custom server) by changing one URL. |

## 10. Approval & next step

This spec has been walked through section-by-section with the user and approved. The next step is the implementation plan, produced via the `writing-plans` skill, which will sequence the work in §5 into reviewable checkpoints.
