#![cfg(target_os = "windows")]

use super::super::StoredToken;
use anyhow::{anyhow, Context, Result};
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct RawCreds {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: OauthBlock,
}

#[derive(Deserialize)]
struct OauthBlock {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at_ms: i64,
}

fn credentials_path() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE")?;
    Some(PathBuf::from(home).join(".claude").join(".credentials.json"))
}

pub fn load() -> Result<Option<StoredToken>> {
    let p = match credentials_path() {
        Some(p) => p,
        None => return Ok(None),
    };
    if !p.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&p).context("read .credentials.json")?;
    let raw: RawCreds = match serde_json::from_str(&text) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    let exp = Utc
        .timestamp_millis_opt(raw.claude_ai_oauth.expires_at_ms)
        .single()
        .ok_or_else(|| anyhow!("invalid expires_at_ms"))?;
    Ok(Some(StoredToken {
        access_token: raw.claude_ai_oauth.access_token,
        refresh_token: raw.claude_ai_oauth.refresh_token,
        expires_at: exp,
    }))
}

pub fn has_creds() -> bool {
    credentials_path().map(|p| p.exists()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_realistic_payload_from_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".credentials.json");
        fs::write(
            &path,
            r#"{"claudeAiOauth":{"accessToken":"a","refreshToken":"r","expiresAt":1840000000000}}"#,
        )
        .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let raw: RawCreds = serde_json::from_str(&text).unwrap();
        assert_eq!(raw.claude_ai_oauth.access_token, "a");
    }
}
