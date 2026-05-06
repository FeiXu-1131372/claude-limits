use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
pub struct Utilization {
    pub utilization: f64,
    // The API returns `resets_at: null` for buckets that exist with
    // `utilization: 0.0` but have no active window yet (e.g. an unused
    // `seven_day_sonnet`). Keeping it required would fail the whole snapshot
    // decode whenever any sibling bucket is idle.
    #[serde(default)]
    #[specta(type = Option<String>)]
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
pub struct ExtraUsage {
    #[serde(default)]
    pub is_enabled: bool,
    #[serde(default)]
    pub monthly_limit_cents: u64,
    #[serde(default)]
    pub used_credits_cents: u64,
    // Sent as `null` when extra usage is not enabled; `#[serde(default)]`
    // alone would not cover that since the field is present-but-null.
    #[serde(default)]
    pub utilization: Option<f64>,
    #[serde(default)]
    #[specta(type = Option<String>)]
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
pub struct UsageSnapshot {
    pub five_hour: Option<Utilization>,
    pub seven_day: Option<Utilization>,
    pub seven_day_sonnet: Option<Utilization>,
    pub seven_day_opus: Option<Utilization>,
    pub extra_usage: Option<ExtraUsage>,

    #[serde(default = "Utc::now")]
    #[specta(type = String)]
    pub fetched_at: DateTime<Utc>,

    // Carries forward unknown fields from the wire so we never reject a payload
    // that the API extends; not exposed to the frontend.
    #[serde(flatten, default, skip_serializing)]
    #[specta(skip)]
    pub unknown: HashMap<String, serde_json::Value>,
}
