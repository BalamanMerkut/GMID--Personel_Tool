//! Video İndirici komutları. Ağa ya da yt-dlp'ye giden her şey arka planda çalışır.

use crate::downloader::{self, bridge::Bridge, tools, Download, Downloader, Live};
use crate::{db::Db, paths::DataDir};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;
const VERSION_KEY: &str = "videos.ytdlpVersion";

async fn blocking<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppHandle) -> T + Send + 'static,
) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| e.to_string())
}

fn tools_state(app: &AppHandle) -> tools::ToolsState {
    let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
    app.state::<tools::Tools>()
        .state(dir.root(), db.setting(VERSION_KEY).ok().flatten())
}

#[tauri::command]
pub fn videos_tools(app: AppHandle) -> tools::ToolsState {
    tools_state(&app)
}

/// Eksik araçları indirir (`force`: hepsini yeniden). Bitince sıradaki indirmeler başlar.
#[tauri::command]
pub async fn videos_install_tools(app: AppHandle, force: bool) -> CmdResult<tools::ToolsState> {
    blocking(app, move |app| {
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        app.state::<tools::Tools>()
            .install(app, dir.root(), force)?;
        if let Some(v) = tools::version(dir.root()) {
            let _ = db.set_setting(VERSION_KEY, &v);
        }
        app.state::<Downloader>().pump(app);
        Ok(tools_state(app))
    })
    .await?
}

/// yt-dlp'yi günceller. `force` değilse günde en fazla bir kez ve sadece boştayken.
#[tauri::command]
pub async fn videos_update(app: AppHandle, force: bool) -> CmdResult<tools::ToolsState> {
    blocking(app, move |app| {
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        if force || app.state::<Downloader>().live().is_empty() {
            if let Some(v) = tools::update_ytdlp(&db, dir.root(), force) {
                let _ = db.set_setting(VERSION_KEY, &v);
            }
        }
        tools_state(app)
    })
    .await
}

#[tauri::command]
pub async fn videos_probe(app: AppHandle, url: String) -> CmdResult<downloader::ytdlp::Probe> {
    blocking(app, move |app| {
        let root = app.state::<DataDir>().root().to_path_buf();
        app.state::<Downloader>().probe(&root, &url)
    })
    .await?
}

#[tauri::command]
pub async fn videos_download(app: AppHandle, url: String, option: String) -> CmdResult<Download> {
    blocking(app, move |app| {
        app.state::<Downloader>()
            .enqueue(app, &url, &option, None, "app")
    })
    .await?
}

#[tauri::command]
pub fn videos_list(db: State<'_, Db>) -> CmdResult<Vec<Download>> {
    downloader::list(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn videos_live(dl: State<'_, Downloader>) -> Vec<Live> {
    dl.live()
}

#[tauri::command]
pub async fn videos_pause(app: AppHandle, id: String) -> CmdResult<()> {
    blocking(app, move |app| app.state::<Downloader>().pause(app, &id)).await
}

#[tauri::command]
pub async fn videos_resume(app: AppHandle, id: String) -> CmdResult<()> {
    blocking(app, move |app| app.state::<Downloader>().resume(app, &id)).await
}

#[tauri::command]
pub async fn videos_cancel(app: AppHandle, id: String) -> CmdResult<()> {
    blocking(app, move |app| app.state::<Downloader>().cancel(app, &id)).await
}

#[tauri::command]
pub async fn videos_remove(app: AppHandle, id: String, delete_file: bool) -> CmdResult<()> {
    blocking(app, move |app| {
        app.state::<Downloader>().remove(app, &id, delete_file)
    })
    .await?
}

fn file_of(db: &Db, id: &str) -> CmdResult<String> {
    downloader::get(db, id)
        .map_err(|e| e.to_string())?
        .and_then(|d| d.file_path)
        .filter(|f| std::path::Path::new(f).is_file())
        .ok_or_else(|| "Dosya bulunamadı; taşınmış ya da silinmiş olabilir.".into())
}

#[tauri::command]
pub fn videos_open(app: AppHandle, db: State<'_, Db>, id: String) -> CmdResult<()> {
    let f = file_of(&db, &id)?;
    app.opener()
        .open_path(f, None::<&str>)
        .map_err(|e| format!("Açılamadı: {e}"))
}

#[tauri::command]
pub fn videos_reveal(app: AppHandle, db: State<'_, Db>, id: String) -> CmdResult<()> {
    let f = file_of(&db, &id)?;
    app.opener()
        .reveal_item_in_dir(f)
        .map_err(|e| format!("Açılamadı: {e}"))
}

#[tauri::command]
pub fn bridge_status(
    db: State<'_, Db>,
    bridge: State<'_, Bridge>,
) -> downloader::bridge::BridgeStatus {
    bridge.status(&db)
}

#[tauri::command]
pub fn bridge_pair(bridge: State<'_, Bridge>) -> String {
    bridge.start_pairing()
}

#[tauri::command]
pub fn bridge_unpair(db: State<'_, Db>, bridge: State<'_, Bridge>) {
    bridge.unpair(&db);
}

/// Eklenti dosyalarını `GMID Tool\Eklenti` klasörüne yazar ve klasörün yolunu döner.
#[tauri::command]
pub fn extension_install(dir: State<'_, DataDir>) -> CmdResult<String> {
    downloader::extension::install(&dir.folder(Some("Eklenti")))
}
