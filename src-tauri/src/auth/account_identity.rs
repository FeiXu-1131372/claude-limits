use super::AccountId;
use anyhow::{anyhow, Result};
use serde::Deserialize;

pub const USERINFO_URL: &str = "https://api.anthropic.com/api/oauth/userinfo";
const ANTHROPIC_BETA: &str = "oauth-2025-04-20";

#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    #[serde(rename = "sub")]
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

pub struct IdentityFetcher {
    endpoint: String,
    client: reqwest::Client,
}

impl IdentityFetcher {
    pub fn new() -> Self {
        Self {
            endpoint: USERINFO_URL.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_endpoint(endpoint: String) -> Self {
        Self {
            endpoint,
            client: reqwest::Client::new(),
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
            return Err(anyhow!(
                "userinfo {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }
        Ok(resp.json().await?)
    }
}

impl From<&UserInfo> for AccountId {
    fn from(u: &UserInfo) -> Self {
        AccountId(u.id.clone())
    }
}
