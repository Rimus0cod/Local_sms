#![forbid(unsafe_code)]

use tauri::menu::{Menu, MenuEvent, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::state::ClientSnapshot;

const TRAY_ID: &str = "localmessenger-tray";
const TRAY_SHOW_ID: &str = "tray-show";
const TRAY_QUIT_ID: &str = "tray-quit";

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>, snapshot: &ClientSnapshot) -> Result<(), String> {
    let show_item = MenuItemBuilder::with_id(TRAY_SHOW_ID, "Show Local Messenger")
        .build(app)
        .map_err(|error| error.to_string())?;
    let quit_item = MenuItemBuilder::with_id(TRAY_QUIT_ID, "Quit")
        .build(app)
        .map_err(|error| error.to_string())?;
    let menu =
        Menu::with_items(app, &[&show_item, &quit_item]).map_err(|error| error.to_string())?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(format_tray_tooltip(snapshot))
        .show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app).map_err(|error| error.to_string())?;
    sync_tray(app, snapshot);
    Ok(())
}

pub fn sync_tray<R: Runtime>(app: &AppHandle<R>, snapshot: &ClientSnapshot) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let unread_count = snapshot.notifications.unread_count;
        let title = if unread_count > 0 {
            format!("{unread_count}")
        } else {
            "Local Messenger".to_string()
        };
        let _ = tray.set_title(Some(title));
        let _ = tray.set_tooltip(Some(format_tray_tooltip(snapshot)));
    }
}

pub fn handle_tray_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    if event.id() == TRAY_SHOW_ID {
        show_main_window(app);
    } else if event.id() == TRAY_QUIT_ID {
        app.exit(0);
    }
}

pub fn handle_tray_icon_event<R: Runtime>(app: &AppHandle<R>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        show_main_window(app);
    }
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn format_tray_tooltip(snapshot: &ClientSnapshot) -> String {
    format!(
        "Local Messenger\n{}\nUnread: {}\nLatest: {}",
        snapshot.notifications.tray_label,
        snapshot.notifications.unread_count,
        snapshot.notifications.last_event
    )
}
