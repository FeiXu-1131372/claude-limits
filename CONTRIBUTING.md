# Contributing

Thanks for taking a look at Claude Switchboard.

## Before you open a PR

- **Typos, docs, small fixes** — just send the PR, no need to open an issue first or discuss branching.
- **New features or behavior changes** — open an issue first so we can agree on the approach before you invest the time.

## Local setup

```bash
pnpm install
pnpm tauri dev
```

## Before you submit

Run the same checks CI runs:

```bash
pnpm lint   # tsc --noEmit
pnpm test   # vitest

cd src-tauri
cargo test --all-features
cargo clippy --all-targets -- -D warnings
```

All four must pass. `test.yml` runs this matrix on macOS, Windows, and Linux (Linux for the Rust suite only — this is a desktop app with no Linux build target).

## Scope

This is a two-platform desktop app (macOS + Windows) built on Tauri v2. Please keep PRs focused — one behavior change per PR is easier to review than a bundle of unrelated fixes.
