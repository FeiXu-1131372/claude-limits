use crate::auth::oauth_paste_back::PkcePair;
use crate::auth::AuthOrchestrator;
use crate::jsonl_parser::PricingTable;
use crate::store::Db;
use crate::usage_api::{UsageClient, UsageSnapshot};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub polling_interval_secs: u64,
    pub thresholds: Vec<u8>,
    pub theme: String,
    pub launch_at_login: bool,
    pub crash_reports: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            polling_interval_secs: 300,
            thresholds: vec![75, 90],
            theme: "system".into(),
            launch_at_login: false,
            crash_reports: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedUsage {
    pub snapshot: UsageSnapshot,
    pub account_id: String,
    pub account_email: String,
    pub last_error: Option<String>,
}

impl CachedUsage {
    pub fn is_stale(&self, now: DateTime<Utc>) -> bool {
        (now - self.snapshot.fetched_at) > chrono::Duration::minutes(15)
            || now < self.snapshot.fetched_at
            || self.last_error.is_some()
    }
}

pub struct AppState {
    pub db: Arc<Db>,
    pub auth: Arc<AuthOrchestrator>,
    pub usage: Arc<UsageClient>,
    pub pricing: Arc<PricingTable>,
    pub settings: RwLock<Settings>,
    pub cached_usage: RwLock<Option<CachedUsage>>,
    pub pending_oauth: RwLock<Option<PkcePair>>,
    pub fallback_dir: std::path::PathBuf,
}

impl AppState {
    pub fn snapshot(&self) -> Option<CachedUsage> {
        self.cached_usage.read().clone()
    }
}
