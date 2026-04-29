use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use url::Url;
use zeroize::ZeroizeOnDrop;

pub const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub const AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
pub const TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";
pub const REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";
pub const SCOPES: &str = "user:profile user:inference";

#[derive(Debug, Clone, ZeroizeOnDrop)]
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
    pub state: String,
}

pub fn generate_pkce() -> PkcePair {
    let mut verifier_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut verifier_bytes);
    let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

    let challenge_bytes = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(challenge_bytes);

    let mut state_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut state_bytes);
    let state = URL_SAFE_NO_PAD.encode(state_bytes);

    PkcePair { verifier, challenge, state }
}

pub fn build_authorize_url(pkce: &PkcePair) -> Result<String> {
    // The `code=true` parameter we used to append is non-standard and causes
    // claude.ai's authorize endpoint to reject the request as "Invalid OAuth
    // Request" / "Invalid request format" — see anthropics/claude-code#29983.
    let mut url = Url::parse(AUTHORIZE_URL)?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", CLIENT_ID)
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("scope", SCOPES)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &pkce.state);
    Ok(url.into())
}

/// Parses "code#state" as rendered on Anthropic's callback page.
pub fn parse_pasted_code(pasted: &str, expected_state: &str) -> Result<String> {
    let trimmed = pasted.trim();
    let (code, state) = trimmed
        .split_once('#')
        .ok_or_else(|| anyhow!("Missing '#state' suffix"))?;
    if state != expected_state {
        return Err(anyhow!("State mismatch: possible replay or mis-paste"));
    }
    if code.is_empty() {
        return Err(anyhow!("Code is empty"));
    }
    Ok(code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_verifier_and_challenge_are_distinct() {
        let p = generate_pkce();
        assert_ne!(p.verifier, p.challenge);
        assert!(p.state.len() >= 16);
    }

    #[test]
    fn authorize_url_contains_expected_params() {
        let p = generate_pkce();
        let url = build_authorize_url(&p).unwrap();
        assert!(url.contains("client_id=9d1c250a-e61b-44d9-88ed-5944d1962f5e"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&format!("state={}", p.state)));
        // code=true is non-standard and was causing claude.ai to reject the
        // request — make sure it stays out (anthropics/claude-code#29983).
        assert!(!url.contains("code=true"));
    }

    #[test]
    fn parse_rejects_missing_hash() {
        let err = parse_pasted_code("abcd", "st1").unwrap_err();
        assert!(err.to_string().contains("Missing"));
    }

    #[test]
    fn parse_rejects_state_mismatch() {
        let err = parse_pasted_code("code#bad", "st1").unwrap_err();
        assert!(err.to_string().contains("State"));
    }

    #[test]
    fn parse_accepts_valid_pasted_code() {
        let code = parse_pasted_code("abc123#st1", "st1").unwrap();
        assert_eq!(code, "abc123");
    }
}
