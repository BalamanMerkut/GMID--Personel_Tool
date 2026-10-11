//! Sistem tepsisi (saatin yanındaki simge) ve menüsü.

use crate::{paths::DataDir, window, Launch};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "GMID Tool'u aç", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Kontrol Odası", true, None::<&str>)?;
    let folder = MenuItem::with_id(app, "folder", "Veri klasörünü aç", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &settings, &folder, &sep, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("GMID Tool")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                let _ = window::show_main(app);
            }
            "settings" => navigate(app, "settings"),
            "folder" => {
                let dir = app.state::<DataDir>();
                let _ = app
                    .opener()
                    .open_path(dir.root().to_string_lossy(), None::<&str>);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = window::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Pencereyi açar ve belirli bir ekrana götürür. Pencere yeni oluşturulduysa arayüz
/// henüz dinlemiyor olabilir; bu yüzden hedef ayrıca saklanır ve açılışta okunur.
fn navigate(app: &AppHandle, route: &str) {
    if let Ok(mut pending) = app.state::<Launch>().pending_route.lock() {
        *pending = Some(route.to_owned());
    }
    if let Ok(w) = window::show_main(app) {
        let _ = w.emit("navigate", route);
    }
}
