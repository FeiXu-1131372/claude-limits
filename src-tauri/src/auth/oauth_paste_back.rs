use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;
use zeroize::ZeroizeOnDrop;

pub const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
// Claude.ai-account login. Anthropic migrated off claude.ai/oauth/authorize
// and console.anthropic.com/v1/oauth/token to the claude.com / platform.claude.com
// hosts; the old URLs now return a generic "Invalid request format" page.
pub const AUTHORIZE_URL: &str = "https://claude.com/cai/oauth/authorize";
pub const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
pub const SCOPES: &str =
    "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload";
// Anthropic only issues long-lived (>1h) tokens for inference-only scope —
// matches `mode === 'setup-token'` in claude-code's ConsoleOAuthFlow.
pub const INFERENCE_ONLY_SCOPES: &str = "user:inference";
pub const LONG_LIVED_EXPIRES_IN_SECS: u64 = 365 * 24 * 60 * 60;

#[derive(Debug, Clone, ZeroizeOnDrop)]
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
    pub state: String,
}

pub fn generate_pkce() -> PkcePair {
    let mut verifier_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut verifier_bytes);
    let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

    let challenge_bytes = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(challenge_bytes);

    let mut state_bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut state_bytes);
    let state = URL_SAFE_NO_PAD.encode(state_bytes);

    PkcePair { verifier, challenge, state }
}

pub fn build_authorize_url(
    pkce: &PkcePair,
    redirect_uri: &str,
    inference_only: bool,
) -> Result<String> {
    let scope = if inference_only { INFERENCE_ONLY_SCOPES } else { SCOPES };
    let mut url = Url::parse(AUTHORIZE_URL)?;
    url.query_pairs_mut()
        .append_pair("code", "true")
        .append_pair("client_id", CLIENT_ID)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", scope)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &pkce.state);
    Ok(url.into())
}

/// Binds an ephemeral HTTP server on a random loopback port. Returns the port
/// and a receiver that resolves to `(code, state)` when the browser hits
/// `/callback`. Mirrors how Claude Code handles the OAuth redirect.
pub async fn start_local_callback_server(
) -> Result<(u16, tokio::sync::oneshot::Receiver<Result<(String, String)>>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let (tx, rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);
            let result = parse_callback_request(&request);

            let (status_line, body) = if result.is_ok() {
                ("200 OK", "<html><body><h2>Authorization complete — you can close this tab.</h2></body></html>")
            } else {
                ("400 Bad Request", "<html><body><h2>Authorization failed — please return to the app and try again.</h2></body></html>")
            };
            let response = format!(
                "HTTP/1.1 {status_line}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = tx.send(result);
        }
    });

    Ok((port, rx))
}

fn parse_callback_request(request: &str) -> Result<(String, String)> {
    // "GET /callback?code=X&state=Y HTTP/1.1"
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");

    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "code" => code = Some(v.to_string()),
                "state" => state = Some(v.to_string()),
                _ => {}
            }
        }
    }

    Ok((
        code.ok_or_else(|| anyhow!("Missing code in callback"))?,
        state.ok_or_else(|| anyhow!("Missing state in callback"))?,
    ))
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
        let redirect = "http://127.0.0.1:12345/callback";
        let url = build_authorize_url(&p, redirect, false).unwrap();
        assert!(url.contains("code=true"));
        assert!(url.contains("client_id=9d1c250a-e61b-44d9-88ed-5944d1962f5e"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&format!("state={}", p.state)));
        assert!(url.contains("127.0.0.1"));
        assert!(url.contains("user%3Aprofile"));
    }

    #[test]
    fn authorize_url_inference_only_uses_narrow_scope() {
        let p = generate_pkce();
        let url = build_authorize_url(&p, "http://127.0.0.1:1/callback", true).unwrap();
        // Long-lived tokens must request `user:inference` only — full-scope
        // long-lived tokens are rejected by Anthropic.
        assert!(url.contains("scope=user%3Ainference"));
        assert!(!url.contains("user%3Aprofile"));
        assert!(!url.contains("org%3Acreate_api_key"));
    }

    #[test]
    fn parse_callback_extracts_code_and_state() {
        let req = "GET /callback?code=abc123&state=xyz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let (code, state) = parse_callback_request(req).unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state, "xyz");
    }

    #[test]
    fn parse_callback_rejects_missing_code() {
        let req = "GET /callback?state=xyz HTTP/1.1\r\n\r\n";
        assert!(parse_callback_request(req).is_err());
    }
}
