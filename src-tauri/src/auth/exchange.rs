use super::oauth_paste_back::{CLIENT_ID, REDIRECT_URI, TOKEN_URL};
use super::StoredToken;
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    #[allow(dead_code)]
    token_type: Option<String>,
}

pub struct TokenExchange {
    endpoint: String,
    client: reqwest::Client,
}

impl TokenExchange {
    pub fn new() -> Self {
        Self {
            endpoint: TOKEN_URL.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_endpoint(endpoint: String) -> Self {
        Self {
            endpoint,
            client: reqwest::Client::new(),
        }
    }

    pub async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> Result<StoredToken, anyhow::Error> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", CLIENT_ID),
            ("code_verifier", pkce_verifier),
        ];
        let resp = self.client.post(&self.endpoint).form(&params).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("token exchange failed: {status}: {text}"));
        }
        let tr: TokenResponse = resp.json().await?;
        Ok(StoredToken {
            access_token: tr.access_token,
            refresh_token: tr.refresh_token,
            expires_at: Utc::now() + Duration::seconds(tr.expires_in),
        })
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<StoredToken, anyhow::Error> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", CLIENT_ID),
        ];
        let resp = self.client.post(&self.endpoint).form(&params).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("refresh failed: {status}: {text}"));
        }
        let tr: TokenResponse = resp.json().await?;
        Ok(StoredToken {
            access_token: tr.access_token,
            refresh_token: tr.refresh_token.or_else(|| Some(refresh_token.to_string())),
            expires_at: Utc::now() + Duration::seconds(tr.expires_in),
        })
    }
}
