pub mod account_identity;
pub mod claude_code_creds;
pub mod exchange;
pub mod oauth_paste_back;
pub mod orchestrator;
pub mod token_store;

pub use orchestrator::{AccountInfo, AuthError, AuthOrchestrator, AuthResult};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthSource {
    OAuth,
    ClaudeCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AccountId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
}
