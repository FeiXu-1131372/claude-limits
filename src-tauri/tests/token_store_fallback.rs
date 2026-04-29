use claude_limits_lib::auth::token_store;
use claude_limits_lib::auth::StoredToken;
use chrono::Utc;
use std::fs;
use tempfile::tempdir;

fn make_token() -> StoredToken {
    StoredToken {
        access_token: "test-access".to_string(),
        refresh_token: Some("test-refresh".to_string()),
        expires_at: Utc::now() + chrono::Duration::hours(1),
    }
}

#[tokio::test]
async fn save_and_load_roundtrip() {
    let dir = tempdir().unwrap();
    let token = make_token();
    token_store::save(&token, dir.path()).await.unwrap();
    let loaded = token_store::load(dir.path()).unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.access_token, "test-access");
    assert_eq!(loaded.refresh_token.as_deref(), Some("test-refresh"));
}

#[tokio::test]
async fn corrupted_fallback_file_returns_ok_none() {
    let dir = tempdir().unwrap();
    let creds_path = dir.path().join("credentials.json");
    fs::write(&creds_path, b"{ invalid json }").unwrap();
    let result = token_store::load(dir.path());
    assert!(result.is_ok(), "should not error on corrupted fallback");
}

#[tokio::test]
async fn clear_removes_fallback_file() {
    let dir = tempdir().unwrap();
    let creds_path = dir.path().join("credentials.json");
    let token = make_token();
    let payload = serde_json::to_string_pretty(&token).unwrap();
    fs::write(&creds_path, payload).unwrap();
    assert!(creds_path.exists(), "file should exist before clear");
    token_store::clear(dir.path()).unwrap();
    assert!(!creds_path.exists(), "file should be removed after clear");
}

#[cfg(unix)]
#[tokio::test]
async fn fallback_file_has_restricted_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let token = make_token();
    token_store::save(&token, dir.path()).await.unwrap();
    let creds_path = dir.path().join("credentials.json");
    if creds_path.exists() {
        let mode = fs::metadata(&creds_path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "credentials file should be mode 0o600");
    }
}
