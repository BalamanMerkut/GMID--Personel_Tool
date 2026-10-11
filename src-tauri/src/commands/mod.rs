//! Arayüzün çağırdığı komutlar.

pub mod dictation;
pub mod games;
pub mod servers;
pub mod sound;
pub mod tools;
pub mod transcripts;
pub mod videos;

use crate::{backup, db::Db, paths::DataDir, Launch};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;

/// Arka plan olarak kabul edilen dosya türleri.
const BACKGROUND_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "mp4", "webm"];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    data_dir: String,
    db_size: u64,
    backups: Vec<backup::BackupInfo>,
}

#[tauri::command]
pub fn get_settings(db: State<'_, Db>) -> CmdResult<HashMap<String, String>> {
    db.all_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(db: State<'_, Db>, key: String, value: String) -> CmdResult<()> {
    db.set_setting(&key, &value).map_err(|e| e.to_string())
}

/// Açılış animasyonu sadece uygulama kapalıyken açıldığında, bir kez oynar.
#[tauri::command]
pub fn take_splash(launch: State<'_, Launch>) -> bool {
    launch.splash.swap(false, Ordering::SeqCst)
}

#[tauri::command]
pub fn take_pending_route(launch: State<'_, Launch>) -> Option<String> {
    launch.pending_route.lock().ok().and_then(|mut r| r.take())
}

#[tauri::command]
pub fn app_info(app: AppHandle, dir: State<'_, DataDir>) -> AppInfo {
    let db_size = ["gmid.db", "gmid.db-wal"]
        .iter()
        .filter_map(|f| std::fs::metadata(dir.root().join(f)).ok())
        .map(|m| m.len())
        .sum();
    AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: dir.root().to_string_lossy().into_owned(),
        db_size,
        backups: backup::list(&dir.backups()),
    }
}

#[tauri::command]
pub fn open_data_folder(
    app: AppHandle,
    dir: State<'_, DataDir>,
    sub: Option<String>,
) -> CmdResult<()> {
    let path = dir.folder(sub.as_deref());
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Klasör açılamadı: {e}"))
}

/// Günlük dosyasını (gmid.log) varsayılan metin düzenleyicide açar.
#[tauri::command]
pub fn open_log(app: AppHandle, dir: State<'_, DataDir>) -> CmdResult<()> {
    let log = dir.root().join("gmid.log");
    if !log.is_file() {
        return Err("Günlük dosyası henüz yok; her şey yolunda demek 🙂".into());
    }
    app.opener()
        .open_path(log.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Açılamadı: {e}"))
}

#[tauri::command]
pub fn backup_now(
    db: State<'_, Db>,
    dir: State<'_, DataDir>,
) -> CmdResult<Vec<backup::BackupInfo>> {
    backup::backup_now(&db, &dir.backups())?;
    Ok(backup::list(&dir.backups()))
}

/// Seçilen resmi/GIF'i/videoyu `Arkaplanlar` klasörüne kopyalar; orijinal silinse de arka plan bozulmaz.
#[tauri::command]
pub fn import_background(dir: State<'_, DataDir>, source: String) -> CmdResult<String> {
    let src = std::path::Path::new(&source);
    let ext = src
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !BACKGROUND_EXTENSIONS.contains(&ext.as_str()) {
        return Err("Bu dosya türü arka plan olamaz. Resim (JPG, PNG, WebP), GIF ya da video (MP4, WebM) seç.".into());
    }
    let stem: String = src
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(60)
        .collect();
    let name = format!(
        "{}-{stem}.{ext}",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    let dest = dir.backgrounds().join(name);
    std::fs::copy(src, &dest).map_err(|e| format!("Dosya kopyalanamadı: {e}"))?;
    Ok(dest.to_string_lossy().into_owned())
}
