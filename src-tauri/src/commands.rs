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
    *state.pending_oauth.write() = Some(pkce);
    Ok(url)
}

#[command]
#[specta::specta]
pub async fn submit_oauth_code(
    pasted: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    use crate::auth::exchange::TokenExchange;
    use crate::auth::oauth_paste_back::parse_pasted_code;
    use crate::auth::token_store;

    let pkce = state
        .pending_oauth
        .read()
        .clone()
        .ok_or_else(|| "No active sign-in — click 'Sign in with Claude' first".to_string())?;

    let code = parse_pasted_code(&pasted, &pkce.state).map_err(err_to_string)?;
    let exchange = TokenExchange::new();
    let token = exchange
        .exchange_code(&code, &pkce.verifier)
        .await
        .map_err(err_to_string)?;
    token_store::save(&token, &state.fallback_dir).map_err(err_to_string)?;

    *state.pending_oauth.write() = None;
    state.auth.set_preferred_source(AuthSource::OAuth).await;
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
    *state.pending_oauth.write() = None;
    tray::set_level(&app, None, None, None, true);
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
    Ok(crate::auth::claude_code_creds::has_creds())
}

#[command]
#[specta::specta]
pub async fn update_settings(s: Settings, state: State<'_, Arc<AppState>>) -> Result<(), String> {
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
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("popover") {
        match mode.as_str() {
            "compact" => {
                let _ = w.set_always_on_top(true);
                let _ = w.set_resizable(false);
                let _ = w.set_size(tauri::Size::Logical(tauri::LogicalSize::new(360.0, 380.0)));
            }
            "expanded" => {
                let _ = w.set_resizable(true);
                let _ = w.set_always_on_top(false);
                let _ = w.set_size(tauri::Size::Logical(tauri::LogicalSize::new(960.0, 640.0)));
                let _ = w.center();
            }
            _ => {}
        }
    }
    Ok(())
}
