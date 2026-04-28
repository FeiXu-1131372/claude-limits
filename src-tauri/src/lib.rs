mod app_state;
pub mod auth;
mod commands;
pub mod jsonl_parser;
mod logging;
pub mod notifier;
mod poll_loop;
pub mod store;
mod tray;
mod tray_icon;
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
        recent_five_hour: parking_lot::RwLock::new(std::collections::VecDeque::new()),
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
        .plugin(
            // Track the report window's size/position/maximized so users keep
            // their preferred layout, but never restore visibility — the report
            // must stay hidden on cold start and only appear when the user
            // clicks "See details". The popover is a fixed 360×360 widget,
            // so don't track it at all.
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED
                        | tauri_plugin_window_state::StateFlags::DECORATIONS,
                )
                .with_denylist(&["popover"])
                .build(),
        )
        .invoke_handler(specta_builder.invoke_handler())
        .setup(|app| {
            use tauri::Manager;
            let handle = app.handle().clone();
            let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();

            // Make this a menubar-only app on macOS — no Dock icon, no app
            // switcher entry. Without this, NSStatusItem can fail to register
            // visibly (the icon ends up at an off-screen position macOS picks
            // for "regular" apps). With Accessory policy, the tray icon is
            // the app's only UI surface and macOS places it correctly.
            #[cfg(target_os = "macos")]
            {
                let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            // Force the popover to its configured fixed size on every launch.
            // The window-state denylist already keeps it untracked, but a
            // historical save from a previous build can still leave it
            // oversized on first run after the upgrade.
            if let Some(popover) = app.get_webview_window("popover") {
                use tauri::{LogicalSize, Size};
                let _ = popover.set_size(Size::Logical(LogicalSize::new(360.0, 380.0)));
            }

            // Always start with the report window hidden — the user opens it
            // explicitly via the tray menu or the "See details" button. This
            // guards against any plugin / OS path that might otherwise leave
            // it visible from a previous session.
            //
            // Also intercept the OS close button: by default Tauri DESTROYS the
            // window, after which get_webview_window("report") returns None and
            // open_expanded_window silently no-ops — the user can never reopen
            // the report. Hide instead so the window survives for next show().
            if let Some(report) = app.get_webview_window("report") {
                let _ = report.hide();
                let report_clone = report.clone();
                report.on_window_event(move |ev| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = ev {
                        api.prevent_close();
                        let _ = report_clone.hide();
                    }
                });
            }

            // Apply native vibrancy to the popover so it reads as a Control
            // Center / Raycast-style menubar widget instead of a flat panel.
            // The radius MUST match the `--radius-lg` token used by `#root`'s
            // border-radius — otherwise the NSVisualEffectView stays
            // rectangular and a sharp-cornered dark plate is visible behind
            // the rounded HTML surface.
            #[cfg(target_os = "macos")]
            if let Some(popover) = app.get_webview_window("popover") {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
                let _ = apply_vibrancy(
                    &popover,
                    NSVisualEffectMaterial::HudWindow,
                    Some(NSVisualEffectState::Active),
                    Some(14.0),
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

            // Tray icon — configure the one Tauri auto-created from the
            // `trayIcon` block in tauri.conf.json. Don't build a NEW one
            // (that would create a second NSStatusItem that competes with
            // the visible config-driven one — when the user reported "two
            // duplicated icons" earlier, that was this exact double-creation,
            // and removing the config block left us with only the invisible
            // programmatic item).
            use tauri::menu::{MenuBuilder, MenuItem};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};

            let show = MenuItem::with_id(app, "show", "Show popover", true, None::<&str>)?;
            let expand = MenuItem::with_id(app, "expand", "Open expanded report", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = MenuBuilder::new(app).items(&[&show, &expand, &quit]).build()?;

            if let Some(tray) = app.tray_by_id("main") {
                tracing::info!("attaching menu + handlers to config-created tray");
                let _ = tray.set_menu(Some(menu));
                tray.on_menu_event(|app, event| match event.id.as_ref() {
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
                });
                tray.on_tray_icon_event(|tray, event| {
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
                });
            } else {
                tracing::error!(
                    "tray_by_id('main') returned None — tauri.conf.json `trayIcon` block missing?"
                );
            }

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
                // The WatcherHandle owns the notify-debouncer that drives the
                // OS file watcher. Drop it and the debouncer is destroyed, the
                // watcher stops, and no new JSONL writes are ever ingested —
                // the report appears to "stop updating" mid-session and only
                // refreshes when the app restarts (because the backfill above
                // re-scans every file from scratch). Leak it so it lives for
                // the process lifetime, which is the lifetime we want anyway.
                match jsonl_parser::watcher::start(
                    state.db.clone(),
                    state.pricing.clone(),
                    root,
                    tx,
                ) {
                    Ok(handle) => {
                        Box::leak(Box::new(handle));
                    }
                    Err(e) => tracing::error!("jsonl watcher failed to start: {e}"),
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
