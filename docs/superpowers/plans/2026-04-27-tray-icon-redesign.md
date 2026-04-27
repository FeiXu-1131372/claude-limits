# Tray Icon Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the four splat-shaped tray icons with a coherent monoline donut-chart family centered on a `C` mark, per `docs/superpowers/specs/2026-04-27-tray-icon-design.md`.

**Architecture:** Author four hand-written SVGs (one per state) committed under `src-tauri/icons/tray/sources/`, then rasterize each to a 44×44 PNG that drops in at the existing `src-tauri/icons/tray/*.png` paths. No Rust source changes — `tray.rs`'s `include_bytes!` calls already point at these paths.

**Tech Stack:** Hand-authored SVG, `rsvg-convert` (Homebrew librsvg) for rasterization. The `C` glyph is hand-drawn as an SVG arc path rather than a font glyph — keeps rasterization deterministic and avoids any fontconfig dependency.

---

## File Structure

**New:**
- `src-tauri/icons/tray/sources/idle-template.svg` — outline ring + C, monochrome (template)
- `src-tauri/icons/tray/sources/warn.svg` — track + 75% amber arc + C
- `src-tauri/icons/tray/sources/danger.svg` — track + 95% coral arc + C
- `src-tauri/icons/tray/sources/paused.svg` — dashed ring + dimmed C, monochrome (template)

**Modified (replaced bytes):**
- `src-tauri/icons/tray/idle-template.png`
- `src-tauri/icons/tray/warn.png`
- `src-tauri/icons/tray/danger.png`
- `src-tauri/icons/tray/paused.png`

**Untouched:**
- `src-tauri/src/tray.rs` — `pick()` switch and `include_bytes!` calls already point at the right paths and the right `set_icon_as_template(template)` flags.
- `src-tauri/tauri.conf.json` — `trayIcon.iconPath` already points at `idle-template.png`.

---

### Task 1: Set up rasterizer and source directory

**Files:** none modified.

- [ ] **Step 1: Check whether `rsvg-convert` is installed**

Run: `which rsvg-convert && rsvg-convert --version`
Expected: prints a path and a version like `rsvg-convert version 2.x.x`. On a fresh dev box this command will fail — that's expected; install librsvg in step 2.

- [ ] **Step 2: Install librsvg if missing**

Run: `brew install librsvg`
Expected: Homebrew installs librsvg and its dependencies. Re-run `rsvg-convert --version` to confirm; on macOS Sonoma+ via Homebrew you should see ≥ 2.57.

If you cannot use Homebrew, alternatives that produce equivalent output: `npx @neocoast/svg-to-png-cli`, `magick` (ImageMagick), or any rasterizer that supports SVG → 44×44 PNG with transparency. The rest of the plan assumes `rsvg-convert`; adapt the command in Task 4 if you use a different tool.

- [ ] **Step 3: Verify modern CSS color support**

Run: `rsvg-convert --version | head -1`
Expected: version 2.54 or newer. The SVGs in Tasks 2–3 use `oklch()` color functions (matching `src/styles/tokens.css`). Versions older than 2.54 will silently render OKLCH values as black.

If your version is older: in each colored SVG, replace the `oklch(...)` strings with the hex fallbacks documented inside the SVG comments (Task 3 includes them). The hex values are sRGB approximations and will be a ~1% perceptual color shift from the OKLCH source — acceptable for tray rendering.

- [ ] **Step 4: Create the SVG source directory**

Run: `mkdir -p src-tauri/icons/tray/sources`
Expected: directory created (no output if already present).

---

### Task 2: Author the idle-template SVG

**Files:**
- Create: `src-tauri/icons/tray/sources/idle-template.svg`

- [ ] **Step 1: Write the SVG**

Path: `src-tauri/icons/tray/sources/idle-template.svg`

```svg
<?xml version="1.0" encoding="UTF-8"?>
<!--
  idle-template.svg — Tray icon, idle state (<75% usage).
  macOS template image: alpha-only. macOS auto-tints to white on dark
  menubar, black on light menubar. Do NOT add color here — it will be
  flattened to the menubar foreground anyway, and any non-black ink
  produces unexpected alpha coverage.
-->
<svg xmlns="http://www.w3.org/2000/svg" width="44" height="44" viewBox="0 0 22 22">
  <!-- Outer ring: r=9, stroke 2.5 grid units -->
  <circle cx="11" cy="11" r="9" fill="none" stroke="#000000" stroke-width="2.5"/>
  <!-- C glyph: 270° arc opening to the right, radius 4.5, stroke 2.
       Arc starts at upper-right (14.7, 7.5) and sweeps CCW to lower-right (14.7, 14.5).
       large-arc-flag=1, sweep-flag=0 → counterclockwise long arc. -->
  <path d="M 14.7 7.5 A 4.5 4.5 0 1 0 14.7 14.5"
        fill="none" stroke="#000000" stroke-width="2" stroke-linecap="round"/>
</svg>
```

- [ ] **Step 2: Visually verify the SVG**

Run: `open src-tauri/icons/tray/sources/idle-template.svg`
Expected: Safari (or default SVG viewer) shows a black ring with a black `C` inside, opening to the right, on a transparent background. The `C` is centered horizontally and vertically inside the ring.

If the C is visibly off-center: the eye is sensitive to ~0.5 grid unit (~1px @44) miscentering of an asymmetric glyph like `C`. Nudge the path's start/end x coordinates by ±0.2 until it looks centered.

---

### Task 3: Author warn, danger, and paused SVGs

**Files:**
- Create: `src-tauri/icons/tray/sources/warn.svg`
- Create: `src-tauri/icons/tray/sources/danger.svg`
- Create: `src-tauri/icons/tray/sources/paused.svg`

Reference geometry (used in warn/danger arc fills):
- Circumference of r=9 circle = 2π × 9 ≈ 56.549
- Warn 75% sweep: filled length = 42.412, gap = 14.137
- Danger 95% sweep: filled length = 53.722, gap = 2.827
- Both apply `transform="rotate(-90 11 11)"` so the dasharray starts at 12 o'clock.

- [ ] **Step 1: Write `warn.svg`**

Path: `src-tauri/icons/tray/sources/warn.svg`

```svg
<?xml version="1.0" encoding="UTF-8"?>
<!--
  warn.svg — Tray icon, warn state (75–89% usage). NOT a template image.
  Track ring + 75%-sweep amber-orange arc + C glyph in warm gray.
  Hex fallbacks in comments are sRGB approximations of the tokens.css OKLCH values.
-->
<svg xmlns="http://www.w3.org/2000/svg" width="44" height="44" viewBox="0 0 22 22">
  <!-- Track ring: oklch(95% 0.02 65 / 0.20) ≈ rgba(244, 238, 233, 0.20) -->
  <circle cx="11" cy="11" r="9" fill="none"
          stroke="oklch(95% 0.02 65 / 0.20)" stroke-width="2.5"/>
  <!-- Warn arc: 75% sweep starting at 12 o'clock, oklch(74% 0.16 55) ≈ #E89149 -->
  <circle cx="11" cy="11" r="9" fill="none"
          stroke="oklch(74% 0.16 55)" stroke-width="2.5"
          stroke-dasharray="42.412 14.137"
          stroke-linecap="round"
          transform="rotate(-90 11 11)"/>
  <!-- C glyph in warm gray: oklch(86% 0.02 65 / 0.78) ≈ rgba(220, 213, 207, 0.78) -->
  <path d="M 14.7 7.5 A 4.5 4.5 0 1 0 14.7 14.5"
        fill="none" stroke="oklch(86% 0.02 65 / 0.78)"
        stroke-width="2" stroke-linecap="round"/>
</svg>
```

- [ ] **Step 2: Write `danger.svg`**

Path: `src-tauri/icons/tray/sources/danger.svg`

```svg
<?xml version="1.0" encoding="UTF-8"?>
<!--
  danger.svg — Tray icon, danger state (≥90% usage). NOT a template image.
  Track ring + 95%-sweep coral-red arc + C glyph in warm gray.
-->
<svg xmlns="http://www.w3.org/2000/svg" width="44" height="44" viewBox="0 0 22 22">
  <!-- Track ring -->
  <circle cx="11" cy="11" r="9" fill="none"
          stroke="oklch(95% 0.02 65 / 0.20)" stroke-width="2.5"/>
  <!-- Danger arc: 95% sweep, oklch(66% 0.20 25) ≈ #D85A45 -->
  <circle cx="11" cy="11" r="9" fill="none"
          stroke="oklch(66% 0.20 25)" stroke-width="2.5"
          stroke-dasharray="53.722 2.827"
          stroke-linecap="round"
          transform="rotate(-90 11 11)"/>
  <!-- C glyph in warm gray -->
  <path d="M 14.7 7.5 A 4.5 4.5 0 1 0 14.7 14.5"
        fill="none" stroke="oklch(86% 0.02 65 / 0.78)"
        stroke-width="2" stroke-linecap="round"/>
</svg>
```

- [ ] **Step 3: Write `paused.svg`**

Path: `src-tauri/icons/tray/sources/paused.svg`

```svg
<?xml version="1.0" encoding="UTF-8"?>
<!--
  paused.svg — Tray icon, signed-out state. macOS template image: alpha-only.
  Dashed ring + dimmed C (stroke-opacity 0.45 on the same black foreground).
-->
<svg xmlns="http://www.w3.org/2000/svg" width="44" height="44" viewBox="0 0 22 22">
  <!-- Dashed outer ring: dasharray "2 2.5" in grid units (4px on, 5px off @ 44px raster) -->
  <circle cx="11" cy="11" r="9" fill="none" stroke="#000000" stroke-width="2.5"
          stroke-dasharray="2 2.5"/>
  <!-- Dimmed C glyph at 45% alpha -->
  <path d="M 14.7 7.5 A 4.5 4.5 0 1 0 14.7 14.5"
        fill="none" stroke="#000000" stroke-opacity="0.45"
        stroke-width="2" stroke-linecap="round"/>
</svg>
```

- [ ] **Step 4: Visually verify the three SVGs**

Run:
```bash
open src-tauri/icons/tray/sources/warn.svg
open src-tauri/icons/tray/sources/danger.svg
open src-tauri/icons/tray/sources/paused.svg
```

Expected:
- **warn**: faint gray track ring + thick orange arc covering 3/4 of the circle (gap centered at the bottom). C in warm gray, centered.
- **danger**: faint gray track ring + coral-red arc covering 95% of the circle (small gap at the bottom). C in warm gray.
- **paused**: black dashed ring (8 dashes-ish around the circumference) + dim black C.

If any color renders as solid black: your `rsvg-convert` doesn't support OKLCH. Either upgrade librsvg (`brew upgrade librsvg`) or replace the OKLCH calls in the SVG with the hex equivalents listed in the comments above.

---

### Task 4: Rasterize SVGs to PNGs

**Files:**
- Modify (replace): `src-tauri/icons/tray/idle-template.png`
- Modify (replace): `src-tauri/icons/tray/warn.png`
- Modify (replace): `src-tauri/icons/tray/danger.png`
- Modify (replace): `src-tauri/icons/tray/paused.png`

- [ ] **Step 1: Render all four PNGs**

Run from the repo root:

```bash
for state in idle-template warn danger paused; do
  rsvg-convert -w 44 -h 44 \
    "src-tauri/icons/tray/sources/${state}.svg" \
    -o "src-tauri/icons/tray/${state}.png"
done
```

Expected: command produces no output (success) and creates four 44×44 PNGs at `src-tauri/icons/tray/{idle-template,warn,danger,paused}.png`, replacing the existing splat-icon files.

- [ ] **Step 2: Confirm the new files are larger than the old ones**

Run: `ls -la src-tauri/icons/tray/*.png`
Expected: each PNG is > 200 bytes (the original splat icons were 126–135 bytes). If any of the new files comes out smaller than the old, the rasterizer probably failed to capture color or strokes — rerun step 1 and check `rsvg-convert`'s exit status.

- [ ] **Step 3: Visually inspect each rendered PNG**

Run:
```bash
open src-tauri/icons/tray/idle-template.png
open src-tauri/icons/tray/warn.png
open src-tauri/icons/tray/danger.png
open src-tauri/icons/tray/paused.png
```

Expected: each PNG opens at 44×44 px showing the same artwork as the SVG, with a transparent background. The `C` should be readable.

---

### Task 5: Smoke test in macOS menu bar

**Files:** none modified by this task. Step 3 instructs a temporary edit to `src-tauri/src/tray.rs` that you must revert before committing.

- [ ] **Step 1: Build the app**

Run: `cd src-tauri && cargo build && cd -`
Expected: `Compiling … Finished` with no errors. The `include_bytes!` macro in `tray.rs` reads the new PNG bytes at compile time, so a successful build confirms the files are present and well-formed.

- [ ] **Step 2: Run the app and observe the idle state**

Run: `pnpm tauri dev`
Expected: the app launches and a tray icon appears in the macOS menubar showing the new monoline ring + `C`, replacing the previous splat. The "14% | 24%" title text alongside is unchanged (rendered separately by `tray.set_title()`).

Toggle System Settings → Appearance between Light and Dark. The idle template image should auto-tint: black ring on light menubar, white ring on dark menubar. If it stays black on dark menubar, `set_icon_as_template(true)` is not being honored — recheck that `idle-template.png` was rendered from `idle-template.svg` (which uses pure `#000` ink) and not accidentally from a colored SVG.

Stop the dev server (`Ctrl-C`) before continuing.

- [ ] **Step 3: Force the warn state via a temporary early-return**

This approach is more reliable than lowering thresholds, because it doesn't depend on real usage data being present.

Add a temporary early return at the top of `pick()` in `src-tauri/src/tray.rs:72-77`. Find:

```rust
fn pick(pct: Option<f64>, paused: bool) -> (&'static [u8], bool) {
    if paused {
        return (
            include_bytes!("../icons/tray/paused.png"),
            true,
        );
    }
```

Insert a smoke-test override above the existing `if paused` block:

```rust
fn pick(pct: Option<f64>, paused: bool) -> (&'static [u8], bool) {
    // TEMP smoke test — remove before commit
    return (include_bytes!("../icons/tray/warn.png"), false);

    if paused {
        return (
            include_bytes!("../icons/tray/paused.png"),
            true,
        );
    }
```

The compiler will warn about the now-unreachable code below — that's expected.

Run `pnpm tauri dev`. Expected: orange ~75% arc with track-gray remainder, neutral `C` in the middle, replacing whatever icon was showing.

Stop the dev server (`Ctrl-C`).

- [ ] **Step 4: Force the danger state**

Edit the same temp-override line to point at `danger.png`:

```rust
return (include_bytes!("../icons/tray/danger.png"), false);
```

Run `pnpm tauri dev`. Expected: coral-red ~95% arc with a tiny bottom gap, neutral `C` in the middle. Should look louder than warn but not clash with the system clock alongside.

If the colored icons render as a flat black silhouette: either `set_icon_as_template(false)` is being overridden somewhere (unlikely — the literal `false` flag in the override block makes that the only signal), OR the PNG itself was rendered with no color (rsvg-convert dropped OKLCH). Open `warn.png`/`danger.png` directly with `open` to rule out the second.

Stop the dev server.

- [ ] **Step 5: Force the paused state**

Edit the same temp-override line to point at `paused.png` and flip the template flag back to `true`:

```rust
return (include_bytes!("../icons/tray/paused.png"), true);
```

Run `pnpm tauri dev`. Expected: dashed ring with dimmed `C` in the menubar. Toggle System Settings → Appearance between Light and Dark — the icon should template-tint correctly (white-on-dark, black-on-light).

Stop the dev server.

- [ ] **Step 6: Revert the temporary debug edit**

Run:
```bash
git diff src-tauri/src/tray.rs
```

Expected: shows the temporary early-return line (added in steps 3–5). Revert it:

```bash
git checkout -- src-tauri/src/tray.rs
```

Then re-run `git diff src-tauri/src/tray.rs` to confirm clean (no output).

---

### Task 6: Commit

- [ ] **Step 1: Confirm only the intended files are staged**

Run:
```bash
git status --short src-tauri/icons/tray/
git diff src-tauri/src/tray.rs
```

Expected:
- `git status --short src-tauri/icons/tray/` shows 4 modified PNGs and 4 untracked SVGs in `sources/`.
- `git diff src-tauri/src/tray.rs` shows nothing (the temporary debug edits were reverted in Task 5 Step 5).

- [ ] **Step 2: Stage the icon assets**

Run:
```bash
git add src-tauri/icons/tray/sources/ \
        src-tauri/icons/tray/idle-template.png \
        src-tauri/icons/tray/warn.png \
        src-tauri/icons/tray/danger.png \
        src-tauri/icons/tray/paused.png
git diff --cached --stat
```

Expected stat: 8 files changed — 4 PNGs replaced (binary), 4 SVGs added.

- [ ] **Step 3: Commit**

Run:
```bash
git commit -m "$(cat <<'EOF'
feat(tray): replace splat icons with donut+C family

Four-state donut-ring tray icons (idle/warn/danger/paused) per
docs/superpowers/specs/2026-04-27-tray-icon-design.md. Authoring
SVGs live alongside the rendered PNGs; rasterized via rsvg-convert.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 4: Verify the commit**

Run: `git log -1 --stat`
Expected: one commit with 8 files (4 PNGs modified, 4 SVGs added).

---

## Acceptance Criteria (mirrors spec)

When all six tasks are complete:

1. ✅ Four PNGs exist at the four `src-tauri/icons/tray/*.png` paths, each 44×44 with alpha (Task 4).
2. ✅ Idle and paused icons template-tint correctly in light + dark menubars (Task 5 steps 2, 5).
3. ✅ Warn and danger icons render in their threshold colors (Task 5 steps 3, 4).
4. ✅ The `C` glyph is legible at 22pt on Retina (Task 5 — subjective check at full app size).
5. ✅ Donut shape reads as "a percentage chart" (Task 5 — subjective check).
6. ✅ `cargo build` passes (Task 5 step 1).
7. ✅ Manual smoke test of all four states completed (Task 5 steps 2–5).
