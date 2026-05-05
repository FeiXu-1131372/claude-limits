use super::{
    account_identity::IdentityFetcher,
    exchange::TokenExchange,
    oauth_paste_back::PkcePair,
};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

pub struct AuthOrchestrator {
    pub fallback_dir: PathBuf,
    pub exchange: TokenExchange,
    pub identity: IdentityFetcher,
    pub pending_oauth: RwLock<Option<(PkcePair, Instant)>>,
}

impl AuthOrchestrator {
    pub fn new(fallback_dir: PathBuf, client: Arc<reqwest::Client>) -> Self {
        Self {
            fallback_dir,
            exchange: TokenExchange::new(client.clone()),
            identity: IdentityFetcher::new(client),
            pending_oauth: RwLock::new(None),
        }
    }

    pub fn with_collaborators(
        fallback_dir: PathBuf,
        exchange: TokenExchange,
        identity: IdentityFetcher,
    ) -> Self {
        Self {
            fallback_dir,
            exchange,
            identity,
            pending_oauth: RwLock::new(None),
        }
    }

    pub fn identity_arc(&self) -> std::sync::Arc<crate::auth::account_identity::IdentityFetcher> {
        std::sync::Arc::new(crate::auth::account_identity::IdentityFetcher::new(
            self.identity.client_arc(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct LiveClaudeCode {
    pub claude_code_oauth_blob: serde_json::Value,
    pub oauth_account_blob: serde_json::Value,
    pub account_uuid: String,
    pub email: String,
}

impl AuthOrchestrator {
    /// Read whatever upstream-CLI is currently logged into. Returns None when
    /// no CC creds are present OR when the global config has no `oauthAccount`.
    pub async fn read_live_claude_code(&self) -> anyhow::Result<Option<LiveClaudeCode>> {
        let cc_blob = match crate::auth::claude_code_creds::load_full_blob().await? {
            Some(b) => b,
            None => return Ok(None),
        };
        let global = match crate::auth::paths::claude_global_config() {
            Some(p) => p,
            None => return Ok(None),
        };
        let oauth_account = match crate::auth::oauth_account_io::read_oauth_account(&global)? {
            Some(s) => s,
            None => return Ok(None),
        };
        let account_uuid = oauth_account
            .get("accountUuid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("oauthAccount missing accountUuid"))?
            .to_string();
        let email = oauth_account
            .get("emailAddress")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(Some(LiveClaudeCode {
            claude_code_oauth_blob: cc_blob,
            oauth_account_blob: oauth_account,
            account_uuid,
            email,
        }))
    }

    /// Returns a usable access token for `slot`.
    /// - Active slot: read straight from CC's live store; never refresh.
    /// - Inactive slot: refresh if expiring within 2 minutes, persist back.
    pub async fn token_for_slot(
        &self,
        slot: u32,
        active_slot: Option<u32>,
        accounts: &crate::auth::accounts::AccountManager,
    ) -> anyhow::Result<String> {
        if Some(slot) == active_slot {
            let live = self
                .read_live_claude_code()
                .await?
                .ok_or_else(|| anyhow::anyhow!("active slot {slot} but no live CC creds"))?;
            return live
                .claude_code_oauth_blob
                .get("accessToken")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| anyhow::anyhow!("live CC blob missing accessToken"));
        }

        let acc = accounts
            .get(slot)?
            .ok_or_else(|| anyhow::anyhow!("slot {slot} not in store"))?;

        let needs_refresh =
            acc.token_expires_at <= chrono::Utc::now() + chrono::Duration::minutes(2);
        if needs_refresh {
            accounts.refresh_inactive(slot, &self.exchange).await?;
            let acc = accounts
                .get(slot)?
                .ok_or_else(|| anyhow::anyhow!("slot {slot} disappeared after refresh"))?;
            return acc
                .claude_code_oauth_blob
                .get("accessToken")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| anyhow::anyhow!("post-refresh blob missing accessToken"));
        }

        acc.claude_code_oauth_blob
            .get("accessToken")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("slot {slot} blob missing accessToken"))
    }
}
