# Execution Handoff Prompt

Copy everything inside the ```prompt``` block into a fresh Claude Code session. The prompt is self-contained — it refers to absolute paths in this repo and does not assume any prior conversation context.

Verified against the project state on 2026-04-24:
- All four reference docs exist at the cited paths
- 31 source files already delivered under `src/` (32 with `.DS_Store`), matching the designer agent's report
- `src-tauri/` does not exist yet
- Git has not been initialized yet

---

```prompt
You are taking over an in-progress project. Your job: execute the implementation plan for a cross-platform Claude subscription usage monitor using the superpowers:subagent-driven-development skill, and parallelize aggressively across independent workstreams.

## Step 1: Read these four documents before doing anything

Read in this order — each builds on the previous:

1. **The project brief (design context):**
   `/Users/feixu/Developer/open Source/claude-usage-monitor/CLAUDE.md`

2. **The design spec (what we're building and why):**
   `/Users/feixu/Developer/open Source/claude-usage-monitor/docs/superpowers/specs/2026-04-24-claude-usage-monitor-design.md`

3. **The implementation plan (your main instructions — 15 phases, 37 tasks, 196 steps):**
   `/Users/feixu/Developer/open Source/claude-usage-monitor/docs/superpowers/plans/2026-04-24-claude-usage-monitor.md`

4. **The design system reference (UI conventions to follow):**
   `/Users/feixu/Developer/open Source/claude-usage-monitor/docs/design-system.md`

Also skim `docs/spec-review.md` — it lists the factual issues we already fixed in spec revision 2 and plan revision 2. Don't relitigate them.

## Step 2: Inspect the current project state

```bash
cd "/Users/feixu/Developer/open Source/claude-usage-monitor"
ls -la
find src -type f | sort
ls src-tauri 2>/dev/null || echo "NO src-tauri YET"
ls .git 2>/dev/null || echo "NO GIT YET"
```

What you should find (verified 2026-04-24):
- **31 UI source files already delivered by a designer agent** under `src/` including `App.tsx`, 11 UI kit components in `src/components/ui/`, 5 lib files (`format.ts`, `icons.ts`, `motion.ts`, `store.ts`, `types.ts`), 2 popover files, 7 report files (ExpandedReport + Sessions/Models/Trends/Projects/Heatmap/Cache), 2 settings files (AuthPanel, SettingsPanel), 2 style files (`globals.css`, `tokens.css`).
- **No `src-tauri/` directory** (Rust backend is green-field).
- **No `.git` directory** (Task 0.1 Step 1 initializes it).
- 5 HTML preview files under `concepts/` (keep them — reference mockups).
- `src/.DS_Store` — ensure `.DS_Store` is in the `.gitignore` written by Task 0.1.

This is important because **the plan's frontend phases (8, 9, 10, 11) were written assuming only the UI kit existed, not the screens**. Since the screens are already there, your frontend work is mostly **integration** — wiring existing components to IPC and Zustand — not creation. See Reconciliation Notes below.

## Step 3: Invoke the executing skills

You must use these two skills together:

- **`superpowers:subagent-driven-development`** — orchestration strategy (fresh subagent per task + two-stage review between tasks)
- **`superpowers:dispatching-parallel-agents`** — for the parts of the DAG where multiple tasks are independent

Invoke `superpowers:subagent-driven-development` first to load its orchestration rules. Follow them exactly — especially the "fresh subagent per task" and "review between tasks" requirements.

## Step 4: Parallelization DAG

The plan has 37 tasks. Naive execution is 37 sequential subagents. Here's the dependency graph — dispatch independent branches in parallel.

**Wave 0 (sequential — bootstrap must land first):**
- Task 0.1 → Task 0.2 → Task 0.3 → Task 0.4

Each of these touches shared config surfaces (`package.json`, `tsconfig`, Vite config, Tauri init, logging) so serialize.

**Wave 1 (after Wave 0, 3 parallel branches — all file-disjoint):**
- **Branch A — Storage & Tests:** Task 1.1 → Task 1.2
- **Branch B — Usage API:** Task 2.1 → Task 2.2
- **Branch C — Auth module:** Task 3.1 → (Task 3.2 + Task 3.3 + Task 3.4 parallel) → Task 3.5

Three branches are file-disjoint (different subdirectories under `src-tauri/src/`). Dispatch three subagents simultaneously. Each branch is sequential internally.

**Wave 2 (after Wave 1, 2 parallel branches):**
- **Branch D — JSONL parser:** Task 4.1 → Task 4.2 → Task 4.3 → Task 4.4
- **Branch E — Notifier:** Task 5.1

Both depend on Branch A (store) being complete.

**Wave 3 (after Wave 2, fan-in, serialized):**
- Task 6.1 (commands + AppState) → Task 6.2 (poll loop) → Task 6.3 (JSONL watcher wiring)

**Wave 4 (after Wave 3, 2 parallel branches):**
- **Branch F — Frontend integration:** Task 7.1 → Task 7.2 → Task 7.3 (specta codegen)
- **Branch G — Tray + Window:** Task 12.1 → Task 12.2

**Wave 5 (after Wave 4, 4 parallel branches):**
- **Branch H — Compact popover wiring:** Task 8.1 (integration only)
- **Branch I — Auth UI wiring:** Tasks 9.1, 9.2 (integration only)
- **Branch J — Expanded report wiring:** Tasks 10.1, 10.2, 10.3
- **Branch K — Settings wiring:** Task 11.1

All four touch different directories and can run in parallel.

**Wave 6 (after Wave 5, 2 parallel branches):**
- **Branch L — CI:** Task 13.1 + Task 13.2
- **Branch M — Docs:** Task 14.1 + Task 14.2

**Critical rule:** Never dispatch subagents whose file-write sets overlap. If a task modifies `lib.rs` or `Cargo.toml`, it cannot run in parallel with another that also does. The DAG above respects this — confirm before dispatching.

## Step 5: Expected wall-clock and progress reporting

This is how long each wave should take on a healthy machine. Use these numbers as sanity checks — if a wave is running much longer, something is stuck and should be investigated, not retried.

| Wave | Work | Expected wall-clock |
|---|---|---|
| 0 | Bootstrap + `pnpm install` + `cargo build` (first-time Rust deps) | **3–5 min** (cargo compile dominates) |
| 1 | Storage + Usage API + Auth (3 parallel) | **15–25 min** |
| 2 | JSONL + Notifier (2 parallel) | **10–15 min** |
| 3 | Commands + Poll + Watcher (serial) | **15–20 min** |
| 4 | IPC + Tray (2 parallel) | **10–15 min** |
| 5 | UI wiring (4 parallel) | **15–25 min** |
| 6 | CI + docs | **5–10 min** |

**Total with parallelism: ~1–2 hours.** Without parallelism the same work is ~3–5 hours.

Report status at the end of each wave. Include: which tasks landed, tests green, commit SHAs, anything that hand-backed.

## Step 6: Reconciliation between plan and delivered UI

The designer agent delivered more than the plan expected. Before executing Phase 8-11, inspect the existing `src/` tree and decide per task whether you are:
- **Creating** a new component (plan wrote from scratch) — DON'T if it already exists
- **Integrating** an existing component with IPC/Zustand — the likely case for 8.1, 9.1, 9.2, 10.1-10.3, 11.1
- **Adapting** the existing component to spec — e.g., delivered Heatmap/Cache tabs

Handle this explicitly per task: run `cat src/popover/CompactPopover.tsx` etc. **before** dispatching the subagent, and include the inspection output in the subagent's prompt so it knows whether to create, integrate, or adapt.

**Specific things to reconcile:**

- **Tab count:** spec v2 says 4 tabs (Sessions, Models with Cache folded in, Trends with 30-day strip, Projects); designer delivered 6 (adds Heatmap, Cache). Either is acceptable — prefer whichever looks more polished in the delivered code. Commit the decision. If keeping 6, update `docs/superpowers/specs/2026-04-24-claude-usage-monitor-design.md` §1 non-goals list to drop the Heatmap/Cache deferrals.
- **`src/lib/types.ts`:** the plan's Task 7.3 deletes this in favor of generated bindings. The designer created one. The plan's intent stands: run Task 7.3 to generate bindings, then delete the hand-written file. Do not skip Task 7.3.
- **`src/lib/store.ts`:** designer created a UI-theme store. Task 7.2 augments it with usage/settings/auth state. Check what's already there and **merge** rather than replace.
- **`src/lib/format.ts`:** designer added this as a post-critique fix. It's not in the plan. Keep it; it may be reused by the reporting tabs.
- **`src/App.tsx`:** designer wrote one; the plan's Task 0.1 Step 7 creates a placeholder. DO NOT overwrite the designer's version. Task 12.2 updates App.tsx to route by window label — that task's rewrite is correct to apply.

## Step 7: Execution protocol per subagent

Each subagent you dispatch must:

1. **Receive absolute file paths and task-scoped context** — brief them like a new hire. They don't have your conversation history.
2. **Read only the files they need**, not the whole plan. Point them at the task subsection.
3. **Run the exact commands specified** in the task steps.
4. **Write code matching the plan's code blocks exactly** — the plan was reviewed twice for compilation correctness.
5. **Run tests before committing** — every task ends with a verification step.
6. **Commit per task** with the conventional-commit message given in the plan.
7. **Report back** with: tests passing, files changed, commit SHA, anything they deviated from and why.

After each subagent returns, you (the orchestrator) do two-stage review:
- **Stage 1:** Quick read of the diff — do the files match the plan? Did tests pass?
- **Stage 2:** Run `cd src-tauri && cargo build && cargo test` yourself (or `pnpm test`) to independently verify. Don't trust the subagent's "tests passing" claim without verification.

If verification fails, either fix inline or dispatch a remediation subagent — don't move to the next wave until the current wave is green.

## Step 8: Known fragilities and how to handle them

These are things previously flagged as likely to go sideways. Handle them yourself, don't just propagate the failure.

### 8a. specta / tauri-specta RC version drift

The plan's `src-tauri/Cargo.toml` pins three pre-release crates to exact versions:
```
specta = "=2.0.0-rc.22"
specta-typescript = "=0.0.9"
tauri-specta = { version = "=2.0.0-rc.21", features = ["derive", "typescript"] }
```

Pre-release crates move frequently. If `cargo build` fails with version conflicts between these three, **do not pin different exact versions blindly**. First try loosening all three to ranges:
```
specta = "2.0.0-rc"
specta-typescript = "0.0"
tauri-specta = { version = "2.0.0-rc", features = ["derive", "typescript"] }
```
and let Cargo resolve. If that still fails, search the specta changelog / GitHub releases for the last known-compatible trio and pin to that. Document what you ended up with in a commit message. Do **not** skip Task 7.3 — codegen is load-bearing for the IPC contract.

### 8b. Platform-dependent test failures

`auth::claude_code_creds::macos::*` tests are gated `#[cfg(target_os = "macos")]` and only run on macOS CI. Same for Windows. If you develop on macOS, the Windows tests don't run locally — that's fine; Task 13.1's CI matrix catches them.

### 8c. Unsigned app first-launch friction

`pnpm tauri dev` should just work on the dev machine. But if at any point you try to run the built `.app` or `.exe` artifact, Gatekeeper / SmartScreen will block it. That's expected — the README documents the workaround.

### 8d. Keyring prompts

On macOS, `keyring`-crate writes trigger Keychain approval prompts the first time. During development this can interrupt tests. If keyring saves fail repeatedly in tests, check that your tests use the fallback-file path (pass a `tempdir` as `fallback_dir`) rather than writing to the real keyring.

## Step 9: Hand back to the user when

You are empowered to make implementation choices, but hand back to the user (don't keep trying blindly) in any of these situations:

- **WebView2 missing on a Windows dev machine** — the app won't launch; need user to install or confirm auto-bootstrap.
- **OAuth paste-back failing against a real Claude account** — could mean Anthropic changed client_id handling or endpoint behavior; do not speculate, get user input with the exact error response body.
- **macOS Keychain prompts repeatedly refusing to stay approved** — usually caused by the binary path changing between runs; user may need to codesign locally or move the binary to a stable location.
- **Dependency resolution failures you can't fix in under two attempts** — don't burn 20 minutes bisecting semver; describe the conflict and ask.
- **Any urge to create a git branch or worktree** — the user's global `~/.claude/CLAUDE.md` forbids this without explicit confirmation. Always ask first.
- **Tests failing for a reason that seems tied to Claude API response shape changes** — don't silently adjust the parser; surface the diff between expected and actual response.

Hand back quickly rather than pushing through. A 30-second ask beats 30 minutes of thrashing.

## Step 10: Git policy

The global `~/.claude/CLAUDE.md` says: **do NOT create branches or worktrees without explicit user confirmation**. Commit everything to the current branch (whatever `git init` produces — likely `main`).

When in doubt, ask. Do not invoke any agent with `isolation: "worktree"`.

## Step 11: Known deferrals (don't implement these in v1)

From the plan's self-review:
- `db_reset` event emission (SQLite corruption recovery) — v1 just errors on corrupt DB
- One additional `ExtraUsageBar` test case — optional

Don't get clever and add them yourself. If you want to propose them, ask first.

## Step 12: Your first actions (in order)

1. Read the four docs listed in Step 1.
2. Run the inspection commands from Step 2.
3. Invoke `superpowers:subagent-driven-development` and `superpowers:dispatching-parallel-agents`.
4. Announce your plan: which tasks are Wave 0, who gets dispatched first, and your verification gate strategy.
5. Execute Wave 0 sequentially (the bootstrap).
6. Verify Wave 0 by running `pnpm build` and `cd src-tauri && cargo build` yourself.
7. Dispatch Wave 1 as three parallel subagents.
8. Continue wave by wave.

Do not skip straight to implementation. Do not dispatch parallel subagents until Wave 0 has landed and verified. Do not trust subagent self-reports without running the tests yourself.

## Success criteria

When you're done:
- `pnpm test` green, `cargo test --all-features` green on macOS + Windows + Ubuntu (CI matrix)
- `pnpm tauri dev` launches the app, tray icon appears, clicking it shows the popover, OAuth paste-back flow works end-to-end with a real Claude account
- Expanded report window opens from "See details", data lazy-loads per tab
- All 196 checkbox steps in the plan are checked or explicitly deviated from with reason
- Git log shows one conventional commit per task, matching the plan's commit messages

Report status at the end of every wave. Hand back to the user if blocked on anything listed in Step 9.

Begin.
```
