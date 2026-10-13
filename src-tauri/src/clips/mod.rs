//! Kliplerim: NVIDIA, Xbox Game Bar, AMD, OBS, Steam ve Medal'ın kaydettiği klip ve ekran görüntüleri.
//! Dosyalar **taşınmaz**, yerinde kalır; GMID onları bulur, oyuna göre gruplar ve gösterir.
//!
//! - **İzleme:** 15 sn'de bir kayıt klasörlerine bakılır; yeni dosya, boyutu iki tur sabit kalınca (yazılması
//!   bitince) eklenir ve ekranda "Klip kaydedildi — Valorant (0:30)" görünür.
//! - **Oyun:** Steam ekran görüntüsünde appid; yoksa çekildiği an açık olan oyun (oturum geçmişi); yoksa
//!   klasör/dosya adındaki oyun adı.
//! - **Discord'a sığdır:** 10 MB'a sığan kopya (`Discord'a Hazır`, 3 gün sonra silinir), panoya dosya olarak.

pub mod detect;
pub mod media;
pub mod win;

use crate::db::Db;
use crate::paths::DataDir;
use crate::util::{new_id, now_ms};
use detect::{FoundFile, Root};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const TICK: Duration = Duration::from_secs(15);
pub const FOLDERS_KEY: &str = "clips.folders";
pub const OSD_KEY: &str = "clips.osd";
pub const CLEANUP_KEY: &str = "clips.cleanupDays";
const READY_DIR: &str = "Discord'a Hazır";

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: String,
    pub path: String,
    /// Dosya adı (uzantısız).
    pub name: String,
    pub kind: String,
    pub source: String,
    pub game_id: Option<String>,
    pub game_hint: Option<String>,
    pub title: Option<String>,
    pub size: i64,
    pub duration: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub thumb: Option<String>,
    pub favorite: bool,
    pub created_at: i64,
}

/// Ayarlarda gösterilen izlenen klasör.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RootInfo {
    pub path: String,
    pub label: String,
    /// Kullanıcı eklediyse kaldırılabilir.
    pub custom: bool,
    pub exists: bool,
}

#[derive(Default)]
pub struct Clips {
    /// Yazılması süren dosyalar: yol → son görülen boyut.
    pending: Mutex<HashMap<String, u64>>,
    /// İlk tarama bitti mi (ilkinde eski klipler için gösterge çıkmaz).
    started: Mutex<bool>,
    /// Discord'a sığdırılan klipler (aynı klip iki kez başlatılmasın).
    fitting: Mutex<HashSet<String>>,
    /// Kapaklar için ffmpeg bu oturumda indirilmeye başlandı mı (bir kez).
    ffmpeg_fetch: std::sync::atomic::AtomicBool,
}

const COLUMNS: &str =
    "id, path, kind, source, game_id, game_hint, title, size, duration, width, height,
    thumb, favorite, created_at";

fn from_row(r: &Row) -> rusqlite::Result<Clip> {
    let path: String = r.get(1)?;
    Ok(Clip {
        id: r.get(0)?,
        name: Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path,
        kind: r.get(2)?,
        source: r.get(3)?,
        game_id: r.get(4)?,
        game_hint: r.get(5)?,
        title: r.get(6)?,
        size: r.get(7)?,
        duration: r.get(8)?,
        width: r.get(9)?,
        height: r.get(10)?,
        thumb: r.get(11)?,
        favorite: r.get::<_, i64>(12)? != 0,
        created_at: r.get(13)?,
    })
}

/// İzlenen klasörler: bilinen programların varsayılanları + kullanıcının ekledikleri.
pub fn roots(app: &AppHandle) -> Vec<(Root, String, bool)> {
    let mut out: Vec<(Root, String, bool)> = Vec::new();
    if let Ok(v) = app.path().video_dir() {
        out.push((
            Root::Videos(v),
            "Videolar (NVIDIA, Xbox Game Bar, AMD, OBS)".into(),
            false,
        ));
    }
    if let Some(steam) = crate::games::scan::steam_dir() {
        out.push((
            Root::SteamUserdata(steam.join("userdata")),
            "Steam ekran görüntüleri".into(),
            false,
        ));
    }
    let medal = PathBuf::from(r"C:\Medal");
    if medal.is_dir() {
        out.push((Root::Medal(medal), "Medal".into(), false));
    }
    let custom: Vec<String> = app
        .state::<Db>()
        .setting_json(FOLDERS_KEY)
        .unwrap_or_default();
    for c in custom {
        out.push((
            Root::Folder(PathBuf::from(&c)),
            "Eklediğin klasör".into(),
            true,
        ));
    }
    out
}

pub fn root_infos(app: &AppHandle) -> Vec<RootInfo> {
    roots(app)
        .into_iter()
        .map(|(r, label, custom)| RootInfo {
            path: r.path().to_string_lossy().into_owned(),
            exists: r.path().is_dir(),
            label,
            custom,
        })
        .collect()
}

pub fn list(db: &Db) -> Result<Vec<Clip>, String> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM captures ORDER BY created_at DESC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], from_row)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// Video İndirici'nin videoları da kırpılıp Discord'a hazırlanabilir: kimlikleri "dl:<indirme kimliği>".
pub const DL_PREFIX: &str = "dl:";

/// Klip ya da (kimlik "dl:" ile başlıyorsa) indirilen video.
fn lookup(db: &Db, id: &str) -> Result<Clip, String> {
    let Some(dl) = id.strip_prefix(DL_PREFIX) else {
        return get(db, id);
    };
    let d = crate::downloader::get(db, dl)
        .map_err(|e| e.to_string())?
        .ok_or("Video bulunamadı")?;
    let path = d.file_path.clone().ok_or("Videonun dosyası yok")?;
    let name = Path::new(&path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| d.title.clone());
    let height = d.quality.trim_end_matches('p').parse::<i64>().ok();
    Ok(Clip {
        id: id.to_string(),
        path,
        name,
        kind: if d.audio_only { "audio" } else { "clip" }.into(),
        source: "video".into(),
        game_id: None,
        game_hint: None,
        title: Some(d.title),
        size: d.size.unwrap_or(0) as i64,
        duration: d.duration,
        width: None,
        height,
        thumb: d.thumbnail,
        favorite: false,
        created_at: d.created_at,
    })
}

pub fn get(db: &Db, id: &str) -> Result<Clip, String> {
    db.conn()
        .query_row(
            &format!("SELECT {COLUMNS} FROM captures WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Klip bulunamadı".into())
}

/// Kütüphanedeki oyunlar: (kimlik, ad).
fn game_names(db: &Db) -> Vec<(String, String)> {
    let conn = db.conn();
    let mut stmt = match conn.prepare("SELECT id, name FROM games") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
}

/// Klibin oyunu: Steam appid → çekildiği an açık olan oyun → klasör/dosya adı.
fn guess_game(
    db: &Db,
    games: &[(String, String)],
    appid: Option<i64>,
    at: i64,
    hint: Option<&str>,
) -> Option<String> {
    let conn = db.conn();
    if let Some(a) = appid {
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM games WHERE steam_appid = ?1 ORDER BY in_library DESC LIMIT 1",
                [a],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten();
        if id.is_some() {
            return id;
        }
    }
    // Oturum: başlamadan 1 dk önce ile bittikten 2 dk sonrası arası (kayıt, oyun kapanınca yazılabiliyor).
    let id: Option<String> = conn
        .query_row(
            "SELECT game_id FROM play_sessions
             WHERE started_at - 60000 <= ?1 AND COALESCE(ended_at, ?2) + 120000 >= ?1
             ORDER BY started_at DESC LIMIT 1",
            params![at, now_ms()],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    if id.is_some() {
        return id;
    }
    drop(conn);
    hint.and_then(|h| detect::match_hint(h, games))
        .map(str::to_string)
}

fn insert(db: &Db, f: &FoundFile, game: Option<String>) -> Result<String, String> {
    let id = new_id();
    db.conn()
        .execute(
            "INSERT OR IGNORE INTO captures (id, path, kind, source, game_id, game_hint, steam_appid, size,
                created_at, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                f.path.to_string_lossy(),
                f.kind,
                f.source,
                game,
                f.hint,
                f.appid,
                f.size as i64,
                f.modified,
                now_ms()
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(id)
}

impl Clips {
    /// Bir tarama turu: yeni dosyaları ekler, silinenleri çıkarır. Yeni eklenen kliplerin kimlikleri.
    pub fn scan(&self, app: &AppHandle) -> Vec<String> {
        let db = app.state::<Db>();
        let roots = roots(app);
        for (r, _, _) in &roots {
            if r.path().is_dir() {
                let _ = app.asset_protocol_scope().allow_directory(r.path(), true);
            }
        }
        let root_list: Vec<Root> = roots.into_iter().map(|(r, _, _)| r).collect();
        let found = detect::scan(&root_list);
        let known: HashSet<String> = {
            let conn = db.conn();
            let mut stmt = match conn.prepare("SELECT path FROM captures") {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };
            stmt.query_map([], |r| r.get::<_, String>(0))
                .map(|rows| rows.filter_map(Result::ok).collect())
                .unwrap_or_default()
        };
        let first = !*self.started.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();
        let games = game_names(&db);
        let running = app.state::<crate::games::Games>().running_ids();
        let mut added = Vec::new();
        let mut inserted = 0;
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        let mut still = HashMap::new();
        for f in &found {
            let key = f.path.to_string_lossy().into_owned();
            if known.contains(&key) {
                continue;
            }
            // Yazılması bitti mi: boyut bir tur sabit kaldı ve son değişiklik 5 sn'den eski.
            let settled = now - f.modified > 5_000 && (first || pending.get(&key) == Some(&f.size));
            if !settled {
                still.insert(key, f.size);
                continue;
            }
            let fresh = !first && now - f.modified < 5 * 60_000;
            // Yeni kaydedilen klip: şu an açık oyun (oturum henüz veritabanına yazılmamış olabilir).
            let game = if fresh && f.appid.is_none() && running.len() == 1 {
                Some(running[0].clone())
            } else {
                guess_game(&db, &games, f.appid, f.modified, f.hint.as_deref())
            };
            if let Ok(id) = insert(&db, f, game) {
                inserted += 1;
                if fresh {
                    added.push(id);
                }
            }
        }
        *pending = still;
        drop(pending);
        // Silinen dosyalar (klasörü duran: çıkarılmış disk ise dokunma).
        let gone: Vec<(String, String, Option<String>)> = {
            let conn = db.conn();
            let mut stmt = match conn.prepare("SELECT id, path, thumb FROM captures") {
                Ok(s) => s,
                Err(_) => return added,
            };
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map(|rows| {
                    rows.filter_map(Result::ok)
                        .filter(|(_, p, _): &(String, String, Option<String>)| {
                            let p = Path::new(p);
                            !p.exists() && p.parent().is_some_and(|d| d.exists())
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        // Kapak dosyası silinmişse (Kapaklar klasörü temizlendi) yeniden çıkarılır.
        {
            let conn = db.conn();
            let lost: Vec<String> = conn
                .prepare("SELECT id, thumb FROM captures WHERE thumb IS NOT NULL")
                .and_then(|mut st| {
                    st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                        .map(|rows| {
                            rows.filter_map(Result::ok)
                                .filter(|(_, t)| !Path::new(t).is_file())
                                .map(|(id, _)| id)
                                .collect()
                        })
                })
                .unwrap_or_default();
            for id in lost {
                let _ = conn.execute(
                    "UPDATE captures SET thumb = NULL, thumb_at = NULL WHERE id = ?1",
                    [id],
                );
            }
        }
        for (id, _, thumb) in &gone {
            let _ = db
                .conn()
                .execute("DELETE FROM captures WHERE id = ?1", [id]);
            if let Some(t) = thumb {
                let _ = std::fs::remove_file(t);
            }
        }
        *self.started.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.thumbs(app);
        if inserted > 0 || !gone.is_empty() {
            let _ = app.emit("clips-changed", ());
        }
        added
    }

    /// Küçük resmi olmayanlar için ffmpeg (tur başına en çok 12). ffmpeg yoksa (Video İndirici'nin araçları hiç
    /// kurulmadıysa) arka planda yalnızca ffmpeg indirilir, bitince kapaklar çıkarılır. Çıkarılamayanlar 6 saat
    /// sonra yeniden denenir; yazılması süren dosyalar (son 2 dk'da değişmiş) sonraki tura kalır.
    fn thumbs(&self, app: &AppHandle) {
        let dir = app.state::<DataDir>();
        let db = app.state::<Db>();
        let retry_before = now_ms() - 6 * 3600 * 1000;
        let todo: Vec<(String, String, String)> = {
            let conn = db.conn();
            let mut stmt = match conn.prepare(
                "SELECT id, path, kind FROM captures
                 WHERE thumb_at IS NULL OR (thumb IS NULL AND thumb_at < ?1)
                 ORDER BY created_at DESC LIMIT 12",
            ) {
                Ok(s) => s,
                Err(_) => return,
            };
            stmt.query_map([retry_before], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map(|rows| rows.filter_map(Result::ok).collect())
                .unwrap_or_default()
        };
        if todo.is_empty() {
            return;
        }
        let Some(ff) = ffmpeg(dir.root()) else {
            use std::sync::atomic::Ordering;
            if cfg!(windows) && !self.ffmpeg_fetch.swap(true, Ordering::SeqCst) {
                crate::log!("Klip kapakları için ffmpeg indiriliyor");
                let _ = app.emit("clips-thumbs", "ffmpeg");
                let app = app.clone();
                std::thread::spawn(move || {
                    let root = app.state::<DataDir>().root().to_path_buf();
                    let r = app
                        .state::<crate::downloader::tools::Tools>()
                        .install_only(&app, &root, "ffmpeg");
                    match r {
                        Ok(()) => {
                            let _ = app.emit("clips-thumbs", "ready");
                            app.state::<Clips>().thumbs(&app);
                        }
                        Err(e) => {
                            crate::log!("Klip kapakları: ffmpeg indirilemedi: {e}");
                            let _ = app.emit("clips-thumbs", "failed");
                        }
                    }
                });
            }
            return;
        };
        let out_dir = dir.folder(Some("Kapaklar")).join("Klipler");
        let fresh = |p: &str| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age < std::time::Duration::from_secs(120))
        };
        let mut any = false;
        for (id, path, kind) in todo {
            if fresh(&path) {
                continue;
            }
            let out = out_dir.join(format!("{id}.jpg"));
            let r = media::thumbnail(&ff, Path::new(&path), &out, kind == "clip");
            let (thumb, dur, size) = match r {
                Ok(p) => (Some(out.to_string_lossy().into_owned()), p.duration, p.size),
                Err(e) => {
                    crate::log!("Klip küçük resmi çıkarılamadı ({path}): {e}");
                    (None, None, None)
                }
            };
            let _ = db.conn().execute(
                "UPDATE captures SET thumb = ?2, duration = ?3, width = ?4, height = ?5, thumb_at = ?6
                 WHERE id = ?1",
                params![
                    id,
                    thumb,
                    dur,
                    size.map(|s| s.0),
                    size.map(|s| s.1),
                    now_ms()
                ],
            );
            any = true;
        }
        if any {
            let _ = app.emit("clips-changed", ());
        }
    }

    /// Discord'a sığan kopya üretir ve panoya koyar; kopyanın yolu ve boyutu.
    pub fn fit_discord(
        &self,
        app: &AppHandle,
        id: &str,
        opts: FitOptions,
    ) -> Result<FitResult, String> {
        let db = app.state::<Db>();
        let clip = lookup(&db, id)?;
        let src = PathBuf::from(&clip.path);
        if !src.is_file() {
            return Err("Dosya bulunamadı (silinmiş ya da taşınmış)".into());
        }
        let limit = opts.limit_mb.map(|mb| mb as u64 * 1_000_000);
        let trimmed = opts.start.is_some_and(|s| s > 0.05) || opts.end.is_some();
        // Kırpılmıyor ve zaten sığıyorsa kendisi.
        if !trimmed {
            if let Some(l) = limit {
                if clip.size as u64 <= l {
                    win::copy_file(&src)?;
                    return Ok(FitResult {
                        path: clip.path.clone(),
                        size: clip.size as u64,
                        height: clip.height.unwrap_or(0) as u32,
                        fps: 0,
                        original: true,
                    });
                }
            }
        }
        if clip.kind != "clip" {
            return Err("Ekran görüntüsü sığmıyor".into());
        }
        if !self
            .fitting
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.to_string())
        {
            return Err("Bu klip zaten hazırlanıyor".into());
        }
        let result = (|| {
            let dir = app.state::<DataDir>();
            let ff = ffmpeg(dir.root())
                .ok_or("Önce Video İndirici'de araçları kur (ffmpeg gerekiyor)")?;
            let (pdur, psize, pfps) = media::probe(&ff, &src);
            let duration = clip
                .duration
                .filter(|d| *d > 0.0)
                .or(pdur)
                .ok_or("Klibin süresi okunamadı")?;
            let start = opts.start.unwrap_or(0.0).clamp(0.0, duration);
            let end = opts
                .end
                .unwrap_or(duration)
                .clamp(start + 0.5, duration.max(start + 0.5));
            let (w, h) = psize
                .or(clip.width.zip(clip.height))
                .unwrap_or((1920, 1080));
            let job = media::Job {
                start,
                end,
                limit,
                codec: opts.codec,
                src: (w.max(16) as u32, h.max(16) as u32),
                src_fps: pfps.unwrap_or(60.0),
                prefer: media::Prefer {
                    smooth: opts.smooth,
                    keep: opts.keep,
                },
            };
            let base = clip.title.clone().unwrap_or(clip.name.clone());
            let out = match limit {
                // Farklı seçenekler yan yana denenebilsin: ad seçeneği söyler ("… (10 MB, akıcı).mp4").
                Some(l) => dir.folder(Some(READY_DIR)).join(format!(
                    "{} {} MB{}{}).mp4",
                    safe_name(&base).trim_end_matches(')'),
                    l / 1_000_000,
                    if opts.smooth {
                        ", akıcı"
                    } else if opts.keep {
                        ", orijinal çözünürlük"
                    } else {
                        ""
                    },
                    if opts.codec == media::Codec::H264 {
                        ", uyumlu"
                    } else {
                        ""
                    },
                )),
                // Kırpılmış kopya: klibin yanına; bir sonraki taramada listeye girer.
                None => {
                    unique(&src.with_file_name(format!("{} (kırpılmış).mp4", clean(&clip.name))))
                }
            };
            let app2 = app.clone();
            let id2 = id.to_string();
            let plan = media::encode(&ff, &src, &out, &job, &move |p| {
                let _ = app2.emit("clip-fit", serde_json::json!({ "id": id2, "progress": p }));
            })?;
            if limit.is_some() {
                win::copy_file(&out)?;
            }
            let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
            // İndirilen videonun kırpılmış kopyası Medya'da yeni bir video olarak görünsün.
            if let (None, Some(dl)) = (limit, id.strip_prefix(DL_PREFIX)) {
                crate::downloader::add_copy(&db, dl, &out, end - start, size)?;
                let _ = app.emit("downloads-changed", ());
            }
            Ok(FitResult {
                size,
                path: out.to_string_lossy().into_owned(),
                height: plan.height,
                fps: plan.fps,
                original: false,
            })
        })();
        self.fitting
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        result
    }

    /// Seçime göre beklenen sonuç (çözünürlük, kare hızı, netlik); kodlamadan önce göstermek için.
    /// Seçime göre hazır seçenekler (kare hızı ffmpeg ile okunur; yoksa 60 sayılır).
    pub fn choices(
        app: &AppHandle,
        id: &str,
        start: Option<f64>,
        end: Option<f64>,
    ) -> Result<Vec<Choice>, String> {
        let clip = lookup(&app.state::<Db>(), id)?;
        let duration = clip.duration.filter(|d| *d > 0.0).unwrap_or(30.0);
        let s = start.unwrap_or(0.0).clamp(0.0, duration);
        let e = end
            .unwrap_or(duration)
            .clamp(s + 0.5, duration.max(s + 0.5));
        let full = s <= 0.05 && e >= duration - 0.05;
        let probed = ffmpeg(app.state::<DataDir>().root())
            .map(|ff| media::probe(&ff, Path::new(&clip.path)));
        let fps = probed.as_ref().and_then(|p| p.2).unwrap_or(60.0);
        // Gerçek boyut (dikey videolar da doğru çıksın); okunamazsa kayıttaki.
        let src = probed
            .and_then(|p| p.1)
            .map(|(w, h)| (w.max(16) as u32, h.max(16) as u32))
            .unwrap_or((
                clip.width.unwrap_or(1920).max(16) as u32,
                clip.height.unwrap_or(1080).max(16) as u32,
            ));
        Ok(choices(
            e - s,
            full,
            clip.size.max(0) as u64,
            src,
            fps,
            (!full).then_some(s),
            (!full).then_some(e),
        ))
    }

    pub fn plan(db: &Db, id: &str, opts: &FitOptions) -> Result<media::Plan, String> {
        let clip = lookup(db, id)?;
        let duration = clip.duration.filter(|d| *d > 0.0).unwrap_or(30.0);
        let start = opts.start.unwrap_or(0.0).clamp(0.0, duration);
        let end = opts
            .end
            .unwrap_or(duration)
            .clamp(start + 0.5, duration.max(start + 0.5));
        let src = (
            clip.width.unwrap_or(1920).max(16) as u32,
            clip.height.unwrap_or(1080).max(16) as u32,
        );
        Ok(match opts.limit_mb {
            Some(mb) => media::plan_with(
                end - start,
                mb as u64 * 1_000_000,
                src,
                60.0,
                opts.codec,
                1.0,
                media::Prefer {
                    smooth: opts.smooth,
                    keep: opts.keep,
                },
            ),
            None => media::Plan {
                video: 0,
                audio: 192,
                height: src.1,
                fps: 60,
                quality: "great",
            },
        })
    }
}

/// "Discord'a Hazır" klasöründeki bir dosya (başka bir yol verilirse hata).
pub fn ready_path(app: &AppHandle, path: &str) -> Result<PathBuf, String> {
    let dir = app.state::<DataDir>().folder(Some(READY_DIR));
    let p = PathBuf::from(path);
    let ok = p.parent().is_some_and(|d| d == dir)
        && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
        && p.is_file();
    if ok {
        Ok(p)
    } else {
        Err("Dosya bulunamadı".into())
    }
}

pub fn copy_ready(app: &AppHandle, path: &str) -> Result<(), String> {
    win::copy_file(&ready_path(app, path)?)
}

/// Kaydedilmiş varsayılanlar (hedef boyut ve kodek); hızlı "Discord'a sığdır" bunları kullanır.
pub const FIT_KEY: &str = "clips.discord";

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct FitDefaults {
    limit_mb: Option<u32>,
    codec: Option<media::Codec>,
    smooth: Option<bool>,
    keep: Option<bool>,
}

pub fn saved_options(db: &Db) -> FitOptions {
    let d: FitDefaults = db.setting_json(FIT_KEY).unwrap_or_default();
    FitOptions {
        limit_mb: d.limit_mb.or(default_limit()),
        codec: d.codec.unwrap_or_default(),
        smooth: d.smooth.unwrap_or(false),
        // Seçenekler penceresinde başka bir şey seçilmediyse çözünürlük korunur.
        keep: d.keep.unwrap_or(d.smooth != Some(true)),
        ..FitOptions::default()
    }
}

/// "Discord'a sığdır" / "Kırp" seçenekleri.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FitOptions {
    #[serde(default)]
    pub start: Option<f64>,
    #[serde(default)]
    pub end: Option<f64>,
    /// Hedef boyut (MB); `None`: sınırsız (yalnız kırp).
    #[serde(default = "default_limit")]
    pub limit_mb: Option<u32>,
    #[serde(default)]
    pub codec: media::Codec,
    /// Kare hızını koru (60 fps), gerekirse çözünürlük düşsün.
    #[serde(default)]
    pub smooth: bool,
    /// Çözünürlüğü koru (tek tık "Discord'a sığdır"ın varsayılanı).
    #[serde(default)]
    pub keep: bool,
}

/// Hazırlanan kopya.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FitResult {
    pub path: String,
    pub size: u64,
    pub height: u32,
    pub fps: u32,
    /// Zaten sığıyordu: klibin kendisi kopyalandı.
    pub original: bool,
}

/// "Discord'a hazırla" penceresindeki hazır seçenekler.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub key: String,
    pub title: String,
    pub note: String,
    pub options: FitOptions,
    pub height: u32,
    pub fps: u32,
    /// Beklenen boyut (bayt).
    pub size: u64,
    /// great | good | ok | poor
    pub quality: &'static str,
    pub recommended: bool,
}

fn rank(q: &str) -> u8 {
    match q {
        "great" => 3,
        "good" => 2,
        "ok" => 1,
        _ => 0,
    }
}

/// Seçime (süre) göre seçenekler: 10 MB'ta net / akıcı / uyumlu, Nitro Basic (50 MB), Nitro (500 MB). Klip zaten
/// sığıyorsa "olduğu gibi". `src_size`: klibin tamamının boyutu, `full`: kırpılmadı.
pub fn choices(
    duration: f64,
    full: bool,
    src_size: u64,
    src: (u32, u32),
    src_fps: f64,
    start: Option<f64>,
    end: Option<f64>,
) -> Vec<Choice> {
    use media::Codec::{Av1, H264};
    let fps_src = src_fps.clamp(1.0, 60.0).round() as u32;
    let mut out: Vec<Choice> = Vec::new();
    let mut add = |key: &str,
                   title: &str,
                   note: &str,
                   mb: u32,
                   codec: media::Codec,
                   prefer: media::Prefer| {
        let limit = mb as u64 * 1_000_000;
        let opts = FitOptions {
            start,
            end,
            limit_mb: Some(mb),
            codec,
            smooth: prefer.smooth,
            keep: prefer.keep,
        };
        if full && src_size <= limit {
            // Kodlamaya gerek yok: klip olduğu gibi gider.
            if out.iter().any(|c| c.size == src_size) {
                return;
            }
            out.push(Choice {
                key: format!("orijinal-{mb}"),
                title: "Olduğu gibi".into(),
                note: format!(
                    "{} · zaten {mb} MB'tan küçük, orijinal kalite",
                    match mb {
                        10 => "Discord ücretsiz",
                        50 => "Nitro Basic",
                        _ => "Nitro",
                    }
                ),
                options: opts,
                height: src.1,
                fps: fps_src,
                size: src_size,
                quality: "great",
                recommended: false,
            });
            return;
        }
        // AV1 hedefi baştan %7 küçük tutulur (bkz. media::encode).
        let factor = if codec == Av1 { 0.93 } else { 1.0 };
        let p = media::plan_with(duration, limit, src, src_fps, codec, factor, prefer);
        // Çözünürlük bu uzunlukta korunamıyorsa bu seçenek gösterilmez (sonuç "Net" ile aynı olurdu).
        if prefer.keep && p.height < src.1.min(2160) {
            return;
        }
        let size = (((p.video + p.audio) as f64 * 1000.0 / 8.0 * duration) as u64).min(limit);
        // Aynı sonucu veren ikinci seçenek gösterilmez (ör. kısa klipte akıcı = net).
        if out.iter().any(|c| {
            c.options.limit_mb == Some(mb)
                && c.options.codec == codec
                && (c.height, c.fps) == (p.height, p.fps)
        }) {
            return;
        }
        out.push(Choice {
            key: key.into(),
            title: title.into(),
            note: note.into(),
            options: opts,
            height: p.height,
            fps: p.fps,
            size,
            quality: p.quality,
            recommended: false,
        });
    };
    use media::Prefer;
    let sharp = Prefer::default();
    add(
        "orijinal-cozunurluk",
        "Orijinal çözünürlük",
        "Tek tık “Discord'a sığdır” böyle yapar · çözünürlük aynı kalır, gerekirse 30 kare",
        10,
        Av1,
        Prefer {
            smooth: false,
            keep: true,
        },
    );
    add(
        "net-10",
        "Net",
        "Discord ücretsiz · AV1: aynı boyutta en net görüntü",
        10,
        Av1,
        sharp,
    );
    if fps_src > 35 {
        add(
            "akici-10",
            "Akıcı",
            "Discord ücretsiz · 60 kare korunur, görüntü biraz yumuşar",
            10,
            Av1,
            Prefer {
                smooth: true,
                keep: false,
            },
        );
    }
    add(
        "uyumlu-10",
        "Uyumlu",
        "Discord ücretsiz · H.264: her telefonda oynar, biraz daha bulanık",
        10,
        H264,
        sharp,
    );
    add(
        "nitro-basic",
        "Nitro Basic",
        "50 MB sınırı olanlar için",
        50,
        Av1,
        sharp,
    );
    add(
        "nitro",
        "Nitro",
        "500 MB · neredeyse orijinal, her yerde oynar",
        500,
        H264,
        sharp,
    );
    // Önerilen: ücretsiz sınıra sığanlar içinde en net olanı (eşitse ilki).
    if let Some(best) = out
        .iter()
        .enumerate()
        .filter(|(_, c)| c.options.limit_mb == Some(10))
        .max_by_key(|(i, c)| (rank(c.quality), std::cmp::Reverse(*i)))
        .map(|(i, _)| i)
    {
        out[best].recommended = true;
    }
    out
}

fn default_limit() -> Option<u32> {
    Some(10)
}

impl Default for FitOptions {
    fn default() -> Self {
        Self {
            start: None,
            end: None,
            limit_mb: default_limit(),
            codec: media::Codec::default(),
            smooth: false,
            keep: true,
        }
    }
}

fn clean(s: &str) -> String {
    s.chars()
        .map(|c| if r#"\/:*?"<>|"#.contains(c) { '_' } else { c })
        .take(100)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Dosya varsa "ad (2).mp4", "ad (3).mp4"…
fn unique(p: &Path) -> PathBuf {
    if !p.exists() {
        return p.to_path_buf();
    }
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    (2..1000)
        .map(|n| p.with_file_name(format!("{stem} ({n}).mp4")))
        .find(|c| !c.exists())
        .unwrap_or_else(|| p.to_path_buf())
}

fn safe_name(s: &str) -> String {
    let n: String = s
        .chars()
        .map(|c| if r#"\/:*?"<>|"#.contains(c) { '_' } else { c })
        .take(80)
        .collect();
    format!("{} (Discord)", n.trim())
}

fn ffmpeg(root: &Path) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    crate::downloader::tools::ffmpeg_dir(root).map(|d| d.join(exe))
}

pub fn set_favorite(db: &Db, id: &str, on: bool) -> Result<(), String> {
    db.conn()
        .execute(
            "UPDATE captures SET favorite = ?2 WHERE id = ?1",
            params![id, on as i64],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn rename(db: &Db, id: &str, title: &str) -> Result<(), String> {
    let t: String = title.trim().chars().take(120).collect();
    db.conn()
        .execute(
            "UPDATE captures SET title = ?2 WHERE id = ?1",
            params![id, (!t.is_empty()).then_some(t)],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Oyunu elle değiştirir (`None`: oyunsuz). Bir daha kendiliğinden değiştirilmez.
pub fn set_game(db: &Db, id: &str, game: Option<&str>) -> Result<(), String> {
    db.conn()
        .execute(
            "UPDATE captures SET game_id = ?2, manual_game = 1 WHERE id = ?1",
            params![id, game],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Dosyayı Geri Dönüşüm Kutusu'na gönderir ve listeden çıkarır.
pub fn delete(db: &Db, id: &str) -> Result<(), String> {
    let clip = get(db, id)?;
    let p = Path::new(&clip.path);
    if p.exists() {
        win::recycle(p)?;
    }
    db.conn()
        .execute("DELETE FROM captures WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    if let Some(t) = clip.thumb {
        let _ = std::fs::remove_file(t);
    }
    Ok(())
}

/// Oyunu bulunamamış klipleri yeniden eşleştirir (kütüphaneye sonradan oyun eklenince).
fn rematch(db: &Db) {
    let games = game_names(db);
    let rows: Vec<(String, Option<i64>, i64, Option<String>)> = {
        let conn = db.conn();
        let mut stmt = match conn.prepare(
            "SELECT id, steam_appid, created_at, game_hint FROM captures WHERE game_id IS NULL AND manual_game = 0",
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map(|rows| rows.filter_map(Result::ok).collect())
            .unwrap_or_default()
    };
    for (id, appid, at, hint) in rows {
        if let Some(g) = guess_game(db, &games, appid, at, hint.as_deref()) {
            let _ = db.conn().execute(
                "UPDATE captures SET game_id = ?2 WHERE id = ?1",
                params![id, g],
            );
        }
    }
}

/// Disk koruması: favori olmayan, N günden eski klipler Geri Dönüşüm Kutusu'na (ayar; varsayılan kapalı).
fn cleanup(db: &Db) {
    let days: i64 = db.setting_json(CLEANUP_KEY).unwrap_or(0);
    if days <= 0 {
        return;
    }
    let limit = now_ms() - days * 86_400_000;
    let old: Vec<String> = {
        let conn = db.conn();
        let mut stmt = match conn.prepare(
            "SELECT id FROM captures WHERE favorite = 0 AND kind = 'clip' AND created_at < ?1",
        ) {
            Ok(s) => s,
            Err(_) => return,
        };
        stmt.query_map([limit], |r| r.get(0))
            .map(|rows| rows.filter_map(Result::ok).collect())
            .unwrap_or_default()
    };
    for id in old {
        if let Err(e) = delete(db, &id) {
            crate::log!("Eski klip silinemedi: {e}");
        }
    }
}

/// "Discord'a Hazır" kopyaları 3 gün sonra silinir. Yalnızca GMID'in ürettiği kopyalar ("… (Discord 10 MB).mp4")
/// ve yalnızca bu klasör: başka bir klasör verilirse hiçbir şeye dokunmaz.
fn purge_ready(dir: &Path) {
    if dir
        .file_name()
        .map(|n| n.to_string_lossy() != READY_DIR)
        .unwrap_or(true)
    {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let limit = std::time::SystemTime::now() - Duration::from_secs(3 * 86_400);
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !(name.contains("(Discord") && name.ends_with(").mp4")) {
            continue;
        }
        if e.metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|t| t < limit)
        {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

fn fmt_secs(s: f64) -> String {
    let s = s.round() as i64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Açılışta: eski kopyaları temizler, izlemeyi başlatır.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::Builder::new()
        .name("klip-izleme".into())
        .spawn(move || {
            purge_ready(&app.state::<DataDir>().folder(Some(READY_DIR)));
            {
                let db = app.state::<Db>();
                cleanup(&db);
                rematch(&db);
            }
            let mut turn: u64 = 0;
            loop {
                let added = app.state::<Clips>().scan(&app);
                for id in added {
                    notify_new(&app, &id);
                }
                turn += 1;
                // 10 dakikada bir: sonradan eklenen oyunlarla yeniden eşleştir.
                if turn % 40 == 0 {
                    rematch(&app.state::<Db>());
                }
                std::thread::sleep(TICK);
            }
        })
        .ok();
}

/// "🎬 Klip kaydedildi — Valorant (0:30)"
fn notify_new(app: &AppHandle, id: &str) {
    let db = app.state::<Db>();
    if !db.setting_json::<bool>(OSD_KEY).unwrap_or(true) {
        return;
    }
    let Ok(c) = get(&db, id) else { return };
    let game: Option<String> = c.game_id.as_deref().and_then(|g| {
        db.conn()
            .query_row("SELECT name FROM games WHERE id = ?1", [g], |r| r.get(0))
            .ok()
    });
    let what = if c.kind == "clip" {
        "Klip kaydedildi"
    } else {
        "Ekran görüntüsü kaydedildi"
    };
    let mut text = what.to_string();
    if let Some(g) = game.or(c.game_hint) {
        text.push_str(&format!(" — {g}"));
    }
    if let Some(d) = c.duration {
        text.push_str(&format!(" ({})", fmt_secs(d)));
    }
    crate::overlay::show_osd(app, "clip", text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purge_only_touches_discord_copies() {
        let root = std::env::temp_dir().join(format!("gmid-ready-{}", std::process::id()));
        let ready = root.join(READY_DIR);
        std::fs::create_dir_all(&ready).unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(10 * 86_400);
        let mk = |p: &Path| {
            std::fs::write(p, b"x").unwrap();
            std::fs::File::options()
                .write(true)
                .open(p)
                .unwrap()
                .set_modified(old)
                .unwrap();
        };
        let db = root.join("gmid.db");
        let copy = ready.join("Ace (Discord).mp4");
        let copy2 = ready.join("Ace (Discord 10 MB, akıcı).mp4");
        let other = ready.join("benim.txt");
        mk(&db);
        mk(&copy);
        mk(&copy2);
        mk(&other);
        purge_ready(&root); // yanlış klasör: dokunmaz
        assert!(db.exists());
        purge_ready(&ready);
        assert!(!copy.exists() && !copy2.exists());
        assert!(other.exists() && db.exists());
    }

    #[test]
    fn choices_cover_sizes_and_skip_duplicates() {
        // 90 sn, 1440p60, 400 MB: ücretsiz için net/akıcı/uyumlu, Nitro Basic; Nitro'da olduğu gibi.
        let c = choices(90.0, true, 400_000_000, (2560, 1440), 60.0, None, None);
        let keys: Vec<&str> = c.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "net-10",
                "akici-10",
                "uyumlu-10",
                "nitro-basic",
                "orijinal-500"
            ]
        );
        assert!(c
            .iter()
            .all(|x| x.size <= x.options.limit_mb.unwrap() as u64 * 1_000_000));
        assert_eq!(c.iter().filter(|x| x.recommended).count(), 1);
        assert!(c[0].recommended, "{c:?}");
        let akici = &c[1];
        assert_eq!(akici.fps, 60);
        assert!(akici.height < c[0].height);
        // Küçük klip: olduğu gibi gider, tekrar eden "olduğu gibi" yok.
        let small = choices(20.0, true, 8_000_000, (1920, 1080), 60.0, None, None);
        assert_eq!(small.len(), 1, "{small:?}");
        assert_eq!(small[0].size, 8_000_000);
        // Kırpılmışsa (full = false) hep kodlanır.
        let cut = choices(
            20.0,
            false,
            8_000_000,
            (1920, 1080),
            30.0,
            Some(1.0),
            Some(21.0),
        );
        assert!(cut.iter().all(|x| x.title != "Olduğu gibi"));
        assert!(
            cut.iter().all(|x| x.key != "akici-10"),
            "30 fps kaynakta akıcı yok"
        );
    }

    #[test]
    fn names_and_times() {
        assert_eq!(fmt_secs(30.4), "0:30");
        assert_eq!(fmt_secs(95.0), "1:35");
        assert_eq!(safe_name("Ace: 5k?"), "Ace_ 5k_ (Discord)");
    }

    #[test]
    fn clips_are_matched_to_games() {
        let dir = std::env::temp_dir().join(format!("gmid-cl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(dir.join("gmid.db"));
        let db = Db::open(&dir.join("gmid.db")).unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO games (id, name, source, source_id, steam_appid, added_at)
                    VALUES ('hades', 'Hades II', 'steam', '1145350', 1145350, 0),
                           ('val', 'VALORANT', 'manual', 'v', NULL, 0);
                 INSERT INTO play_sessions (id, game_id, started_at, ended_at, seconds)
                    VALUES ('s', 'val', 1000000, 2000000, 1000);",
            )
            .unwrap();
        let games = game_names(&db);
        // appid
        assert_eq!(
            guess_game(&db, &games, Some(1145350), 0, None).as_deref(),
            Some("hades")
        );
        // oturum sırasında (ve bitişten 1 dk sonra) çekilen
        assert_eq!(
            guess_game(&db, &games, None, 1_500_000, None).as_deref(),
            Some("val")
        );
        assert_eq!(
            guess_game(&db, &games, None, 2_060_000, None).as_deref(),
            Some("val")
        );
        // oturum dışı: klasör adı
        assert_eq!(
            guess_game(&db, &games, None, 9_000_000, Some("Hades II")).as_deref(),
            Some("hades")
        );
        assert_eq!(guess_game(&db, &games, None, 9_000_000, None), None);

        let f = FoundFile {
            path: dir.join("a.mp4"),
            kind: "clip",
            source: "nvidia",
            hint: Some("Valorant".into()),
            appid: None,
            size: 1,
            modified: 9_000_000,
        };
        let id = insert(&db, &f, None).unwrap();
        rematch(&db);
        assert_eq!(get(&db, &id).unwrap().game_id.as_deref(), Some("val"));
        set_game(&db, &id, None).unwrap();
        rematch(&db);
        assert_eq!(get(&db, &id).unwrap().game_id, None, "elle seçilen korunur");
        set_favorite(&db, &id, true).unwrap();
        rename(&db, &id, "  Ace  ").unwrap();
        let c = get(&db, &id).unwrap();
        assert!(c.favorite);
        assert_eq!(c.title.as_deref(), Some("Ace"));
        assert_eq!(c.name, "a");
        delete(&db, &id).unwrap();
        assert!(list(&db).unwrap().is_empty());
    }

    #[test]
    fn downloaded_videos_can_be_trimmed() {
        let dir = std::env::temp_dir().join(format!("gmid-dlv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(dir.join("gmid.db"));
        let db = Db::open(&dir.join("gmid.db")).unwrap();
        db.conn()
            .execute(
                "INSERT INTO downloads (id, url, video_id, title, uploader, site, thumbnail, duration, quality,
                    option_format, audio_only, status, progress, size, file_path, source, created_at, finished_at)
                 VALUES ('v1', 'https://x', 'abc', 'Scarface Edit', 'Diesel', 'Youtube', 't.jpg', 47.0, '2160p',
                    'f', 0, 'done', 1, 9000000, ?1, 'app', 5, 5)",
                [r"C:\GMID\Videolar\Youtube\Scarface Edit.mp4"],
            )
            .unwrap();
        let c = lookup(&db, "dl:v1").unwrap();
        assert_eq!(c.id, "dl:v1");
        assert_eq!(c.kind, "clip");
        assert_eq!(c.height, Some(2160));
        assert_eq!(c.duration, Some(47.0));
        assert_eq!(c.title.as_deref(), Some("Scarface Edit"));
        assert!(lookup(&db, "dl:yok").is_err());

        let new = crate::downloader::add_copy(&db, "v1", Path::new("kırpık.mp4"), 12.5, 2_000_000)
            .unwrap();
        let d = crate::downloader::get(&db, &new).unwrap().unwrap();
        assert_eq!(d.title, "Scarface Edit (kırpılmış)");
        assert_eq!(d.status, "done");
        assert_eq!(d.duration, Some(12.5));
        assert_eq!(d.size, Some(2_000_000));
        assert_eq!(d.file_path.as_deref(), Some("kırpık.mp4"));
        assert_eq!(d.playlist, None);
    }
}
