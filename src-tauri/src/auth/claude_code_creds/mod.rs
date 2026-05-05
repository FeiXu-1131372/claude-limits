use super::StoredToken;
use anyhow::Result;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

pub async fn load() -> Result<Option<StoredToken>> {
    #[cfg(target_os = "macos")]
    return macos::load().await;
    #[cfg(target_os = "windows")]
    return windows::load();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Ok(None);
}

pub async fn load_full_blob() -> Result<Option<serde_json::Value>> {
    #[cfg(target_os = "macos")]
    return macos::load_full_blob().await;
    #[cfg(target_os = "windows")]
    return Ok(windows::load_full_blob()?);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return Ok(None);
}

pub async fn write_full_blob(blob: &serde_json::Value) -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::write_full_blob(blob).await;
    #[cfg(target_os = "windows")]
    return windows::write_full_blob(blob);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = blob;
        anyhow::bail!("multi-account swap is unsupported on this platform")
    }
}

pub async fn has_creds() -> bool {
    #[cfg(target_os = "macos")]
    return macos::has_creds().await;
    #[cfg(target_os = "windows")]
    return windows::has_creds();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return false;
}
