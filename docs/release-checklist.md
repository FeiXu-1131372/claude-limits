# Release Checklist

Before tagging a release, complete every item on both macOS and Windows.

## macOS (14+)
- [ ] Fresh install (download `.dmg`, drag to Applications, remove quarantine)
- [ ] OAuth paste-back: click "Sign in with Claude", complete in browser, paste `code#state`, verify usage loads
- [ ] Use Claude Code credentials shortcut: sign out, click "Use Claude Code credentials", verify usage loads
- [ ] `debug_force_threshold(five_hour, 75)` fires a notification once
- [ ] Re-run `debug_force_threshold(five_hour, 75)` before reset -> no notification
- [ ] Open expanded report; all 6 tabs render
- [ ] Disconnect network -> stale indicator appears within 15m; notifications do not fire
- [ ] System clock moved backward 2h -> `CachedUsage` marks stale; countdown does not go negative

## Windows (11)
- [ ] Fresh install (`.msi`), SmartScreen "Run anyway"
- [ ] Repeat every macOS step that uses auth + tabs + debug threshold
- [ ] Verify DACL on `credentials.json` fallback (icacls shows user-only access)

## Windows (10)
- [ ] WebView2 auto-bootstrap succeeds
- [ ] Popover renders with translucent-solid fallback (no Mica)
