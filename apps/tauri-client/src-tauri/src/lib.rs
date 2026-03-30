#![forbid(unsafe_code)]

mod commands;
mod runtime;
mod state;

use std::error::Error;

use state::{ClientState, SharedClientState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn Error>> {
    let state = tauri::async_runtime::block_on(ClientState::bootstrap())?;

    tauri::Builder::default()
        .manage(SharedClientState::new(state))
        .invoke_handler(tauri::generate_handler![
            commands::load_client_snapshot,
            commands::refresh_peer_discovery,
            commands::send_message,
            commands::verify_device,
        ])
        .run(tauri::generate_context!())
        .map_err(Into::into)
}
