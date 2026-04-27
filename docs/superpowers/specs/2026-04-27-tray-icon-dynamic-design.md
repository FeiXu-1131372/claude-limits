# Tray Icon — Dynamic Dual-Pie Design

**Status:** Approved (2026-04-27)
**Supersedes:** `docs/superpowers/specs/2026-04-27-tray-icon-design.md` (static donut + C-mark — replaced before any commit landed)

## Why

The previous static design put a decorative `C` mark inside a single donut and kept the live percentages in the menubar title text. After implementation, that direction was rejected: the user wants the live numbers visible inside the icon itself, with a separate ring per usage bucket so each bucket's pressure is glanceable independently.

## What we're building

A tray icon that renders dynamically from the live (5-hour, 7-day) usage percentages. The macOS variant is a wide rectangular icon with two side-by-side pies, each pie showing one bucket's percentage as both ring fill and inscribed digits. The Windows variant is a square concentric design (one outer ring + one inner ring + one digit pair) because Windows tray icons must be square at 16/32 px.

The menubar title text (`tray.set_title()`) — currently "14% | 24%" — is removed. Numbers now live exclusively in the icon.

## Design decisions

The brainstorm pinned these. Not up for debate without re-spec.

| Decision | Choice | Rationale |
|---|---|---|
| Static vs dynamic | Dynamic | User wants live numbers in the icon |
| Single vs dual indicator | Dual (one pie per bucket) | Each bucket's pressure glanceable independently |
| Layout (macOS) | Side-by-side, ~44pt × 22pt | Two distinct numbers need dedicated space; concentric loses one |
| Title text | Removed | Avoids redundancy now that numbers are in the icon |
| Ring fill direction | Fills as percentage **used** | Matches battery / progress UI convention; ring + number rise together |
| Per-pie threshold colors | Each pie colored by its own threshold | A 5h-warn + 7d-safe state must visibly differ from 5h-safe + 7d-warn |
| Cross-platform | Different icon on Windows | Square 32×32 cannot fit two side-by-side pies legibly |
| ≥99% display | Clamps to "99" + full danger color | Three-digit "100" doesn't fit; 99 vs 100 is a distinction without a difference at this point |
| Tooltip | Kept ("Claude usage — 5h X%, 7d Y%") | Hover gives exact numbers for accessibility / verification |

## Visual specification — macOS

**Canvas:** 88 × 44 px (rectangular, ~44pt × 22pt logical at 2x).

**Layout:** two 44 × 44 cells side by side. Each cell contains one pie.

**Per-cell geometry (in 44-px cell space, with the same 22-unit authoring grid as the prior design — 1 grid unit = 2 px):**

- **Track ring:** circle at (cell-center 11, 11), radius 9, stroke 2.5 (grid units). Color: `oklch(95% 0.02 65 / 0.20)` (track gray, 20% alpha).
- **Arc fill:** same circle, stroked over the track. Sweep starts at 12 o'clock and goes clockwise. Sweep length = `pct/100 × 360°`. Stroke width 2.5. Round line caps. Color depends on threshold (see below).
- **Digits:** two-digit number (or `—` if data not loaded), centered horizontally and vertically inside the ring. ~10 grid units (20 px) tall. Stroke / fill: `oklch(96% 0.01 65 / 0.96)` (text-primary).
- **Bucket label:** `5h` for the left pie, `7d` for the right pie. Tiny — ~3 grid units tall, placed beneath the pie or just inside the bottom of the ring (renderer's call within ~1 grid unit). Color: `oklch(78% 0.025 65 / 0.62)` (text-muted).

**Per-pie threshold colors** (each pie colored independently by its own bucket's percentage):

| Bucket value | Arc color token | Approx hex |
|---|---|---|
| < 75% | `--color-accent` (terracotta) | `#D97757`-ish |
| 75–89% | `--color-warn` (amber) | `#E89149` |
| ≥ 90% | `--color-danger` (coral) | `#D85A45` |

(Exact OKLCH values from `src/styles/tokens.css`. The renderer converts OKLCH → sRGB at compile time and uses sRGB at runtime — `tiny-skia` doesn't do CSS color parsing.)

**State coverage** (handled inside the renderer, not via fallback PNGs):

| State | Visual |
|---|---|
| Both percentages present | Dual-pie with digits, full color |
| One missing | Present pie renders normally; missing pie shows track-only ring with `—` (em-dash) instead of digits |
| Both missing or `paused=true` | Both cells render as track-only rings with `—` digits, all elements at ~50% alpha (signals "no data / signed out") |
| Bucket value ≥ 99% | Digits show as `99` (no rounding up to 100). Arc renders 100% sweep (full ring). Color from the danger threshold. |

The renderer always produces an 88 × 44 PNG — never returns `None`. There is no separate "idle" PNG.

## Visual specification — Windows

**Canvas:** 32 × 32 px.

**Layout:** concentric, Apple-Watch-activity-rings style.

- **Outer track ring:** radius 14, stroke 3. Track gray.
- **Outer arc:** 7-day usage. Color from 7d's threshold.
- **Inner track ring:** radius 9, stroke 2.5. Track gray.
- **Inner arc:** 5-hour usage. Color from 5h's threshold.
- **Center digits:** the **worse** (higher) of the two percentages, two digits, ~9 px tall.
- **No bucket label** — space won't allow.

A user reading just the Windows icon sees one number (the worst case) plus two ring positions giving a coarse read on which bucket is hotter. Less information than macOS, but the best fit for 32×32.

(The Windows tooltip carries the explicit "5h X% · 7d Y%" — same as before.)

## Architecture

**New module tree** under `src-tauri/src/`:

```
tray_icon/
├── mod.rs       — public entry point: render(five_hour, seven_day, paused) -> Vec<u8>
│                  Compile-time platform dispatch via #[cfg].
├── shared.rs    — color tokens (sRGB-resolved from CLAUDE.md OKLCH at compile time),
│                  threshold→color logic, common geometry constants.
├── digits.rs    — hand-converted glyph paths for digits 0-9 and the em-dash, embedded
│                  as tiny_skia path constants. Sized for the 22-grid cell.
├── macos.rs     — #[cfg(macos)] 88×44 dual-pie renderer.
└── windows.rs   — #[cfg(windows)] 32×32 concentric renderer.
```

`mod.rs` exposes:

```rust
pub fn render(five_hour: Option<f64>, seven_day: Option<f64>, paused: bool) -> Vec<u8>;
```

That signature is identical on both platforms (the `#[cfg]` selects which file's `render` is compiled). Returns PNG bytes.

**Modified module:** `src-tauri/src/tray.rs`. The existing `pick()` and the four PNG `include_bytes!` calls are deleted. `set_level()` becomes:

```rust
pub fn set_level(app: &AppHandle, five_hour: Option<f64>, seven_day: Option<f64>, paused: bool) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let bytes = tray_icon::render(five_hour, seven_day, paused);
    let _ = tray.set_icon(Some(Image::from_bytes(&bytes).expect("icon bytes")));
    let _ = tray.set_icon_as_template(false);
    let _ = tray.set_title(None);
    let _ = tray.set_tooltip(Some(tooltip(five_hour, seven_day, paused)));
}

fn tooltip(five_hour: Option<f64>, seven_day: Option<f64>, paused: bool) -> String {
    // unchanged from current behavior
}
```

**Dependencies added to `src-tauri/Cargo.toml`:**

- `tiny-skia = "0.11"` — pure-Rust 2D rasterization. No system fonts, no fontconfig.
- `png = "0.17"` — encoding the canvas to PNG bytes. (Already pulled in transitively by Tauri's `image-png` feature; we use it directly.)

No new transitive risk — `tiny-skia` is widely used (Servo, fontdrop, several Tauri example apps) and pure-Rust.

## Performance

Rendering an 88×44 PNG with tiny-skia takes well under a millisecond on Apple Silicon. The poll loop fires roughly once per minute. No caching, debouncing, or change-detection needed — `set_level` synthesizes fresh bytes on each call. KISS.

## Removed assets

These four files are deleted as part of this work:

```
src-tauri/icons/tray/idle-template.png
src-tauri/icons/tray/warn.png
src-tauri/icons/tray/danger.png
src-tauri/icons/tray/paused.png
```

The `trayIcon.iconPath` in `src-tauri/tauri.conf.json` currently points at `icons/tray/idle-template.png`. Tauri requires *something* there at startup to register the tray, before our renderer takes over on the first `set_level` call. Two options:

1. Keep one minimal placeholder PNG (e.g., a transparent 44×44) at the `iconPath` only for boot. Replace bytes via `set_icon` immediately on app ready.
2. Generate the boot icon at build time via a `build.rs` step and emit it to `OUT_DIR` / a known path.

Decision: option 1, with a transparent 44×44 placeholder. Simplest. The placeholder is replaced within milliseconds of app startup by the first `set_level(None, None, paused=true)` call.

The clean-up also reverts the previous brainstorm's uncommitted work — the 4 SVGs in `src-tauri/icons/tray/sources/` and the 4 modified PNGs are abandoned. `git checkout -- src-tauri/icons/tray/*.png` and `rm -rf src-tauri/icons/tray/sources/` before deleting the four PNGs entirely.

## Out of scope

- Animation / transitions when the percentage changes (pie fills don't animate; just snap to new value).
- Number formatting beyond two digits (no decimals, no `%` symbol — implied).
- Per-bucket sparklines or trend indicators.
- Light vs dark mode auto-tinting — colored icons stay colored, the threshold colors have enough contrast on both menubar modes.
- Cross-platform parity at 100% — Windows shows less info than macOS by design.

## Acceptance criteria

1. `tiny-skia` and (if not transitively present) `png` are added to `src-tauri/Cargo.toml`.
2. The new `tray_icon` module tree exists with the file structure above.
3. `cargo build` passes on macOS. `cargo build --target=x86_64-pc-windows-gnu` (or equivalent Windows-target) is not required for this change to ship — the Windows path compiles when built on Windows but the macOS dev environment can ignore it.
4. The four old static PNGs are deleted from the repo. The previous brainstorm's `sources/` SVGs are deleted.
5. `tauri.conf.json` `iconPath` points at a transparent 44×44 placeholder PNG (added once, then never re-rendered — it's only the boot artifact).
6. `tray.rs::set_level` no longer references `pick()` or `include_bytes!`. The diff for `tray.rs` is contained — the function signature and call sites are unchanged.
7. On launch on macOS: the menubar shows two side-by-side pies with two-digit percentages inside, no title text alongside, threshold-colored arcs that fill clockwise from 12 o'clock in proportion to each bucket.
8. Hover tooltip still reads "Claude usage — 5h X%, 7d Y%" (unchanged behavior).
9. Sign out → both pies render in the muted "no-data" state with em-dash digits.
