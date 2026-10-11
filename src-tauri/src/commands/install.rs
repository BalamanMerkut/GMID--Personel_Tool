//! Format Sonrası Kurulum ve GMID'in tam yedeği / geri yüklenmesi.

use crate::db::Db;
use crate::install::kit::{self, KitRequest, KitResult};
use crate::install::{self, Hit, Installer, Item, RunState};
use crate::paths::DataDir;
use crate::restore::{self, Manifest, Options};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> CmdResult<T> + Send + 'static,
) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

fn home(app: &AppHandle) -> PathBuf {
    app.path().home_dir().unwrap_or_default()
}

#[tauri::command]
pub fn install_catalog() -> Vec<Item> {
    install::catalog()
}

#[tauri::command]
pub fn install_list(db: State<'_, Db>) -> Vec<Item> {
    install::list(&db)
}

#[tauri::command]
pub fn install_save_list(db: State<'_, Db>, items: Vec<Item>) -> CmdResult<()> {
    install::save_list(&db, &items)
}

#[tauri::command]
pub fn install_state(installer: State<'_, Installer>) -> RunState {
    installer.state()
}

/// Sırayla sessiz kurulum; ilerleme "install" olayıyla gelir.
#[tauri::command]
pub async fn install_run(app: AppHandle, items: Vec<Item>) -> CmdResult<RunState> {
    // winget'in hazır olup olmadığına bakmak bir an sürer; arayüz donmasın.
    blocking(move || app.state::<Installer>().start(&app, items)).await
}

#[tauri::command]
pub fn install_stop(app: AppHandle) {
    app.state::<Installer>().stop(&app);
}

/// Kurulu programların kimlikleri (küçük harf). İlk bakış birkaç saniye sürebilir.
#[tauri::command]
pub async fn install_installed(app: AppHandle, refresh: bool) -> CmdResult<Vec<String>> {
    blocking(move || app.state::<Installer>().installed(refresh)).await
}

#[tauri::command]
pub async fn install_search(query: String) -> CmdResult<Vec<Hit>> {
    blocking(move || install::search(&query)).await
}

#[tauri::command]
pub fn install_find_setup(app: AppHandle) -> Option<String> {
    kit::find_setup(&home(&app)).map(|p| p.to_string_lossy().into_owned())
}

/// Yedeğin aşağı yukarı boyutu.
#[tauri::command]
pub async fn install_backup_size(app: AppHandle, options: Options) -> CmdResult<u64> {
    blocking(move || Ok(restore::estimate(app.state::<DataDir>().root(), options))).await
}

#[derive(Serialize, Clone)]
struct KitProgress<'a> {
    stage: &'a str,
    done: u64,
    total: u64,
}

/// Format Kiti'ni oluşturur; ilerleme "install-kit" olayıyla gelir.
#[tauri::command]
pub async fn install_export(app: AppHandle, request: KitRequest) -> CmdResult<KitResult> {
    blocking(move || {
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        let mut last = Instant::now() - Duration::from_secs(1);
        kit::build(
            &db,
            dir.root(),
            &home(&app),
            &request,
            &mut |stage, done, total| {
                if done == total || last.elapsed() >= Duration::from_millis(200) {
                    last = Instant::now();
                    let _ = app.emit("install-kit", KitProgress { stage, done, total });
                }
            },
        )
        .inspect_err(|e| crate::log!("Format Kiti oluşturulamadı: {e}"))
    })
    .await
}

#[tauri::command]
pub fn install_open_kit(app: AppHandle, path: String) -> CmdResult<()> {
    let p = Path::new(&path);
    if !p.is_dir() || p.file_name().map_or(true, |n| n != kit::KIT_DIR) {
        return Err("Kit klasörü bulunamadı".into());
    }
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| format!("Klasör açılamadı: {e}"))
}

/// Kontrol Odası → Veri: GMID-Yedek.zip'i doğrular ve hazırlar (henüz hiçbir şey değişmez).
#[tauri::command]
pub async fn backup_restore_stage(app: AppHandle, path: String) -> CmdResult<Manifest> {
    blocking(move || restore::stage(Path::new(&path), app.state::<DataDir>().root())).await
}

#[tauri::command]
pub fn backup_restore_cancel(dir: State<'_, DataDir>) {
    restore::cancel_staged(dir.root());
}

/// Hazırlanan yedeği uygulamak için yeniden başlatır (veritabanı kapalıyken yerine konur).
#[tauri::command]
pub fn backup_restore_apply(app: AppHandle) -> CmdResult<()> {
    restore::confirm(app.state::<DataDir>().root())?;
    crate::log!("Yedekten geri yükleme için yeniden başlatılıyor");
    app.request_restart();
    Ok(())
}
