//! Tauri host: popup, manager, settings windows; tray bootstrap.
//! Frontend (popup/manager/settings views, components, state) lives under `../src`.

mod client;
mod commands;
mod sanitize;
mod tray;

use std::sync::Arc;

use clip_ipc::protocol::Event;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

use client::{spawn_event_forwarder, ClientHandle};
use tray::TrayState;

const DAEMON_EVENT_CHANNEL: &str = "daemon-event";

/// Hides the settings window. The settings window has no native decorations
/// (its close/minimize/maximize buttons were unreliable on this Linux/GTK
/// setup), so the settings view's own in-app Close button calls this instead.
#[tauri::command]
fn close_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Shows the settings window, invoked from the Manager UI's own Settings
/// button rather than the tray menu.
#[tauri::command]
fn show_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let socket_path = clip_core::config::AppPaths::resolve().config_dir.join("clipd.sock");
            let ipc_client = tauri::async_runtime::block_on(clip_ipc::client::IpcClient::connect(&socket_path))
                .expect("failed to connect to clipd - is the daemon running?");
            let client: Arc<dyn client::Client> = Arc::new(ipc_client);
            app.manage(ClientHandle(client.clone()));
            app.manage(TrayState::new());

            let pause_item = MenuItem::with_id(app, "pause_capture", "Pause capture", true, None::<&str>)?;
            let menu = MenuBuilder::new(app)
                .text("show", "Show ClipDeck")
                .text("hide", "Hide")
                .separator()
                .item(&pause_item)
                .text("clear_history", "Clear History")
                .separator()
                .text("quit", "Quit")
                .build()?;

            // Closing a window via its [X] button would otherwise destroy it
            // outright, leaving the tray's "Show ClipDeck"/"Settings" items
            // with nothing to show (`get_webview_window` returns `None`
            // after destruction). Intercept the close request and hide
            // instead, so each window can always be brought back from the
            // tray.
            for label in ["main", "settings"] {
                if let Some(window) = app.get_webview_window(label) {
                    let window_to_hide = window.clone();
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            let _ = window_to_hide.hide();
                        }
                    });
                }
            }

            // Loaded directly via `include_bytes!` (tracked by rustc as a
            // real compile dependency) rather than `app.default_window_icon()`
            // (backed by the `generate_context!()` macro's own icon
            // embedding, which was observed to keep serving a stale icon
            // across rebuilds even after a full `cargo clean`).
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

            TrayIconBuilder::new()
                .menu(&menu)
                .icon(tray_icon)
                .on_menu_event(move |app_handle, event| {
                    let client = app_handle.state::<ClientHandle>().0.clone();
                    match event.id().as_ref() {
                        "pause_capture" => {
                            let app_handle = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                let tray_state = app_handle.state::<TrayState>();
                                let _ = tray::handle_pause_toggle(client.as_ref(), tray_state.inner()).await;
                            });
                        }
                        "clear_history" => {
                            tauri::async_runtime::spawn(async move {
                                let _ = tray::handle_clear_history(client.as_ref()).await;
                            });
                        }
                        "quit" => {
                            tray::handle_quit(&|| std::process::exit(0));
                        }
                        "show" => {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "hide" => {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.hide();
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            let app_handle_for_events = app.handle().clone();
            let pause_item_for_events = pause_item.clone();
            spawn_event_forwarder(client, move |event: Event| {
                if let Event::CapturePaused { paused } = &event {
                    let tray_state = app_handle_for_events.state::<TrayState>();
                    tray_state.set_paused(*paused);
                    let _ = pause_item_for_events.set_text(tray::pause_menu_label(tray_state.inner()));
                }
                if let Event::HotkeyPressed = &event {
                    if let Some(popup) = app_handle_for_events.get_webview_window("popup") {
                        let _ = popup.show();
                        let _ = popup.set_focus();
                    }
                }
                let _ = app_handle_for_events.emit(DAEMON_EVENT_CHANNEL, event);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search_clips,
            commands::get_clip,
            commands::paste_clip,
            commands::copy_clip,
            commands::pin_clip,
            commands::delete_clip,
            commands::clear_history,
            commands::list_rules,
            commands::save_rule,
            commands::delete_rule,
            commands::get_settings,
            commands::update_settings,
            commands::get_diagnostics,
            commands::pause_capture,
            sanitize::sanitize_clip_html,
            close_settings_window,
            show_settings_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
