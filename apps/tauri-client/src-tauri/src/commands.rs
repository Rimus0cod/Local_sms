use tauri::State;

use crate::state::{ClientSnapshot, SharedClientState, VerificationAction};

#[tauri::command]
pub async fn load_client_snapshot(
    state: State<'_, SharedClientState>,
) -> Result<ClientSnapshot, String> {
    let guard = state.lock().await;
    Ok(guard.snapshot())
}

#[tauri::command]
pub async fn refresh_peer_discovery(
    state: State<'_, SharedClientState>,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard.refresh_peer_discovery();
    Ok(guard.snapshot())
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, SharedClientState>,
    chat_id: String,
    body: String,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard.send_message(&chat_id, &body).await?;
    Ok(guard.snapshot())
}

#[tauri::command]
pub async fn verify_device(
    state: State<'_, SharedClientState>,
    device_id: String,
    method: String,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    let action = VerificationAction::parse(&method)?;
    guard.verify_device(&device_id, action)?;
    Ok(guard.snapshot())
}
