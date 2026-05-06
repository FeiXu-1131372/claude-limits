use crate::app_state::{AppState, BurnRateProjection, CachedUsage};
use crate::auth::AuthSource;
use crate::notifier;
use crate::tray;
use crate::usage_api::{next_backoff, FetchOutcome, UsageSnapshot};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

static STALE_EMITTED: AtomicBool = AtomicBool::new(false);

pub fn spawn(handle: AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let mut burn_buffers: HashMap<u32, VecDeque<(DateTime<Utc>, f64)>> = HashMap::new();
        let _ = poll_all(&handle, &state, &mut burn_buffers).await;
        loop {
            let interval = {
                let s = state.settings.read();
                Duration::from_secs(s.polling_interval_secs.max(60))
            };
            tokio::select! {
                _ = tokio::time::sleep(interval) => {}
                _ = state.force_refresh.notified() => {}
            }
            let _ = poll_all(&handle, &state, &mut burn_buffers).await;
        }
    });
}

async fn poll_all(
    handle: &AppHandle,
    state: &AppState,
    burn_buffers: &mut HashMap<u32, VecDeque<(DateTime<Utc>, f64)>>,
) -> Result<(), anyhow::Error> {
    // 1. Reconcile active slot.
    let live = state.auth.read_live_claude_code().await.ok().flatten();
    let accounts = state.accounts.list().unwrap_or_default();
    let active_slot = live.as_ref().and_then(|l| {
        accounts
            .iter()
            .find(|a| a.account_uuid == l.account_uuid)
            .map(|a| a.slot)
    });
    *state.active_slot.write() = active_slot;

    // 2. Empty-state + unmanaged-active signals.
    if accounts.is_empty() && live.is_none() {
        let _ = handle.emit("requires_setup", ());
    }
    if let Some(live) = &live {
        if active_slot.is_none() {
            let _ = handle.emit(
                "unmanaged_active_account",
                json!({
                    "email": live.email,
                    "account_uuid": live.account_uuid,
                }),
            );
        }
    }

    // 3. Fan out per-slot fetches in parallel, respecting per-slot backoff windows.
    let due_slots: Vec<u32> = accounts
        .iter()
        .filter(|a| {
            let backoff_map = state.backoff_by_slot.read();
            // Treat backoff entries as "skip once" — cleared on successful poll
            // or natural expiry below.
            backoff_map.get(&a.slot).is_none()
        })
        .map(|a| a.slot)
        .collect();

    // Clear one backoff entry per skipped slot so they retry next tick.
    {
        let mut backoff_map = state.backoff_by_slot.write();
        for a in &accounts {
            if !due_slots.contains(&a.slot) {
                backoff_map.remove(&a.slot);
            }
        }
    }

    let fetches: Vec<_> = due_slots
        .iter()
        .map(|&slot| async move {
            let token_result = state
                .auth
                .token_for_slot(slot, active_slot, &state.accounts)
                .await;
            let token_failed = token_result.is_err();
            let outcome = match token_result {
                Ok(tok) => Some(state.usage.fetch(&tok).await),
                Err(e) => {
                    tracing::warn!("token_for_slot({slot}) failed: {e}");
                    None
                }
            };
            (slot, outcome, token_failed)
        })
        .collect();
    let results = futures::future::join_all(fetches).await;

    // 4. Update per-slot cache + emit events; also drive tray + notifier from active.
    for (slot, outcome, token_failed) in results {
        let acc = accounts.iter().find(|a| a.slot == slot).cloned();
        let Some(acc) = acc else { continue };
        if token_failed {
            let _ = handle.emit(
                "auth_required_for_slot",
                json!({ "slot": slot, "email": acc.email }),
            );
            continue;
        }
        let Some(outcome) = outcome else { continue };
        match outcome {
            FetchOutcome::Ok(snapshot) => {
                let buf = burn_buffers.entry(slot).or_default();
                let burn_rate = update_burn_rate(buf, &snapshot, Utc::now());
                let cached = CachedUsage {
                    snapshot: snapshot.clone(),
                    account_id: acc.account_uuid.clone(),
                    account_email: acc.email.clone(),
                    last_error: None,
                    burn_rate,
                    auth_source: if Some(slot) == active_slot {
                        AuthSource::ClaudeCode
                    } else {
                        AuthSource::OAuth
                    },
                };
                state.cached_usage_by_slot.write().insert(slot, cached.clone());
                state.backoff_by_slot.write().remove(&slot);
                let _ = handle.emit(
                    "usage_updated",
                    json!({ "slot": slot, "cached": cached }),
                );

                if Some(slot) == active_slot {
                    *state.cached_usage.write() = Some(cached.clone());
                    tray::set_level(
                        handle,
                        snapshot.five_hour.as_ref().map(|u| u.utilization),
                        snapshot.seven_day.as_ref().map(|u| u.utilization),
                        snapshot.five_hour.as_ref().and_then(|u| u.resets_at),
                        snapshot.seven_day.as_ref().and_then(|u| u.resets_at),
                        false,
                    );
                    let thresholds = state.settings.read().thresholds.clone();
                    if let Ok(fired) = notifier::evaluate(
                        &state.db,
                        &cached.account_id,
                        &snapshot,
                        &thresholds,
                        Utc::now(),
                    ) {
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
                    STALE_EMITTED.store(false, Ordering::Relaxed);
                }
            }
            FetchOutcome::Unauthorized => {
                let _ = handle.emit(
                    "auth_required_for_slot",
                    json!({ "slot": slot, "email": acc.email }),
                );
                let mut entry = state
                    .cached_usage_by_slot
                    .write()
                    .remove(&slot)
                    .unwrap_or_else(|| placeholder_cached(&acc, "auth_required"));
                entry.last_error = Some("auth_required".into());
                state.cached_usage_by_slot.write().insert(slot, entry);
            }
            FetchOutcome::RateLimited(retry_after) => {
                let prev = state
                    .backoff_by_slot
                    .read()
                    .get(&slot)
                    .copied()
                    .unwrap_or(Duration::from_secs(60));
                let next = retry_after.unwrap_or_else(|| next_backoff(prev));
                state.backoff_by_slot.write().insert(slot, next);
                let mut entry = state
                    .cached_usage_by_slot
                    .write()
                    .remove(&slot)
                    .unwrap_or_else(|| placeholder_cached(&acc, "rate-limited (429)"));
                entry.last_error = Some("rate-limited (429)".into());
                state.cached_usage_by_slot.write().insert(slot, entry);
            }
            FetchOutcome::Transient(e) => {
                let mut entry = state
                    .cached_usage_by_slot
                    .write()
                    .remove(&slot)
                    .unwrap_or_else(|| placeholder_cached(&acc, &e));
                entry.last_error = Some(e);
                state.cached_usage_by_slot.write().insert(slot, entry);
            }
        }
    }

    Ok(())
}

fn placeholder_cached(
    acc: &crate::auth::accounts::ManagedAccount,
    err: &str,
) -> CachedUsage {
    CachedUsage {
        snapshot: UsageSnapshot {
            five_hour: None,
            seven_day: None,
            seven_day_sonnet: None,
            seven_day_opus: None,
            extra_usage: None,
            fetched_at: Utc::now(),
            unknown: Default::default(),
        },
        account_id: acc.account_uuid.clone(),
        account_email: acc.email.clone(),
        last_error: Some(err.to_string()),
        burn_rate: None,
        auth_source: AuthSource::OAuth,
    }
}

fn update_burn_rate(
    buf: &mut VecDeque<(DateTime<Utc>, f64)>,
    snapshot: &UsageSnapshot,
    now: DateTime<Utc>,
) -> Option<BurnRateProjection> {
    let five_hour = snapshot.five_hour.as_ref()?;
    let resets_at = five_hour.resets_at?;
    let window_start = resets_at - ChronoDuration::hours(5);
    while let Some(&(ts, _)) = buf.front() {
        if ts < window_start {
            buf.pop_front();
        } else {
            break;
        }
    }
    buf.push_back((now, five_hour.utilization));
    if buf.len() < 2 {
        return None;
    }
    let &(t0, u0) = buf.front()?;
    let &(t1, u1) = buf.back()?;
    let span_minutes = (t1 - t0).num_seconds() as f64 / 60.0;
    if span_minutes < 2.0 {
        return None;
    }
    let slope = (u1 - u0) / span_minutes;
    let mins_until_reset = ((resets_at - now).num_seconds() as f64 / 60.0).max(0.0);
    Some(BurnRateProjection {
        utilization_per_min: slope,
        projected_at_reset: u1 + slope * mins_until_reset,
    })
}
