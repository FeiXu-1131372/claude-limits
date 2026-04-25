mod app_state;
pub mod auth;
mod commands;
pub mod jsonl_parser;
mod logging;
pub mod notifier;
mod poll_loop;
pub mod store;
mod tray;
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
        force_refresh: tokio::sync::Notify::new(),
    });

    // tauri-specta's Builder::commands replaces previously registered commands rather
    // than appending, so debug-only handlers must be folded into the same collect_commands! call.
    #[cfg(not(debug_assertions))]
    let specta_builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
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
            commands::open_expanded_window,
            commands::force_refresh,
        ]);

    #[cfg(debug_assertions)]
    let specta_builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
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
            commands::open_expanded_window,
            commands::force_refresh,
            commands::debug_force_threshold,
        ]);

    #[cfg(debug_assertions)]
    specta_builder
        .export(
            specta_typescript::Typescript::default()
                .bigint(specta_typescript::BigIntExportBehavior::Number),
            "../src/lib/generated/bindings.ts",
        )
        .expect("failed to export specta bindings");

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
        .invoke_handler(specta_builder.invoke_handler())
        .setup(|app| {
            use tauri::Manager;
            let handle = app.handle().clone();
            let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();

            // Apply native vibrancy to the popover so it reads as a Control
            // Center / Raycast-style menubar widget instead of a flat panel.
            #[cfg(target_os = "macos")]
            if let Some(popover) = app.get_webview_window("popover") {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
                let _ = apply_vibrancy(
                    &popover,
                    NSVisualEffectMaterial::HudWindow,
                    Some(NSVisualEffectState::Active),
                    None,
                );
            }
            #[cfg(target_os = "windows")]
            if let Some(popover) = app.get_webview_window("popover") {
                use window_vibrancy::{apply_acrylic, apply_mica};
                // Try Mica first (Windows 11), fall back to acrylic (Windows 10).
                if apply_mica(&popover, Some(true)).is_err() {
                    let _ = apply_acrylic(&popover, Some((24, 22, 20, 200)));
                }
            }

            // Tray icon
            use tauri::menu::{MenuBuilder, MenuItem};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let show = MenuItem::with_id(app, "show", "Show popover", true, None::<&str>)?;
            let expand = MenuItem::with_id(app, "expand", "Open expanded report", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = MenuBuilder::new(app).items(&[&show, &expand, &quit]).build()?;

            TrayIconBuilder::with_id("main")
                .tooltip("Claude Usage Monitor")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray/idle-template.png"
                ))?)
                .icon_as_template(true)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("popover") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "expand" => {
                        if let Some(w) = app.get_webview_window("report") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("popover") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            poll_loop::spawn(handle.clone(), state.clone());

            if let Some(root) = jsonl_parser::walker::claude_projects_root() {
                let bf_root = root.clone();
                let bf_state = state.clone();
                tauri::async_runtime::spawn(async move {
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
                tauri::async_runtime::spawn(async move {
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
