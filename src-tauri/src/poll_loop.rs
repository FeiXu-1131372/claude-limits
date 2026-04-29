use crate::app_state::{AppState, BurnRateProjection, CachedUsage};
use crate::auth::AuthError;
use crate::notifier;
use crate::tray;
use crate::usage_api::{next_backoff, FetchOutcome, UsageSnapshot};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

static STALE_EMITTED: AtomicBool = AtomicBool::new(false);

pub fn reset_stale_flag() {
    STALE_EMITTED.store(false, Ordering::SeqCst);
}

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
            tray::set_level(handle, None, None, None, None, true);
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
            tray::set_level(handle, None, None, None, None, true);
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
            tray::set_level(handle, None, None, None, None, true);
            let _ = handle.emit("auth_required", ());
            return PollResult::Transient;
        }
    };

    match state.usage.fetch(&token).await {
        FetchOutcome::Ok(snapshot) => {
            // Update the rolling sample buffer + recompute burn rate
            // BEFORE building CachedUsage so the cache picks up the new
            // projection. Resets between windows are handled by trimming
            // entries that fall outside [resets_at - 5h, resets_at].
            let burn_rate = update_history_and_compute_burn_rate(
                state,
                &snapshot,
                Utc::now(),
            );

            let cached = CachedUsage {
                snapshot: snapshot.clone(),
                account_id: account.id.0.clone(),
                account_email: account.email.clone(),
                last_error: None,
                burn_rate,
            };
            *state.cached_usage.write() = Some(cached.clone());
            let _ = handle.emit("usage_updated", &cached);
            tray::set_level(
                handle,
                snapshot.five_hour.as_ref().map(|u| u.utilization),
                snapshot.seven_day.as_ref().map(|u| u.utilization),
                snapshot.five_hour.as_ref().map(|u| u.resets_at),
                snapshot.seven_day.as_ref().map(|u| u.resets_at),
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
            tray::set_level(handle, None, None, None, None, true);
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
                    burn_rate: None,
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

/// Append the latest five_hour utilization to the rolling buffer, drop
/// entries that fall outside the live window, and return a projection if
/// we have enough samples (≥2 points spanning ≥2 minutes).
fn update_history_and_compute_burn_rate(
    state: &AppState,
    snapshot: &UsageSnapshot,
    now: DateTime<Utc>,
) -> Option<BurnRateProjection> {
    let five_hour = snapshot.five_hour.as_ref()?;
    let resets_at = five_hour.resets_at;
    let window_start = resets_at - ChronoDuration::hours(5);

    let mut buf = state.recent_five_hour.write();

    // Drop samples from previous windows. Keeps memory bounded and avoids
    // basing slope on a stale window's values when a reset has happened.
    while let Some(&(ts, _)) = buf.front() {
        if ts < window_start {
            buf.pop_front();
        } else {
            break;
        }
    }
    buf.push_back((now, five_hour.utilization));

    project_burn_rate(&buf, resets_at, now)
}

/// Pure function — no AppState, no I/O — so it can be tested directly.
/// Returns None if we don't have enough data for a meaningful slope.
fn project_burn_rate(
    samples: &VecDeque<(DateTime<Utc>, f64)>,
    resets_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Option<BurnRateProjection> {
    if samples.len() < 2 {
        return None;
    }
    let &(t0, u0) = samples.front()?;
    let &(t1, u1) = samples.back()?;
    let span_minutes = (t1 - t0).num_seconds() as f64 / 60.0;
    if span_minutes < 2.0 {
        // Two polls within a minute give a noisy slope — wait for more.
        return None;
    }
    let slope = (u1 - u0) / span_minutes;
    let mins_until_reset = ((resets_at - now).num_seconds() as f64 / 60.0).max(0.0);
    Some(BurnRateProjection {
        utilization_per_min: slope,
        projected_at_reset: u1 + slope * mins_until_reset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(min: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 28, 10, 0, 0).unwrap()
            + ChronoDuration::minutes(min)
    }

    #[test]
    fn returns_none_with_under_two_samples() {
        let mut buf = VecDeque::new();
        buf.push_back((t(0), 10.0));
        let r = project_burn_rate(&buf, t(300), t(0));
        assert!(r.is_none());
    }

    #[test]
    fn returns_none_when_samples_span_under_two_minutes() {
        let mut buf = VecDeque::new();
        buf.push_back((t(0), 10.0));
        buf.push_back((t(0) + ChronoDuration::seconds(45), 11.0));
        let r = project_burn_rate(&buf, t(300), t(1));
        assert!(r.is_none(), "spans only 45s — too noisy");
    }

    /// 10% → 30% across 60 minutes = 0.333%/min. With 60 minutes left in
    /// the window, projection lands at 30 + 0.333 × 60 ≈ 50%.
    #[test]
    fn linear_slope_extrapolates_correctly() {
        let mut buf = VecDeque::new();
        buf.push_back((t(0), 10.0));
        buf.push_back((t(60), 30.0));
        let now = t(60);
        let resets_at = t(120); // 60 minutes ahead of `now`
        let r = project_burn_rate(&buf, resets_at, now).expect("has samples");
        assert!((r.utilization_per_min - (20.0 / 60.0)).abs() < 1e-6);
        assert!((r.projected_at_reset - 50.0).abs() < 1e-6);
    }

    /// At the moment of reset, projection equals the latest sample —
    /// no extrapolation distance left.
    #[test]
    fn projection_equals_latest_at_reset_time() {
        let mut buf = VecDeque::new();
        buf.push_back((t(0), 50.0));
        buf.push_back((t(30), 80.0));
        let r = project_burn_rate(&buf, t(30), t(30)).expect("has samples");
        assert!((r.projected_at_reset - 80.0).abs() < 1e-6);
    }

    /// Past-reset clamping: if `now` is somehow past `resets_at` (clock
    /// skew, late poll), projection doesn't extrapolate backward.
    #[test]
    fn past_reset_does_not_project_backward() {
        let mut buf = VecDeque::new();
        buf.push_back((t(0), 50.0));
        buf.push_back((t(30), 80.0));
        let r = project_burn_rate(&buf, t(20), t(30)).expect("has samples");
        // resets_at is 10 min before `now`; mins_until_reset clamped to 0
        assert!((r.projected_at_reset - 80.0).abs() < 1e-6);
    }
}
