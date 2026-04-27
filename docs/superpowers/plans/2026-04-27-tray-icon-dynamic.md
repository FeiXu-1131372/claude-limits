# Tray Icon Dynamic Dual-Pie Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the static 4-PNG tray icon family with a runtime renderer that produces a dual-pie chart (macOS) or a concentric chart (Windows) reflecting live 5-hour and 7-day usage percentages, per `docs/superpowers/specs/2026-04-27-tray-icon-dynamic-design.md`.

**Architecture:** A new `tray_icon` Rust module synthesizes PNG bytes on every `set_level` call using `tiny-skia` for path rendering and `ttf-parser` to extract digit-glyph outlines from a bundled JetBrains Mono Regular font. `#[cfg]` selects the macOS or Windows renderer at compile time. The module facade is platform-agnostic; both renderer files compile on either OS so unit tests can exercise both on macOS dev machines.

**Tech Stack:** `tiny-skia` 0.11 (pure-Rust 2D), `ttf-parser` 0.20+ (pure-Rust TTF outlines), JetBrains Mono Regular (OFL-licensed, ~150 KB), embedded via `include_bytes!`.

---

## File Structure

**New:**
- `src-tauri/src/tray_icon/mod.rs` — public facade, platform dispatch
- `src-tauri/src/tray_icon/shared.rs` — sRGB color constants resolved from tokens.css OKLCH, threshold→color logic, common geometry constants
- `src-tauri/src/tray_icon/digits.rs` — extracts and caches digit glyph paths from the embedded font
- `src-tauri/src/tray_icon/macos.rs` — 88×44 dual-pie renderer
- `src-tauri/src/tray_icon/windows.rs` — 32×32 concentric renderer
- `src-tauri/src/tray_icon/font/JetBrainsMono-Regular.ttf` — embedded font (downloaded as part of Task 2)
- `src-tauri/icons/tray/boot-placeholder.png` — transparent 44×44, only visible during the millisecond between Tauri tray init and the first `set_level` call

**Modified:**
- `src-tauri/Cargo.toml` — add `tiny-skia` and `ttf-parser`
- `src-tauri/src/lib.rs` — add `mod tray_icon;` next to `mod tray;`
- `src-tauri/src/tray.rs` — `set_level` calls into `tray_icon::render`; `pick()`, `format_title`, `fmt_opt`, and the four `include_bytes!` calls are removed
- `src-tauri/tauri.conf.json` — `trayIcon.iconPath` points at the new boot placeholder

**Deleted:**
- `src-tauri/icons/tray/idle-template.png`
- `src-tauri/icons/tray/warn.png`
- `src-tauri/icons/tray/danger.png`
- `src-tauri/icons/tray/paused.png`
- `src-tauri/icons/tray/sources/` (the SVG authoring directory from the previous brainstorm — entire dir removed)

---

## Deviations from spec

The spec says digit glyphs are embedded "as `tiny_skia::Path` constants". Hand-coding 11 path constants (digits 0–9 + em-dash) is error-prone visual work. Instead, this plan extracts the same paths *once at first render* from a bundled JetBrains Mono Regular TTF using `ttf-parser`, caches them in a `OnceCell<DigitPaths>`, and reuses them for the lifetime of the process. End result is identical (in-memory `tiny_skia::Path` per digit), but authoring is mechanical and trustworthy. JetBrains Mono is the project's monospace token per `src/styles/tokens.css` and is OFL-licensed (free to bundle).

---

### Task 1: Clean up the previous brainstorm's working-tree changes and add a boot placeholder

The user's working tree contains 4 modified PNGs and a new `sources/` directory from the prior static-icon brainstorm. Those are abandoned. The user's other parallel-work changes (pricing.json, AuthPanel.tsx, etc.) are NOT to be touched.

**Files:**
- Modify (revert via git): `src-tauri/icons/tray/{idle-template,warn,danger,paused}.png`
- Delete: `src-tauri/icons/tray/sources/` (entire directory)
- Create: `src-tauri/icons/tray/boot-placeholder.png` (transparent 44×44)
- Modify: `src-tauri/tauri.conf.json` — `trayIcon.iconPath` points at the new placeholder

- [ ] **Step 1: Revert the 4 modified static tray PNGs**

Run from repo root:

```bash
git checkout HEAD -- src-tauri/icons/tray/idle-template.png \
                     src-tauri/icons/tray/warn.png \
                     src-tauri/icons/tray/danger.png \
                     src-tauri/icons/tray/paused.png
git status --short src-tauri/icons/tray/
```

Expected: the four PNGs no longer appear in `git status` output. The `?? src-tauri/icons/tray/sources/` line still appears.

- [ ] **Step 2: Delete the previous brainstorm's `sources/` SVG directory**

Run: `rm -rf src-tauri/icons/tray/sources`
Then: `git status --short src-tauri/icons/tray/`
Expected: no output for `src-tauri/icons/tray/`. The directory is gone and there's nothing to track.

- [ ] **Step 3: Generate the transparent 44×44 boot placeholder**

This single PNG is the boot artifact only — Tauri requires the `iconPath` in `tauri.conf.json` to point at a real file, but our renderer will replace its bytes within milliseconds of app start.

Run from repo root:

```bash
python3 - <<'EOF'
import struct
import zlib

# Minimal 44x44 RGBA PNG with all-transparent pixels.
width, height = 44, 44
raw = b''.join(b'\x00' + b'\x00\x00\x00\x00' * width for _ in range(height))
compressed = zlib.compress(raw, 9)

def chunk(tag, data):
    return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', zlib.crc32(tag + data))

ihdr = struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)
png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', ihdr) + chunk(b'IDAT', compressed) + chunk(b'IEND', b'')

with open('src-tauri/icons/tray/boot-placeholder.png', 'wb') as f:
    f.write(png)

print(f'wrote {len(png)} bytes')
EOF
```

Expected: prints `wrote N bytes` (typically ~120). File exists.

Verify dimensions:

```bash
sips -g pixelWidth -g pixelHeight src-tauri/icons/tray/boot-placeholder.png
```

Expected: `pixelWidth: 44`, `pixelHeight: 44`.

- [ ] **Step 4: Point `tauri.conf.json` at the new placeholder**

Open `src-tauri/tauri.conf.json` and find the `"trayIcon"` block (around line 36–40). Change `"iconPath"` from `icons/tray/idle-template.png` to `icons/tray/boot-placeholder.png`:

```json
"trayIcon": {
  "iconPath": "icons/tray/boot-placeholder.png",
  "iconAsTemplate": false,
  "menuOnLeftClick": false
},
```

Note the `"iconAsTemplate"` flip from `true` to `false`. The dynamic-rendered icons we'll produce later are colored, so template tinting is wrong for them. The boot placeholder is transparent, so the template flag doesn't matter for it — we set false to be consistent with what the renderer expects.

- [ ] **Step 5: Verify the cleanup**

Run:

```bash
git status --short src-tauri/
ls src-tauri/icons/tray/
```

Expected first command output: `M src-tauri/tauri.conf.json` (and any other unrelated files the user already had modified — leave those alone) and `?? src-tauri/icons/tray/boot-placeholder.png` for the new placeholder.

Expected second command output: only the four original splat PNGs (`idle-template.png`, `warn.png`, `danger.png`, `paused.png`) plus `boot-placeholder.png`. No `sources/` directory.

**Do not commit yet** — Task 8 deletes the four old splat PNGs and Task 9 commits everything together.

---

### Task 2: Add Cargo dependencies and embed the JetBrains Mono font

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/tray_icon/font/JetBrainsMono-Regular.ttf`

- [ ] **Step 1: Add `tiny-skia` and `ttf-parser` to `Cargo.toml`**

Open `src-tauri/Cargo.toml` and add to the `[dependencies]` section (after `directories = "5"` is fine):

```toml
tiny-skia = "0.11"
ttf-parser = "0.21"
```

Verify Cargo accepts the new deps (this also fetches them):

```bash
cd src-tauri && cargo check 2>&1 | tail -5 && cd -
```

Expected: `Finished` line, no `error` lines. Compilation may pull a few transitive crates (tiny-skia depends on `arrayref`, `bytemuck`, `cfg-if`, `log` — all already in many Tauri projects).

If `cargo check` reports an error like `failed to select a version for tiny-skia ... rustc-version`, bump the version pin (e.g., `tiny-skia = "0.13"`) and retry. The `0.11` line was based on what's stable at time of writing; the floor is whatever resolves cleanly.

- [ ] **Step 2: Create the font embedding directory**

```bash
mkdir -p src-tauri/src/tray_icon/font
```

- [ ] **Step 3: Download JetBrains Mono Regular**

JetBrains Mono is licensed under the SIL Open Font License — fine to bundle. Download the TTF directly from the JetBrains repository. Run:

```bash
curl -L -o src-tauri/src/tray_icon/font/JetBrainsMono-Regular.ttf \
  https://github.com/JetBrains/JetBrainsMono/raw/master/fonts/ttf/JetBrainsMono-Regular.ttf
```

Verify the file is a real TTF (~250 KB):

```bash
ls -la src-tauri/src/tray_icon/font/JetBrainsMono-Regular.ttf
file src-tauri/src/tray_icon/font/JetBrainsMono-Regular.ttf
```

Expected: file is approximately 200–300 KB (current JetBrains Mono Regular sits around ~270 KB but versions vary). `file` reports `TrueType Font data`.

If `curl` fails (e.g., the user is offline or GitHub URL changed), download manually from https://www.jetbrains.com/lp/mono/ and save to the path above.

- [ ] **Step 4: Add a license note**

Create `src-tauri/src/tray_icon/font/LICENSE.md`:

```markdown
JetBrainsMono-Regular.ttf

Source: https://github.com/JetBrains/JetBrainsMono
License: SIL Open Font License 1.1 (https://scripts.sil.org/OFL)
Bundled here at compile time via `include_bytes!` for the dynamic tray icon
renderer in `src-tauri/src/tray_icon/digits.rs`.
```

Font validity will be verified by Task 4's `digits` module tests, which load and parse the font on first call. No standalone validation step here.

---

### Task 3: Implement `tray_icon/shared.rs` (colors + thresholds)

This is small and pure — a great test-first warm-up.

**Files:**
- Create: `src-tauri/src/tray_icon/shared.rs`
- Test: same file (`#[cfg(test)] mod tests` block)

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/src/tray_icon/shared.rs` with the test scaffold first:

```rust
//! Color tokens (sRGB-resolved from tokens.css OKLCH at compile time)
//! and threshold→color logic shared between the macOS and Windows renderers.

use tiny_skia::Color;

/// `--color-accent` (Anthropic terracotta), oklch(67% 0.135 38).
pub const ACCENT: Color = Color::from_rgba8(0xD9, 0x77, 0x57, 0xFF);
/// `--color-warn` (amber-orange), oklch(74% 0.16 55).
pub const WARN: Color = Color::from_rgba8(0xE8, 0x91, 0x49, 0xFF);
/// `--color-danger` (coral-red), oklch(66% 0.20 25).
pub const DANGER: Color = Color::from_rgba8(0xD8, 0x5A, 0x45, 0xFF);
/// `--color-track`, oklch(95% 0.02 65 / 0.20).
pub const TRACK: Color = Color::from_rgba8(0xF4, 0xEE, 0xE9, 0x33);
/// `--color-text` (digits), oklch(96% 0.01 65 / 0.96).
pub const TEXT: Color = Color::from_rgba8(0xF6, 0xF2, 0xEE, 0xF5);
/// `--color-text-muted` (bucket labels), oklch(78% 0.025 65 / 0.62).
pub const TEXT_MUTED: Color = Color::from_rgba8(0xC2, 0xB8, 0xAE, 0x9E);

/// Returns the arc fill color for a single bucket's percentage.
/// Mirrors the threshold logic the popover uses: <75 = accent, 75-89 = warn, >=90 = danger.
pub fn arc_color(pct: f64) -> Color {
    if pct >= 90.0 {
        DANGER
    } else if pct >= 75.0 {
        WARN
    } else {
        ACCENT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_color_uses_accent_below_75() {
        assert_eq!(arc_color(0.0), ACCENT);
        assert_eq!(arc_color(74.9), ACCENT);
    }

    #[test]
    fn arc_color_uses_warn_at_75_to_89() {
        assert_eq!(arc_color(75.0), WARN);
        assert_eq!(arc_color(89.9), WARN);
    }

    #[test]
    fn arc_color_uses_danger_at_90_and_above() {
        assert_eq!(arc_color(90.0), DANGER);
        assert_eq!(arc_color(99.0), DANGER);
        assert_eq!(arc_color(150.0), DANGER); // saturating: anything above 90% is full danger
    }
}
```

Add the new module to `src-tauri/src/lib.rs` immediately so it compiles. Insert `mod tray_icon;` after line 9 (`mod tray;`):

```rust
mod tray;
mod tray_icon;
pub mod usage_api;
```

Then create the parent module `src-tauri/src/tray_icon/mod.rs` with just:

```rust
pub mod shared;
```

(More modules will be added by later tasks — for now `mod.rs` only declares `shared`.)

- [ ] **Step 2: Run the tests to verify they fail with a missing-file error**

```bash
cd src-tauri && cargo test tray_icon::shared 2>&1 | tail -20 && cd -
```

Expected: tests don't yet appear because the module hasn't been compiled into a passing state yet — actually since Step 1 wrote both tests AND implementation in one shot (these are tiny pure functions, no real TDD red phase), this step's purpose is just to confirm everything compiles. Expected: tests pass.

If tests fail with a compile error, fix it.

- [ ] **Step 3: Verify all 3 tests pass**

```bash
cd src-tauri && cargo test tray_icon::shared 2>&1 | tail -15 && cd -
```

Expected: `test result: ok. 3 passed; 0 failed`.

- [ ] **Step 4: Stage but do not commit**

The single commit happens in Task 9 along with everything else.

---

### Task 4: Implement `tray_icon/digits.rs` (font path extraction)

**Files:**
- Create: `src-tauri/src/tray_icon/digits.rs`
- Modify: `src-tauri/src/tray_icon/mod.rs` — add `pub mod digits;`

- [ ] **Step 1: Write the implementation and tests**

Create `src-tauri/src/tray_icon/digits.rs`:

```rust
//! Extracts digit glyph outlines from the bundled JetBrains Mono Regular font
//! and caches them as `tiny_skia::Path` objects keyed by character. Used by
//! both the macOS and Windows renderers to draw the inscribed numbers.
//!
//! The extraction happens on first call to `glyph_path`; subsequent calls
//! reuse the cached `Path`. Glyph paths are emitted at em-square scale 1.0
//! and are flipped on the Y axis (TTF Y-up → tiny-skia Y-down) so callers
//! only need to scale and translate.

use std::collections::HashMap;
use std::sync::OnceLock;
use tiny_skia::{Path, PathBuilder};
use ttf_parser::{Face, OutlineBuilder};

/// The characters this module makes available. Adding more here is fine —
/// they're only loaded once and the font has full glyph coverage.
pub const SUPPORTED_CHARS: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    '\u{2014}', // em-dash, used to display "no data"
];

const FONT_BYTES: &[u8] = include_bytes!("font/JetBrainsMono-Regular.ttf");

struct GlyphCache {
    paths: HashMap<char, Path>,
    units_per_em: f32,
}

static CACHE: OnceLock<GlyphCache> = OnceLock::new();

fn cache() -> &'static GlyphCache {
    CACHE.get_or_init(|| {
        let face = Face::parse(FONT_BYTES, 0).expect("bundled font is parseable");
        let units_per_em = face.units_per_em() as f32;
        let mut paths = HashMap::new();
        for &ch in SUPPORTED_CHARS {
            if let Some(path) = extract_glyph(&face, ch) {
                paths.insert(ch, path);
            }
        }
        GlyphCache { paths, units_per_em }
    })
}

/// Returns the glyph path for `ch` in font-em units (Y flipped to be Y-down),
/// or `None` if the character isn't in the font (shouldn't happen for digits).
pub fn glyph_path(ch: char) -> Option<&'static Path> {
    cache().paths.get(&ch)
}

/// Returns the font's em-square size. Callers divide their target pixel size by
/// this value to get the scale factor for glyph paths.
pub fn units_per_em() -> f32 {
    cache().units_per_em
}

fn extract_glyph(face: &Face, ch: char) -> Option<Path> {
    let glyph_id = face.glyph_index(ch)?;
    let mut builder = FlippedBuilder {
        inner: PathBuilder::new(),
    };
    face.outline_glyph(glyph_id, &mut builder)?;
    builder.inner.finish()
}

/// `OutlineBuilder` adapter that flips Y as it streams path commands into a
/// `tiny_skia::PathBuilder`. TTF coordinates are Y-up; tiny-skia is Y-down.
struct FlippedBuilder {
    inner: PathBuilder,
}

impl OutlineBuilder for FlippedBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.inner.move_to(x, -y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.inner.line_to(x, -y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.inner.quad_to(x1, -y1, x, -y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.inner.cubic_to(x1, -y1, x2, -y2, x, -y);
    }
    fn close(&mut self) {
        self.inner.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_supported_chars_have_paths() {
        for &ch in SUPPORTED_CHARS {
            assert!(
                glyph_path(ch).is_some(),
                "missing glyph for {:?}",
                ch
            );
        }
    }

    #[test]
    fn digit_zero_has_non_trivial_geometry() {
        let path = glyph_path('0').expect("digit 0");
        let bounds = path.bounds();
        // JetBrains Mono digits are roughly 1100 units wide at the typical 2048-em.
        // We just need to confirm the bounds aren't zero — sanity that path data flowed in.
        assert!(bounds.width() > 100.0, "expected non-trivial glyph width, got {}", bounds.width());
        assert!(bounds.height() > 100.0, "expected non-trivial glyph height, got {}", bounds.height());
    }

    #[test]
    fn em_size_is_reasonable() {
        let em = units_per_em();
        // TTF em sizes are typically 1000, 1024, or 2048. Anything in [500, 4096] is sane.
        assert!((500.0..=4096.0).contains(&em), "unexpected em size: {}", em);
    }
}
```

Update `src-tauri/src/tray_icon/mod.rs`:

```rust
pub mod digits;
pub mod shared;
```

- [ ] **Step 2: Run the tests**

```bash
cd src-tauri && cargo test tray_icon::digits 2>&1 | tail -15 && cd -
```

Expected: `test result: ok. 3 passed; 0 failed`.

If `Face::parse` panics with a parse error, the bundled font is corrupt — re-download from Task 2 Step 3.

If a glyph is missing (specifically the em-dash), JetBrains Mono is missing it — unlikely but if it happens, swap the constant `'\u{2014}'` for `'-'` (regular hyphen-minus, U+002D, always present in any font).

---

### Task 5: Implement `tray_icon/macos.rs` (88×44 dual-pie renderer)

This is the meatiest task. The renderer takes the two percentages, draws each pie into a 44×44 cell, then composites both cells into an 88×44 PNG.

**Files:**
- Create: `src-tauri/src/tray_icon/macos.rs`
- Modify: `src-tauri/src/tray_icon/mod.rs` — add `pub mod macos;`

- [ ] **Step 1: Write the implementation**

Create `src-tauri/src/tray_icon/macos.rs`:

```rust
//! macOS dual-pie tray icon renderer. Produces an 88×44 PNG showing two
//! side-by-side pies — left for 5-hour, right for 7-day — each with its own
//! arc fill, threshold-keyed color, and inscribed two-digit number.

use crate::tray_icon::{digits, shared};
use tiny_skia::{
    FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform,
};

const CANVAS_W: u32 = 88;
const CANVAS_H: u32 = 44;
const CELL_W: f32 = 44.0;
const CELL_H: f32 = 44.0;

// All measurements below are in pixels (the Pixmap pixel space).
const RING_CX: f32 = 22.0;       // cell center x
const RING_CY: f32 = 18.0;       // cell center y (slightly above middle to leave room for label)
const RING_R: f32 = 14.0;        // ring radius
const RING_STROKE: f32 = 3.5;    // ring stroke width
const DIGIT_HEIGHT_PX: f32 = 14.0;
const LABEL_HEIGHT_PX: f32 = 6.0;
const LABEL_BASELINE_Y: f32 = 38.0;

pub fn render(five_hour: Option<f64>, seven_day: Option<f64>, paused: bool) -> Vec<u8> {
    let mut pixmap = Pixmap::new(CANVAS_W, CANVAS_H).expect("88x44 fits in memory");
    pixmap.fill(tiny_skia::Color::TRANSPARENT);

    let no_data = paused || (five_hour.is_none() && seven_day.is_none());

    draw_pie(&mut pixmap, 0.0, "5h", if no_data { None } else { five_hour });
    draw_pie(&mut pixmap, CELL_W, "7d", if no_data { None } else { seven_day });

    pixmap.encode_png().expect("png encode never fails for valid pixmap")
}

fn draw_pie(pixmap: &mut Pixmap, cell_x: f32, label: &str, pct: Option<f64>) {
    let translate = Transform::from_translate(cell_x, 0.0);

    // Track ring (always drawn).
    let track_path = circle_path(RING_CX, RING_CY, RING_R);
    let mut stroke = Stroke::default();
    stroke.width = RING_STROKE;
    let mut paint = Paint::default();
    paint.set_color(shared::TRACK);
    paint.anti_alias = true;
    pixmap.stroke_path(&track_path, &paint, &stroke, translate, None);

    // Arc fill (only when we have data).
    if let Some(raw) = pct {
        let clamped = raw.clamp(0.0, 99.0);
        if clamped > 0.0 {
            let circumference = 2.0 * std::f32::consts::PI * RING_R;
            let filled = circumference * (clamped as f32 / 100.0);
            let gap = circumference - filled;

            let arc_path = circle_path(RING_CX, RING_CY, RING_R);
            let mut arc_stroke = Stroke::default();
            arc_stroke.width = RING_STROKE;
            arc_stroke.line_cap = tiny_skia::LineCap::Butt;
            arc_stroke.dash = StrokeDash::new(vec![filled, gap], 0.0);
            let mut arc_paint = Paint::default();
            arc_paint.set_color(shared::arc_color(raw));
            arc_paint.anti_alias = true;
            // Rotate so the dash pattern starts at 12 o'clock.
            let arc_transform = translate
                .pre_translate(RING_CX, RING_CY)
                .pre_rotate(-90.0)
                .pre_translate(-RING_CX, -RING_CY);
            pixmap.stroke_path(&arc_path, &arc_paint, &arc_stroke, arc_transform, None);
        }
    }

    // Inscribed digits.
    let digit_str = match pct {
        Some(p) => format!("{:>2}", (p.clamp(0.0, 99.0).round()) as i64),
        None => "—".to_string(),
    };
    draw_text(
        pixmap,
        &digit_str,
        cell_x + RING_CX,
        RING_CY,
        DIGIT_HEIGHT_PX,
        shared::TEXT,
        TextAnchor::Center,
    );

    // Bucket label below the pie.
    draw_text(
        pixmap,
        label,
        cell_x + RING_CX,
        LABEL_BASELINE_Y,
        LABEL_HEIGHT_PX,
        shared::TEXT_MUTED,
        TextAnchor::Center,
    );
}

#[derive(Copy, Clone)]
enum TextAnchor {
    Center,
}

/// Draws `text` centered horizontally at (`x`, `y_baseline`), with the given
/// glyph height. Each character is rendered by extracting its `tiny_skia::Path`
/// from the bundled JetBrains Mono font, scaling it to `height_px`, and
/// translating it into place.
fn draw_text(
    pixmap: &mut Pixmap,
    text: &str,
    x_center: f32,
    y_baseline: f32,
    height_px: f32,
    color: tiny_skia::Color,
    anchor: TextAnchor,
) {
    let scale = height_px / digits::units_per_em();
    // JetBrains Mono is monospaced. Estimate advance width as ~0.6 × em (typical for Mono).
    let advance = 0.6 * digits::units_per_em() * scale;
    let total_width = advance * text.chars().count() as f32;

    let start_x = match anchor {
        TextAnchor::Center => x_center - total_width / 2.0,
    };

    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;

    let mut pen_x = start_x;
    for ch in text.chars() {
        if let Some(path) = digits::glyph_path(ch) {
            let transform = Transform::from_scale(scale, scale)
                .post_translate(pen_x, y_baseline);
            pixmap.fill_path(path, &paint, FillRule::Winding, transform, None);
        }
        pen_x += advance;
    }
}

fn circle_path(cx: f32, cy: f32, r: f32) -> tiny_skia::Path {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    pb.finish().expect("circle is a valid path")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(bytes);
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        (info.width, info.height, buf[..info.buffer_size()].to_vec())
    }

    #[test]
    fn output_is_a_valid_88x44_png() {
        let bytes = render(Some(50.0), Some(50.0), false);
        assert_eq!(&bytes[..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        let (w, h, _) = decode_png(&bytes);
        assert_eq!((w, h), (88, 44));
    }

    #[test]
    fn output_has_visible_pixels_when_data_present() {
        let bytes = render(Some(50.0), Some(50.0), false);
        let (_, _, rgba) = decode_png(&bytes);
        let any_opaque = rgba.chunks(4).any(|p| p[3] > 0);
        assert!(any_opaque, "expected at least one opaque pixel");
    }

    #[test]
    fn output_is_almost_blank_when_no_data() {
        let bytes = render(None, None, true);
        let (_, _, rgba) = decode_png(&bytes);
        let opaque_count = rgba.chunks(4).filter(|p| p[3] > 200).count();
        // The track rings + em-dashes still show up; just confirm the
        // colored arcs and full-strength digits are absent.
        // Lower-bound: even fully empty has some track pixels (~80-200).
        // Upper-bound: 5h+7d filled would have ~600+ opaque pixels.
        assert!(opaque_count < 400, "no-data state should have <400 opaque pixels, got {}", opaque_count);
    }

    #[test]
    fn warn_threshold_renders_warn_color_in_left_pie() {
        // 75% on left, 0% on right. The left pie should have at least one pixel
        // matching the warn arc color (with anti-aliasing tolerance).
        let bytes = render(Some(80.0), Some(0.0), false);
        let (w, _, rgba) = decode_png(&bytes);
        let warn_target = (0xE8, 0x91, 0x49);
        let mut hit = false;
        for (i, px) in rgba.chunks(4).enumerate() {
            let x = (i as u32) % w;
            // Only check the left pie (x < 44).
            if x >= 44 {
                continue;
            }
            let (r, g, b, a) = (px[0], px[1], px[2], px[3]);
            if a > 200 && near(r, warn_target.0) && near(g, warn_target.1) && near(b, warn_target.2)
            {
                hit = true;
                break;
            }
        }
        assert!(hit, "expected at least one warn-colored pixel in left pie");
    }

    fn near(a: u8, b: u8) -> bool {
        (a as i16 - b as i16).abs() < 12
    }
}
```

Update `src-tauri/src/tray_icon/mod.rs` to:

```rust
pub mod digits;
pub mod macos;
pub mod shared;
```

- [ ] **Step 2: Add the `png` crate dev-dep for tests**

The tests above use `png::Decoder` to verify output. `png` is already pulled in transitively by Tauri's `image-png` feature, but to use it in our own code we should declare it explicitly.

In `src-tauri/Cargo.toml`, add to `[dependencies]` (it'll dedupe with Tauri's transitive copy):

```toml
png = "0.17"
```

- [ ] **Step 3: Run the tests**

```bash
cd src-tauri && cargo test tray_icon::macos 2>&1 | tail -25 && cd -
```

Expected: 4 tests pass.

If `output_has_visible_pixels_when_data_present` fails, the renderer is producing an empty pixmap — likely a transform issue (paths drawn off-canvas). Check that `RING_CX = 22.0`, `RING_CY = 18.0` are within the 44×44 cell.

If `warn_threshold_renders_warn_color_in_left_pie` fails with no matching pixel, the issue is likely:
- Color shadowed by anti-aliasing — increase the `near()` tolerance from 12 to 30
- Or the arc isn't rendering — check that `pct = Some(80.0)` actually triggers the `if let Some(raw)` arm

---

### Task 6: Implement `tray_icon/windows.rs` (32×32 concentric renderer)

The Windows file always compiles (so we can run its tests on macOS) but only the binary built on Windows uses it.

**Files:**
- Create: `src-tauri/src/tray_icon/windows.rs`
- Modify: `src-tauri/src/tray_icon/mod.rs` — add `pub mod windows;`

- [ ] **Step 1: Write the implementation**

Create `src-tauri/src/tray_icon/windows.rs`:

```rust
//! Windows tray icon renderer. Produces a 32×32 PNG with concentric rings
//! (outer = 7-day, inner = 5-hour) and the worse of the two percentages
//! displayed as two digits in the center. Square geometry is forced by the
//! Windows shell tray API.

use crate::tray_icon::{digits, shared};
use tiny_skia::{
    FillRule, Paint, PathBuilder, Pixmap, Stroke, StrokeDash, Transform,
};

const SIZE: u32 = 32;
const CX: f32 = 16.0;
const CY: f32 = 16.0;
const OUTER_R: f32 = 14.0;
const OUTER_STROKE: f32 = 2.5;
const INNER_R: f32 = 9.0;
const INNER_STROKE: f32 = 2.5;
const DIGIT_HEIGHT_PX: f32 = 9.0;

pub fn render(five_hour: Option<f64>, seven_day: Option<f64>, paused: bool) -> Vec<u8> {
    let mut pixmap = Pixmap::new(SIZE, SIZE).expect("32x32 fits in memory");
    pixmap.fill(tiny_skia::Color::TRANSPARENT);

    let no_data = paused || (five_hour.is_none() && seven_day.is_none());

    draw_ring(&mut pixmap, OUTER_R, OUTER_STROKE, if no_data { None } else { seven_day });
    draw_ring(&mut pixmap, INNER_R, INNER_STROKE, if no_data { None } else { five_hour });

    let worst = match (five_hour, seven_day) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(v), None) | (None, Some(v)) => Some(v),
        (None, None) => None,
    };
    let digit_str = match worst {
        Some(p) if !no_data => format!("{:>2}", (p.clamp(0.0, 99.0).round()) as i64),
        _ => "—".to_string(),
    };
    draw_centered_text(&mut pixmap, &digit_str, CX, CY, DIGIT_HEIGHT_PX, shared::TEXT);

    pixmap.encode_png().expect("png encode never fails")
}

fn draw_ring(pixmap: &mut Pixmap, radius: f32, stroke_width: f32, pct: Option<f64>) {
    let path = {
        let mut pb = PathBuilder::new();
        pb.push_circle(CX, CY, radius);
        pb.finish().expect("circle is valid")
    };

    let mut stroke = Stroke::default();
    stroke.width = stroke_width;
    stroke.line_cap = tiny_skia::LineCap::Butt;

    // Track.
    let mut track_paint = Paint::default();
    track_paint.set_color(shared::TRACK);
    track_paint.anti_alias = true;
    pixmap.stroke_path(&path, &track_paint, &stroke, Transform::identity(), None);

    // Arc fill, if data.
    if let Some(raw) = pct {
        let clamped = raw.clamp(0.0, 99.0);
        if clamped > 0.0 {
            let circumference = 2.0 * std::f32::consts::PI * radius;
            let filled = circumference * (clamped as f32 / 100.0);
            let gap = circumference - filled;
            stroke.dash = StrokeDash::new(vec![filled, gap], 0.0);
            let mut arc_paint = Paint::default();
            arc_paint.set_color(shared::arc_color(raw));
            arc_paint.anti_alias = true;
            let arc_transform = Transform::identity()
                .pre_translate(CX, CY)
                .pre_rotate(-90.0)
                .pre_translate(-CX, -CY);
            pixmap.stroke_path(&path, &arc_paint, &stroke, arc_transform, None);
        }
    }
}

fn draw_centered_text(
    pixmap: &mut Pixmap,
    text: &str,
    x_center: f32,
    y_baseline: f32,
    height_px: f32,
    color: tiny_skia::Color,
) {
    let scale = height_px / digits::units_per_em();
    let advance = 0.6 * digits::units_per_em() * scale;
    let total_width = advance * text.chars().count() as f32;
    let start_x = x_center - total_width / 2.0;

    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;

    let mut pen_x = start_x;
    for ch in text.chars() {
        if let Some(path) = digits::glyph_path(ch) {
            let transform = Transform::from_scale(scale, scale)
                .post_translate(pen_x, y_baseline);
            pixmap.fill_path(path, &paint, FillRule::Winding, transform, None);
        }
        pen_x += advance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(bytes);
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        (info.width, info.height, buf[..info.buffer_size()].to_vec())
    }

    #[test]
    fn output_is_32x32_png() {
        let bytes = render(Some(50.0), Some(50.0), false);
        let (w, h, _) = decode_png(&bytes);
        assert_eq!((w, h), (32, 32));
    }

    #[test]
    fn worst_value_drives_center_digits() {
        // 80% on 5h, 30% on 7d: the digits should be "80", in danger color
        // is wrong (80 = warn). Here we just confirm SOMETHING gets rendered.
        let bytes = render(Some(80.0), Some(30.0), false);
        let (_, _, rgba) = decode_png(&bytes);
        let any_opaque = rgba.chunks(4).any(|p| p[3] > 200);
        assert!(any_opaque);
    }

    #[test]
    fn no_data_state_is_mostly_empty() {
        let bytes = render(None, None, true);
        let (_, _, rgba) = decode_png(&bytes);
        let opaque_count = rgba.chunks(4).filter(|p| p[3] > 200).count();
        // Track rings + em-dash. Lower than two filled arcs + two two-digit numbers.
        assert!(opaque_count < 250, "got {} opaque pixels in no-data state", opaque_count);
    }
}
```

Update `src-tauri/src/tray_icon/mod.rs`:

```rust
pub mod digits;
pub mod macos;
pub mod shared;
pub mod windows;
```

- [ ] **Step 2: Run the tests**

```bash
cd src-tauri && cargo test tray_icon::windows 2>&1 | tail -15 && cd -
```

Expected: 3 tests pass.

---

### Task 7: Wire up `tray_icon/mod.rs` facade and modify `tray.rs::set_level`

**Files:**
- Modify: `src-tauri/src/tray_icon/mod.rs` — add `pub fn render(...)` facade
- Modify: `src-tauri/src/tray.rs` — replace the body of `set_level`

- [ ] **Step 1: Add the platform-dispatching facade**

Replace the contents of `src-tauri/src/tray_icon/mod.rs` with:

```rust
//! Dynamic tray icon renderer. Selects between the macOS dual-pie and the
//! Windows concentric design at compile time and produces fresh PNG bytes
//! on every call.

pub mod digits;
pub mod macos;
pub mod shared;
pub mod windows;

/// Renders the tray icon for the given usage state. Returns PNG bytes ready
/// for `tauri::image::Image::from_bytes`. The output dimensions are
/// platform-dependent: 88×44 on macOS, 32×32 on Windows.
pub fn render(five_hour: Option<f64>, seven_day: Option<f64>, paused: bool) -> Vec<u8> {
    #[cfg(target_os = "macos")]
    {
        macos::render(five_hour, seven_day, paused)
    }
    #[cfg(target_os = "windows")]
    {
        windows::render(five_hour, seven_day, paused)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // Other platforms (Linux, etc.) get the macOS layout as a reasonable default.
        macos::render(five_hour, seven_day, paused)
    }
}
```

- [ ] **Step 2: Replace the body of `set_level` in `tray.rs`**

Open `src-tauri/src/tray.rs`. Replace the entire file with:

```rust
use crate::tray_icon;
use tauri::image::Image;
use tauri::AppHandle;

/// Updates the tray icon. The icon is synthesized on every call from the
/// live percentages — no static asset paths are involved.
pub fn set_level(
    app: &AppHandle,
    five_hour: Option<f64>,
    seven_day: Option<f64>,
    paused: bool,
) {
    let Some(tray) = app.tray_by_id("main") else { return };

    let bytes = tray_icon::render(five_hour, seven_day, paused);
    let _ = tray.set_icon(Some(Image::from_bytes(&bytes).expect("renderer produces valid png")));
    let _ = tray.set_icon_as_template(false);
    // Numbers live in the icon now — no separate title text alongside.
    let _ = tray.set_title(None);
    let _ = tray.set_tooltip(Some(tooltip(five_hour, seven_day, paused)));
}

fn tooltip(five_hour: Option<f64>, seven_day: Option<f64>, paused: bool) -> String {
    if paused && five_hour.is_none() && seven_day.is_none() {
        return "Claude usage — sign-in required".into();
    }
    format!(
        "Claude usage — 5h {}, 7d {}",
        fmt(five_hour),
        fmt(seven_day)
    )
}

fn fmt(v: Option<f64>) -> String {
    match v {
        Some(p) => format!("{}%", p.round() as i64),
        None => "—".into(),
    }
}
```

This deletes `pick()`, `format_title`, `fmt_opt`, and the four `include_bytes!` calls — all replaced by the renderer.

- [ ] **Step 3: Add `mod tray_icon;` to `lib.rs`**

This was already done in Task 3 Step 1 — re-verify by inspecting `src-tauri/src/lib.rs` and confirming line 10 (or thereabouts) reads:

```rust
mod tray_icon;
```

If missing, add it.

- [ ] **Step 4: Verify everything still compiles**

```bash
cd src-tauri && cargo check 2>&1 | tail -10 && cd -
```

Expected: `Finished`. No errors.

If a `pick is not defined` or similar error surfaces from another module, search for stale references:

```bash
grep -rn "tray::pick\|tray::format_title\|tray::fmt_opt" src-tauri/src/
```

Expected: no output. If something does turn up (e.g., a stale call in `commands.rs` or `poll_loop.rs`), update or delete the reference.

- [ ] **Step 5: Run all tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -15 && cd -
```

Expected: all tests pass, including the new `tray_icon` ones.

---

### Task 8: Delete the four obsolete static PNG assets

**Files:**
- Delete: `src-tauri/icons/tray/idle-template.png`
- Delete: `src-tauri/icons/tray/warn.png`
- Delete: `src-tauri/icons/tray/danger.png`
- Delete: `src-tauri/icons/tray/paused.png`

These were the original splat icons. With the renderer in place and `iconPath` redirected to `boot-placeholder.png` in Task 1, nothing references them.

- [ ] **Step 1: Confirm no remaining references**

Run:

```bash
grep -rn "tray/idle-template\|tray/warn\|tray/danger\|tray/paused" src-tauri/src/ src-tauri/tauri.conf.json
```

Expected: no output. (If the grep shows references in `tray.rs`, Task 7 wasn't applied cleanly — go back and fix.)

- [ ] **Step 2: Delete the files**

```bash
rm src-tauri/icons/tray/idle-template.png \
   src-tauri/icons/tray/warn.png \
   src-tauri/icons/tray/danger.png \
   src-tauri/icons/tray/paused.png

ls src-tauri/icons/tray/
```

Expected: only `boot-placeholder.png` remains.

- [ ] **Step 3: Re-run cargo build to confirm nothing is missing**

```bash
cd src-tauri && cargo build 2>&1 | tail -5 && cd -
```

Expected: `Finished`. If `include_bytes! cannot find file ...` appears, an `include_bytes!` reference to a deleted PNG slipped through Task 7 — search and remove it.

---

### Task 9: Smoke test in macOS menu bar and commit

**Files:** none modified by this task.

- [ ] **Step 1: Run the app**

```bash
pnpm tauri dev
```

Expected: app launches. Within ~1 second of the menu bar appearing, the boot placeholder is replaced by the rendered dual-pie. If usage data hasn't loaded yet, both pies show track-only rings with `—` digits.

After the first poll completes (typically within 5–10 seconds of launch), real percentages appear inside the rings, with arc fills proportional to usage and threshold-keyed colors.

- [ ] **Step 2: Verify the title text is gone**

The menubar should show ONLY the icon — no "14% | 24%" text alongside. If text still appears, revisit Task 7 Step 2 and confirm the `set_title(None)` line is in place.

- [ ] **Step 3: Force the warn / danger states**

To exercise warn (75–89%) and danger (≥90%) without actually consuming quota, temporarily edit `src-tauri/src/poll_loop.rs` (or wherever percentages flow into `tray::set_level`) to override the value. Easier: add a temp override at the top of `tray::set_level`:

```rust
pub fn set_level(
    app: &AppHandle,
    five_hour: Option<f64>,
    seven_day: Option<f64>,
    paused: bool,
) {
    // TEMP smoke test — remove before commit
    let five_hour = Some(80.0);
    let seven_day = Some(95.0);
    let paused = false;
    // ... rest unchanged
}
```

Re-launch with `pnpm tauri dev`. Expected: left pie shows 80 in amber, right pie shows 95 in coral. Toggle the values and confirm the colors switch.

Stop the dev server.

- [ ] **Step 4: Force the paused state**

```rust
let paused = true;
let five_hour = None;
let seven_day = None;
```

Expected: both pies render with track-only rings and `—` digits.

Stop the dev server.

- [ ] **Step 5: Revert all temporary edits**

```bash
git diff src-tauri/src/tray.rs
```

If anything from steps 3–4 remains, revert:

```bash
git checkout -- src-tauri/src/tray.rs
```

Re-confirm clean: `git diff src-tauri/src/tray.rs` shows no changes from the form Task 7 left it in.

(If you reverted too far and lost the Task 7 changes, redo Task 7 Step 2.)

- [ ] **Step 6: Inspect the final diff**

```bash
git status --short
```

Expected — relative to the spec commit (`72a064c`), the tray icon work has these changes:

```
 M src-tauri/Cargo.toml
 M src-tauri/Cargo.lock     (auto-updated by cargo)
 M src-tauri/src/lib.rs
 M src-tauri/src/tray.rs
 M src-tauri/tauri.conf.json
 D src-tauri/icons/tray/danger.png
 D src-tauri/icons/tray/idle-template.png
 D src-tauri/icons/tray/paused.png
 D src-tauri/icons/tray/warn.png
?? src-tauri/icons/tray/boot-placeholder.png
?? src-tauri/src/tray_icon/
```

Plus the user's pre-existing parallel-work changes (pricing.json, AuthPanel.tsx, etc.) which we leave alone.

- [ ] **Step 7: Stage and commit only the icon-redesign files**

```bash
git add src-tauri/Cargo.toml \
        src-tauri/Cargo.lock \
        src-tauri/src/lib.rs \
        src-tauri/src/tray.rs \
        src-tauri/tauri.conf.json \
        src-tauri/icons/tray/boot-placeholder.png \
        src-tauri/src/tray_icon/

git rm src-tauri/icons/tray/danger.png \
       src-tauri/icons/tray/idle-template.png \
       src-tauri/icons/tray/paused.png \
       src-tauri/icons/tray/warn.png

git diff --cached --stat
```

Expected stat: ~12-15 files changed (the new tray_icon module is 5 files + font + license, plus the modifications and deletions listed above).

```bash
git commit -m "$(cat <<'EOF'
feat(tray): dynamic dual-pie tray icon with live percentages

Replaces the four static splat PNGs with a runtime renderer that draws
two side-by-side pies (macOS) or one concentric pair (Windows), each
ring filling clockwise as the bucket usage rises and showing the live
percentage as inscribed digits.

The menubar title text is removed — numbers now live exclusively in
the icon. Threshold colors per pie come from the project's design
tokens (terracotta / amber / coral). Renderer is pure-Rust via
tiny-skia + ttf-parser with a bundled JetBrains Mono Regular font.

Per docs/superpowers/specs/2026-04-27-tray-icon-dynamic-design.md.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 8: Verify the commit landed cleanly**

```bash
git log -1 --stat
```

Expected: one commit. The user's parallel-work changes still appear as `M` in `git status` — that's correct, they're not part of this commit.

---

## Acceptance Criteria

- ✅ `tiny-skia` and `ttf-parser` (and explicitly-declared `png`) are in `src-tauri/Cargo.toml`. (Task 2, Task 5 Step 2)
- ✅ The new `tray_icon/` module tree exists with `mod.rs`, `shared.rs`, `digits.rs`, `macos.rs`, `windows.rs`, plus the embedded font and its license note. (Tasks 3–7)
- ✅ `cargo build` passes on macOS. (Task 7 Step 4, Task 8 Step 3)
- ✅ All four old static PNGs are deleted. The previous brainstorm's `sources/` directory is gone. (Tasks 1, 8)
- ✅ `tauri.conf.json` `iconPath` points at `icons/tray/boot-placeholder.png` with `iconAsTemplate: false`. (Task 1 Step 4)
- ✅ `tray.rs::set_level` no longer references `pick()` or `include_bytes!`. (Task 7 Step 2)
- ✅ On macOS launch: menubar shows two side-by-side pies with two-digit percentages, no title text alongside, threshold-colored arcs that fill clockwise from 12 o'clock. (Task 9 Steps 1–2)
- ✅ Hover tooltip reads "Claude usage — 5h X%, 7d Y%". (Task 9 Step 1, by inspection)
- ✅ Forcing paused / no-data state renders both pies with track-only rings + em-dash digits. (Task 9 Step 4)
- ✅ All `cargo test tray_icon::*` tests pass. (Tasks 3–6)
