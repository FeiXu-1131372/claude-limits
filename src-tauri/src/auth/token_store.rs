use super::StoredToken;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

// We deliberately do NOT use the OS keychain (Keychain Access on macOS,
// Credential Manager on Windows, Secret Service on Linux) for the OAuth
// refresh token. Reasons:
//
//   1. macOS prompt on every rebuild. Keychain ACLs are bound to the
//      calling binary's code signature. An unsigned (or ad-hoc-signed)
//      open-source app gets a different signature on each `cargo build`,
//      and the user's "Always Allow" stops applying. Every restart shows
//      a "claude-limits wants to use your confidential information"
//      prompt asking for the login keychain password — which to a
//      privacy-conscious user looks indistinguishable from malware
//      asking for credentials.
//
//   2. The threat model the keychain protects against is largely already
//      covered. FileVault (default on every macOS install since Big Sur)
//      encrypts the home directory at rest. POSIX mode-0600 (and the
//      Windows ACL grant we apply) prevents other local users from
//      reading the file. Root can read either the keychain or the file
//      regardless. For an attacker without root and without a logged-in
//      session, both are equally inaccessible.
//
//   3. Code that doesn't run can't break. The previous keyring-first
//      logic carried two latent bugs (refresh writing CLI tokens into
//      the OAuth slot; per-rebuild prompts). Routing through a single
//      well-understood path eliminates both classes.
//
// The token lives at `<app-data-dir>/credentials.json` with mode 0o600
// on Unix and a single-user-grant ACL on Windows.

pub async fn save(token: &StoredToken, fallback_dir: &Path) -> Result<()> {
    save_fallback(token, fallback_dir).await
}

pub fn load(fallback_dir: &Path) -> Result<Option<StoredToken>> {
    load_fallback(fallback_dir)
}

pub fn clear(fallback_dir: &Path) -> Result<()> {
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
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "Administrator".to_string());
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
    .map_err(io::Error::other)??;
    if !status.success() {
        anyhow::bail!("icacls returned non-zero");
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
async fn restrict_permissions(_: PathBuf) -> Result<()> {
    Ok(())
}
