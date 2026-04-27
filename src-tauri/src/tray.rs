use crate::tray_icon;
use chrono::{DateTime, Utc};
use tauri::image::Image;
use tauri::AppHandle;

/// Updates the tray icon. The icon is synthesized on every call from the
/// live percentages — no static asset paths are involved.
///
/// `five_hour_resets_at` is the absolute timestamp at which the 5-hour
/// rolling window resets; the tooltip formats it as "resets in Xh Ym" for
/// at-a-glance answer to "can I keep coding?".
pub fn set_level(
    app: &AppHandle,
    five_hour: Option<f64>,
    seven_day: Option<f64>,
    five_hour_resets_at: Option<DateTime<Utc>>,
    paused: bool,
) {
    let Some(tray) = app.tray_by_id("main") else { return };

    let bytes = tray_icon::render(five_hour, seven_day, paused);
    let _ = tray.set_icon(Some(Image::from_bytes(&bytes).expect("renderer produces valid png")));
    let _ = tray.set_icon_as_template(false);
    // Numbers live in the icon now — no separate title text alongside.
    let _ = tray.set_title(None::<&str>);
    let _ = tray.set_tooltip(Some(tooltip(
        five_hour,
        seven_day,
        five_hour_resets_at,
        paused,
        Utc::now(),
    )));
}

fn tooltip(
    five_hour: Option<f64>,
    seven_day: Option<f64>,
    five_hour_resets_at: Option<DateTime<Utc>>,
    paused: bool,
    now: DateTime<Utc>,
) -> String {
    if paused && five_hour.is_none() && seven_day.is_none() {
        return "Claude usage — sign-in required".into();
    }

    let mut parts: Vec<String> = Vec::new();
    if let Some(p) = five_hour {
        parts.push(format!("5h {}%", p.round() as i64));
    }
    if let Some(reset) = five_hour_resets_at {
        let minutes = (reset - now).num_minutes().max(0);
        let h = minutes / 60;
        let m = minutes % 60;
        if h == 0 {
            parts.push(format!("reset in {}m", m));
        } else {
            parts.push(format!("reset in {}h {}m", h, m));
        }
    }
    if let Some(p) = seven_day {
        parts.push(format!("7d {}%", p.round() as i64));
    }

    if parts.is_empty() {
        "Claude usage".into()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(min_from_now: i64) -> DateTime<Utc> {
        let now: DateTime<Utc> = "2026-04-27T12:00:00Z".parse().unwrap();
        now + chrono::Duration::minutes(min_from_now)
    }
    fn now() -> DateTime<Utc> {
        "2026-04-27T12:00:00Z".parse().unwrap()
    }

    #[test]
    fn tooltip_full_with_reset_in_hours_and_minutes() {
        let s = tooltip(Some(63.0), Some(28.0), Some(t(134)), false, now());
        assert_eq!(s, "5h 63%, reset in 2h 14m, 7d 28%");
    }

    #[test]
    fn tooltip_full_with_reset_under_an_hour() {
        let s = tooltip(Some(63.0), Some(28.0), Some(t(45)), false, now());
        assert_eq!(s, "5h 63%, reset in 45m, 7d 28%");
    }

    #[test]
    fn tooltip_clamps_negative_reset_to_zero() {
        let s = tooltip(Some(99.0), Some(50.0), Some(t(-10)), false, now());
        assert_eq!(s, "5h 99%, reset in 0m, 7d 50%");
    }

    #[test]
    fn tooltip_paused_with_no_data_shows_signin_prompt() {
        let s = tooltip(None, None, None, true, now());
        assert_eq!(s, "Claude usage — sign-in required");
    }

    #[test]
    fn tooltip_drops_missing_pieces() {
        // Partial data — only 7d known, no 5h reading and no reset time.
        let s = tooltip(None, Some(28.0), None, false, now());
        assert_eq!(s, "7d 28%");
    }

    #[test]
    fn tooltip_unpaused_with_nothing_falls_back_to_generic() {
        let s = tooltip(None, None, None, false, now());
        assert_eq!(s, "Claude usage");
    }
}
