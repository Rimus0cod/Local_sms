fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "load_client_snapshot",
        "refresh_peer_discovery",
        "send_message",
        "send_media",
        "toggle_reaction",
        "forward_message",
        "verify_device",
        "export_device_registration",
        "preview_invite",
        "accept_invite",
        "check_for_updates",
    ]);

    if let Err(error) =
        tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
    {
        panic!("failed to build tauri application manifest: {error}");
    }
}
