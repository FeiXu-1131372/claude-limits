use super::StoredToken;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const KEYRING_SERVICE: &str = "claude-limits";
const KEYRING_USER: &str = "oauth_refresh";

pub async fn save(token: &StoredToken, fallback_dir: &Path) -> Result<()> {
    let payload = serde_json::to_string(token)?;
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => match entry.set_password(&payload) {
            Ok(_) => {
                let _ = fs::remove_file(fallback_path(fallback_dir));
                Ok(())
            }
            Err(e) => {
                tracing::warn!("keyring save failed ({e}); falling back to restricted file");
                save_fallback(token, fallback_dir).await
            }
        },
        Err(e) => {
            tracing::warn!("keyring unavailable ({e}); using restricted file");
            save_fallback(token, fallback_dir).await
        }
    }
}

pub fn load(fallback_dir: &Path) -> Result<Option<StoredToken>> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if let Ok(s) = entry.get_password() {
            if let Ok(t) = serde_json::from_str::<StoredToken>(&s) {
                return Ok(Some(t));
            }
        }
    }
    load_fallback(fallback_dir)
}

pub fn clear(fallback_dir: &Path) -> Result<()> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.delete_credential();
    }
    let p = fallback_path(fallback_dir);
    let _ = fs::remove_file(p);
    Ok(())
}

fn fallback_path(dir: &Path) -> PathBuf {
    dir.join("credentials.json")
}

async fn save_fallback(token: &StoredToken, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let final_path = fallback_path(dir);
    let tmp_path = dir.join("credentials.json.tmp");
    let payload = serde_json::to_string_pretty(token)?;
    fs::write(&tmp_path, &payload).context("write temp credential file")?;
    restrict_permissions(tmp_path.clone()).await?;
    fs::rename(&tmp_path, &final_path).context("rename temp credential file into place")?;
    Ok(())
}

fn load_fallback(dir: &Path) -> Result<Option<StoredToken>> {
    let p = fallback_path(dir);
    if !p.exists() {
        return Ok(None);
    }
    let s = fs::read_to_string(&p)?;
    Ok(serde_json::from_str(&s).ok())
}

#[cfg(unix)]
async fn restrict_permissions(p: PathBuf) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&p)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&p, perms)?;
    Ok(())
}

#[cfg(windows)]
async fn restrict_permissions(p: PathBuf) -> Result<()> {
    use std::io;
    use std::process::Command;
    let username = whoami::username();
    let status = tokio::task::spawn_blocking(move || {
        Command::new("icacls")
            .arg(&p)
            .args([
                "/inheritance:r",
                "/grant:r",
                &format!("{}:F", username),
            ])
            .status()
            .context("icacls failed to run")
    })
    .await
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;
    if !status.success() {
        anyhow::bail!("icacls returned non-zero");
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
async fn restrict_permissions(_: PathBuf) -> Result<()> {
    Ok(())
}
