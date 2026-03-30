fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "load_client_snapshot",
        "refresh_peer_discovery",
        "send_message",
        "verify_device",
    ]);

    if let Err(error) =
        tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
    {
        panic!("failed to build tauri application manifest: {error}");
    }
}
