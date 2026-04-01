use tauri::{AppHandle, State};

use crate::state::{ClientSnapshot, SharedClientState, VerificationAction};
use crate::tray::sync_tray;

#[tauri::command]
pub async fn load_client_snapshot(
    state: State<'_, SharedClientState>,
) -> Result<ClientSnapshot, String> {
    let guard = state.lock().await;
    Ok(guard.snapshot())
}

#[tauri::command]
pub async fn refresh_peer_discovery(
    app: AppHandle,
    state: State<'_, SharedClientState>,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard.refresh_peer_discovery();
    let snapshot = guard.snapshot();
    sync_tray(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    state: State<'_, SharedClientState>,
    chat_id: String,
    body: String,
    reply_to_message_id: Option<String>,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard
        .send_message(&chat_id, &body, reply_to_message_id.as_deref())
        .await?;
    let snapshot = guard.snapshot();
    sync_tray(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn send_media(
    app: AppHandle,
    state: State<'_, SharedClientState>,
    chat_id: String,
    file_name: String,
    mime_type: String,
    bytes_base64: String,
    reply_to_message_id: Option<String>,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard
        .send_media(
            &chat_id,
            &file_name,
            &mime_type,
            &bytes_base64,
            reply_to_message_id.as_deref(),
        )
        .await?;
    let snapshot = guard.snapshot();
    sync_tray(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn toggle_reaction(
    app: AppHandle,
    state: State<'_, SharedClientState>,
    chat_id: String,
    message_id: String,
    reaction: String,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard.toggle_reaction(&chat_id, &message_id, &reaction)?;
    let snapshot = guard.snapshot();
    sync_tray(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn forward_message(
    app: AppHandle,
    state: State<'_, SharedClientState>,
    source_chat_id: String,
    target_chat_id: String,
    message_id: String,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard.forward_message(&source_chat_id, &target_chat_id, &message_id)?;
    let snapshot = guard.snapshot();
    sync_tray(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn verify_device(
    app: AppHandle,
    state: State<'_, SharedClientState>,
    device_id: String,
    method: String,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    let action = VerificationAction::parse(&method)?;
    guard.verify_device(&device_id, action)?;
    let snapshot = guard.snapshot();
    sync_tray(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn export_device_registration(
    state: State<'_, SharedClientState>,
    path: String,
) -> Result<(), String> {
    let guard = state.lock().await;
    guard.export_device_registration(&path)
}

#[tauri::command]
pub async fn preview_invite(
    app: AppHandle,
    state: State<'_, SharedClientState>,
    invite_link: String,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard.preview_invite(&invite_link)?;
    let snapshot = guard.snapshot();
    sync_tray(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn accept_invite(
    app: AppHandle,
    state: State<'_, SharedClientState>,
    invite_link: String,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard.accept_invite(&invite_link).await?;
    let snapshot = guard.snapshot();
    sync_tray(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn check_for_updates(
    app: AppHandle,
    state: State<'_, SharedClientState>,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard.check_for_updates()?;
    let snapshot = guard.snapshot();
    sync_tray(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub async fn start_chat_with_peer(
    app: AppHandle,
    state: State<'_, SharedClientState>,
    device_id: String,
) -> Result<ClientSnapshot, String> {
    let mut guard = state.lock().await;
    guard.start_chat_with_peer(&device_id).await?;
    let snapshot = guard.snapshot();
    sync_tray(&app, &snapshot);
    Ok(snapshot)
}
