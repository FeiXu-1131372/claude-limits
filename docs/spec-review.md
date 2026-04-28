# Spec Review — Claude Limits (2026-04-24)

**Reviewer stance:** Adversarial — find problems before implementation burns time.
**Verdict ordering:** Critical issues block implementation. Recommended changes improve the plan. Nice-to-haves are optional. Risks-accepted are called out so nothing is "ambushed" later.

---

## 1. Summary verdict

**Ship with changes — but the changes are material.** The architecture is sound and the module boundaries are the right shape, but the spec has at least **three load-bearing factual errors** about the systems it claims to interoperate with: the OAuth redirect flow, the shape of the usage-API response, and the Windows Claude Code credential location. Any one of those, uncaught, will force a significant re-plan mid-implementation. On top of that, the cross-platform-parity claim is stronger than the spec's own content supports. Revise Sections 2, 3, 4, 5, 6, and 8 before writing an implementation plan.

---

## 2. Critical issues

### C1. OAuth flow is incompatible with how Claude OAuth actually works

**Where:** §2 ("deep-link handler for `claude-monitor://auth/callback`"), §3 (`oauth_pkce.rs`), §5 Scenario A steps 4–5.

**Problem:** The spec describes a custom-scheme redirect (`claude-monitor://auth/callback`) caught by Tauri's deep-link handler. The only reference implementation — claude-usage-bar, which the spec cites as "the only project doing auth right" — does not use a custom-scheme redirect. It uses Anthropic's hosted out-of-band callback that **displays a `code#state` string to the user, who then pastes it back into the app**.

Evidence (claude-usage-bar `macos/Sources/ClaudeUsageBar/UsageService.swift`):
- Line 41: `defaultRedirectURI = "https://platform.claude.com/oauth/code/callback"` (web URL, not custom scheme)
- Line 136: authorize URL includes `?code=true` — this is Anthropic's "show-the-code" mode
- Lines 152–165 (`submitOAuthCode`): parses `code#state` out of user-pasted input
- README line 58: "Paste the code back into the app"

The reason this matters is not stylistic: the `client_id` used by all known Claude-compatible OAuth apps (`9d1c250a-e61b-44d9-88ed-5944d1962f5e`) is Claude Code's public client_id and its registered redirect is `https://platform.claude.com/oauth/code/callback`. Claude's OAuth server will reject any other `redirect_uri` — including `claude-monitor://auth/callback` — with `invalid_redirect_uri`. And Anthropic has no public developer portal to register a different client_id for a third-party app.

Net: the custom-scheme flow described in §5 Scenario A **cannot work at all** against the real Claude OAuth endpoints.

**What to change:**
- Replace the custom-scheme deep-link flow with the out-of-band `code#state` paste-back flow. Update §2 (remove deep-link handler from the Tauri-shell responsibilities list), §3 (rename `oauth_pkce.rs` responsibilities), §5 Scenario A (step 4 becomes "User sees code on callback page, copies it; app shows a text field and a 'Paste code' action"), and the AuthPanel UI in §3.
- Explicitly name `client_id = 9d1c250a-e61b-44d9-88ed-5944d1962f5e`, `redirect_uri = https://platform.claude.com/oauth/code/callback`, and `token_endpoint = https://platform.claude.com/v1/oauth/token` in §2 or §4, or mark them as "TBD — verify with Anthropic". Hand-waving "OAuth 2.0 + PKCE" leaves implementers to rediscover these from competitor code.
- Call out the client_id reuse as a **risk-accepted** item (§5 below) — the spec is quietly adopting Claude Code's public OAuth client_id, which is the only viable option but is not explicitly an Anthropic-sanctioned path for third-party apps.

### C2. `UsageSnapshot` schema doesn't match the endpoint's actual shape

**Where:** §4 `pub struct UsageSnapshot { five_hour, seven_day, per_model: HashMap<Model, Utilization>, ... }`; §5 Scenario B.

**Problem:** The `api.anthropic.com/api/oauth/usage` endpoint returns **five fixed fields**, not a generic `per_model` map:

Evidence (ai-token-monitor `src-tauri/src/oauth_usage.rs:20–29, 283–303`):
```rust
pub struct OAuthUsage {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
    pub seven_day_sonnet: Option<UsageWindow>,
    pub seven_day_opus: Option<UsageWindow>,
    pub extra_usage: Option<ExtraUsage>,
    ...
}
```
Confirmed identically in claude-usage-bar `UsageModel.swift:3–16` and Claude-Code-Usage-Monitor `src/poller.rs:34–44`.

Three concrete consequences:
1. Per-model breakdown is **only for seven-day**, not five-hour, and is **only Sonnet + Opus** — no Haiku bucket exists.
2. There is a separate `extra_usage` object (pay-as-you-go credits with `is_enabled`, `monthly_limit`, `used_credits`, `utilization`, amounts in cents). The spec ignores this entirely — any user on the extra-usage plan will see a wrong dashboard.
3. The field is named `utilization` (number, appears to be percentage 0–100 based on claude-usage-bar line 72 `/ 100.0`) and `resets_at` (ISO string) — not `used_pct` + `reset_at` as the spec writes.

**What to change:**
- Replace `per_model: HashMap<Model, Utilization>` with concrete `seven_day_opus: Option<Utilization>` and `seven_day_sonnet: Option<Utilization>`.
- Add `extra_usage: Option<ExtraUsage>` with `is_enabled`, `monthly_limit_usd`, `used_credits_usd`, `utilization`. Decide whether the popover shows it (recommend yes — it's a real user state).
- Rename spec fields to match wire names (`utilization`, `resets_at`) or explicitly document the translation at the `types.rs` boundary.
- Add the required HTTP headers to the spec: `Authorization: Bearer <token>`, `anthropic-beta: oauth-2025-04-20`. Ai-token-monitor also sends `User-Agent: claude-code/<version>`; it's worth verifying whether that's required or cosmetic. Without `anthropic-beta`, the endpoint returns 404/4xx.

### C3. Windows Claude Code credentials are in a file, not Credential Manager

**Where:** §2 (credential storage table: "Windows Credential Manager"), §5 Scenario A local-creds path step 1 ("Windows: credential manager API via `keyring` crate"), §3 `claude_code_creds.rs`.

**Problem:** Claude Code on Windows stores OAuth credentials in a plaintext JSON file at `%USERPROFILE%\.claude\.credentials.json`, **not** Windows Credential Manager.

Evidence (Claude-Code-Usage-Monitor `src/poller.rs`):
- Line 379–383 (`fn windows_credential_source`): `home.join(".claude").join(".credentials.json")`
- `read_windows_credentials` calls `std::fs::read_to_string(&cred_path)` — direct file read, no Credential Manager API.

Same fallback in ai-token-monitor `oauth_usage.rs:265–279` (`read_oauth_token_file` for non-macOS).

The `keyring` crate on Windows **can** write to Credential Manager — but Claude Code doesn't, so reading from it to get Claude Code's creds will always fail. The spec conflates "where we store our own refresh token" with "where Claude Code stores its token" — those are different decisions for different OSes.

**Secondary:** On macOS, Claude Code v2.1.52+ uses **multiple** keychain service names (`Claude Code-credentials`, `Claude Code-credentials-{hash}`). Evidence: ai-token-monitor `oauth_usage.rs:149–177` has explicit discovery logic using `security dump-keychain`. The spec says only "`security find-generic-password -s \"Claude Code-credentials\"`", which will miss newer Claude Code installs.

**Tertiary:** On macOS, the raw keychain data sometimes has a prepended non-ASCII byte before the JSON — ai-token-monitor `oauth_usage.rs:253–263` has a workaround. The spec doesn't mention this edge case.

**What to change:**
- Rewrite the "Claude Code creds" row in §2 to show two OS paths:
  - macOS: Keychain via `security` CLI, with discovery of `Claude Code-credentials*` service names, and the stray-byte trim.
  - Windows: file read from `%USERPROFILE%\.claude\.credentials.json`.
  - (Optional) WSL: `wsl.exe -d <distro> -- cat ~/.claude/.credentials.json` — call out as v2 or explicit non-goal.
- In §3, split `claude_code_creds.rs` into OS-conditional modules or use `cfg(target_os)` branches and document both.
- Update §5 Scenario A step 1 under "Local creds path" to match.

### C4. Cross-platform parity claim is stronger than the spec supports

**Where:** §1 ("True cross-platform parity"), §2 (architecture promises), §8 ("True cross-platform from day one").

**Problem:** The spec asserts true parity but the content concentrates on macOS details, and at least five specific parity gaps are not even named:

1. **Keychain prompt UX.** macOS keychain access triggers a prompt on first access per-identifier; Windows Credential Manager does not. The spec's §6 "macOS keychain prompts on every launch — store stable `app_identifier`" acknowledges only one facet. It doesn't say whether keychain prompts interrupt polling, nor what the Windows-equivalent quiet path looks like.
2. **Popover vibrancy / translucency.** macOS has `NSVisualEffectView`; Windows 11 has Mica/acrylic (only via `tauri-plugin-window-vibrancy` or unstable APIs); Windows 10 has neither. Section 1's "Glassy modern UI (Raycast / Linear / Arc aesthetic, on both platforms)" does not acknowledge that the Win10 popover will either look different or require a custom blur shader. Decide now, document the fallback.
3. **Tray icon.** macOS tray icons are template images (auto-tinted for dark menu bar); Windows taskbar tray icons are full-color ICO at multiple sizes. Color coding (<75% / amber / red) from §5 Scenario B must use template-compatible paths on macOS (color badges overlay the template image — Tauri's `set_icon` works, but the spec doesn't say how color tint gets rendered without killing template behavior).
4. **Auto-launch at login.** macOS uses `SMAppService` (requires sandbox entitlement, or LaunchAgent plist for non-sandboxed apps). Windows uses `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. `tauri-plugin-autostart` abstracts this but has known issues on unsigned macOS apps (Gatekeeper revalidation). Call the plugin out by name and note its limits.
5. **Deep-link registration** (if C1 is not resolved in favor of paste-back). macOS Info.plist `CFBundleURLTypes`; Windows registry under `HKCU\Software\Classes\<scheme>`. An unsigned app on Windows silently succeeds at writing the registry key but there's no cross-install migration, and another app can claim the same scheme first-wins.

**What to change:**
- Add a "Cross-platform parity matrix" subsection to §2 or a new §2.5 that lists every platform-specific surface (keychain, vibrancy, tray icon, autolaunch, deep-link, notifications, updater) and names the concrete library/API per OS plus the degraded fallback when needed. Without this, "true parity" is aspirational.
- Rewrite §1's parity claim more conservatively: "True feature parity (UI fidelity may differ on Windows 10 where Mica/acrylic is unavailable)".

### C5. Pricing-model scope for JSONL parsing understates the pricing set

**Where:** §3 `jsonl_parser/pricing.rs # Model → cost table (Opus 4.7, Sonnet 4.6, etc.)`; §7 tier-1 tests for "Opus 4.7, Sonnet 4.6, Haiku 4.5".

**Problem:** JSONL files contain **historical** model invocations, including ones retired months or years ago. A parser that only knows Opus 4.7 + Sonnet 4.6 + Haiku 4.5 will zero-cost every session older than ~a few months.

Evidence (ai-token-monitor `src-tauri/pricing.json`): the Claude table includes `opus-4-7`, `opus-4-6`, `opus-4-5`, `opus-4-1`, `opus-4`, `sonnet-4-6`, `sonnet-4-5`, `sonnet-4`, `haiku-4-5`, `haiku-3-5` — ten distinct match patterns, with catch-all `opus` / `sonnet` / `haiku` fallbacks. The spec's v1 covers three of those.

**What to change:**
- Replace `Model` enum with a string-keyed pricing table using prefix-match (as ai-token-monitor does) so unknown models are scored against the best matching family rather than silently becoming zero cost.
- In §7, expand the pricing test to cover the full current family set (Opus 4.x, Sonnet 4.x, Haiku 4.x, Haiku 3.5) and require the test to fail if any model in fixtures isn't priced.
- Externalize the pricing table to `src-tauri/pricing.json` (same shape as ai-token-monitor) so pricing can be updated without a rebuild; document a runtime-override path.

### C6. The "both auth sources coexist" case is undefined

**Where:** §6 error-handling tables cover "OAuth fails" and "Claude Code creds missing", but not both-present.

**Problem:** The spec's `auth::get_access_token() -> Result<(String, AuthSource)>` returns a token without specifying priority. If the user has both an OAuth refresh token in our keychain AND a valid Claude Code credentials file, which wins? And what if the two are for **different Anthropic accounts**? The resulting popover will show usage for one account while the user assumes it's the other — a subtle and wrong-feeling bug that's hard to diagnose.

**What to change:**
- §4: define the precedence explicitly — e.g., "App's own OAuth token wins over Claude Code creds when both are valid; if they resolve to different accounts, emit `auth_source_conflict` event and show a 'Which account?' chooser in the popover."
- §5 Scenario A: add a branch for "both present".
- §6: add a row for "auth sources disagree on account identity" (requires fetching `/api/oauth/userinfo` at auth-source switch time to compare).

### C7. JSONL parsing contract under-specifies failure modes that will happen

**Where:** §3 `jsonl_parser/record.rs # SessionEvent { ts, project, model, tokens, cost }`; §6 JSONL parser failures.

**Problem gaps:**
- **New/unknown fields.** Claude Code has already changed its JSONL schema at least once (e.g., `cache_creation_5m_tokens` / `cache_creation_1h_tokens` split). Today's parser will encounter fields it doesn't know. Spec says "skip malformed lines" but doesn't distinguish "malformed" from "newer schema with extra fields" — the latter should succeed, not be counted as a failure.
- **Partial writes.** Claude Code writes JSONL append-only; a crash mid-line produces a half-line. If the watcher fires on mtime change while Claude Code is mid-write, the current approach ("re-read from stored offset to EOF") will read a half-line as malformed. Debouncing the watcher by 500ms (§5 Scenario C) reduces but doesn't eliminate this.
- **File truncation / rotation.** If Claude Code rewrites a file (newer versions deduplicate logs), the cursor offset is now past the new EOF, and the walker will silently skip all new data. Spec doesn't have a reset-cursor-on-truncation path.
- **Timezone confusion.** Claude Code writes timestamps as ISO-8601 (probably UTC); reset timers and daily bucketing need a user-local tz decision. Spec doesn't declare it.

**What to change:**
- Add a row to the §6 JSONL table for "file shorter than stored offset → reset cursor to 0, re-parse".
- Explicitly adopt `serde(default)` / `#[serde(deny_unknown_fields = false)]` semantics and document that unknown fields are ignored, not errors.
- In §3 `record.rs`, declare the canonical timezone handling (recommend: store UTC in DB, convert to local only for display).
- In §7, add an integration test: "partial line at EOF gets retried successfully on next watcher fire".

### C8. Testing strategy undercovers the things most likely to silently break

**Where:** §7 Tier 1 table; "No tests needed" section.

**Problem:** The six-module unit-test list is reasonable but skips:
- **`usage_api::types`**: the deserialization contract. This is the *most* likely module to silently break when Anthropic adds a field or renames one. A serde round-trip test with a committed real-world response fixture is cheap and load-bearing. (Evidence that shape changes happen: the difference between Claude-Code-Usage-Monitor's 2-bucket parser and ai-token-monitor's 5-field parser already shows divergence in what each saw from the endpoint.)
- **`auth::claude_code_creds`**: per-OS file-read logic. "Covered by manual test" is not enough — an OS detection bug (e.g., WSL path) will only reproduce in one contributor's setup.
- **`tray::update_badge`** (if added — per §5 Scenario B): the three color thresholds + stable icon-identity behavior on macOS template rendering.

Also, the manual release checklist (§7 Tier 4) has items that can't be completed in one session by one person: "Leave app running 2+ hours, confirm tray updates + threshold alert at 75%" requires either a mock endpoint or real account burn. Either provide a debug menu to force-trigger thresholds, or replace the 2h item with "run against mock endpoint that synthesizes crossing events".

**What to change:**
- Add `usage_api::types` with a fixture-backed serde round-trip test.
- Add `auth::claude_code_creds` platform-gated tests (run actual macOS test on macOS CI, Windows test on Windows CI).
- Add a dev-only `debug_force_threshold(bucket, pct)` IPC command so the release checklist threshold item is deterministic.
- Acknowledge that the `ubuntu-latest` CI (§7) won't run any platform-gated tests that matter — add the macOS+Windows matrix to `test.yml`, not only `release.yml`.

---

## 3. Recommended changes

### R1. `is_stale` belongs in the poll loop, not the snapshot shape

**Where:** §4 `UsageSnapshot { ..., is_stale: bool }`.

`is_stale` is a derived property (age > 15min, or last-poll failed) computed by the caller. Embedding it in the wire type forces every producer to decide stale-ness at creation time. Split into `fetched_at: DateTime<Utc>` on the snapshot and `is_stale(now)` method or a separate `CachedUsage { snapshot, fetched_at, last_error }` wrapper for the UI. See ai-token-monitor `oauth_usage.rs:57–68` where stale-ness is recomputed at every cache read.

### R2. Polling-interval defaults and bounds are optimistic

**Where:** §5 Scenario B ("configured_interval 1m–30m"), §9 ("default 5m").

- 1-minute polling against `api.anthropic.com/api/oauth/usage` risks rate-limiting users who run the app plus Claude Code plus other tools against the same token. claude-usage-bar's default is **30 minutes** with options [5, 15, 30, 60] (UsageService.swift:33–34). A 5-minute default is 6× more aggressive with no demonstrated signal that justifies it.
- Clamp 1m lower bound to at least 5m, match ai-token-monitor and claude-usage-bar convention, and reduce 30m cap to 60m or 120m — subscription limits change slowly.

### R3. Refresh-token storage is under-specified

**Where:** §3 `token_store.rs # Encrypted refresh token at ~/.claude-monitor/`; §2 keyring crate.

The spec lists `keyring` crate in §2, then says refresh tokens go to `~/.claude-monitor/` in §3. These are contradictory — is it OS keychain or a filesystem file? Pick one, document the on-disk encryption scheme if file-based (recommend: use `keyring` crate on both OSes; fall back to 0o600-permission file like claude-usage-bar `StoredCredentials.swift:42–47` on keychain failure — then document the fallback explicitly).

### R4. Analytics tabs: Heatmap and Cache have weakest v1 value

**Where:** §3 tabs (Sessions, Models, Trends, Projects, Heatmap, Cache).

Six tabs for v1 is front-loaded:
- **Heatmap** extrapolates 90-day backfill into 365-day cells (§5 Scenario D) — the "year view" is ~75% synthesized/empty and misleading at first launch. Defer to v1.1 and replace with a simpler 90-day strip.
- **Cache** stats (hit ratio + cost savings) is interesting only to users who already understand the 5m/1h cache tiers; it's a second-week feature, not first-week. Merge into Models tab as a collapsible section, or cut to v1.1.

Four tabs (Sessions, Models, Trends, Projects) cover the "where did my tokens go" question for 90% of users.

### R5. "No external telemetry" is good; "no crash reporting" is a gap

**Where:** §6 "No external telemetry — logs stay local".

Defensible for v1, but without any crash reporting the project maintainer is blind to Windows-specific bugs reported as "it just crashes". At minimum, add a Settings toggle "Send anonymous crash reports" defaulting to **off**, with the hook stubbed; users who self-opt in provide a valuable signal for low-frequency failures. Mark implementation as v1.1 if needed — just leave the toggle so the v1 decision isn't locked in.

### R6. Error-handling rows for clock skew and single-instance

Add to §6:
- **System clock moves backward** (user VPN hopping across timezones, or manual adjustment) — `reset_at` comparisons flip. Recommend: always compute reset countdown as `max(0, reset_at - now)`; if `now` is after a cached `fetched_at`, invalidate the stale cache. Today's spec doesn't defend against `now < fetched_at`.
- **Two instances of the app running** (user double-clicks the icon; or one in /Applications, one in ~/Downloads). Both will poll independently, both will file notifications, both will fight over the SQLite write lock. Use a file lock on the DB directory or Tauri's single-instance plugin.

### R7. JSONL walker tail follows symlinks & global patterns are risky

**Where:** §3 walker, §5 Scenario C.

`glob::glob("~/.claude/projects/**/*.jsonl")` (ai-token-monitor-style) will silently traverse symlinks if any project directory happens to link to a huge tree. Spec should say whether symlinks are followed and declare a max traversal depth (recommend: 10 levels, skip symlinks). Also, projects containing binaries or `node_modules`-sized subtrees with accidentally-named `.jsonl` files will get scanned — bound the walker to a specific layout (`~/.claude/projects/<slug>/*.jsonl`, one level), not `**`.

### R8. "Tauri CLI 2 cross-compiles" is partially false

**Where:** §2 tech stack ("Tauri CLI 2 … Cross-compiles `.dmg` + `.exe` + `.msi` + `.AppImage`").

Tauri 2 does NOT cross-compile macOS from non-macOS hosts (code-signing, codesign-less bundling, and `bundletool` require Apple tooling on a Mac host). It does not cross-compile Windows from macOS for anything involving `wix`/NSIS without extra Docker setup. `release.yml` (§7) already uses a matrix — that's the correct pattern — but the §2 table misleadingly implies single-host cross-compile. Rewrite: "Produces `.dmg` on macOS hosts and `.exe`+`.msi` on Windows hosts; CI matrix required."

### R9. Is Tauri v2 actually the right choice, or motivated reasoning?

The spec chose Tauri v2 anchored on ai-token-monitor. For a menu-bar app with a <200-line popover, alternatives deserve a one-line dismissal:
- **Native (SwiftUI + WinUI3 separate codebases):** zero shared code but better per-platform UX fidelity, no WebView dependency on Windows. Cost: double the code. Three of the 7 competitors took this hit.
- **Electron:** dismissed for footprint — fair.
- **wry/tao directly (no Tauri):** shaves another 2–4 MB and removes Tauri CLI ceremony. Overkill for a team-of-one.

Tauri v2 is probably correct, but §8 should include a one-paragraph "why not native-per-platform" that names the concrete cost saved (single React codebase, single type-generated IPC, shared SQLite schema) rather than letting the choice ride on "ai-token-monitor uses it".

---

## 4. Nice-to-haves

### N1. Drop the `claude-monitor://` name early

The working-name caveat in §1 is already there, but renaming after implementation means renaming the URL scheme (if C1 resolves in favor of keeping any scheme), macOS bundle ID, Windows registry key, config-directory name — touches six to ten files. Pick the real name before starting.

### N2. Add an "observability tab" in Settings → Diagnostics

Just a scrollable log viewer backed by the existing `~/.claude-monitor/logs/` files. Cheap, and makes every bug report actionable. Already half-implemented by the mention of "Open logs folder" in §6.

### N3. First-run-check: is Claude Code even installed?

If neither `~/.claude/projects/` nor the keychain/credential-manager entry exists, the app is useful only as a rate-limit reminder — the Sessions/Models/Trends/Projects tabs will be empty forever. Detect this on first run and either hide those tabs or show a "Install Claude Code to see per-session data" banner. The spec's §6 JSONL table mentions empty-state UI but doesn't make the inference "user doesn't use Claude Code".

### N4. ETag / `If-None-Match` on the usage endpoint

If Anthropic's endpoint supports conditional GETs (worth verifying), the polling loop can skip DB writes on 304 responses. Minor, but reduces SQLite churn for idle users.

### N5. Settings → "Export my data"

Zero-code effort: `sqlite3 data.db .dump > export.sql` via a button. Useful for users who want to share their data with debug reports or switch to another tool.

---

## 5. Risks accepted

These aren't flagged as issues — just things to record now so nobody later says "we never decided":

- **Client-ID reuse.** If C1 resolves by using Claude Code's client_id (`9d1c250a-e61b-44d9-88ed-5944d1962f5e`), the app is effectively impersonating Claude Code's OAuth client. Two other open-source projects (claude-usage-bar, Claude-Code-Usage-Monitor) already do this. Anthropic could break this at any time by tightening OAuth client-credential verification. There is no alternative without an Anthropic developer-registration portal.
- **Unsigned distribution.** First-run friction is real (Gatekeeper `xattr -d com.apple.quarantine`, Windows SmartScreen "Run anyway"). §6 acknowledges this. Accept that discovery-via-friends-and-GitHub is the growth model; Homebrew/Winget later.
- **No telemetry.** Bugs on platforms the maintainer doesn't use daily will go unreported for weeks. §6 commits to opt-in-only in v2. Acceptable for an MIT hobby project.
- **Pricing data is a point-in-time copy.** Even with an external JSON file, new Claude model launches will take a spec update + release. Users who invoke a model released mid-release-cycle will see $0 cost until then.
- **Cache-tier cost math depends on 5m-vs-1h identification in the JSONL.** If the Claude Code JSONL stops distinguishing `cache_creation_5m_tokens` vs `cache_creation_1h_tokens` (schema regression), cache-cost numbers degrade silently. Mitigation in C7 above.
- **OAuth `/api/oauth/userinfo` endpoint.** Claude-usage-bar uses it for email display (`UsageService.swift:299–322`). Spec doesn't mention it — decide whether to surface account email in Settings (recommend yes; many users have multiple Anthropic accounts).
- **Single-person release checklist.** §7 Tier 4 has 7 items that need both macOS and Windows hardware. Accept that solo releases require dual-boot or a Windows VM; document that the checklist isn't CI-automatable.

---

## Appendix — things checked and found fine

- React 19 + Tailwind v4 + Zustand + Recharts + Framer Motion stack — all production-stable as of 2026-04.
- `notify` crate cross-platform behavior — proven across multiple surveyed competitors.
- `rusqlite` for a single-digit-MB database and 90-day retention — trivially fine, no concerns at this scale.
- `reqwest` with backoff — ai-token-monitor proves it works at this exact use case.
- Module-boundary decomposition (§2 "5 isolated modules") is clean and testable — that part of the spec is strong.
- Scope of non-goals (§1) is well-chosen and should be held firm.
