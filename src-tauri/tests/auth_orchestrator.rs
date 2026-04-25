use claude_usage_monitor_lib::auth::{orchestrator::AuthOrchestrator, AuthError};
use tempfile::tempdir;

#[tokio::test]
async fn no_sources_errors_with_typed_variant() {
    // This test only passes on machines without any Claude Code credentials
    // in the system keychain. If Claude Code is installed, the orchestrator
    // will find real credentials and attempt to use them.
    if claude_usage_monitor_lib::auth::claude_code_creds::has_creds() {
        return; // skip: real Claude Code creds found in keychain
    }
    let dir = tempdir().unwrap();
    let orc = AuthOrchestrator::new(dir.path().to_path_buf());
    match orc.get_access_token().await {
        Err(AuthError::NoSource) => {}
        other => panic!("expected AuthError::NoSource, got {other:?}"),
    }
}
