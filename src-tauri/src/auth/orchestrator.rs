use super::{
    account_identity::{IdentityFetcher, UserInfo}, claude_code_creds, exchange::TokenExchange,
    oauth_paste_back::PkcePair, token_store, AccountId, AuthSource, StoredToken,
};
use chrono::{Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("no auth source configured")]
    NoSource,
    #[error(
        "two Claude accounts detected: {oauth_email} (OAuth) vs {cli_email} (Claude Code)"
    )]
    Conflict {
        oauth_email: String,
        cli_email: String,
    },
    #[error("no refresh token available")]
    NoRefreshToken,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type AuthResult<T> = std::result::Result<T, AuthError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct AccountInfo {
    pub id: AccountId,
    pub email: String,
    pub display_name: Option<String>,
}

const IDENTITY_CACHE_TTL: StdDuration = StdDuration::from_secs(3600);

pub struct AuthOrchestrator {
    pub fallback_dir: PathBuf,
    pub exchange: TokenExchange,
    pub identity: IdentityFetcher,
    pub preferred_source: Mutex<Option<AuthSource>>,
    identity_cache: Mutex<HashMap<AuthSource, (UserInfo, Instant)>>,
    /// In-flight PKCE session for the OAuth paste-back flow.  Set by
    /// `start_oauth_flow`, consumed by `submit_oauth_code`, and cleared by
    /// `sign_out`.  Stored here (not in `AppState`) because it is purely an
    /// auth-layer concern.
    pub pending_oauth: RwLock<Option<(PkcePair, Instant)>>,
}

impl AuthOrchestrator {
    pub fn new(
        fallback_dir: PathBuf,
        preferred_source: Option<AuthSource>,
        client: Arc<reqwest::Client>,
    ) -> Self {
        Self {
            fallback_dir,
            exchange: TokenExchange::new(client.clone()),
            identity: IdentityFetcher::new(client),
            preferred_source: Mutex::new(preferred_source),
            identity_cache: Mutex::new(HashMap::new()),
            pending_oauth: RwLock::new(None),
        }
    }

    /// Construct with explicitly provided collaborators.  Intended for tests
    /// that need to inject mock-endpoint `TokenExchange` / `IdentityFetcher`
    /// instances built via their `with_endpoint` constructors.
    pub fn with_collaborators(
        fallback_dir: PathBuf,
        preferred_source: Option<AuthSource>,
        exchange: TokenExchange,
        identity: IdentityFetcher,
    ) -> Self {
        Self {
            fallback_dir,
            exchange,
            identity,
            preferred_source: Mutex::new(preferred_source),
            identity_cache: Mutex::new(HashMap::new()),
            pending_oauth: RwLock::new(None),
        }
    }

    pub async fn get_access_token(&self) -> AuthResult<(String, AuthSource, AccountInfo)> {
        let preferred = *self.preferred_source.lock().await;

        let token_oauth = token_store::load(&self.fallback_dir).map_err(AuthError::from)?;
        let token_cli = claude_code_creds::load().await.map_err(AuthError::from)?;

        match (token_oauth, token_cli, preferred) {
            (Some(t), None, _) => {
                let refreshed = self.refresh_if_needed(t, AuthSource::OAuth).await?;
                self.finalize(refreshed, AuthSource::OAuth).await
            }
            (None, Some(t), _) => {
                let refreshed = self.refresh_if_needed(t, AuthSource::ClaudeCode).await?;
                self.finalize(refreshed, AuthSource::ClaudeCode).await
            }
            (None, None, _) => Err(AuthError::NoSource),
            (Some(a), Some(b), Some(pref)) => {
                let chosen = if pref == AuthSource::OAuth {
                    (a, AuthSource::OAuth)
                } else {
                    (b, AuthSource::ClaudeCode)
                };
                // Refresh regardless of source — both OAuth and ClaudeCode
                // tokens can be refreshed via the OAuth token endpoint.
                let refreshed = self.refresh_if_needed(chosen.0, chosen.1).await?;
                self.finalize(refreshed, chosen.1).await
            }
            (Some(oauth_tok), Some(cli_tok), None) => {
                // If the CLI file token is already expired we must not try to
                // refresh it here: its refresh token was rotated the last time
                // the app refreshed it into the keyring, so calling refresh
                // again would yield `invalid_grant`.  Skip the conflict check
                // and trust the keyring (OAuth) token which is already valid.
                if cli_tok.expires_at <= Utc::now() {
                    let refreshed = self.refresh_if_needed(oauth_tok, AuthSource::OAuth).await?;
                    return self.finalize(refreshed, AuthSource::OAuth).await;
                }
                // Both tokens are still live — refresh both in case either is
                // close to expiry, then compare identities to detect conflicts.
                let refreshed_oauth = self.refresh_if_needed(oauth_tok, AuthSource::OAuth).await?;
                let refreshed_cli = self.refresh_if_needed(cli_tok, AuthSource::ClaudeCode).await?;
                match (
                    self.fetch_identity(AuthSource::OAuth, &refreshed_oauth.access_token).await,
                    self.fetch_identity(AuthSource::ClaudeCode, &refreshed_cli.access_token).await,
                ) {
                    (Ok(oauth_info), Ok(cli_info)) if oauth_info.id != cli_info.id => {
                        // Confirmed two different accounts — surface the conflict.
                        Err(AuthError::Conflict {
                            oauth_email: oauth_info.email,
                            cli_email: cli_info.email,
                        })
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        // One or both identity fetches failed (e.g. 404 for
                        // Claude Code tokens).  We cannot confirm a conflict, so
                        // prefer the keyring token and log a warning.
                        tracing::warn!(
                            "could not verify both account identities; \
                             defaulting to OAuth keyring token: {e}"
                        );
                        self.finalize(refreshed_oauth, AuthSource::OAuth).await
                    }
                    _ => {
                        // Same account (or one returned Ok and ids match).
                        self.finalize(refreshed_oauth, AuthSource::OAuth).await
                    }
                }
            }
        }
    }

    pub async fn set_preferred_source(&self, src: AuthSource) {
        *self.preferred_source.lock().await = Some(src);
    }

    async fn fetch_identity(&self, source: AuthSource, token: &str) -> anyhow::Result<UserInfo> {
        {
            let cache = self.identity_cache.lock().await;
            if let Some((info, fetched_at)) = cache.get(&source) {
                if fetched_at.elapsed() < IDENTITY_CACHE_TTL {
                    return Ok(info.clone());
                }
            }
        }
        let info = self.identity.fetch(token).await?;
        self.identity_cache.lock().await.insert(source, (info.clone(), Instant::now()));
        Ok(info)
    }

    async fn refresh_if_needed(
        &self,
        tok: StoredToken,
        source: AuthSource,
    ) -> AuthResult<StoredToken> {
        if tok.expires_at > Utc::now() + Duration::minutes(2) {
            return Ok(tok);
        }
        let refresh = tok.refresh_token.clone().ok_or(AuthError::NoRefreshToken)?;
        let new_tok = self.exchange.refresh(&refresh).await.map_err(AuthError::from)?;
        // Only persist to our keyring entry when the source is OAuth — CLI
        // tokens belong to Claude Code's keychain and writing to ours would
        // (a) prompt the user for keychain access on macOS and
        // (b) overwrite any existing OAuth token sharing the slot.
        if source == AuthSource::OAuth {
            token_store::save(&new_tok, &self.fallback_dir).await.map_err(AuthError::from)?;
        }
        Ok(new_tok)
    }

    async fn finalize(
        &self,
        tok: StoredToken,
        source: AuthSource,
    ) -> AuthResult<(String, AuthSource, AccountInfo)> {
        // The userinfo endpoint is only needed for the two-account conflict
        // chooser (handled in the conflict branch above) and cosmetic display
        // in Settings.  It consistently returns 404 for Claude Code-origin
        // tokens, so we skip it here and use a placeholder instead.
        // If the Settings panel ever needs real account info, it can fetch it
        // on demand via a separate command.
        let acc = AccountInfo {
            id: AccountId(format!("unknown-{:?}", source)),
            email: String::new(),
            display_name: None,
        };
        Ok((tok.access_token.clone(), source, acc))
    }
}
