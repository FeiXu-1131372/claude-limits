use crate::auth::oauth_paste_back::PkcePair;
use crate::auth::AuthOrchestrator;
use crate::jsonl_parser::PricingTable;
use crate::store::Db;
use crate::usage_api::{UsageClient, UsageSnapshot};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(default)]
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

/// Linear projection of where 5h utilization will land at the current
/// window's reset_at, based on observed slope so far this window. Borrowed
/// from ccusage's burn-rate idea — answers "should I keep coding?" with a
/// concrete number instead of just the bare current %. None when we don't
/// yet have enough samples (need at least 2 polls ≥ 2 minutes apart).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct BurnRateProjection {
    /// Slope of five_hour.utilization, in percentage points per minute.
    /// Positive means consumption is rising; negative is rare but possible
    /// if Anthropic adjusts the metric mid-window.
    pub utilization_per_min: f64,
    /// Projected utilization at five_hour.resets_at if the current pace
    /// continues. Not clamped — values >100 are meaningful (means you'd
    /// hit the cap before reset).
    pub projected_at_reset: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CachedUsage {
    pub snapshot: UsageSnapshot,
    pub account_id: String,
    pub account_email: String,
    pub last_error: Option<String>,
    #[serde(default)]
    pub burn_rate: Option<BurnRateProjection>,
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
    pub pending_oauth: RwLock<Option<(PkcePair, std::time::Instant)>>,
    pub fallback_dir: std::path::PathBuf,
    // Wakes the poll loop early when the user requests an immediate refresh.
    pub force_refresh: Notify,
    /// In-memory rolling history of (poll_time, five_hour.utilization)
    /// samples for the current 5h window. Used to compute the burn-rate
    /// projection. Trimmed to entries inside the live window on every
    /// successful poll, so its size is bounded by polling_interval × 5h.
    /// Resets on app restart — burn rate is unavailable until at least 2
    /// polls have completed (~2 minutes after launch with default config).
    pub recent_five_hour: RwLock<VecDeque<(DateTime<Utc>, f64)>>,
}

impl AppState {
    pub fn snapshot(&self) -> Option<CachedUsage> {
        self.cached_usage.read().clone()
    }
}
