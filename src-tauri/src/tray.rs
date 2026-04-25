use tauri::image::Image;
use tauri::{AppHandle, Manager, tray::TrayIcon};

pub fn set_level(app: &AppHandle, pct: Option<f64>, paused: bool) {
    let tray = app.tray_by_id("main");
    if let Some(tray) = tray {
        let (bytes, template) = pick(pct, paused);
        let _ = tray.set_icon(Some(Image::from_bytes(bytes).expect("icon bytes")));
        let _ = tray.set_icon_as_template(template);
        if let Some(pct) = pct {
            let _ = tray.set_tooltip(Some(format!("Claude {}%", pct.round() as i64)));
        }
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
