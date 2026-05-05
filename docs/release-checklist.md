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

## Multi-account swap (added 2026-05-05)

- [ ] Fresh install → upstream `/login` as account A → tray app launches → A appears as active in Accounts list
- [ ] Add B via "Use upstream's current login" path (after upstream `/login` as B)
- [ ] Add C via paste-back OAuth (without changing upstream's login)
- [ ] All three show usage in the Accounts sub-screen with correct numbers
- [ ] Click row B → swap → verify CC primary store + `~/.claude.json` reflect B; restart upstream and confirm B is active
- [ ] Repeat with VS Code extension running — toast shows running-process hint, restart extension and confirm B
- [ ] Run `cswap --switch-to A` externally → tray app's active dot moves to A within one poll interval; no false `unmanaged_active_account` banner
- [ ] Upstream `/login` as new D externally → `unmanaged_active_account` banner appears; click "Add to accounts" → D appears, banner clears
- [ ] Remove C → upstream's active login (A or B) untouched
- [ ] Single-account upgrade: install previous version with one OAuth account → upgrade to multi-account → existing account appears as Slot 1, no manual action
- [ ] Org-shared accounts: add two in same org → bars show identical numbers, "shares quota with…" hint appears
