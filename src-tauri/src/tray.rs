use tauri::image::Image;
use tauri::AppHandle;

/// Updates the tray icon and the macOS menubar title.
///
/// On macOS the title shows "5h XX%  7d YY%" so a glance at the menubar
/// answers "can I keep coding?" without opening the popover. On Windows the
/// status item has no text affordance — `set_title` is silently ignored, the
/// icon color carries the signal there.
///
/// The icon color reflects the *worst* of the two utilizations, so a green
/// tray with "5h 9% · 7d 92%" still goes amber/red because the higher value
/// is what the user cares about.
pub fn set_level(
    app: &AppHandle,
    five_hour: Option<f64>,
    seven_day: Option<f64>,
    paused: bool,
) {
    let tray = match app.tray_by_id("main") {
        Some(t) => t,
        None => return,
    };

    let worst = match (five_hour, seven_day) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    };

    let (bytes, template) = pick(worst, paused);
    let _ = tray.set_icon(Some(Image::from_bytes(bytes).expect("icon bytes")));
    let _ = tray.set_icon_as_template(template);

    let title = if paused {
        None
    } else {
        format_title(five_hour, seven_day)
    };
    let _ = tray.set_title(title.as_deref());

    let tooltip = match worst {
        Some(_) => format!(
            "Claude usage — 5h {}, 7d {}",
            fmt_opt(five_hour),
            fmt_opt(seven_day)
        ),
        None if paused => "Claude usage — sign-in required".into(),
        None => "Claude usage".into(),
    };
    let _ = tray.set_tooltip(Some(tooltip));
}

fn fmt_opt(v: Option<f64>) -> String {
    match v {
        Some(p) => format!("{}%", p.round() as i64),
        None => "—".into(),
    }
}

fn format_title(five_hour: Option<f64>, seven_day: Option<f64>) -> Option<String> {
    // Two numbers separated by a pipe — the position itself encodes the bucket
    // (left = 5h, right = 7d). Tooltip on hover spells it out for new users.
    match (five_hour, seven_day) {
        (Some(a), Some(b)) => Some(format!("{}% | {}%", a.round() as i64, b.round() as i64)),
        (Some(a), None) => Some(format!("{}%", a.round() as i64)),
        (None, Some(b)) => Some(format!("{}%", b.round() as i64)),
        (None, None) => None,
    }
}

fn pick(pct: Option<f64>, paused: bool) -> (&'static [u8], bool) {
    if paused {
        return (
            include_bytes!("../icons/tray/paused.png"),
            true,
        );
    }
    match pct {
        Some(p) if p >= 90.0 => (
            include_bytes!("../icons/tray/danger.png"),
            false,
        ),
        Some(p) if p >= 75.0 => (
            include_bytes!("../icons/tray/warn.png"),
            false,
        ),
        _ => (
            include_bytes!("../icons/tray/idle-template.png"),
            true,
        ),
    }
}
