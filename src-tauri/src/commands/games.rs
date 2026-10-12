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

/// `kind`: game (varsayılan) | title (film/dizi).
#[tauri::command]
pub fn reviews_list(
    db: State<'_, Db>,
    kind: Option<String>,
) -> CmdResult<Vec<crate::reviews::Review>> {
    crate::reviews::list(&db, kind.as_deref().unwrap_or("game"))
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
    let _ = app.emit("titles-changed", ());
    Ok(())
}

// ---- Film & Dizi Arşivi ----

#[tauri::command]
pub fn titles_list(db: State<'_, Db>) -> CmdResult<Vec<crate::titles::Title>> {
    crate::titles::list(&db)
}

/// Wikipedia'da film/dizi arar.
#[tauri::command]
pub async fn titles_search(app: AppHandle, term: String) -> CmdResult<Vec<crate::titles::Hit>> {
    if term.trim().chars().count() < 2 {
        return Ok(Vec::new());
    }
    blocking(app, move |_| crate::titles::search(&term)).await?
}

/// Arama sonucundan (`hit`) ya da yalnızca adıyla ekler.
#[tauri::command]
pub fn titles_add(
    app: AppHandle,
    hit: Option<crate::titles::Hit>,
    name: Option<String>,
    kind: Option<String>,
) -> CmdResult<crate::titles::Title> {
    let t = crate::titles::add(
        &app,
        hit,
        name.as_deref().unwrap_or_default(),
        kind.as_deref().unwrap_or("movie"),
    )?;
    let _ = app.emit("titles-changed", ());
    Ok(t)
}

/// Puanlanmadan vazgeçildiyse eklenen film/dizi geri alınır.
#[tauri::command]
pub fn titles_discard(app: AppHandle, id: String) -> CmdResult<()> {
    crate::titles::remove_if_unrated(&app.state::<Db>(), &id)?;
    let _ = app.emit("titles-changed", ());
    Ok(())
}

#[tauri::command]
pub fn titles_refresh(app: AppHandle, id: String) -> CmdResult<()> {
    crate::titles::refresh(&app, &id)
}

/// Elle seçilen afiş.
#[tauri::command]
pub fn titles_set_poster(app: AppHandle, id: String, source: String) -> CmdResult<()> {
    let posters = app
        .state::<crate::paths::DataDir>()
        .folder(Some("Kapaklar"))
        .join("Filmler");
    crate::titles::set_poster(&app.state::<Db>(), &posters, &id, &source)?;
    let _ = app.emit("titles-changed", ());
    Ok(())
}

/// Kütüphanede olmayan, açıldığı görülen oyunlar ("Kütüphaneye ekleyeyim mi?").
#[tauri::command]
pub fn games_suggestions(
    games: State<'_, Games>,
    db: State<'_, Db>,
) -> Vec<crate::games::Suggestion> {
    games.suggestions(&db)
}

/// `action`: add | later | never. `name` ekleme sırasında adı düzeltmek için.
#[tauri::command]
pub fn games_suggestion_answer(
    app: AppHandle,
    path: String,
    action: String,
    name: Option<String>,
) -> CmdResult<()> {
    app.state::<Games>()
        .answer_suggestion(&app, &path, &action, name)
}

// ---- Oyun Modu ----

#[tauri::command]
pub fn gamemode_status(gm: State<'_, crate::gamemode::GameMode>) -> crate::gamemode::Status {
    gm.status()
}

/// Elle açar/kapatır; kapatınca her şey geri alınır.
#[tauri::command]
pub async fn gamemode_set(app: AppHandle, on: bool) -> CmdResult<crate::gamemode::Status> {
    tauri::async_runtime::spawn_blocking(move || {
        let gm = app.state::<crate::gamemode::GameMode>();
        if on {
            gm.enable(&app, "manual")
        } else {
            gm.disable(&app)
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Oyun İçi Panel'i kapatır, odağı oyuna geri verir.
#[tauri::command]
pub fn panel_hide(app: AppHandle) {
    crate::panel::hide(&app);
}

/// Panel açıkken saniyede bir: FPS, işlemci, RAM, ekran kartı.
#[tauri::command]
pub async fn panel_perf(app: AppHandle) -> CmdResult<crate::perf::Sample> {
    tauri::async_runtime::spawn_blocking(move || {
        let pid = crate::panel::game_pid(&app);
        let root = app.state::<crate::paths::DataDir>().root().to_path_buf();
        app.state::<crate::perf::Perf>().sample(pid, &root)
    })
    .await
    .map_err(|e| e.to_string())
}

/// FPS ölçümünün (PresentMon) durumu.
#[tauri::command]
pub fn fps_status(app: AppHandle) -> crate::fps::Status {
    let root = app.state::<crate::paths::DataDir>().root().to_path_buf();
    let requested = app
        .state::<Db>()
        .setting_json::<bool>("fps.requested2")
        .unwrap_or(false);
    app.state::<crate::perf::Perf>()
        .presentmon
        .status(&root, requested)
}

/// FPS ölçümünü aç: PresentMon'u indir, bir kez yönetici izniyle izin ver.
#[tauri::command]
pub async fn fps_setup(app: AppHandle) -> CmdResult<crate::fps::Status> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = app.state::<crate::paths::DataDir>().root().to_path_buf();
        let st = app.state::<crate::perf::Perf>().presentmon.setup(&root)?;
        if st.relogin {
            let _ = app.state::<Db>().set_setting("fps.requested2", "true");
        }
        Ok(st)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Kenarlıksız tam ekrana çevrilen oyunlar (exe adları).
#[tauri::command]
pub fn borderless_list(db: State<'_, Db>) -> Vec<String> {
    crate::borderless::list(&db)
}

#[tauri::command]
pub fn borderless_set(db: State<'_, Db>, exe: String, on: bool) -> CmdResult<Vec<String>> {
    crate::borderless::set(&db, &exe, on)
}

/// Bakış kipinden etkileşime: panel odak alır, oyun fareyi bırakır.
#[tauri::command]
pub fn panel_interact(app: AppHandle) {
    crate::panel::interact(&app);
}

/// Not yazmak için panel klavye odağını alır.
#[tauri::command]
pub fn panel_focus(app: AppHandle) {
    crate::panel::take_focus(&app);
}
