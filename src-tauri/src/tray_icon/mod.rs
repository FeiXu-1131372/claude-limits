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
