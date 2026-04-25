use crate::app_state::{AppState, CachedUsage};
use crate::auth::AuthError;
use crate::notifier;
use crate::usage_api::{next_backoff, FetchOutcome};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

static STALE_EMITTED: AtomicBool = AtomicBool::new(false);

pub fn spawn(handle: AppHandle, state: Arc<AppState>) {
    tokio::spawn(async move {
        let _ = poll_once(&handle, &state).await;
        let mut backoff = Duration::from_secs(60);
        loop {
            let interval = {
                let s = state.settings.read();
                Duration::from_secs(s.polling_interval_secs.max(60))
            };
            tokio::time::sleep(interval).await;

            if let Some(cached) = &*state.cached_usage.read() {
                if cached.is_stale(Utc::now()) {
                    if !STALE_EMITTED.swap(true, Ordering::Relaxed) {
                        let _ = handle.emit("stale_data", ());
                    }
                }
            }

            match poll_once(&handle, &state).await {
                PollResult::Ok => {
                    STALE_EMITTED.store(false, Ordering::Relaxed);
                    backoff = Duration::from_secs(60);
                }
                PollResult::Backoff => {
                    tokio::time::sleep(backoff).await;
                    backoff = next_backoff(backoff);
                }
                PollResult::Transient => {}
            }
        }
    });
}

enum PollResult {
    Ok,
    Backoff,
    Transient,
}

async fn poll_once(handle: &AppHandle, state: &AppState) -> PollResult {
    let (token, _source, account) = match state.auth.get_access_token().await {
        Ok(t) => t,
        Err(AuthError::NoSource) => {
            return PollResult::Transient;
        }
        Err(AuthError::Conflict {
            oauth_email,
            cli_email,
        }) => {
            let _ = handle.emit(
                "auth_source_conflict",
                json!({
                    "oauth_email": oauth_email,
                    "cli_email":   cli_email,
                }),
            );
            return PollResult::Transient;
        }
        Err(e) => {
            tracing::warn!("auth failure: {e}");
            let _ = handle.emit("auth_required", ());
            return PollResult::Transient;
        }
    };

    match state.usage.fetch(&token).await {
        FetchOutcome::Ok(snapshot) => {
            let cached = CachedUsage {
                snapshot: snapshot.clone(),
                account_id: account.id.0.clone(),
                account_email: account.email.clone(),
                last_error: None,
            };
            *state.cached_usage.write() = Some(cached.clone());
            let _ = handle.emit("usage_updated", &cached);

            let thresholds = state.settings.read().thresholds.clone();
            match notifier::evaluate(
                &state.db,
                &cached.account_id,
                &snapshot,
                &thresholds,
                Utc::now(),
            ) {
                Ok(fired) => {
                    for f in fired {
                        use tauri_plugin_notification::NotificationExt;
                        let _ = handle
                            .notification()
                            .builder()
                            .title(f.title)
                            .body(f.body)
                            .show();
                    }
                }
                Err(e) => tracing::warn!("notifier evaluate failed: {e}"),
            }
            PollResult::Ok
        }
        FetchOutcome::Unauthorized => {
            let _ = handle.emit("auth_required", ());
            PollResult::Transient
        }
        FetchOutcome::RateLimited => PollResult::Backoff,
        FetchOutcome::Transient(e) => {
            let current = state.cached_usage.read().clone();
            if let Some(mut c) = current {
                c.last_error = Some(e);
                *state.cached_usage.write() = Some(c.clone());
                let _ = handle.emit("usage_updated", &c);
            }
            PollResult::Transient
        }
    }
}
