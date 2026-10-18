//! Ses Paneli komutları.

use crate::soundpad::{self, Config, Pad, Soundpad, Status};
use std::path::Path;
use tauri::{AppHandle, State};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn soundpad_status(app: AppHandle, pad: State<'_, Soundpad>) -> Status {
    pad.status(&app)
}

/// Bütün ayarları (sesler, tuşlar, seviyeler) kaydeder ve uygular.
#[tauri::command]
pub async fn soundpad_save(app: AppHandle, config: Config) -> CmdResult<Status> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        app.state::<Soundpad>().save(&app, config)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Dosyaları yeni ses olarak ekler; her birine sıradaki boş numpad tuşu verilir.
#[tauri::command]
pub async fn soundpad_add(app: AppHandle, paths: Vec<String>) -> CmdResult<Status> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let mut config = soundpad::config(&app);
        for path in paths {
            let p = Path::new(&path);
            let ext = p
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !soundpad::AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                return Err(format!(
                    "Bu dosya ses olarak eklenemez: {}",
                    p.file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default()
                ));
            }
            if config.pads.iter().any(|x| x.path == path) {
                continue;
            }
            let name = p
                .file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            config.pads.push(Pad {
                id: format!(
                    "{:x}",
                    chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
                ),
                key: config.free_key(),
                name: name.chars().take(60).collect(),
                path,
                volume: 1.0,
            });
        }
        app.state::<Soundpad>().save(&app, config)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Tıklayınca: çalıyorsa durdurur, çalmıyorsa çalar.
#[tauri::command]
pub async fn soundpad_toggle(app: AppHandle, id: String) -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        app.state::<Soundpad>().toggle(&app, &id, false)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn soundpad_stop(app: AppHandle, pad: State<'_, Soundpad>) {
    pad.stop(&app, None);
}

#[tauri::command]
pub fn soundpad_cable_page(app: AppHandle) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(soundpad::CABLE_URL, None::<&str>)
        .map_err(|e| e.to_string())
}
