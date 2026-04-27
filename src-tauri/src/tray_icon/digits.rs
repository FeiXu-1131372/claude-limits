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
    advances: HashMap<char, f32>,
    units_per_em: f32,
    cap_height: f32,
}

static CACHE: OnceLock<GlyphCache> = OnceLock::new();

fn cache() -> &'static GlyphCache {
    CACHE.get_or_init(|| {
        let face = Face::parse(FONT_BYTES, 0).expect("bundled font is parseable");
        let units_per_em = face.units_per_em() as f32;
        // JetBrains Mono reports cap_height directly in its OS/2 table. Fall back
        // to ~70% of em if the table is missing (vanishingly unlikely for this font).
        let cap_height = face.capital_height().map(|v| v as f32).unwrap_or(0.7 * units_per_em);

        let mut paths = HashMap::new();
        let mut advances = HashMap::new();
        for &ch in SUPPORTED_CHARS {
            if let Some(glyph_id) = face.glyph_index(ch) {
                if let Some(path) = extract_glyph(&face, ch) {
                    paths.insert(ch, path);
                }
                if let Some(advance) = face.glyph_hor_advance(glyph_id) {
                    advances.insert(ch, advance as f32);
                }
            }
        }
        GlyphCache { paths, advances, units_per_em, cap_height }
    })
}

/// Returns the glyph path for `ch` in font-em units (Y flipped to be Y-down),
/// or `None` if the character isn't in the font (shouldn't happen for digits).
pub fn glyph_path(ch: char) -> Option<&'static Path> {
    cache().paths.get(&ch)
}

/// Returns the horizontal advance for `ch` in font-em units. For monospace fonts
/// this is the same value for every glyph, but querying per-character is robust.
pub fn glyph_advance(ch: char) -> Option<f32> {
    cache().advances.get(&ch).copied()
}

/// Returns the font's em-square size. Callers divide their target pixel size by
/// this value to get the scale factor for glyph paths.
pub fn units_per_em() -> f32 {
    cache().units_per_em
}

/// Returns the font's cap height in em units (the height of capital letters
/// from baseline). Used for visually centering digits inside a ring — the
/// digits' visual center is `baseline - cap_height/2`.
pub fn cap_height() -> f32 {
    cache().cap_height
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
