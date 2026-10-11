//! Oyun kütüphanesi ve oturum geçmişi.

use crate::db::Db;
use crate::games::{scan::Found, Game, Games, RunningApp, Session};
use tauri::{AppHandle, Emitter, Manager, State};
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

/// Kapağı elle seçilen resimle değiştirir (Steam'de olmayan oyunlar: Riot, Battle.net…). Resim
/// `Kapaklar\Oyunlar` içine kopyalanır; orijinal silinse de kapak kalır.
#[tauri::command]
pub fn games_set_cover(
    app: AppHandle,
    dir: State<'_, crate::paths::DataDir>,
    db: State<'_, Db>,
    id: String,
    source: String,
) -> CmdResult<()> {
    let src = std::path::Path::new(&source);
    let ext = src
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| ["jpg", "jpeg", "png", "webp", "gif"].contains(&e.as_str()))
        .ok_or("Bir resim seç (JPG, PNG, WebP, GIF)")?;
    let dest = dir
        .folder(Some("Kapaklar"))
        .join("Oyunlar")
        .join(format!("elle-{id}-{}.{ext}", crate::util::now_ms()));
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::copy(src, &dest).map_err(|e| format!("Resim kopyalanamadı: {e}"))?;
    db.conn()
        .execute(
            "UPDATE games SET cover = ?2 WHERE id = ?1",
            rusqlite::params![id, dest.to_string_lossy()],
        )
        .map_err(|e| e.to_string())?;
    let _ = app.emit("games-changed", ());
    Ok(())
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
        // `"C:/Riot Games/Riot Client/RiotClientServices.exe" --launch-product=valorant` gibi argümanlı da olabilir.
        let (exe, args) = crate::games::scan::split_command(&target);
        let path = std::path::Path::new(&exe);
        let mut cmd = std::process::Command::new(path);
        cmd.args(args);
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

// ---- Oyun Arşivi ----

/// Steam mağazasında adıyla arar (arşive kütüphanede olmayan oyun eklemek için).
#[tauri::command]
pub async fn games_store_search(
    app: AppHandle,
    term: String,
) -> CmdResult<Vec<crate::games::meta::StoreHit>> {
    if term.trim().len() < 2 {
        return Ok(Vec::new());
    }
    blocking(app, move |_| crate::games::meta::store_search(&term)).await?
}

/// Arşive oyun ekler (Steam kimliğiyle ya da yalnızca adıyla); oyunun kimliği.
#[tauri::command]
pub fn games_add_archive(app: AppHandle, appid: Option<i64>, name: String) -> CmdResult<String> {
    app.state::<Games>().add_archive(&app, appid, &name)
}

#[tauri::command]
pub fn reviews_list(db: State<'_, Db>) -> CmdResult<Vec<crate::reviews::Review>> {
    crate::reviews::list(&db, "game")
}

#[tauri::command]
pub fn review_save(
    app: AppHandle,
    input: crate::reviews::ReviewInput,
) -> CmdResult<crate::reviews::Review> {
    let r = crate::reviews::save(&app.state::<Db>(), input)?;
    let _ = app.emit("reviews-changed", ());
    Ok(r)
}

#[tauri::command]
pub fn review_delete(app: AppHandle, id: String) -> CmdResult<()> {
    crate::reviews::delete(&app.state::<Db>(), &id)?;
    let _ = app.emit("reviews-changed", ());
    let _ = app.emit("games-changed", ());
    Ok(())
}
