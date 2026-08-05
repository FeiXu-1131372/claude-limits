# Security Policy

Claude Switchboard handles sensitive material: OAuth tokens for your Claude accounts and API keys for any third-party model providers you configure. We take reports about either seriously.

## Reporting a vulnerability

Please **do not** open a public issue for security reports.

Instead, use GitHub's private reporting flow: go to the [Security tab](https://github.com/FeiXu-1131372/claude-switchboard/security/advisories/new) and click "Report a vulnerability." This opens a private advisory that only maintainers can see until it's resolved.

Include what you'd include in any report: affected version, platform (macOS/Windows), reproduction steps, and impact.

## Scope

Things we consider in scope: credential handling (OS keychain / DPAPI, `accounts.json`, `~/.claude/settings.json` writes), the OAuth flow, provider API key storage, and anything that could leak tokens or keys to disk, logs, or process arguments in plaintext.

## Supported versions

Only the latest release is supported. Please upgrade before reporting to confirm the issue still reproduces.
