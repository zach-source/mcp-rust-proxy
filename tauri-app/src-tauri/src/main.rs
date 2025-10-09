#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod backend_manager;
mod commands;
mod events;
mod proxy;
mod state;

use backend_manager::{BackendManager, BackendMonitor};
use state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let state = AppState::new();
            app.manage(state);

            // Initialize backend manager
            let config_path = app
                .path()
                .app_config_dir()
                .expect("Failed to get app config dir")
                .join("mcp-proxy-config.yaml");

            let backend_manager = Arc::new(BackendManager::new(config_path, 3000));
            app.manage(Arc::new(RwLock::new(backend_manager.clone())));

            // Start backend monitor for auto-recovery
            let monitor = Arc::new(BackendMonitor::new(
                backend_manager.clone(),
                app.handle().clone(),
            ));

            let monitor_clone = monitor.clone();
            tauri::async_runtime::spawn(async move {
                monitor_clone.start_monitoring().await;
            });

            // Start the embedded proxy server
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = proxy::start_embedded_proxy(app_handle).await {
                    tracing::error!("Failed to start embedded proxy: {}", e);
                }
            });

            // Setup system tray
            #[cfg(desktop)]
            {
                use tauri::{
                    menu::{Menu, MenuItem},
                    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
                };

                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &quit])?;

                let _tray = TrayIconBuilder::new()
                    .menu(&menu)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
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
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_servers,
            commands::server_action,
            commands::get_metrics,
            commands::get_logs,
            commands::get_config,
            commands::update_config,
            commands::stream_logs,
            commands::start_backend,
            commands::stop_backend,
            commands::restart_backend,
            commands::get_backend_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
