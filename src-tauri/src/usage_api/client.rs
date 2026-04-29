use super::types::UsageSnapshot;
use anyhow::Result;
use chrono::Utc;
use reqwest::{Client, StatusCode};
use std::sync::Arc;
use std::time::Duration;

pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
pub const ANTHROPIC_BETA: &str = "oauth-2025-04-20";

#[derive(Debug)]
pub enum FetchOutcome {
    Ok(UsageSnapshot),
    Unauthorized,
    RateLimited,
    Transient(String),
}

pub struct UsageClient {
    base_url: String,
    inner: Arc<Client>,
    app_version: String,
}

impl UsageClient {
    pub fn new(client: Arc<Client>, app_version: String) -> Self {
        Self {
            base_url: USAGE_URL.to_string(),
            inner: client,
            app_version,
        }
    }

    pub fn with_base_url(base_url: String, app_version: String) -> Result<Self> {
        let inner = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            base_url,
            inner: Arc::new(inner),
            app_version,
        })
    }

    pub async fn fetch(&self, access_token: &str) -> FetchOutcome {
        let req = self
            .inner
            .get(&self.base_url)
            .bearer_auth(access_token)
            .header("anthropic-beta", ANTHROPIC_BETA)
            .header(
                "User-Agent",
                format!("claude-limits/{}", self.app_version),
            );

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) if e.is_timeout() => {
                tracing::warn!("usage fetch timed out: {e}");
                return FetchOutcome::Transient("timeout".into());
            }
            Err(e) => {
                tracing::warn!("usage fetch network error: {e}");
                return FetchOutcome::Transient(e.to_string());
            }
        };

        match resp.status() {
            StatusCode::OK => match resp.json::<UsageSnapshot>().await {
                Ok(mut s) => {
                    s.fetched_at = Utc::now();
                    FetchOutcome::Ok(s)
                }
                Err(e) => {
                    tracing::warn!("usage decode failed: {e}");
                    FetchOutcome::Transient(format!("decode: {e}"))
                }
            },
            StatusCode::UNAUTHORIZED => {
                tracing::warn!("usage fetch returned 401 unauthorized");
                FetchOutcome::Unauthorized
            }
            StatusCode::TOO_MANY_REQUESTS => {
                tracing::warn!("usage fetch returned 429 rate-limited");
                FetchOutcome::RateLimited
            }
            s if s.is_server_error() => {
                tracing::warn!("usage fetch server error: {s}");
                FetchOutcome::Transient(format!("status: {s}"))
            }
            other => {
                tracing::warn!("usage fetch unexpected status: {other}");
                FetchOutcome::Transient(format!("unexpected status: {other}"))
            }
        }
    }
}

/// Exponential backoff ladder: 1m, 2m, 4m, 8m, 16m, 30m (cap).
pub fn next_backoff(previous: Duration) -> Duration {
    let doubled = previous.saturating_mul(2);
    let cap = Duration::from_secs(30 * 60);
    if doubled > cap {
        cap
    } else {
        doubled.max(Duration::from_secs(60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_ladder() {
        let mut d = Duration::from_secs(60);
        d = next_backoff(d);
        assert_eq!(d, Duration::from_secs(120));
        d = next_backoff(d);
        assert_eq!(d, Duration::from_secs(240));
        d = next_backoff(d);
        assert_eq!(d, Duration::from_secs(480));
        d = next_backoff(d);
        assert_eq!(d, Duration::from_secs(960));
        d = next_backoff(d);
        assert_eq!(d, Duration::from_secs(1800));
        d = next_backoff(d);
        assert_eq!(d, Duration::from_secs(1800)); // cap
    }
}
