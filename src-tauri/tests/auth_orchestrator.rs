use claude_limits_lib::auth::{
    account_identity::IdentityFetcher,
    exchange::TokenExchange,
    orchestrator::AuthOrchestrator,
    AuthError, AuthSource, StoredToken,
};
use chrono::{Duration, Utc};
use mockito::Server;
use tempfile::tempdir;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Write a `StoredToken` to the temp dir so `token_store::load` picks it up
/// via the fallback file (keyring is unset for the test service name in CI).
fn write_oauth_token(dir: &std::path::Path, tok: &StoredToken) {
    let path = dir.join("credentials.json");
    std::fs::write(path, serde_json::to_string(tok).unwrap()).unwrap();
}

fn valid_token(access: &str) -> StoredToken {
    StoredToken {
        access_token: access.to_string(),
        refresh_token: Some("refresh-xxx".to_string()),
        expires_at: Utc::now() + Duration::hours(1),
    }
}

fn expired_token(access: &str) -> StoredToken {
    StoredToken {
        access_token: access.to_string(),
        refresh_token: Some("refresh-old".to_string()),
        // expired 10 minutes ago — outside the 2-minute refresh window
        expires_at: Utc::now() - Duration::minutes(10),
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn no_sources_errors_with_typed_variant() {
    // This test only passes on machines without any Claude Code credentials
    // in the system keychain. If Claude Code is installed, the orchestrator
    // will find real credentials and attempt to use them.
    if claude_limits_lib::auth::claude_code_creds::has_creds().await {
        return; // skip: real Claude Code creds found in keychain
    }
    let dir = tempdir().unwrap();
    let exchange = TokenExchange::with_endpoint("http://127.0.0.1:1".to_string());
    let identity = IdentityFetcher::with_endpoint("http://127.0.0.1:1".to_string());
    let orc = AuthOrchestrator::with_collaborators(dir.path().to_path_buf(), None, exchange, identity);
    match orc.get_access_token().await {
        Err(AuthError::NoSource) => {}
        other => panic!("expected AuthError::NoSource, got {other:?}"),
    }
}

/// When only an OAuth token exists (no CC creds) and the token is still valid,
/// `get_access_token` should return it immediately without hitting the refresh
/// or userinfo endpoints.
#[tokio::test]
async fn oauth_only_valid_token_is_returned() {
    if claude_limits_lib::auth::claude_code_creds::has_creds().await {
        return; // skip: CC creds would give us two sources
    }

    let dir = tempdir().unwrap();
    write_oauth_token(dir.path(), &valid_token("access-abc"));

    // Exchange and identity endpoints must not be called — pointing them at an
    // unreachable address would panic if they were hit.
    let exchange = TokenExchange::with_endpoint("http://127.0.0.1:1".to_string());
    let identity = IdentityFetcher::with_endpoint("http://127.0.0.1:1".to_string());
    let orc =
        AuthOrchestrator::with_collaborators(dir.path().to_path_buf(), None, exchange, identity);

    let (token, source, _acc) = orc.get_access_token().await.expect("should succeed");
    assert_eq!(token, "access-abc");
    assert_eq!(source, AuthSource::OAuth);
}

/// When the OAuth token is expired, the orchestrator must call the token
/// endpoint to refresh it and return the new access token.
#[tokio::test]
async fn expired_oauth_token_is_refreshed() {
    if claude_limits_lib::auth::claude_code_creds::has_creds().await {
        return; // skip: CC creds would complicate the source resolution
    }

    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("grant_type".into(), "refresh_token".into()),
            mockito::Matcher::UrlEncoded("refresh_token".into(), "refresh-old".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"access_token":"refreshed-token","refresh_token":"refresh-new","expires_in":3600}"#,
        )
        .create_async()
        .await;

    let dir = tempdir().unwrap();
    write_oauth_token(dir.path(), &expired_token("stale-token"));

    let exchange = TokenExchange::with_endpoint(server.url());
    let identity = IdentityFetcher::with_endpoint("http://127.0.0.1:1".to_string());
    let orc =
        AuthOrchestrator::with_collaborators(dir.path().to_path_buf(), None, exchange, identity);

    let (token, source, _acc) = orc.get_access_token().await.expect("refresh should succeed");
    assert_eq!(token, "refreshed-token", "should return the refreshed access token");
    assert_eq!(source, AuthSource::OAuth);
}

/// When both sources are present and `preferred_source = OAuth`, the
/// orchestrator must use the OAuth token without raising a conflict error,
/// provided the identity fetcher confirms same-account (or succeeds with any
/// identity — since preferred_source skips the conflict check entirely).
///
/// This test only runs on machines where Claude Code credentials exist in the
/// keychain (otherwise there is only one source and preferred_source has no
/// effect on routing).
#[tokio::test]
async fn preferred_source_oauth_skips_conflict_check() {
    if !claude_limits_lib::auth::claude_code_creds::has_creds().await {
        return; // skip: need two sources for this test to be meaningful
    }

    let mut token_server = Server::new_async().await;
    // Refresh endpoint — OAuth token is already valid, but set up the mock in
    // case it is close to expiry on the test machine.
    let _refresh_mock = token_server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"access_token":"oauth-fresh","refresh_token":"r2","expires_in":3600}"#,
        )
        .create_async()
        .await;

    let dir = tempdir().unwrap();
    write_oauth_token(dir.path(), &valid_token("oauth-access"));

    let exchange = TokenExchange::with_endpoint(token_server.url());
    // Identity fetcher is not called when preferred_source is set — point it
    // at an unreachable address to verify it stays idle.
    let identity = IdentityFetcher::with_endpoint("http://127.0.0.1:1".to_string());
    let orc = AuthOrchestrator::with_collaborators(
        dir.path().to_path_buf(),
        Some(AuthSource::OAuth),
        exchange,
        identity,
    );

    let (_token, source, _acc) = orc.get_access_token().await.expect("should succeed");
    assert_eq!(source, AuthSource::OAuth);
}

/// When both sources are present, no preferred_source is set, and the identity
/// fetcher returns the same `sub` for both tokens, the orchestrator should
/// succeed (same account, no conflict).
#[tokio::test]
async fn same_account_both_sources_no_conflict() {
    if !claude_limits_lib::auth::claude_code_creds::has_creds().await {
        return; // skip: need CC source for this test
    }

    let mut identity_server = Server::new_async().await;
    // Both OAuth and CC identity calls hit the same mock URL; return same sub.
    let _id_mock = identity_server
        .mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"sub":"user-123","email":"dev@example.com"}"#)
        .expect(2) // called once for OAuth token, once for CC token
        .create_async()
        .await;

    let mut token_server = Server::new_async().await;
    let _refresh_mock = token_server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"access_token":"oauth-fresh","refresh_token":"r2","expires_in":3600}"#,
        )
        .create_async()
        .await;

    let dir = tempdir().unwrap();
    write_oauth_token(dir.path(), &valid_token("oauth-access"));

    let exchange = TokenExchange::with_endpoint(token_server.url());
    let identity = IdentityFetcher::with_endpoint(identity_server.url());
    let orc =
        AuthOrchestrator::with_collaborators(dir.path().to_path_buf(), None, exchange, identity);

    // Should not error — same account detected.
    let result = orc.get_access_token().await;
    assert!(
        result.is_ok(),
        "same-account both-source should not conflict: {result:?}"
    );
}

/// When both sources are present, no preferred_source is set, and the identity
/// fetcher returns different `sub` values for the two tokens, the orchestrator
/// must surface `AuthError::Conflict`.
#[tokio::test]
async fn different_accounts_both_sources_surfaces_conflict() {
    if !claude_limits_lib::auth::claude_code_creds::has_creds().await {
        return; // skip: need CC source for this test
    }

    let mut identity_server = Server::new_async().await;

    // First call (OAuth token) → account A
    let _id_oauth = identity_server
        .mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"sub":"user-AAA","email":"alice@example.com"}"#)
        .expect(1)
        .create_async()
        .await;

    // Second call (CC token) → account B
    let _id_cc = identity_server
        .mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"sub":"user-BBB","email":"bob@example.com"}"#)
        .expect(1)
        .create_async()
        .await;

    let mut token_server = Server::new_async().await;
    let _refresh_mock = token_server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"access_token":"oauth-fresh","refresh_token":"r2","expires_in":3600}"#,
        )
        .create_async()
        .await;

    let dir = tempdir().unwrap();
    write_oauth_token(dir.path(), &valid_token("oauth-access"));

    let exchange = TokenExchange::with_endpoint(token_server.url());
    let identity = IdentityFetcher::with_endpoint(identity_server.url());
    let orc =
        AuthOrchestrator::with_collaborators(dir.path().to_path_buf(), None, exchange, identity);

    match orc.get_access_token().await {
        Err(AuthError::Conflict { oauth_email, cli_email }) => {
            assert_eq!(oauth_email, "alice@example.com");
            assert_eq!(cli_email, "bob@example.com");
        }
        other => panic!("expected AuthError::Conflict, got {other:?}"),
    }
}
