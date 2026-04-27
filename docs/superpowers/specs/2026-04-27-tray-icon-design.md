# Tray Icon Redesign — Donut Ring with C Mark

**Status:** Approved (2026-04-27)
**Scope:** Replace the four static tray icons (`idle-template.png`, `warn.png`, `danger.png`, `paused.png`) with a coherent donut-chart icon family. No code changes to `src-tauri/src/tray.rs` — the existing `pick()` switch is preserved.

## Why

The current `idle-template.png` is a black blob (a paint-splat shape) with no semantic relationship to "usage tracking". It does not read as a chart, does not read as Claude, and on Retina displays the single-resolution PNG looks soft. The new family communicates "percentage tracker for Claude" in the menu bar at a glance, matches the app's design tokens, and ships at three resolutions for crisp rendering at every display scale.

## Design

### Shape

A monoline donut chart, rendered at 22pt logical (the macOS menubar icon size), with a centered `C` glyph (Claude wordmark feel) inside the ring.

### Four state variants

The four files mirror the existing assets so `pick()` in `src-tauri/src/tray.rs` continues to work unchanged. The state mapping (idle <75%, warn 75–89%, danger ≥90%, paused signed-out) is unchanged.

| File | Stroke | Decorative arc fill | Center `C` | macOS template? |
|---|---|---|---|---|
| `idle-template.png` | full ring, foreground color | — | foreground | yes (auto-tints B/W) |
| `warn.png` | full ring, track gray | ~75% sweep, warn color | warm gray | no (colored) |
| `danger.png` | full ring, track gray | ~95% sweep, danger color | warm gray | no (colored) |
| `paused.png` | full ring, dashed | — | dimmed foreground | yes |

The "decorative arc fill" on warn and danger is intentional — it makes the icon read as "a chart that goes higher when usage is higher", even though the percentages displayed in the menubar title are the live source of truth. Idle and paused omit the arc because they have no percentage to communicate.

### Geometry

All measurements are given on a **22-unit authoring grid** (so a "stroke width of 2.5" means 2.5 grid units). The final raster is 44×44 px, so every grid unit = 2 px.

- **Outer ring:** circle centered at (11, 11), radius 9, stroke width 2.5.
- **Arc fill** (warn/danger): same circle, stroked over the track, starts at 12 o'clock (−90°) and sweeps clockwise. Warn = 270° sweep (75%). Danger = 342° sweep (95%). Round line caps.
- **Center `C` glyph:** SF Pro Bold, ~10 grid units tall, centered. The `C` opening faces right (standard orientation). Optical adjustment: shift the glyph baseline up ~0.5 grid units so it appears vertically centered within the donut.
- **Dashed paused stroke:** stroke-dasharray `2 2.5` (grid units), otherwise identical to the outer ring.

### Color tokens

Sourced from `src/styles/tokens.css` so the icon family stays in lockstep with the rest of the app's threshold colors.

| Role | Token | OKLCH value (from tokens.css) |
|---|---|---|
| Foreground (idle template) | foreground | rendered black/white by macOS — alpha only |
| Track (warn/danger ring background) | `--color-track` | `oklch(95% 0.02 65 / 0.20)` |
| Warn arc | `--color-warn` | `oklch(74% 0.16 55)` |
| Danger arc | `--color-danger` | `oklch(66% 0.20 25)` |
| Center `C` (warn/danger) | warm gray | `oklch(86% 0.02 65 / 0.78)` (text-secondary) |
| Center `C` (paused) | dimmed | foreground at ~45% opacity |

For the colored icons (warn, danger) we render the `C` in `--color-text-secondary` rather than the accent color — the arc carries the chromatic signal, the glyph stays neutral so the eye reads "this many percent" rather than "look at the letter".

### Resolution & file format

Tauri's tray API takes a single `iconPath` (or a single byte slice via `Image::from_bytes`) per state, not a multi-resolution asset. Two options to handle Retina:

1. Ship a single PNG sized for @2x (44×44) and let macOS downsample for non-Retina menubars. Simplest. Matches what most menubar apps actually ship.
2. Ship a macOS `.icns` packed format containing 22, 44, and 66 px variants. More work, marginally crisper at @1x.

**Decision: option 1.** Ship one **44×44 PNG per state**, replacing the existing files at the same paths. No source changes — `tray.rs` already loads these by path via `include_bytes!`.

(If QA later sees softness on @3x displays, revisit and switch to `.icns` or a multi-resolution path. Out of scope for this spec.)

## File changes

```
src-tauri/icons/tray/
  idle-template.png     ← replaced (44×44 PNG, template image)
  warn.png              ← replaced (44×44 PNG, color)
  danger.png            ← replaced (44×44 PNG, color)
  paused.png            ← replaced (44×44 PNG, template image)
```

No source changes to `src-tauri/src/tray.rs`. The four `include_bytes!` calls already point at these paths, and `set_icon_as_template(template)` already wires up the right behavior per file.

## Production approach

The icons are static raster outputs — no runtime generation. We author them as SVG (one master template, four variants), then export to PNG at 44×44.

**Authoring path:**
1. Hand-write the master SVG for each of the four states, parameterized only by stroke color, arc sweep, and dash pattern.
2. Render to 44×44 PNG via `rsvg-convert` or a similar headless rasterizer (any tool the implementation plan picks — implementer's choice).
3. Verify visually at 22pt and 44pt zoom against the macOS menubar in both light and dark modes.

The implementation plan will pick the rasterizer and pin the exact pixel-level placement.

## Out of scope

- Runtime icon generation (option B from brainstorm — the dynamic donut that fills with live percentage). Deferred indefinitely.
- Pre-rendered stepped variants (option C — pre-rendered 0/25/50/75/100 donuts). Deferred.
- Windows tray icon — Windows uses a different state-color signal (CLAUDE.md notes Windows has no title affordance) and the same icons should work, but the implementation plan will verify cross-platform rendering.
- Replacing the menubar title text. The "14% | 24%" title rendered by `tray.set_title()` stays exactly as it is.
- App icon (`src-tauri/icons/icon.png`, `icon.icns`, etc.). Out of scope — those are the dock/about icon, not the tray.

## Acceptance criteria

1. Four PNG files exist at the paths above, 44×44, with alpha.
2. Idle and paused icons render correctly as macOS template images (auto-tint to black on light menubar, white on dark menubar).
3. Warn and danger icons render in their respective threshold colors and remain identifiable in both light and dark menubars.
4. The `C` glyph is legible at 22pt on a Retina display (read from arm's length).
5. The donut shape is recognizable as "a percentage chart" without prior context.
6. `cargo build` passes — `tray.rs` is not modified, but `include_bytes!` re-checks the byte sources at build time.
7. Manual smoke test: launch the app, observe each of the four states (force them via the existing dev affordance or temporary code), confirm correct rendering in macOS light + dark menubar.
