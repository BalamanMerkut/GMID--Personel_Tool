//! Ana pencere. Kapatılınca tamamen kapanır (arayüz bellekten çıkar), çekirdek tepside kalır;
//! tepsiden açılınca yeniden oluşturulur.

use crate::db::Db;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const MAIN: &str = "main";

/// Pencereler sırayla oluşturulsun (aynı pencere iki kez açılmaya çalışılmasın).
static UI_LOCK: Mutex<()> = Mutex::new(());

/// Pencere işini ayrı bir iş parçacığında yapar. Windows'ta pencereyi ana iş parçacığında
/// (eşzamanlı komutlar, tepsi menüsü, olay işleyicileri) oluşturmak uygulamayı kilitler;
/// bu yüzden pencere açan her şey buradan geçer.
pub fn on_ui_thread(
    app: &AppHandle,
    what: &'static str,
    f: impl FnOnce(&AppHandle) -> tauri::Result<()> + Send + 'static,
) {
    let app = app.clone();
    std::thread::spawn(move || {
        let _guard = UI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = f(&app) {
            crate::log!("{what} açılamadı: {e}");
        }
    });
}

/// Ana pencereyi açar (kapatıldıysa yeniden oluşturur) ve öne getirir.
pub fn show_main(app: &AppHandle) {
    on_ui_thread(app, "Ana pencere", |app| show_main_now(app).map(|_| ()));
}

/// Ana pencereyi açar ve bir ekrana götürür.
pub fn show_main_at(app: &AppHandle, route: String) {
    on_ui_thread(app, "Ana pencere", move |app| {
        let w = show_main_now(app)?;
        let _ = w.emit("navigate", route);
        Ok(())
    });
}

fn show_main_now(app: &AppHandle) -> tauri::Result<WebviewWindow> {
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

pub const QUICK_NOTE: &str = "quicknote";
const STICKY_PREFIX: &str = "sticky-";
const STICKIES_SETTING: &str = "notes.stickies";

/// Her yerden açılan küçük hızlı not penceresi.
pub fn open_quick_note(app: &AppHandle) {
    on_ui_thread(app, "Hızlı not", open_quick_note_now);
}

fn open_quick_note_now(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window(QUICK_NOTE) {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    let w = WebviewWindowBuilder::new(app, QUICK_NOTE, WebviewUrl::App("index.html".into()))
        .title("Hızlı not")
        .inner_size(420.0, 380.0)
        .min_inner_size(300.0, 240.0)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .center()
        .build()?;
    w.set_focus()?;
    Ok(())
}

fn sticky_label(id: &str) -> String {
    format!("{STICKY_PREFIX}{id}")
}

fn stickies(app: &AppHandle) -> Vec<String> {
    app.state::<Db>()
        .setting_json(STICKIES_SETTING)
        .unwrap_or_default()
}

fn save_stickies(app: &AppHandle, ids: &[String]) {
    if let Ok(json) = serde_json::to_string(ids) {
        let _ = app.state::<Db>().set_setting(STICKIES_SETTING, &json);
    }
}

/// Notu masaüstünde "her zaman üstte" küçük bir yapışkan not olarak açar.
pub fn open_sticky(app: &AppHandle, id: &str) {
    let id = id.to_string();
    on_ui_thread(app, "Yapışkan not", move |app| open_sticky_now(app, &id));
}

fn open_sticky_now(app: &AppHandle, id: &str) -> tauri::Result<()> {
    let label = sticky_label(id);
    if let Some(w) = app.get_webview_window(&label) {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title("Yapışkan not")
        .inner_size(300.0, 300.0)
        .min_inner_size(220.0, 160.0)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()?;
    let mut ids = stickies(app);
    if !ids.iter().any(|i| i == id) {
        ids.push(id.to_string());
        save_stickies(app, &ids);
    }
    Ok(())
}

pub fn close_sticky(app: &AppHandle, id: &str) {
    if let Some(w) = app.get_webview_window(&sticky_label(id)) {
        let _ = w.destroy();
    }
    let ids: Vec<String> = stickies(app).into_iter().filter(|i| i != id).collect();
    save_stickies(app, &ids);
}

/// Açılışta, kapatılmadan bırakılan yapışkan notları geri açar (silinmiş notlar hariç).
pub fn restore_stickies(app: &AppHandle) {
    let db = app.state::<Db>();
    let alive: Vec<String> = stickies(app)
        .into_iter()
        .filter(|id| {
            crate::notes::get(&db, id)
                .ok()
                .flatten()
                .is_some_and(|n| n.deleted_at.is_none())
        })
        .collect();
    save_stickies(app, &alive);
    for id in alive {
        open_sticky(app, &id);
    }
}
