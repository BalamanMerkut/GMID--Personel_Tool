//! Kliplerim.

use crate::clips::{self, Clip, Clips, RootInfo};
use crate::db::Db;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn clips_list(db: State<'_, Db>) -> CmdResult<Vec<Clip>> {
    clips::list(&db)
}

#[tauri::command]
pub fn clips_roots(app: AppHandle) -> Vec<RootInfo> {
    clips::root_infos(&app)
}

/// Klasör ayarı değişince ya da "Yeniden tara" ile hemen bir tur.
#[tauri::command]
pub async fn clips_rescan(app: AppHandle) -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Clips>().scan(&app);
        let _ = app.emit("clips-changed", ());
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clips_set_favorite(app: AppHandle, id: String, on: bool) -> CmdResult<()> {
    clips::set_favorite(&app.state::<Db>(), &id, on)?;
    let _ = app.emit("clips-changed", ());
    Ok(())
}

#[tauri::command]
pub fn clips_rename(app: AppHandle, id: String, title: String) -> CmdResult<()> {
    clips::rename(&app.state::<Db>(), &id, &title)?;
    let _ = app.emit("clips-changed", ());
    Ok(())
}

#[tauri::command]
pub fn clips_set_game(app: AppHandle, id: String, game: Option<String>) -> CmdResult<()> {
    clips::set_game(&app.state::<Db>(), &id, game.as_deref())?;
    let _ = app.emit("clips-changed", ());
    Ok(())
}

#[tauri::command]
pub fn clips_delete(app: AppHandle, id: String) -> CmdResult<()> {
    clips::delete(&app.state::<Db>(), &id)?;
    let _ = app.emit("clips-changed", ());
    Ok(())
}

#[tauri::command]
pub fn clips_open(app: AppHandle, id: String) -> CmdResult<()> {
    let c = clips::get(&app.state::<Db>(), &id)?;
    app.opener()
        .open_path(c.path, None::<&str>)
        .map_err(|e| format!("Açılamadı: {e}"))
}

#[tauri::command]
pub fn clips_reveal(app: AppHandle, id: String) -> CmdResult<()> {
    let c = clips::get(&app.state::<Db>(), &id)?;
    app.opener()
        .reveal_item_in_dir(c.path)
        .map_err(|e| format!("Klasör açılamadı: {e}"))
}

/// Discord'a sığan kopya (ya da zaten sığıyorsa kendisi) panoya; ilerleme "clip-fit" olayıyla.
#[tauri::command]
pub async fn clips_fit_discord(app: AppHandle, id: String) -> CmdResult<String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Clips>().fit_discord(&app, &id))
        .await
        .map_err(|e| e.to_string())?
}
