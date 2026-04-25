mod app_state;
pub mod auth;
mod commands;
pub mod jsonl_parser;
mod logging;
pub mod notifier;
mod poll_loop;
pub mod store;
pub mod usage_api;

use app_state::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_dir = logging::log_dir();
    let _log_guard = logging::init(log_dir.clone());

    let data_dir = store::default_dir();
    let db = Arc::new(store::Db::open(&data_dir).expect("open db"));
    let pricing = Arc::new(jsonl_parser::PricingTable::bundled().expect("pricing"));
    let auth = Arc::new(auth::AuthOrchestrator::new(data_dir.clone()));
    let usage_client = Arc::new(
        usage_api::UsageClient::new(env!("CARGO_PKG_VERSION").to_string()).expect("client"),
    );

    let app_state = Arc::new(AppState {
        db: db.clone(),
        auth,
        usage: usage_client,
        pricing: pricing.clone(),
        settings: parking_lot::RwLock::new(app_state::Settings::default()),
        cached_usage: parking_lot::RwLock::new(None),
        pending_oauth: parking_lot::RwLock::new(None),
        fallback_dir: data_dir.clone(),
    });

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            use tauri::Manager;
            if let Some(w) = app.get_webview_window("popover") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_current_usage,
            commands::get_session_history,
            commands::get_daily_trends,
            commands::get_model_breakdown,
            commands::get_project_breakdown,
            commands::get_cache_stats,
            commands::start_oauth_flow,
            commands::submit_oauth_code,
            commands::use_claude_code_creds,
            commands::pick_auth_source,
            commands::sign_out,
            commands::has_claude_code_creds,
            commands::update_settings,
            commands::get_settings,
            #[cfg(debug_assertions)]
            commands::debug_force_threshold,
        ])
        .setup(|app| {
            use tauri::Manager;
            let handle = app.handle().clone();
            let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
            poll_loop::spawn(handle.clone(), state.clone());

            if let Some(root) = jsonl_parser::walker::claude_projects_root() {
                let bf_root = root.clone();
                let bf_state = state.clone();
                tokio::spawn(async move {
                    if let Ok(files) = jsonl_parser::walker::discover_jsonl_files(&bf_root) {
                        for f in files {
                            let _ = jsonl_parser::walker::ingest_file(
                                &bf_state.db,
                                &bf_state.pricing,
                                &f,
                            );
                        }
                    }
                });

                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<usize>();
                let handle_for_events = handle.clone();
                tokio::spawn(async move {
                    use tauri::Emitter;
                    while let Some(n) = rx.recv().await {
                        let _ = handle_for_events.emit("session_ingested", n);
                    }
                });
                let _ = jsonl_parser::watcher::start(
                    state.db.clone(),
                    state.pricing.clone(),
                    root,
                    tx,
                );
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
