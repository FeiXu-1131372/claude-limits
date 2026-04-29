use super::AccountId;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::sync::Arc;

pub const USERINFO_URL: &str = "https://api.anthropic.com/api/oauth/userinfo";
const ANTHROPIC_BETA: &str = "oauth-2025-04-20";

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UserInfo {
    #[serde(rename = "sub")]
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

pub struct IdentityFetcher {
    endpoint: String,
    client: Arc<reqwest::Client>,
}

impl IdentityFetcher {
    pub fn new(client: Arc<reqwest::Client>) -> Self {
        Self {
            endpoint: USERINFO_URL.to_string(),
            client,
        }
    }

    /// Test-only constructor: builds a fresh client pointed at a mock endpoint.
    pub fn with_endpoint(endpoint: String) -> Self {
        Self {
            endpoint,
            client: Arc::new(reqwest::Client::new()),
        }
    }

    pub async fn fetch(&self, access_token: &str) -> Result<UserInfo> {
        let resp = self
            .client
            .get(&self.endpoint)
            .bearer_auth(access_token)
            .header("anthropic-beta", ANTHROPIC_BETA)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            tracing::debug!("userinfo error body: {text}");
            return Err(anyhow!("userinfo failed: {status}"));
        }
        Ok(resp.json().await?)
    }
}

impl From<&UserInfo> for AccountId {
    fn from(u: &UserInfo) -> Self {
        AccountId(u.id.clone())
    }
}
