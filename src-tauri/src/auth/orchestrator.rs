use super::{
    account_identity::IdentityFetcher, claude_code_creds, exchange::TokenExchange, token_store,
    AccountId, AuthSource, StoredToken,
};
use chrono::{Duration, Utc};
use std::path::PathBuf;
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

pub struct AuthOrchestrator {
    pub fallback_dir: PathBuf,
    pub exchange: TokenExchange,
    pub identity: IdentityFetcher,
    pub preferred_source: Mutex<Option<AuthSource>>,
}

impl AuthOrchestrator {
    pub fn new(fallback_dir: PathBuf) -> Self {
        Self {
            fallback_dir,
            exchange: TokenExchange::new(),
            identity: IdentityFetcher::new(),
            preferred_source: Mutex::new(None),
        }
    }

    pub async fn get_access_token(&self) -> AuthResult<(String, AuthSource, AccountInfo)> {
        let preferred = *self.preferred_source.lock().await;

        let token_oauth = token_store::load(&self.fallback_dir).map_err(AuthError::from)?;
        let token_cli = claude_code_creds::load().map_err(AuthError::from)?;

        match (token_oauth, token_cli, preferred) {
            (Some(t), None, _) => {
                let refreshed = self.refresh_if_needed(t).await?;
                self.finalize(refreshed, AuthSource::OAuth).await
            }
            (None, Some(t), _) => self.finalize(t, AuthSource::ClaudeCode).await,
            (None, None, _) => Err(AuthError::NoSource),
            (Some(a), Some(b), Some(pref)) => {
                let chosen = if pref == AuthSource::OAuth {
                    (a, AuthSource::OAuth)
                } else {
                    (b, AuthSource::ClaudeCode)
                };
                let refreshed = if chosen.1 == AuthSource::OAuth {
                    self.refresh_if_needed(chosen.0).await?
                } else {
                    chosen.0
                };
                self.finalize(refreshed, chosen.1).await
            }
            (Some(oauth_tok), Some(cli_tok), None) => {
                let oauth_info = self
                    .identity
                    .fetch(&oauth_tok.access_token)
                    .await
                    .map_err(AuthError::from)?;
                let cli_info = self
                    .identity
                    .fetch(&cli_tok.access_token)
                    .await
                    .map_err(AuthError::from)?;
                if oauth_info.id == cli_info.id {
                    let refreshed = self.refresh_if_needed(oauth_tok).await?;
                    self.finalize(refreshed, AuthSource::OAuth).await
                } else {
                    Err(AuthError::Conflict {
                        oauth_email: oauth_info.email,
                        cli_email: cli_info.email,
                    })
                }
            }
        }
    }

    pub async fn set_preferred_source(&self, src: AuthSource) {
        *self.preferred_source.lock().await = Some(src);
    }

    async fn refresh_if_needed(&self, tok: StoredToken) -> AuthResult<StoredToken> {
        if tok.expires_at > Utc::now() + Duration::minutes(2) {
            return Ok(tok);
        }
        let refresh = tok.refresh_token.clone().ok_or(AuthError::NoRefreshToken)?;
        let new_tok = self.exchange.refresh(&refresh).await.map_err(AuthError::from)?;
        token_store::save(&new_tok, &self.fallback_dir).map_err(AuthError::from)?;
        Ok(new_tok)
    }

    async fn finalize(
        &self,
        tok: StoredToken,
        source: AuthSource,
    ) -> AuthResult<(String, AuthSource, AccountInfo)> {
        let info = self
            .identity
            .fetch(&tok.access_token)
            .await
            .map_err(AuthError::from)?;
        let acc = AccountInfo {
            id: (&info).into(),
            email: info.email,
            display_name: info.name,
        };
        Ok((tok.access_token, source, acc))
    }
}
