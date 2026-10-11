//! Video İndirici komutları. Ağa ya da yt-dlp'ye giden her şey arka planda çalışır.

use crate::downloader::subs::{self, Sidecar};
use crate::downloader::ytdlp::{Context, Langs};
use crate::downloader::{self, bridge::Bridge, tools, Download, Downloader, Live};
use crate::transcript::parse::{self, Segment};
use crate::{db::Db, paths::DataDir};
use std::path::Path;
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

/// Bağlantıyı çözümler. `whole_list`: video bağlantısındaki çalma listesinin tamamını getir.
#[tauri::command]
pub async fn videos_probe(
    app: AppHandle,
    url: String,
    whole_list: bool,
) -> CmdResult<downloader::ytdlp::Parsed> {
    blocking(app, move |app| {
        let root = app.state::<DataDir>().root().to_path_buf();
        app.state::<Downloader>()
            .probe(&root, &url, &Context::default(), whole_list)
    })
    .await?
}

#[tauri::command]
pub async fn videos_download(
    app: AppHandle,
    url: String,
    option: String,
    langs: Option<Langs>,
) -> CmdResult<Download> {
    blocking(app, move |app| {
        app.state::<Downloader>().enqueue(
            app,
            &url,
            &option,
            None,
            "app",
            &Context::default(),
            langs,
        )
    })
    .await?
}

/// Çalma listesinin (`ids` verilirse sadece seçilen) videolarını sıraya ekler; eklenen sayısını döner.
#[tauri::command]
pub async fn videos_download_playlist(
    app: AppHandle,
    url: String,
    option: String,
    ids: Option<Vec<String>>,
) -> CmdResult<usize> {
    blocking(app, move |app| {
        app.state::<Downloader>()
            .enqueue_playlist(app, &url, &option, ids, "app")
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

/// Videonun yanındaki altyazılar (film/dizi altyazıları ve uygulamanın yazıya döktüğü).
#[tauri::command]
pub fn media_tracks(db: State<'_, Db>, id: String) -> CmdResult<Vec<Sidecar>> {
    let d = downloader::get(&db, &id)
        .map_err(|e| e.to_string())?
        .ok_or("Video bulunamadı")?;
    Ok(d.file_path
        .map(|f| subs::sidecars(Path::new(&f)))
        .unwrap_or_default())
}

/// Seçilen altyazının satırları.
#[tauri::command]
pub fn media_track(db: State<'_, Db>, id: String, file: String) -> CmdResult<Vec<Segment>> {
    let d = downloader::get(&db, &id)
        .map_err(|e| e.to_string())?
        .ok_or("Video bulunamadı")?;
    let video = d.file_path.ok_or("Video dosyası yok")?;
    let path = subs::sidecar_path(Path::new(&video), &file).ok_or("Altyazı bulunamadı")?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    // Bazı altyazılar UTF-8 değil (eski Türkçe .srt dosyaları Windows-1254 olabilir).
    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => e.into_bytes().iter().map(|b| cp1254(*b)).collect(),
    };
    Ok(parse::parse_vtt(text.trim_start_matches('\u{feff}')))
}

/// Windows-1254 (Türkçe) → karakter; ASCII aynen kalır.
fn cp1254(b: u8) -> char {
    match b {
        0xD0 => 'Ğ',
        0xDD => 'İ',
        0xDE => 'Ş',
        0xF0 => 'ğ',
        0xFD => 'ı',
        0xFE => 'ş',
        0x80 => '€',
        _ => char::from(b),
    }
}

#[tauri::command]
pub fn playlists_list(db: State<'_, Db>) -> CmdResult<Vec<crate::playlists::Playlist>> {
    crate::playlists::list(&db)
}

/// `id` boşsa yeni liste oluşturur; doluysa adını ve şarkılarını günceller.
#[tauri::command]
pub fn playlist_save(
    db: State<'_, Db>,
    id: Option<String>,
    name: String,
    items: Vec<String>,
) -> CmdResult<crate::playlists::Playlist> {
    match id {
        Some(id) => crate::playlists::update(&db, &id, &name, &items),
        None => crate::playlists::create(&db, &name, &items),
    }
}

#[tauri::command]
pub fn playlist_delete(db: State<'_, Db>, id: String) -> CmdResult<()> {
    crate::playlists::delete(&db, &id)
}
