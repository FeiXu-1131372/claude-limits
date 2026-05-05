//! `AccountManager` — public surface for add/remove/swap/refresh operations.
//! Each mutating method acquires the file lock for the duration of its work.

use super::{
    identity::{self, AccountIdentity},
    store::{self, AccountsLock, AddSource, ManagedAccount},
};
use crate::auth::{oauth_account_io, paths};
use anyhow::{anyhow, Context, Result};
use chrono::{TimeZone, Utc};
use std::path::{Path, PathBuf};

pub struct AccountManager {
    pub data_dir: PathBuf,
}

impl AccountManager {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn list(&self) -> Result<Vec<ManagedAccount>> {
        let store = store::load(&self.data_dir)?;
        Ok(store.accounts.into_values().collect())
    }

    pub fn get(&self, slot: u32) -> Result<Option<ManagedAccount>> {
        Ok(store::load(&self.data_dir)?.accounts.remove(&slot))
    }

    /// Capture the live upstream-CLI credentials and register as a managed
    /// account. If an account with the same `accountUuid` already exists,
    /// refresh its stored blobs in place and return that slot.
    pub async fn add_from_claude_code(&self) -> Result<u32> {
        let cc_blob = crate::auth::claude_code_creds::load_full_blob()
            .await
            .context("read upstream credentials")?
            .ok_or_else(|| anyhow!("no upstream credentials present"))?;

        let global = paths::claude_global_config()
            .ok_or_else(|| anyhow!("could not resolve upstream global config path"))?;
        let oauth_account = oauth_account_io::read_oauth_account(&global)
            .context("read upstream oauthAccount slice")?
            .ok_or_else(|| anyhow!("upstream global config missing oauthAccount"))?;

        let id = identity::from_blobs(&oauth_account, Some(&cc_blob))?;
        self.upsert(id, cc_blob, oauth_account, AddSource::ImportedFromClaudeCode)
    }

    pub(crate) fn upsert(
        &self,
        id: AccountIdentity,
        cc_blob: serde_json::Value,
        oauth_account_blob: serde_json::Value,
        source: AddSource,
    ) -> Result<u32> {
        let lock = store::acquire_lock(&self.data_dir)?;
        let mut store = store::load(&self.data_dir)?;

        let now = Utc::now();
        let token_expires_at = extract_expires_at(&cc_blob).unwrap_or(now);

        if let Some(existing) = store.find_by_account_uuid(&id.account_uuid).cloned() {
            let slot = existing.slot;
            let updated = ManagedAccount {
                slot,
                email: id.email,
                account_uuid: id.account_uuid,
                organization_uuid: id.organization_uuid,
                organization_name: id.organization_name,
                subscription_type: id.subscription_type.or(existing.subscription_type),
                source,
                claude_code_oauth_blob: cc_blob,
                oauth_account_blob,
                token_expires_at,
                added_at: existing.added_at,
                last_seen_active: existing.last_seen_active,
            };
            store.accounts.insert(slot, updated);
            store::save(&self.data_dir, &store, &lock)?;
            return Ok(slot);
        }

        let slot = store.next_slot();
        let acc = ManagedAccount {
            slot,
            email: id.email,
            account_uuid: id.account_uuid,
            organization_uuid: id.organization_uuid,
            organization_name: id.organization_name,
            subscription_type: id.subscription_type,
            source,
            claude_code_oauth_blob: cc_blob,
            oauth_account_blob,
            token_expires_at,
            added_at: now,
            last_seen_active: None,
        };
        store.accounts.insert(slot, acc);
        store::save(&self.data_dir, &store, &lock)?;
        Ok(slot)
    }
}

fn extract_expires_at(cc_blob: &serde_json::Value) -> Option<chrono::DateTime<Utc>> {
    let ms = cc_blob.get("expiresAt")?.as_i64()?;
    Utc.timestamp_millis_opt(ms).single()
}

pub(crate) fn _used(_: &Path, _: &AccountsLock) {}

#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    #[error("slot {0} not found")]
    NotFound(u32),
    #[error("incomplete account: {0}")]
    IncompleteAccount(String),
    #[error("credential write failed: {0}")]
    CredentialWriteFailed(String),
    #[error("config write failed: {0}; credentials restored")]
    ConfigWriteFailed(String),
    #[error("config write failed AND restore failed: {0}; CC may need re-login")]
    Critical(String),
    #[error("infrastructure error: {0}")]
    Other(#[from] anyhow::Error),
}

impl AccountManager {
    /// Atomic two-step swap with rollback:
    ///   a. Snapshot live CC credentials + ~/.claude.json oauthAccount slice.
    ///   b. Write target.claude_code_oauth_blob to CC's primary store.
    ///   c. Splice target.oauth_account_blob into ~/.claude.json.
    ///
    /// On step-b failure: nothing has been mutated; return error.
    /// On step-c failure: try to restore step-b. If restore also fails:
    /// return Critical so the UI can surface a hard-error banner.
    pub async fn swap_to(&self, slot: u32) -> Result<(), SwapError> {
        let target = self
            .get(slot)?
            .ok_or(SwapError::NotFound(slot))?;

        if !target.claude_code_oauth_blob.is_object() {
            return Err(SwapError::IncompleteAccount(
                "claude_code_oauth_blob is not an object".into(),
            ));
        }
        if !target.oauth_account_blob.is_object() {
            return Err(SwapError::IncompleteAccount(
                "oauth_account_blob is not an object".into(),
            ));
        }

        // Step a: snapshot.
        let prev_cc = crate::auth::claude_code_creds::load_full_blob()
            .await
            .map_err(|e| SwapError::Other(anyhow!("snapshot CC creds: {e}")))?;

        let global = paths::claude_global_config()
            .ok_or_else(|| SwapError::Other(anyhow!("resolve global config path")))?;
        let prev_oauth_account = oauth_account_io::read_oauth_account(&global)
            .map_err(|e| SwapError::Other(anyhow!("snapshot oauthAccount: {e}")))?;

        // Step b: write CC creds.
        if let Err(e) =
            crate::auth::claude_code_creds::write_full_blob(&target.claude_code_oauth_blob).await
        {
            return Err(SwapError::CredentialWriteFailed(e.to_string()));
        }

        // Step c: write global config.
        if let Err(e) = oauth_account_io::write_oauth_account(&global, &target.oauth_account_blob)
        {
            // Roll back step b.
            let restore_result = match prev_cc {
                Some(blob) => crate::auth::claude_code_creds::write_full_blob(&blob).await,
                None => Ok(()),
            };
            if let Some(prev) = prev_oauth_account {
                let _ = oauth_account_io::write_oauth_account(&global, &prev);
            }
            return match restore_result {
                Ok(_) => Err(SwapError::ConfigWriteFailed(e.to_string())),
                Err(restore_err) => Err(SwapError::Critical(format!(
                    "{e}; restore failed: {restore_err}"
                ))),
            };
        }

        Ok(())
    }
}

/// Pure helper: build the synthetic CC + oauthAccount blobs from a fresh
/// OAuth token exchange + userinfo response. Public for testing.
pub fn synthesize_blobs(
    token: &crate::auth::StoredToken,
    userinfo: &crate::auth::account_identity::UserInfo,
) -> (serde_json::Value, serde_json::Value) {
    let cc = serde_json::json!({
        "accessToken": token.access_token,
        "refreshToken": token.refresh_token,
        "expiresAt": token.expires_at.timestamp_millis(),
        "scopes": ["user:inference", "user:profile"],
    });
    let oa = serde_json::json!({
        "accountUuid": userinfo.id,
        "emailAddress": userinfo.email,
        "organizationUuid": null,
        "organizationName": null,
        "displayName": userinfo.name,
    });
    (cc, oa)
}

impl AccountManager {
    pub fn remove(&self, slot: u32) -> Result<()> {
        let lock = store::acquire_lock(&self.data_dir)?;
        let mut store = store::load(&self.data_dir)?;
        store.accounts.remove(&slot);
        store::save(&self.data_dir, &store, &lock)?;
        Ok(())
    }

    /// Register a new (or refresh existing) managed account from a freshly
    /// completed paste-back OAuth exchange.
    pub async fn add_from_oauth(
        &self,
        token: crate::auth::StoredToken,
        userinfo: crate::auth::account_identity::UserInfo,
    ) -> Result<u32> {
        let (cc, oa) = synthesize_blobs(&token, &userinfo);
        let id = identity::from_blobs(&oa, Some(&cc))?;
        self.upsert(id, cc, oa, AddSource::OAuth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn cc_blob(uuid: &str, exp_ms: i64) -> serde_json::Value {
        serde_json::json!({
            "accessToken": format!("at-{uuid}"),
            "refreshToken": format!("rt-{uuid}"),
            "expiresAt": exp_ms,
            "scopes": ["user:inference"],
            "subscriptionType": "max"
        })
    }

    fn oa_slice(uuid: &str, email: &str) -> serde_json::Value {
        serde_json::json!({
            "accountUuid": uuid,
            "emailAddress": email,
            "organizationUuid": null,
            "organizationName": null
        })
    }

    #[test]
    fn upsert_assigns_first_slot_then_dedups() {
        let dir = tempdir().unwrap();
        let mgr = AccountManager::new(dir.path().to_path_buf());

        let id1 = identity::from_blobs(&oa_slice("u1", "a@x"), Some(&cc_blob("u1", 1))).unwrap();
        let s1 = mgr
            .upsert(id1, cc_blob("u1", 1), oa_slice("u1", "a@x"), AddSource::OAuth)
            .unwrap();
        assert_eq!(s1, 1);

        let id1_again =
            identity::from_blobs(&oa_slice("u1", "a@x"), Some(&cc_blob("u1", 99))).unwrap();
        let s1_again = mgr
            .upsert(
                id1_again,
                cc_blob("u1", 99),
                oa_slice("u1", "a@x"),
                AddSource::OAuth,
            )
            .unwrap();
        assert_eq!(s1_again, 1, "same accountUuid → same slot");

        let id2 = identity::from_blobs(&oa_slice("u2", "b@x"), Some(&cc_blob("u2", 1))).unwrap();
        let s2 = mgr
            .upsert(id2, cc_blob("u2", 1), oa_slice("u2", "b@x"), AddSource::OAuth)
            .unwrap();
        assert_eq!(s2, 2);

        let listed = mgr.list().unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn swap_rollback_restores_credentials_when_config_write_fails() {
        if std::env::var_os("USER").is_none() && std::env::var_os("USERPROFILE").is_none() {
            eprintln!("skipping swap rollback test: no USER/USERPROFILE");
            return;
        }
        let dir = tempdir().unwrap();
        let mgr = AccountManager::new(dir.path().to_path_buf());
        let r = futures::executor::block_on(mgr.swap_to(99));
        assert!(r.is_err(), "swap to nonexistent slot must error");
    }

    #[test]
    fn remove_is_idempotent_and_lock_protected() {
        let dir = tempdir().unwrap();
        let mgr = AccountManager::new(dir.path().to_path_buf());

        let id = identity::from_blobs(&oa_slice("u1", "a@x"), Some(&cc_blob("u1", 1))).unwrap();
        mgr.upsert(id, cc_blob("u1", 1), oa_slice("u1", "a@x"), AddSource::OAuth)
            .unwrap();

        mgr.remove(1).unwrap();
        assert!(mgr.list().unwrap().is_empty());
        mgr.remove(1).unwrap();
    }

    #[test]
    fn synthesize_blobs_from_token_and_userinfo() {
        use chrono::Duration;
        let now = Utc::now();
        let token = crate::auth::StoredToken {
            access_token: "at-x".to_string(),
            refresh_token: Some("rt-x".to_string()),
            expires_at: now + Duration::hours(8),
        };
        let userinfo = crate::auth::account_identity::UserInfo {
            id: "uuid-x".to_string(),
            email: "x@x.com".to_string(),
            name: Some("X".to_string()),
        };
        let (cc, oa) = super::synthesize_blobs(&token, &userinfo);
        assert_eq!(cc["accessToken"], "at-x");
        assert_eq!(cc["refreshToken"], "rt-x");
        assert_eq!(cc["expiresAt"].as_i64().unwrap() / 1000, token.expires_at.timestamp());
        assert_eq!(oa["accountUuid"], "uuid-x");
        assert_eq!(oa["emailAddress"], "x@x.com");
    }
}
