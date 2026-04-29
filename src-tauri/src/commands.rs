use crate::app_state::{AppState, CachedUsage, Settings};
use crate::auth::AuthSource;
use crate::store::StoredSessionEvent;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, State};

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct DailyBucket {
    pub date: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct ModelStats {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct ProjectStats {
    pub project: String,
    pub session_count: u64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct CacheStats {
    pub total_cache_read_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub estimated_savings_usd: f64,
    pub hit_ratio: f64,
}

fn err_to_string<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[command]
#[specta::specta]
pub async fn get_current_usage(state: State<'_, Arc<AppState>>) -> Result<Option<CachedUsage>, String> {
    Ok(state.snapshot())
}

#[command]
#[specta::specta]
pub async fn get_pricing(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::jsonl_parser::pricing::PricingEntry>, String> {
    Ok(state.pricing.entries().to_vec())
}

#[command]
#[specta::specta]
pub async fn get_session_history(
    days: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<StoredSessionEvent>, String> {
    let to = Utc::now();
    let from = to - Duration::days(days as i64);
    state.db.events_between(from, to).map_err(err_to_string)
}

#[command]
#[specta::specta]
pub async fn get_daily_trends(
    days: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<DailyBucket>, String> {
    let events = get_session_history(days, state).await?;
    use std::collections::BTreeMap;
    let mut by_day: BTreeMap<String, DailyBucket> = BTreeMap::new();
    for e in events {
        let date = e
            .ts
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d")
            .to_string();
        let slot = by_day
            .entry(date.clone())
            .or_insert_with(|| DailyBucket {
                date,
                input_tokens: 0,
                output_tokens: 0,
                cost_usd: 0.0,
            });
        slot.input_tokens += e.input_tokens;
        slot.output_tokens += e.output_tokens;
        slot.cost_usd += e.cost_usd;
    }
    Ok(by_day.into_values().collect())
}

#[command]
#[specta::specta]
pub async fn get_model_breakdown(
    days: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ModelStats>, String> {
    let events = get_session_history(days, state).await?;
    use std::collections::HashMap;
    let mut by_model: HashMap<String, ModelStats> = HashMap::new();
    for e in events {
        let entry = by_model
            .entry(e.model.clone())
            .or_insert_with(|| ModelStats {
                model: e.model.clone(),
                input_tokens: 0,
                output_tokens: 0,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
                cost_usd: 0.0,
            });
        entry.input_tokens += e.input_tokens;
        entry.output_tokens += e.output_tokens;
        entry.cache_read_tokens += e.cache_read_tokens;
        entry.cache_creation_tokens += e.cache_creation_5m_tokens + e.cache_creation_1h_tokens;
        entry.cost_usd += e.cost_usd;
    }
    Ok(by_model.into_values().collect())
}

#[command]
#[specta::specta]
pub async fn get_project_breakdown(
    days: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProjectStats>, String> {
    let events = get_session_history(days, state).await?;
    use std::collections::HashMap;
    let mut by_project: HashMap<String, ProjectStats> = HashMap::new();
    for e in events {
        let entry = by_project
            .entry(e.project.clone())
            .or_insert_with(|| ProjectStats {
                project: e.project.clone(),
                session_count: 0,
                total_cost_usd: 0.0,
            });
        entry.session_count += 1;
        entry.total_cost_usd += e.cost_usd;
    }
    Ok(by_project.into_values().collect())
}

#[command]
#[specta::specta]
pub async fn get_cache_stats(
    days: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<CacheStats, String> {
    let events = get_session_history(days, state).await?;
    let mut read = 0u64;
    let mut created = 0u64;
    for e in &events {
        read += e.cache_read_tokens;
        created += e.cache_creation_5m_tokens + e.cache_creation_1h_tokens;
    }
    let total = read + created;
    let hit_ratio = if total > 0 {
        (read as f64) / (total as f64)
    } else {
        0.0
    };
    let savings = (read as f64 / 1_000_000.0) * 2.7;
    Ok(CacheStats {
        total_cache_read_tokens: read,
        total_cache_creation_tokens: created,
        estimated_savings_usd: savings,
        hit_ratio,
    })
}

#[command]
#[specta::specta]
pub async fn start_oauth_flow(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    use crate::auth::oauth_paste_back::{build_authorize_url, generate_pkce};
    let pkce = generate_pkce();
    let url = build_authorize_url(&pkce).map_err(err_to_string)?;
    // Explicitly drop any existing pair before replacing so secrets don't
    // linger in memory longer than necessary.
    let old = state.auth.pending_oauth.write().replace((pkce, std::time::Instant::now()));
    drop(old);
    Ok(url)
}

#[command]
#[specta::specta]
pub async fn submit_oauth_code(
    pasted: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    use crate::auth::oauth_paste_back::parse_pasted_code;
    use crate::auth::token_store;

    const PKCE_TTL: std::time::Duration = std::time::Duration::from_secs(600);

    let entry = state.auth.pending_oauth.read().clone();
    let pkce = match entry {
        None => return Err("No active sign-in — click 'Sign in with Claude' first".to_string()),
        Some((pair, started_at)) if started_at.elapsed() > PKCE_TTL => {
            // Expired — drop the pair immediately to clear the secret from memory,
            // then return an error so the user re-initiates the flow.
            drop(state.auth.pending_oauth.write().take());
            return Err("Sign-in session expired (10-minute limit). Click 'Sign in with Claude' to start again.".to_string());
        }
        Some((pair, _)) => pair,
    };

    let code = parse_pasted_code(&pasted, &pkce.state).map_err(err_to_string)?;
    let token = state.auth.exchange
        .exchange_code(&code, &pkce.verifier)
        .await
        .map_err(err_to_string)?;
    token_store::save(&token, &state.fallback_dir).await.map_err(err_to_string)?;

    *state.auth.pending_oauth.write() = None;
    state.auth.set_preferred_source(AuthSource::OAuth).await;
    let mut settings = state.settings.read().clone();
    settings.preferred_auth_source = Some(AuthSource::OAuth);
    state.db.save_settings(&settings).map_err(err_to_string)?;
    *state.settings.write() = settings;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn use_claude_code_creds(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.auth.set_preferred_source(AuthSource::ClaudeCode).await;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn pick_auth_source(
    source: AuthSource,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.auth.set_preferred_source(source).await;
    let mut settings = state.settings.read().clone();
    settings.preferred_auth_source = Some(source);
    state.db.save_settings(&settings).map_err(err_to_string)?;
    *state.settings.write() = settings;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn sign_out(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    use crate::auth::token_store;
    use crate::tray;
    token_store::clear(&state.fallback_dir).map_err(err_to_string)?;
    *state.cached_usage.write() = None;
    *state.auth.pending_oauth.write() = None;
    crate::poll_loop::reset_stale_flag();
    tray::set_level(&app, None, None, None, None, true);
    let mut settings = state.settings.read().clone();
    settings.preferred_auth_source = None;
    state.db.save_settings(&settings).map_err(err_to_string)?;
    *state.settings.write() = settings;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn force_refresh(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.force_refresh.notify_one();
    Ok(())
}

#[command]
#[specta::specta]
pub async fn has_claude_code_creds() -> Result<bool, String> {
    Ok(crate::auth::claude_code_creds::has_creds().await)
}

#[command]
#[specta::specta]
pub async fn update_settings(s: Settings, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    if s.polling_interval_secs < 60 {
        return Err("polling_interval_secs must be at least 60".to_string());
    }
    if s.thresholds.iter().any(|&t| t > 100) {
        return Err("threshold values must be between 0 and 100".to_string());
    }
    state.db.save_settings(&s).map_err(|e| e.to_string())?;
    *state.settings.write() = s;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, String> {
    Ok(state.settings.read().clone())
}

#[cfg(debug_assertions)]
#[command]
#[specta::specta]
pub async fn debug_force_threshold(
    bucket: String,
    pct: u8,
    _state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    tracing::info!("debug_force_threshold({bucket}, {pct})");
    Ok(())
}

#[command]
#[specta::specta]
pub async fn resize_window(mode: String, app: tauri::AppHandle) -> Result<(), String> {
    use tauri::{LogicalPosition, LogicalSize, Manager, Position, Size};

    let Some(w) = app.get_webview_window("popover") else {
        return Ok(());
    };

    let target_size = match mode.as_str() {
        "compact" => (360.0_f64, 380.0_f64),
        "expanded" => (960.0_f64, 640.0_f64),
        _ => return Ok(()),
    };

    // Apply flag changes upfront so the rest of the animation runs in the
    // target mode's resize profile (resizable + always-on-top affect how
    // window-managers respond to subsequent set_size calls on some
    // platforms).
    match mode.as_str() {
        "compact" => {
            let _ = w.set_always_on_top(true);
            let _ = w.set_resizable(false);
        }
        "expanded" => {
            let _ = w.set_resizable(true);
            let _ = w.set_always_on_top(false);
        }
        _ => {}
    }

    // Capture the starting frame in logical coordinates so the math is
    // resolution-independent across retina/non-retina displays.
    let scale = w.scale_factor().map_err(|e| e.to_string())?;
    let cur_size = w.outer_size().map_err(|e| e.to_string())?;
    let cur_pos = w.outer_position().map_err(|e| e.to_string())?;
    let from_w = cur_size.width as f64 / scale;
    let from_h = cur_size.height as f64 / scale;
    let from_x = cur_pos.x as f64 / scale;
    let from_y = cur_pos.y as f64 / scale;

    // Where to end up. Compact stays anchored at the current center (the
    // post-animation TrayCenter snap below handles the reposition cleanly).
    // Expanded glides to the monitor's center so the bigger window doesn't
    // shoot off-screen when called from the tray-anchored compact view.
    let (to_x, to_y) = if mode == "expanded" {
        match w.current_monitor().map_err(|e| e.to_string())? {
            Some(m) => {
                let m_size = m.size();
                let m_pos = m.position();
                let mw = m_size.width as f64 / scale;
                let mh = m_size.height as f64 / scale;
                let mx = m_pos.x as f64 / scale;
                let my = m_pos.y as f64 / scale;
                (mx + (mw - target_size.0) / 2.0, my + (mh - target_size.1) / 2.0)
            }
            None => {
                // Fallback: keep the center fixed.
                let cx = from_x + from_w / 2.0;
                let cy = from_y + from_h / 2.0;
                (cx - target_size.0 / 2.0, cy - target_size.1 / 2.0)
            }
        }
    } else {
        let cx = from_x + from_w / 2.0;
        let cy = from_y + from_h / 2.0;
        (cx - target_size.0 / 2.0, cy - target_size.1 / 2.0)
    };

    // ~280ms total over 24 frames ≈ 12ms/frame. Cubic ease-out so the
    // motion feels native (fast start, gentle settle), matching macOS
    // Control Center / window-resize timing.
    const STEPS: u32 = 24;
    const STEP_MS: u64 = 12;

    for i in 1..=STEPS {
        let t = i as f64 / STEPS as f64;
        let eased = 1.0 - (1.0 - t).powi(3);
        let nw = from_w + (target_size.0 - from_w) * eased;
        let nh = from_h + (target_size.1 - from_h) * eased;
        let nx = from_x + (to_x - from_x) * eased;
        let ny = from_y + (to_y - from_y) * eased;

        let _ = w.set_size(Size::Logical(LogicalSize::new(nw, nh)));
        let _ = w.set_position(Position::Logical(LogicalPosition::new(nx, ny)));
        tokio::time::sleep(std::time::Duration::from_millis(STEP_MS)).await;
    }

    // Compact mode re-anchors to the tray after the animation so the
    // popover lives where the user's eye expects it. Expanded was already
    // animated to monitor center, no follow-up needed.
    if mode == "compact" {
        use tauri_plugin_positioner::{Position as TrayPos, WindowExt};
        let _ = w.move_window(TrayPos::TrayCenter);
    }

    Ok(())
}

#[command]
#[specta::specta]
pub async fn check_for_updates_now(app: tauri::AppHandle) -> Result<(), String> {
    crate::updater::check_and_emit(&app).await;
    Ok(())
}

#[command]
#[specta::specta]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    crate::updater::install_now(&app).await
}
