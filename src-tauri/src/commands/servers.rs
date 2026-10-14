//! Sunucular ekranının komutları. Ağa çıkanlar arayüzü dondurmasın diye arka planda çalışır.

use crate::servers::{
    self, games, JoinResult, Launch, Server, ServerInput, Servers, Status, SyncResult,
};
use crate::{db::Db, paths::DataDir};
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
pub fn servers_games() -> Vec<games::GameInfo> {
    games::catalog()
}

#[tauri::command]
pub fn servers_list(db: State<'_, Db>) -> CmdResult<Vec<Server>> {
    servers::list(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn server_save(
    db: State<'_, Db>,
    dir: State<'_, DataDir>,
    input: ServerInput,
) -> CmdResult<Server> {
    servers::save(&db, &dir.servers(), input)
}

#[tauri::command]
pub fn server_delete(db: State<'_, Db>, dir: State<'_, DataDir>, id: String) -> CmdResult<()> {
    servers::delete(&db, &dir.servers(), &id)
}

#[tauri::command]
pub fn server_import_logo(dir: State<'_, DataDir>, source: String) -> CmdResult<String> {
    servers::import_logo(&dir.servers(), &source)
}

/// Topluluk listesini indirir (gerekiyorsa) ve tüm sunucuları döner.
#[tauri::command]
pub async fn servers_sync(app: AppHandle, force: bool) -> CmdResult<SyncResult> {
    blocking(app, move |app| {
        let (db, dir, state) = (
            app.state::<Db>(),
            app.state::<DataDir>(),
            app.state::<Servers>(),
        );
        let r = servers::sync(&db, &dir.servers(), &state, force);
        // Davet bağlantısı listeyle gelir: yan menü güncellensin.
        let _ = tauri::Emitter::emit(app, "community", servers::discord_link(&db));
        r
    })
    .await
}

/// GMID Discord davet bağlantısı (yoksa `None`).
#[tauri::command]
pub fn community_discord(db: State<'_, Db>) -> Option<String> {
    servers::discord_link(&db)
}

/// Son bilinen durumlar (ekran açılır açılmaz göstermek için).
#[tauri::command]
pub fn servers_cached(state: State<'_, Servers>) -> Vec<Status> {
    state.cached()
}

/// Verilen sunucuları (boşsa hepsini) şimdi sorgular.
#[tauri::command]
pub async fn servers_query(app: AppHandle, ids: Vec<String>) -> CmdResult<Vec<Status>> {
    blocking(app, move |app| {
        let all = servers::list(&app.state::<Db>()).unwrap_or_default();
        let targets: Vec<Server> = all
            .into_iter()
            .filter(|s| ids.is_empty() || ids.contains(&s.id))
            .collect();
        app.state::<Servers>().query(&targets)
    })
    .await
}

/// Oyunu açıp sunucuya sokar. Oyunun .exe'si bilinmiyorsa `EXE_NEEDED:<oyun>` hatası döner;
/// arayüz dosyayı seçtirip [`server_set_exe`] ile kaydeder ve tekrar dener.
#[tauri::command]
pub fn server_join(app: AppHandle, db: State<'_, Db>, id: String) -> CmdResult<JoinResult> {
    let server = servers::get(&db, &id)
        .map_err(|e| e.to_string())?
        .ok_or("Sunucu bulunamadı")?;
    let address = format!("{}:{}", server.host, server.port);
    match servers::launch_plan(&db, &server)? {
        Launch::Url(url) => {
            app.opener().open_url(&url, None::<&str>).map_err(|_| {
                "Oyun açılamadı. Oyun (ya da Steam) bu bilgisayarda kurulu mu?".to_string()
            })?;
        }
        Launch::Exe {
            path,
            address,
            password,
        } => servers::run_exe(&path, &address, &password)?,
        Launch::Copy(address) => {
            return Ok(JoinResult {
                kind: "copy",
                address,
            })
        }
    }
    Ok(JoinResult {
        kind: "launched",
        address,
    })
}

#[tauri::command]
pub fn server_set_exe(db: State<'_, Db>, game: String, path: String) -> CmdResult<()> {
    servers::set_exe(&db, &game, &path)
}

/// Sunucu bildirim kuralları (sunucu kimliği → kural).
#[tauri::command]
pub fn servers_watch_rules(
    db: State<'_, Db>,
) -> std::collections::HashMap<String, servers::watch::Rule> {
    servers::watch::rules(&db)
}

#[tauri::command]
pub fn servers_watch_set(
    db: State<'_, Db>,
    id: String,
    rule: servers::watch::Rule,
) -> CmdResult<()> {
    servers::watch::set_rule(&db, &id, rule)
}
