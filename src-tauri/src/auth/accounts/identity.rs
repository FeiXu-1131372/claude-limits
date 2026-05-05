//! Extract identity fields from the captured `oauthAccount` blob, with
//! fallback to userinfo for OAuth-added accounts whose blob lacks them.

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountIdentity {
    pub email: String,
    pub account_uuid: String,
    pub organization_uuid: Option<String>,
    pub organization_name: Option<String>,
    pub subscription_type: Option<String>,
}

/// Extract identity from `oauthAccount` slice. `subscription_type` is
/// usually only present on the `claudeAiOauth` blob, so accept it as
/// a separate optional input when known.
pub fn from_blobs(
    oauth_account: &serde_json::Value,
    claude_code_oauth: Option<&serde_json::Value>,
) -> Result<AccountIdentity> {
    let email = oauth_account
        .get("emailAddress")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing oauthAccount.emailAddress"))?
        .to_string();
    let account_uuid = oauth_account
        .get("accountUuid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing oauthAccount.accountUuid"))?
        .to_string();
    let organization_uuid = oauth_account
        .get("organizationUuid")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let organization_name = oauth_account
        .get("organizationName")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let subscription_type = claude_code_oauth
        .and_then(|b| b.get("subscriptionType"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(AccountIdentity {
        email,
        account_uuid,
        organization_uuid,
        organization_name,
        subscription_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_personal_account() {
        let oauth_account = serde_json::json!({
            "accountUuid": "uuid-1",
            "emailAddress": "me@x.com",
            "organizationUuid": null,
            "organizationName": null
        });
        let cc = serde_json::json!({ "subscriptionType": "pro" });
        let id = from_blobs(&oauth_account, Some(&cc)).unwrap();
        assert_eq!(id.email, "me@x.com");
        assert_eq!(id.account_uuid, "uuid-1");
        assert_eq!(id.organization_uuid, None);
        assert_eq!(id.subscription_type.as_deref(), Some("pro"));
    }

    #[test]
    fn extracts_org_account() {
        let oauth_account = serde_json::json!({
            "accountUuid": "uuid-2",
            "emailAddress": "alice@acme.com",
            "organizationUuid": "org-1",
            "organizationName": "Acme"
        });
        let id = from_blobs(&oauth_account, None).unwrap();
        assert_eq!(id.organization_uuid.as_deref(), Some("org-1"));
        assert_eq!(id.organization_name.as_deref(), Some("Acme"));
        assert_eq!(id.subscription_type, None);
    }

    #[test]
    fn missing_required_fields_errors() {
        let bad = serde_json::json!({ "emailAddress": "x@x.com" });
        assert!(from_blobs(&bad, None).is_err());
    }
}
