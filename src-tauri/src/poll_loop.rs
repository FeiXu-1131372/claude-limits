use crate::app_state::{AppState, CachedUsage};
use crate::auth::AuthError;
use crate::notifier;
use crate::tray;
use crate::usage_api::{next_backoff, FetchOutcome, UsageSnapshot};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

static STALE_EMITTED: AtomicBool = AtomicBool::new(false);

pub fn spawn(handle: AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let _ = poll_once(&handle, &state).await;
        let mut backoff = Duration::from_secs(60);
        loop {
            let interval = {
                let s = state.settings.read();
                Duration::from_secs(s.polling_interval_secs.max(60))
            };
            // Sleep up to `interval`, but wake immediately if the user pressed
            // the refresh button (or anything else called force_refresh.notify_one()).
            tokio::select! {
                _ = tokio::time::sleep(interval) => {}
                _ = state.force_refresh.notified() => {}
            }

            if let Some(cached) = &*state.cached_usage.read() {
                if cached.is_stale(Utc::now())
                    && !STALE_EMITTED.swap(true, Ordering::Relaxed)
                {
                    let _ = handle.emit("stale_data", ());
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
            tray::set_level(handle, None, None, None, true);
            // Brand-new install with no creds at all — surface the auth panel
            // explicitly. Without this the popover would just hang on "Loading…"
            // forever because no signal ever tells it that sign-in is required.
            let _ = handle.emit("auth_required", ());
            return PollResult::Transient;
        }
        Err(AuthError::Conflict {
            oauth_email,
            cli_email,
        }) => {
            tray::set_level(handle, None, None, None, true);
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
            tray::set_level(handle, None, None, None, true);
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
            tray::set_level(
                handle,
                snapshot.five_hour.as_ref().map(|u| u.utilization),
                snapshot.seven_day.as_ref().map(|u| u.utilization),
                snapshot.five_hour.as_ref().map(|u| u.resets_at),
                false,
            );

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
            tracing::warn!("usage api unauthorized; surfacing auth_required");
            tray::set_level(handle, None, None, None, true);
            let _ = handle.emit("auth_required", ());
            PollResult::Transient
        }
        FetchOutcome::RateLimited => {
            tracing::warn!("usage api rate-limited; backing off");
            PollResult::Backoff
        }
        FetchOutcome::Transient(e) => {
            tracing::warn!("usage api transient error: {e}");
            // On the very first poll, cached_usage is None and the previous
            // implementation dropped the error here — leaving the popover in
            // an indefinite "Loading…" state with nothing in logs and no
            // event to the frontend. Synthesize an empty placeholder so the
            // popover can render its normal layout (em-dashes for missing
            // numbers) plus the stale banner driven by last_error.
            let placeholder = state.cached_usage.read().clone().map_or_else(
                || CachedUsage {
                    snapshot: UsageSnapshot {
                        five_hour: None,
                        seven_day: None,
                        seven_day_sonnet: None,
                        seven_day_opus: None,
                        extra_usage: None,
                        fetched_at: Utc::now(),
                        unknown: Default::default(),
                    },
                    account_id: account.id.0.clone(),
                    account_email: account.email.clone(),
                    last_error: Some(e.clone()),
                },
                |mut c| {
                    c.last_error = Some(e.clone());
                    c
                },
            );
            *state.cached_usage.write() = Some(placeholder.clone());
            let _ = handle.emit("usage_updated", &placeholder);
            PollResult::Transient
        }
    }
}

