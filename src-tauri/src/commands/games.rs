//! Oyun kütüphanesi ve oturum geçmişi.

use crate::db::Db;
use crate::games::{scan::Found, Game, Games, RunningApp, Session};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppHandle) -> T + Send + 'static,
) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn games_list(games: State<'_, Games>, db: State<'_, Db>) -> CmdResult<Vec<Game>> {
    games.list(&db)
}

/// Steam ve Epic'te kurulu oyunları bulur (henüz eklemez).
#[tauri::command]
pub async fn games_scan(app: AppHandle) -> CmdResult<Vec<Found>> {
    blocking(app, |app| app.state::<Games>().scan(&app.state::<Db>())).await
}

/// Seçilen oyunları kütüphaneye ekler; eklenen sayısı.
#[tauri::command]
pub async fn games_import(app: AppHandle, found: Vec<Found>) -> CmdResult<usize> {
    blocking(app, move |app| app.state::<Games>().import(app, &found)).await?
}

#[tauri::command]
pub fn games_add_manual(app: AppHandle, name: String, exe: String) -> CmdResult<()> {
    app.state::<Games>().add_manual(&app, &name, &exe)
}

#[tauri::command]
pub fn games_running_apps(games: State<'_, Games>, db: State<'_, Db>) -> Vec<RunningApp> {
    games.running_apps(&db)
}

#[tauri::command]
pub fn games_set_hidden(app: AppHandle, id: String, hidden: bool) -> CmdResult<()> {
    app.state::<Games>().set_hidden(&app, &id, hidden)
}

#[tauri::command]
pub fn games_rename(app: AppHandle, id: String, name: String) -> CmdResult<()> {
    app.state::<Games>().rename(&app, &id, &name)
}

#[tauri::command]
pub fn games_remove(app: AppHandle, id: String) -> CmdResult<()> {
    app.state::<Games>().remove(&app, &id)
}

#[tauri::command]
pub fn games_refresh_meta(app: AppHandle, id: String) -> CmdResult<()> {
    app.state::<Games>().refresh_meta(&app, &id)
}

/// `since` (ms) sonrasında biten ya da süren oturumlar.
#[tauri::command]
pub fn games_sessions(
    games: State<'_, Games>,
    db: State<'_, Db>,
    since: i64,
) -> CmdResult<Vec<Session>> {
    games.sessions(&db, since)
}

/// Oyunu başlatır: Steam/Epic bağlantısıyla ya da exe'yi çalıştırarak.
#[tauri::command]
pub fn games_launch(
    app: AppHandle,
    games: State<'_, Games>,
    db: State<'_, Db>,
    id: String,
) -> CmdResult<()> {
    let g = games
        .list(&db)?
        .into_iter()
        .find(|g| g.id == id)
        .ok_or("Oyun bulunamadı")?;
    let target = g.launch.ok_or("Bu oyun nasıl başlatılacağını bilmiyor")?;
    if target.contains("://") {
        app.opener()
            .open_url(&target, None::<&str>)
            .map_err(|e| e.to_string())
    } else {
        let path = std::path::Path::new(&target);
        let mut cmd = std::process::Command::new(path);
        if let Some(dir) = path.parent() {
            cmd.current_dir(dir);
        }
        cmd.spawn()
            .map(|_| ())
            .map_err(|e| format!("Başlatılamadı: {e}"))
    }
}

/// Oyunun klasörünü açar.
#[tauri::command]
pub fn games_reveal(
    app: AppHandle,
    games: State<'_, Games>,
    db: State<'_, Db>,
    id: String,
) -> CmdResult<()> {
    let g = games
        .list(&db)?
        .into_iter()
        .find(|g| g.id == id)
        .ok_or("Oyun bulunamadı")?;
    let dir = g
        .install_dir
        .or_else(|| {
            g.launch.filter(|l| !l.contains("://")).and_then(|l| {
                std::path::Path::new(&l)
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
            })
        })
        .ok_or("Klasörü bilinmiyor")?;
    app.opener()
        .open_path(dir, None::<&str>)
        .map_err(|e| e.to_string())
}
