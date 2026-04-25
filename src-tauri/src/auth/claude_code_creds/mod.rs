use super::StoredToken;
use anyhow::Result;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

pub fn load() -> Result<Option<StoredToken>> {
    #[cfg(target_os = "macos")]
    return macos::load();
    #[cfg(target_os = "windows")]
    return windows::load();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Ok(None);
}

pub fn has_creds() -> bool {
    #[cfg(target_os = "macos")]
    return macos::has_creds();
    #[cfg(target_os = "windows")]
    return windows::has_creds();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return false;
}
