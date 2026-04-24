# Designer Agent Handoff Prompt

Copy everything in the ```prompt``` block below and paste it into a fresh Claude Code session (or agent instance). It is fully self-contained and does not depend on our current conversation.

---

```prompt
You are being dispatched to design the UI system for a cross-platform Claude usage monitor app. The backend architecture is already designed and locked. Your job is **only** the visual + interaction design: a complete, shippable UI system that is simple, consistent, and genuinely standout beautiful — not generic AI-assistant slop.

## Read these first

1. The full design spec (already written, do not modify):
   `/Users/feixu/Developer/open Source/claude-usage-monitor/docs/superpowers/specs/2026-04-24-claude-usage-monitor-design.md`

   Read it end-to-end before proposing anything. Pay specific attention to:
   - Section 1 (Overview + requirements table)
   - Section 5 (Data flow — especially the two screens: CompactPopover and ExpandedReport)
   - Section 3 (File layout under `src/` — what components exist and what they do)

2. The seven competitor projects we surveyed are in `/Users/feixu/Developer/open Source/`. The two UI references most worth studying:
   - `claude-usage-bar/` (native SwiftUI, minimal data-dense bars — good reference for the compact tray view)
   - `ai-token-monitor/` (Tauri + React, full analytics dashboard — good reference for the expanded view, though its aesthetic is more utilitarian than what we want)

   Look at these; note what they do well and what feels generic or dated. We are explicitly trying to leapfrog both.

## Install the designer-skills

You have access to a collection of design skills from https://github.com/Owl-Listener/designer-skills.git. If they are not already installed in your environment, clone and install that repository first. Then check the skill list — you should see skills like `frontend-design`, `distill`, `polish`, `delight`, `animate`, `adapt`, `critique`, `extract`, `normalize`, `clarify`, `harden`, `teach-impeccable`, `svg-logo-designer`, and others.

Use the skills rigorously. In particular:

- `teach-impeccable` — run once at the start to establish design context for this project (save guidelines into this project's config)
- `frontend-design` — the core skill for producing the UI
- `distill` — ruthlessly cut complexity; nothing ships unless every element earns its place
- `normalize` — once the components exist, make sure they share one consistent visual system
- `polish` — final pass for alignment, spacing, micro-details
- `animate` — purposeful motion only; no animation for animation's sake
- `critique` — self-review before handing back

## Hard constraints (non-negotiable)

1. **Stack:** Tauri v2 + React 19 + TypeScript + Tailwind CSS v4 + Framer Motion. No CSS-in-JS libraries. No Material UI, Chakra, or prebuilt component libraries — the whole point is a distinctive custom system. Recharts is allowed for charts only.

2. **Cross-platform parity:** Everything must work and look identical on both macOS (14+) and Windows (10/11). Design once, render the same on both. Platform-specific enhancements are allowed only when graceful:
   - macOS: `NSVisualEffectView` vibrancy for popover background
   - Windows 11: Mica backdrop effect
   - Windows 10: translucent solid fallback
   - All other visuals: identical CSS across both.

3. **Two primary screens:**
   - **CompactPopover** — fixed ~360×420px, translucent, shows 5h bar + 7d bar + reset countdowns + tiny per-model breakdown + "See details" affordance. This is the default user experience. Must feel instant, calm, scannable in under 1 second.
   - **ExpandedReport** — separate resizable window (~960×640 default, min ~800×560), 6 tabs: Sessions, Models, Trends, Projects, Heatmap, Cache. Data-rich but still calm.
   - Plus **SettingsPanel** (in popover or expanded window, your call) and **AuthPanel** (first-run flow).

4. **Glassy / modern aesthetic.** Reference feel: Raycast, Linear, Arc Browser, 1Password 8, macOS Control Center. Specifically:
   - Translucent backgrounds with soft blur
   - Soft gradients on progress bars (not flat fills)
   - Generous whitespace, tight type
   - Monospace accents for numbers (JetBrains Mono or similar)
   - Spring-physics animations, never linear ease
   - Subtle depth via blur + light borders, not drop shadows
   - Light + dark mode both polished (system-aware by default)

5. **Simplicity and consistency over cleverness.** The user explicitly said "simplicity and consistency is the key." If a visual flourish doesn't serve a user need, it doesn't ship. Every color, radius, shadow, and animation should come from the same tight token set.

6. **No emojis in the UI.** Icons only — use Lucide (`lucide-react`) for all iconography for cross-platform consistency.

## Deliverables

Produce all of the following as concrete files in the project at `/Users/feixu/Developer/open Source/claude-usage-monitor/`:

### 1. Design tokens (`src/styles/tokens.css`)

A single CSS file exporting all design tokens as custom properties:
- Colors: semantic (bg, surface, surface-raised, border, text, text-muted, accent, warn, danger, success) for both light and dark
- Glass: blur radii, tint opacities, border colors for translucent surfaces
- Type scale: display / title / body / label / mono / micro with line-heights
- Spacing scale: 4px base, named steps (xs through 3xl)
- Radii: sharp through pill
- Motion: easing curves (spring presets), durations
- Shadows (minimal — prefer blur + border)

Tokens should be the only place these values are defined. Every component references tokens, never hard-coded values.

### 2. UI kit (`src/components/ui/`)

The minimal set needed for both screens — nothing more. Expected components:
- `Button` — primary / ghost / destructive variants
- `IconButton`
- `Card` / `GlassCard` (translucent variant)
- `ProgressBar` — the primary data-vis element; must handle 5h and 7d bucket rendering with color transitions at 75% and 90% thresholds
- `Tabs` — for expanded report
- `Popover` (or reuse Tauri's window as the popover surface)
- `Toggle` / `Switch`
- `Slider` (for threshold config)
- `Select`
- `Banner` — stale-data / auth-required / DB-reset
- `Badge` — status pills (Opus / Sonnet / Haiku, live / stale)
- `EmptyState` — used in tabs when no data

Every component ships with: types, Tailwind classes referencing tokens, dark-mode support, keyboard-accessible states, reduced-motion fallback.

### 3. Screen components (`src/popover/` and `src/report/`)

Implement all the screens described in the spec's file layout (Section 3). Wire them to placeholder data (not real IPC) so they can be previewed:
- `CompactPopover.tsx` + `UsageBar.tsx`
- `ExpandedReport.tsx` + all 6 tab components
- `SettingsPanel.tsx`
- `AuthPanel.tsx`

Each should be directly usable once backend IPC wrappers are plumbed in.

### 4. Animation patterns (`src/lib/motion.ts`)

Shared Framer Motion variants for:
- Popover mount/unmount (spring from tray anchor)
- Tab transitions (slide + fade)
- Number tickers (counting up on snapshot updates)
- Progress bar fills (spring, not linear)
- Stale-data pulse
- Threshold-crossed flash

All motion honors `prefers-reduced-motion`.

### 5. Icon set decision

Document which Lucide icons map to which semantic roles (e.g., `Activity` → polling, `AlertTriangle` → danger threshold, `RefreshCw` → stale). Put this in `src/lib/icons.ts` so usage is consistent.

### 6. Design system documentation (`docs/design-system.md`)

A concise reference doc covering:
- The visual language ("calm, premium, scannable" — one paragraph)
- Color system with usage guidance
- Type scale
- When to use each component variant
- Motion principles
- Screenshots or inline SVG mockups of the two primary screens

## Process

Work in this order — do not skip steps:

1. **Context** — `teach-impeccable` with this project. Save generated guidelines to project-local config.
2. **Brainstorm** — briefly explore 2–3 directional concepts for the compact popover (use the `frontend-design` skill for generation). Present them as small SVG or HTML mockups. Choose the strongest with a paragraph of reasoning.
3. **Tokens first** — build `tokens.css` before any components. Everything downstream references this.
4. **UI kit** — build components in isolation, each with a minimal self-contained preview (small Storybook-style harness acceptable but not required; simple test page is fine).
5. **Compact popover** — the hero screen. Iterate until it genuinely stands out. Apply `distill` to remove anything that doesn't earn its place. Apply `polish` for micro-details.
6. **Expanded report** — all 6 tabs. Apply `normalize` afterward to make sure they feel unified. Apply `adapt` to handle window resizing gracefully.
7. **Auth + Settings** — simpler but must feel part of the same system.
8. **Motion** — layer in animations with `animate`. Be restrained.
9. **Self-critique** — run `critique` on the complete system. Fix what it finds.
10. **Document** — write `docs/design-system.md`.

Present intermediate work for review at milestones 2, 5, 7, and 9. Do not do the whole thing silently and dump it at the end.

## Explicit non-goals

- No marketing site, no landing page — app UI only.
- No logo / brand identity — a simple monogram icon for the tray is enough.
- No onboarding carousel beyond the single AuthPanel first-run screen.
- No illustrations — the aesthetic is type + data + restrained color.
- No feature flag UI, no telemetry consent dialogs, no changelog viewer — out of scope for v1.

## Success criteria

When you're done, hand back:
1. All deliverable files committed to the project directory.
2. Two static mockup screenshots (or SVG exports) — one of CompactPopover, one of ExpandedReport — embedded in `docs/design-system.md`.
3. A one-paragraph summary of the design philosophy you settled on.
4. A short list of any decisions where you deviated from this prompt and why.

A reviewer should look at your work and immediately understand: "this is the same single system applied everywhere, and it's clearly better than the seven competitor apps."

Do not cut corners on consistency. Do not introduce aesthetics not grounded in the stated references. Do not generate generic dashboard UI. The user has explicitly flagged that simplicity + consistency is the differentiator — honor it.
```

---

## How to use this prompt

1. Open a new Claude Code session (separate from the one where the backend is being built).
2. First, install the designer-skills: `git clone https://github.com/Owl-Listener/designer-skills.git` and follow its install instructions.
3. Paste the prompt above as the opening message.
4. Review at each milestone (2, 5, 7, 9) — the prompt instructs the designer agent to stop for feedback.
5. Once the UI system is complete, the output can be merged into the main project and wired to real IPC when the backend implementation lands.
