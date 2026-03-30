fn main() {
    if let Err(error) = localmessenger_tauri_client::run() {
        eprintln!("failed to launch Local Messenger desktop shell: {error}");
        std::process::exit(1);
    }
}
