#![forbid(unsafe_code)]

mod commands;
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
    let state = tauri::async_runtime::block_on(ClientState::bootstrap())?;

    tauri::Builder::default()
        .manage(SharedClientState::new(state))
        .setup(|app| {
            let snapshot = {
                let state = app.state::<SharedClientState>();
                tauri::async_runtime::block_on(async {
                    let guard = state.lock().await;
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
            commands::check_for_updates,
            commands::start_chat_with_peer,
        ])
        .run(tauri::generate_context!())
        .map_err(Into::into)
}
