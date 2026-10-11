//! Ana pencere. Kapatılınca tamamen kapanır (arayüz bellekten çıkar), çekirdek tepside kalır;
//! tepsiden açılınca yeniden oluşturulur.

use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};

pub const MAIN: &str = "main";

pub fn show_main(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(w) = app.get_webview_window(MAIN) {
        if w.is_minimized().unwrap_or(false) {
            w.unminimize()?;
        }
        w.show()?;
        w.set_focus()?;
        return Ok(w);
    }

    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == MAIN)
        .cloned()
        .expect("tauri.conf.json içinde 'main' penceresi tanımlı olmalı");
    let w = WebviewWindowBuilder::from_config(app, &config)?.build()?;
    keep_on_screen(&w);
    w.set_focus()?;
    Ok(w)
}

/// Monitör çıkarıldıysa ya da çözünürlük değiştiyse pencere ekran dışında açılmasın.
fn keep_on_screen(w: &WebviewWindow) {
    let (Ok(pos), Ok(size), Ok(monitors)) =
        (w.outer_position(), w.outer_size(), w.available_monitors())
    else {
        return;
    };
    let (w_w, w_h) = (size.width as i32, size.height as i32);
    let visible = monitors.iter().any(|m| {
        let (mp, ms) = (m.position(), m.size());
        let overlap_x = (pos.x + w_w).min(mp.x + ms.width as i32) - pos.x.max(mp.x);
        let overlap_y = (pos.y + w_h).min(mp.y + ms.height as i32) - pos.y.max(mp.y);
        // Başlık çubuğundan tutup çekebilecek kadar alan görünüyor mu?
        overlap_x >= 120 && overlap_y >= 60
    });
    if !visible {
        let _ = w.center();
    }
}
