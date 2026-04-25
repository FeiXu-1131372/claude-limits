use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Utilization {
    pub utilization: f64,                // 0..100
    pub resets_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtraUsage {
    pub is_enabled: bool,
    #[serde(default)]
    pub monthly_limit_cents: u64,
    #[serde(default)]
    pub used_credits_cents: u64,
    #[serde(default)]
    pub utilization: f64,
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageSnapshot {
    pub five_hour: Option<Utilization>,
    pub seven_day: Option<Utilization>,
    pub seven_day_sonnet: Option<Utilization>,
    pub seven_day_opus: Option<Utilization>,
    pub extra_usage: Option<ExtraUsage>,

    #[serde(default = "Utc::now", skip_serializing)]
    pub fetched_at: DateTime<Utc>,

    #[serde(flatten, default)]
    pub unknown: HashMap<String, serde_json::Value>,
}
