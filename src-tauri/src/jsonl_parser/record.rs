use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionEvent {
    pub ts: DateTime<Utc>,
    pub project: String,
    pub model: String,

    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub cache_creation_5m_tokens: u64,
    #[serde(default)]
    pub cache_creation_1h_tokens: u64,

    #[serde(default)]
    pub cost_usd: f64,

    #[serde(flatten, default)]
    pub unknown: HashMap<String, serde_json::Value>,
}
