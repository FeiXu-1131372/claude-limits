# Claude Usage Monitor

Cross-platform menu-bar utility for monitoring Claude subscription rate-limits on macOS and Windows.

## Features
- 5-hour and 7-day usage buckets (with Opus / Sonnet splits)
- Extra-usage credits view (if enabled on your account)
- Per-session analytics from local Claude Code logs
- OAuth 2.0 + PKCE authentication (paste-back flow)
- Optional: reuse existing Claude Code credentials
- Threshold alerts at user-configurable percentages

## First launch
Downloads are **unsigned**. On first launch:

- **macOS:** `xattr -d com.apple.quarantine "/Applications/Claude Usage Monitor.app"` or right-click -> Open from Finder.
- **Windows:** SmartScreen -> "More info" -> "Run anyway".

WebView2 is required on Windows 10 (Windows 11 ships it). If missing, the installer auto-bootstraps it.

## Development
```bash
pnpm install
pnpm tauri dev
```

## License
MIT
