#![forbid(unsafe_code)]

mod commands;
mod connection_manager;
mod media;
mod relay_client;
mod runtime;
mod state;
mod tray;

use std::error::Error;

use state::{ClientState, SharedClientState};
use tauri::Manager;
use tray::{handle_tray_icon_event, handle_tray_menu_event, setup_tray};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn Error>> {
    tauri::Builder::default()
        .setup(|app| {
            // Resolve the platform-specific app-data directory.
            // On Linux:   ~/.local/share/com.rimus.localmessenger/
            // On macOS:   ~/Library/Application Support/com.rimus.localmessenger/
            // On Windows: %APPDATA%\com.rimus.localmessenger\
            let app_data_dir = app.path().app_data_dir()?;

            // Bootstrap with real persistent identity (generates keys on first
            // launch and reloads them on every subsequent launch).
            let state =
                tauri::async_runtime::block_on(ClientState::bootstrap_persistent(app_data_dir))
                    .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg))?;

            app.manage(SharedClientState::new(state));

            let snapshot = {
                let state_handle = app.state::<SharedClientState>();
                tauri::async_runtime::block_on(async {
                    let guard = state_handle.lock().await;
                    guard.snapshot()
                })
            };
            setup_tray(&app.handle().clone(), &snapshot)?;
            Ok(())
        })
        .on_menu_event(handle_tray_menu_event)
        .on_tray_icon_event(handle_tray_icon_event)
        .invoke_handler(tauri::generate_handler![
            commands::load_client_snapshot,
            commands::refresh_peer_discovery,
            commands::send_message,
            commands::send_media,
            commands::toggle_reaction,
            commands::forward_message,
            commands::verify_device,
            commands::export_device_registration,
            commands::preview_invite,
            commands::accept_invite,
            commands::create_contact_invite,
            commands::preview_contact_invite,
            commands::accept_contact_invite,
            commands::check_for_updates,
            commands::start_chat_with_peer,
        ])
        .run(tauri::generate_context!())
        .map_err(Into::into)
}
