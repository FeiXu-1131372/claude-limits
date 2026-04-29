use super::super::StoredToken;
use anyhow::{anyhow, Context, Result};
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use std::io;
use std::process::Command;

const SERVICE_PREFIX: &str = "Claude Code-credentials";

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

pub async fn load() -> Result<Option<StoredToken>> {
    let services = discover_services().await?;
    let mut candidates = Vec::new();
    for svc in services {
        if let Ok(Some(tok)) = read_one(svc).await {
            candidates.push(tok);
        }
    }
    candidates.sort_by_key(|t| t.expires_at);
    Ok(candidates.pop())
}

async fn discover_services() -> Result<Vec<String>> {
    let output = tokio::task::spawn_blocking(|| {
        Command::new("security").arg("dump-keychain").output()
    })
    .await
    .map_err(io::Error::other)?;

    let stdout = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Ok(vec![SERVICE_PREFIX.to_string()]),
    };
    let mut services = Vec::new();
    for line in stdout.lines() {
        if let Some(idx) = line.find("\"svce\"<blob>=\"") {
            let rest = &line[idx + 14..];
            if let Some(end) = rest.find('"') {
                let name = &rest[..end];
                if name.starts_with(SERVICE_PREFIX) && !services.contains(&name.to_string()) {
                    services.push(name.to_string());
                }
            }
        }
    }
    if services.is_empty() {
        services.push(SERVICE_PREFIX.to_string());
    }
    Ok(services)
}

async fn read_one(service: String) -> Result<Option<StoredToken>> {
    let out = tokio::task::spawn_blocking(move || {
        Command::new("security")
            .args(["find-generic-password", "-s", &service, "-w"])
            .output()
    })
    .await
    .map_err(io::Error::other)?
    .context("spawn security find-generic-password")?;

    if !out.status.success() {
        return Ok(None);
    }

    let mut bytes = out.stdout;
    if let Some(&last) = bytes.last() {
        if last == b'\n' {
            bytes.pop();
        }
    }

    if !bytes.is_empty() && bytes[0] > 0x7F {
        bytes.remove(0);
    }

    let text = String::from_utf8(bytes).context("keychain payload not utf-8")?;
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

pub async fn has_creds() -> bool {
    tokio::task::spawn_blocking(|| {
        Command::new("security")
            .args(["find-generic-password", "-s", SERVICE_PREFIX])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn parse_sample_payload() {
        let sample = r#"{"claudeAiOauth":{"accessToken":"a","refreshToken":"r","expiresAt":1840000000000}}"#;
        let raw: RawCreds = serde_json::from_str(sample).unwrap();
        assert_eq!(raw.claude_ai_oauth.access_token, "a");
        assert_eq!(
            raw.claude_ai_oauth.refresh_token.as_deref(),
            Some("r")
        );
        let expected = Utc
            .timestamp_millis_opt(1_840_000_000_000)
            .single()
            .unwrap();
        assert!(expected > Utc::now() - Duration::days(365 * 100));
    }
}
