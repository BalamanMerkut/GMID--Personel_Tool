//! Program Kaldırıcı komutları.

use crate::paths::DataDir;
use crate::uninstall::{CleanResult, ProgramList, Scan, State as UState, Uninstaller};
use tauri::{AppHandle, Manager, State};

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
pub async fn uninstall_list(app: AppHandle) -> CmdResult<ProgramList> {
    blocking(app, |app| app.state::<Uninstaller>().list()).await
}

/// Program simgeleri (kimlik → PNG data URL).
#[tauri::command]
pub async fn uninstall_icons(
    app: AppHandle,
) -> CmdResult<std::collections::HashMap<String, String>> {
    blocking(app, |app| app.state::<Uninstaller>().icons()).await
}

/// Programın kurulu olduğu klasörü Gezgin'de açar.
#[tauri::command]
pub fn uninstall_open_folder(
    app: AppHandle,
    u: State<'_, Uninstaller>,
    id: String,
) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let f = u.folder(&id)?;
    app.opener()
        .open_path(f.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Klasör açılamadı: {e}"))
}

/// Programın kendi kaldırıcısını açar.
#[tauri::command]
pub fn uninstall_run(u: State<'_, Uninstaller>, id: String) -> CmdResult<()> {
    u.run(&id)
}

#[tauri::command]
pub fn uninstall_state(u: State<'_, Uninstaller>, id: String) -> CmdResult<UState> {
    u.state(&id)
}

#[tauri::command]
pub async fn uninstall_scan(app: AppHandle, id: String) -> CmdResult<Scan> {
    blocking(app, move |app| app.state::<Uninstaller>().scan(&id)).await?
}

/// Seçilen artıkları siler (dosyalar Geri Dönüşüm Kutusu'na, kayıtlar önce yedeklenir).
#[tauri::command]
pub async fn uninstall_clean(
    app: AppHandle,
    id: String,
    keys: Vec<String>,
) -> CmdResult<CleanResult> {
    blocking(app, move |app| {
        let dir = app.state::<DataDir>();
        app.state::<Uninstaller>().clean(&dir.backups(), &id, &keys)
    })
    .await?
}
