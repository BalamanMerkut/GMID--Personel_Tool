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
pub async fn clips_fit_discord(
    app: AppHandle,
    id: String,
    options: Option<clips::FitOptions>,
) -> CmdResult<clips::FitResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let opts = options.unwrap_or_else(|| clips::saved_options(&app.state::<Db>()));
        app.state::<Clips>().fit_discord(&app, &id, opts)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Sosyal'de paylaşma isteği (paylaşım penceresinden).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareRequest {
    id: String,
    server: String,
    token: String,
    title: Option<String>,
    game: Option<String>,
    /// Kısa açıklama.
    note: Option<String>,
    /// Seçilen bölüm (saniye); yoksa klibin tamamı.
    start: Option<f64>,
    end: Option<f64>,
}

/// Klibi Sosyal'de paylaş: (istenirse kırpılıp) küçültülür (H.264, en çok 50 MB; ilerleme "clip-fit"
/// olayıyla), sonra GMID sunucusuna yüklenir. Akışta 7 gün görünür; aldığı beğeniler profilde kalır.
#[tauri::command]
pub async fn clips_share_social(
    app: AppHandle,
    request: ShareRequest,
) -> CmdResult<serde_json::Value> {
    tauri::async_runtime::spawn_blocking(move || {
        let r = request;
        let clip = clips::lookup(&app.state::<Db>(), &r.id)?;
        if clip.kind != "clip" {
            return Err("Yalnız video klipler paylaşılabilir".to_string());
        }
        let (hazir, kapak) = app
            .state::<Clips>()
            .fit_for_share(&app, &r.id, r.start, r.end)?;
        let ad = r
            .title
            .filter(|t| !t.trim().is_empty())
            .or(clip.title.clone())
            .unwrap_or(clip.name.clone());
        // Kırpıldıysa seçilen bölümün uzunluğu.
        let sure = r
            .end
            .or(clip.duration)
            .map(|e| (e - r.start.unwrap_or(0.0).max(0.0)).max(0.0));
        let bilgi = crate::social::KlipBilgi {
            ad: &ad,
            oyun: r.game.as_deref(),
            not: r.note.as_deref(),
            sure,
        };
        let sonuc = crate::social::klip_yukle(
            &r.server,
            &r.token,
            std::path::Path::new(&hazir.path),
            kapak
                .as_deref()
                .or(clip.thumb.as_deref().map(std::path::Path::new)),
            &bilgi,
        );
        // Yüklenen kopya ve kapağı artık gerekmiyor (klibin kendisi yerinde duruyor).
        if !hazir.original {
            let _ = std::fs::remove_file(&hazir.path);
        }
        if let Some(k) = kapak {
            let _ = std::fs::remove_file(k);
        }
        sonuc
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Seçime göre beklenen çözünürlük, kare hızı ve netlik.
#[tauri::command]
pub fn clips_plan(
    db: State<'_, Db>,
    id: String,
    options: clips::FitOptions,
) -> CmdResult<crate::clips::media::Plan> {
    Clips::plan(&db, &id, &options)
}

/// "Discord'a hazırla" seçenekleri (seçime göre: boyut, çözünürlük, kare hızı, beklenen netlik).
#[tauri::command]
pub async fn clips_choices(
    app: AppHandle,
    id: String,
    start: Option<f64>,
    end: Option<f64>,
) -> CmdResult<Vec<clips::Choice>> {
    tauri::async_runtime::spawn_blocking(move || Clips::choices(&app, &id, start, end))
        .await
        .map_err(|e| e.to_string())?
}

/// Hazırlanan kopyayı yeniden panoya koyar (yalnız "Discord'a Hazır" klasöründekiler).
#[tauri::command]
pub fn clips_ready_copy(app: AppHandle, path: String) -> CmdResult<()> {
    clips::copy_ready(&app, &path)
}

/// Hazırlanan kopyayı klasöründe gösterir.
#[tauri::command]
pub fn clips_ready_reveal(app: AppHandle, path: String) -> CmdResult<()> {
    let p = clips::ready_path(&app, &path)?;
    app.opener()
        .reveal_item_in_dir(p)
        .map_err(|e| format!("Klasör açılamadı: {e}"))
}
