//! First-launch migration from the legacy single-account `credentials.json`
//! (and optionally CC's live store) into multi-account `accounts.json`.
//!
//! Behavior:
//!   - If `accounts.json` already exists (with any accounts) → no-op.
//!   - Otherwise, attempt to import each present source. Each successful
//!     import becomes a slot. Dedup by accountUuid handled by `upsert`.
//!   - On identity-fetch failure, leave the legacy file in place and return
//!     the slot list (empty) so the caller can show a "retry on next launch"
//!     banner.
//!   - On full success, delete the legacy `credentials.json`.

use super::{store, AccountManager};
use crate::auth::{account_identity::IdentityFetcher, claude_code_creds, StoredToken};
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct MigrationReport {
    pub imported_slots: Vec<u32>,
    pub had_legacy_oauth: bool,
    pub had_live_cc: bool,
    pub errors: Vec<String>,
}

/// Returns the report. The caller decides whether to emit a UI event.
pub async fn migrate_legacy(
    data_dir: &Path,
    identity: Arc<IdentityFetcher>,
) -> Result<MigrationReport> {
    let existing = store::load(data_dir)?;
    if !existing.accounts.is_empty() {
        return Ok(MigrationReport::default());
    }

    let mgr = AccountManager::new(data_dir.to_path_buf());
    let mut report = MigrationReport::default();

    // 1. Legacy OAuth token at <data_dir>/credentials.json
    let legacy_path = data_dir.join("credentials.json");
    if legacy_path.exists() {
        report.had_legacy_oauth = true;
        match import_legacy_oauth(&legacy_path, &identity, &mgr).await {
            Ok(slot) => report.imported_slots.push(slot),
            Err(e) => report.errors.push(format!("legacy oauth: {e}")),
        }
    }

    // 2. Live upstream-CLI credentials
    if claude_code_creds::has_creds().await {
        report.had_live_cc = true;
        match mgr.add_from_claude_code().await {
            Ok(slot) => {
                if !report.imported_slots.contains(&slot) {
                    report.imported_slots.push(slot);
                }
            }
            Err(e) => report.errors.push(format!("live cc: {e}")),
        }
    }

    // Only delete the legacy file when the import landed cleanly.
    if report.errors.is_empty() && report.had_legacy_oauth {
        let _ = std::fs::remove_file(&legacy_path);
    }

    Ok(report)
}

async fn import_legacy_oauth(
    path: &Path,
    identity: &IdentityFetcher,
    mgr: &AccountManager,
) -> Result<u32> {
    let text = std::fs::read_to_string(path)?;
    let token: StoredToken = serde_json::from_str(&text)?;
    let userinfo = identity.fetch(&token.access_token).await?;
    mgr.add_from_oauth(token, userinfo).await
}
