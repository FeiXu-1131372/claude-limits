//! Color tokens (sRGB-resolved from tokens.css OKLCH at compile time)
//! and threshold→color logic shared between the macOS and Windows renderers.

use tiny_skia::Color;

// The tray icon lives on the macOS menu bar, which can be light beige (light
// mode) or near-black (dark mode). The popover-theme tokens are tuned for a
// dark surface and become invisible against a light menubar. Use neutral
// colors here that have enough contrast on both menubar themes.

/// Anthropic terracotta — saturated enough to read on both light and dark menubars.
pub fn accent() -> Color { Color::from_rgba8(0xD9, 0x77, 0x57, 0xFF) }
/// Amber-orange. Saturated; visible on both menubar themes.
pub fn warn() -> Color { Color::from_rgba8(0xE8, 0x91, 0x49, 0xFF) }
/// Coral-red. Saturated; visible on both menubar themes.
pub fn danger() -> Color { Color::from_rgba8(0xD8, 0x5A, 0x45, 0xFF) }
/// Track (faint backing ring) — mid gray with moderate alpha so it shows
/// faintly on both menubar themes without overwhelming the colored arc.
pub fn track() -> Color { Color::from_rgba8(0x88, 0x88, 0x88, 0x55) }
/// Digit ink — reads cleanly on light menubars (macOS) and dark taskbars (Windows).
#[cfg(not(target_os = "windows"))]
pub fn text() -> Color { Color::from_rgba8(0x1C, 0x1C, 0x1C, 0xF0) }

#[cfg(target_os = "windows")]
pub fn text() -> Color { Color::from_rgba8(0xF0, 0xF0, 0xF0, 0xF0) }

/// Muted ink for the no-data em-dash.
#[cfg(not(target_os = "windows"))]
pub fn text_muted() -> Color { Color::from_rgba8(0x6A, 0x6A, 0x6A, 0xC0) }

#[cfg(target_os = "windows")]
pub fn text_muted() -> Color { Color::from_rgba8(0xA0, 0xA0, 0xA0, 0xC0) }

/// Returns the arc fill color for a single bucket's percentage.
/// Mirrors the threshold logic the popover uses: <75 = accent, 75-89 = warn, >=90 = danger.
pub fn arc_color(pct: f64) -> Color {
    if pct >= 90.0 {
        danger()
    } else if pct >= 75.0 {
        warn()
    } else {
        accent()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_color_uses_accent_below_75() {
        assert_eq!(arc_color(0.0), accent());
        assert_eq!(arc_color(74.9), accent());
    }

    #[test]
    fn arc_color_uses_warn_at_75_to_89() {
        assert_eq!(arc_color(75.0), warn());
        assert_eq!(arc_color(89.9), warn());
    }

    #[test]
    fn arc_color_uses_danger_at_90_and_above() {
        assert_eq!(arc_color(90.0), danger());
        assert_eq!(arc_color(99.0), danger());
        assert_eq!(arc_color(150.0), danger()); // saturating: anything above 90% is full danger
    }
}
