//! Karaoke: şarkıdan sesi ayırma, mikrofon kaydını müzikle karıştırma.

use crate::karaoke::{self, Karaoke, Mix, Status};
use crate::{db::Db, paths::DataDir};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn karaoke_status(k: State<'_, Karaoke>, dir: State<'_, DataDir>, id: String) -> Status {
    k.status(dir.root(), &id)
}

/// Şarkıyı hazırlar (ilk seferde indirir); ilerleme `karaoke-progress` olayıyla.
#[tauri::command]
pub async fn karaoke_prepare(app: AppHandle, id: String, cpu: Option<bool>) -> CmdResult<Status> {
    tauri::async_runtime::spawn_blocking(move || {
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        let file = song_file(&db, &id)?;
        app.state::<Karaoke>().prepare(
            &app,
            dir.root(),
            &id,
            Path::new(&file),
            cpu.unwrap_or(false),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

/// YouTube'da şarkının temiz (instrumental / karaoke) sürümünü arar.
#[tauri::command]
pub async fn karaoke_search(app: AppHandle, query: String) -> CmdResult<Vec<karaoke::online::Hit>> {
    tauri::async_runtime::spawn_blocking(move || {
        karaoke::online::search(app.state::<DataDir>().root(), &query)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Seçilen sürümü indirir ve şarkıya hizalar.
#[tauri::command]
pub async fn karaoke_online(
    app: AppHandle,
    id: String,
    url: String,
    title: String,
) -> CmdResult<Status> {
    tauri::async_runtime::spawn_blocking(move || {
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        let file = song_file(&db, &id)?;
        app.state::<Karaoke>()
            .fetch_online(&app, dir.root(), &id, Path::new(&file), &url, &title)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// İnternetten bulunan müziği bırakır (varsa yapay zekâyla ayrılana dönülür).
#[tauri::command]
pub fn karaoke_online_remove(k: State<'_, Karaoke>, dir: State<'_, DataDir>, id: String) -> Status {
    karaoke::online::remove(&karaoke::song_dir(dir.root(), &id));
    k.status(dir.root(), &id)
}

fn song_file(db: &Db, id: &str) -> CmdResult<String> {
    crate::downloader::get(db, id)
        .map_err(|e| e.to_string())?
        .ok_or("Şarkı bulunamadı")?
        .file_path
        .filter(|f| Path::new(f).is_file())
        .ok_or_else(|| "Dosya bulunamadı; taşınmış ya da silinmiş olabilir.".to_string())
}

#[tauri::command]
pub fn karaoke_cancel(k: State<'_, Karaoke>) {
    k.cancel();
}

/// Mikrofon kaydını (base64) saklar; yolunu döner.
#[tauri::command]
pub fn karaoke_voice(dir: State<'_, DataDir>, data: String, ext: String) -> CmdResult<String> {
    use base64::Engine;
    let b64 = data.split_once(',').map_or(data.as_str(), |(_, b)| b);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| "Kayıt okunamadı".to_string())?;
    karaoke::store_voice(dir.root(), &bytes, &ext)
}

#[tauri::command]
pub async fn karaoke_mix(app: AppHandle, id: String, mix: Mix) -> CmdResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        karaoke::mix(app.state::<DataDir>().root(), &id, &mix)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    path: String,
    name: String,
    size: u64,
    modified: i64,
}

/// Kaydedilen karaoke şarkıları, en yeni önce.
#[tauri::command]
pub fn karaoke_recordings(dir: State<'_, DataDir>) -> Vec<Recording> {
    let mut out: Vec<Recording> = std::fs::read_dir(karaoke::recordings_dir(dir.root()))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let m = e.metadata().ok()?;
            let name = p.file_name()?.to_string_lossy().into_owned();
            (m.is_file() && name.ends_with(".m4a") && !name.contains(".yaziliyor")).then(|| {
                Recording {
                    path: p.to_string_lossy().into_owned(),
                    name: name.trim_end_matches(".m4a").to_string(),
                    size: m.len(),
                    modified: m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_millis() as i64),
                }
            })
        })
        .collect();
    out.sort_by_key(|r| -r.modified);
    out
}

fn inside(dir: &DataDir, path: &str) -> CmdResult<std::path::PathBuf> {
    let p = std::path::PathBuf::from(path);
    if p.starts_with(karaoke::recordings_dir(dir.root())) && p.is_file() {
        Ok(p)
    } else {
        Err("Kayıt bulunamadı".into())
    }
}

#[tauri::command]
pub fn karaoke_reveal(app: AppHandle, dir: State<'_, DataDir>, path: String) -> CmdResult<()> {
    let p = inside(&dir, &path)?;
    app.opener()
        .reveal_item_in_dir(p)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn karaoke_delete(dir: State<'_, DataDir>, path: String) -> CmdResult<()> {
    std::fs::remove_file(inside(&dir, &path)?).map_err(|e| e.to_string())
}
