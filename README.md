# Claude Limits

A menu-bar app for tracking your Claude rate limits — 5-hour, weekly, and per-model — at a glance.

> Am I about to hit my limit, and if so, when?

That's the question Claude Limits answers. The numbers are the same ones the Anthropic console shows, just always visible in your menu bar so you don't have to remember to check.

## Screenshots

### Windows 11

The tray icon shows the live 5-hour percentage as a ring badge — the answer to "where am I at?" is visible without opening anything. Hover for a one-line text summary; click for the full popover.

<p align="center">
  <img src="docs/screenshots/windows-tray.png" alt="System tray with live percentage badge" width="320" />
</p>

<p align="center">
  <img src="docs/screenshots/windows-tray-tooltip.png" alt="Tray tooltip on hover — 5h and 7d percentages with reset times" width="480" />
</p>

<p align="center">
  <img src="docs/screenshots/windows-popover.png" alt="Compact popover — 5H and 7D buckets, per-model usage, pay-as-you-go" width="420" />
</p>

Click the expand arrow for the full report — Sessions, Models, Trends, Projects, Heatmap, and Cache tabs.

<p align="center">
  <img src="docs/screenshots/windows-expanded-models.png" alt="Expanded report — Models tab with per-model token breakdown" width="720" />
</p>

<p align="center">
  <img src="docs/screenshots/windows-expanded-heatmap.png" alt="Expanded report — Heatmap tab showing six months of usage" width="720" />
</p>

### macOS

Same layout, rendered with native vibrancy. _(Screenshots coming soon.)_

## How this is different

There are several Claude usage trackers out there. Most fall into one of two shapes:

- **CLI tools** (`ccusage`, terminal monitors) — powerful, but you have to run them.
- **Stock menu-bar bars** — a tray icon with a percent number on it, and not much else.

Claude Limits is a designed menu-bar app with a real UI: a glassy popover with multi-tab analytics, plus a separate expanded report window for deeper analysis. The aesthetic target is macOS Control Center and Raycast, not stock SwiftUI. Every color, radius, and animation comes from a tight token set.

## Features

- **Live utilization** — 5-hour and 7-day buckets pulled from Anthropic's official usage endpoint. Same numbers their console shows, refreshed at your configured interval.
- **Burn-rate projection** — extrapolates your current pace and shows where utilization will land at reset, color-cued against your threshold.
- **Per-model breakdown** — Opus and Sonnet 7-day quotas tracked separately.
- **Pay-as-you-go credits** — surfaced when enabled on your account.
- **Local session analytics** — Sessions, Models, Projects, Trends, Heatmap, and Cache tabs in the expanded report, sourced from your Claude Code JSONL transcripts.
- **Tier-aware cost** — handles Sonnet 4's 1M-context tier correctly (rates double above 200k input tokens). Cache writes split 5-minute vs 1-hour at the right rate.
- **Threshold notifications** — warn / danger levels you choose.
- **Cross-platform** — macOS (vibrancy) and Windows 10/11 (Mica / acrylic).

## Install

No signed release yet — build from source:

```bash
pnpm install
pnpm tauri dev
```

When binaries ship, first-launch notes for unsigned apps:

- **macOS:** `xattr -d com.apple.quarantine "/Applications/Claude Limits.app"` or right-click → Open from Finder.
- **Windows:** SmartScreen → "More info" → "Run anyway". WebView2 is required on Windows 10 (Windows 11 ships it).

## Authentication

By default Claude Limits reuses your existing Claude Code credentials from the OS keychain — no separate sign-in. If you'd rather authenticate independently, an OAuth 2.0 + PKCE paste-back flow is available in Settings.

The app never logs in on your behalf. It reads the token your OS already holds and uses it only against `api.anthropic.com`.

## Privacy

- All data stays on your machine. Usage history is in SQLite at `~/Library/Application Support/com.claude-limits.ClaudeLimits/data.db` (macOS) or the platform equivalent on Windows.
- The only outbound traffic is to Anthropic's official API.
- No telemetry, no analytics, no third-party services.

## Stack

Tauri v2 (Rust + WebView) · React 19 · TypeScript · Tailwind CSS v4 · Framer Motion · Recharts · SQLite.

## Development

```bash
# Frontend typecheck
pnpm exec tsc --noEmit

# Backend tests (75+ unit + integration tests)
cd src-tauri && cargo test
```

## License

MIT
