//! Oyun kütüphanesi ve süre takibi (Discord gibi):
//! - **İçe aktarma:** Steam ve Epic'te kurulu oyunlar, Steam'in tuttuğu süre (`scan`).
//! - **Canlı algılama:** 5 sn'de bir çalışan programlara bakılır; bir oyunun klasöründen (ya da bilinen
//!   exe adıyla) çalışan program varsa oturum başlar, kapanınca biter (`play_sessions`).
//! - **Bilgi ve kapak:** Steam mağazasından, Türkçe (`meta`).
//!
//! Her oyunda iki süre görünür: **gerçek oynama** (GMID'in ölçtüğü) ve **Steam'deki süre** (kart toplama
//! programlarıyla şişebilir).

pub mod meta;
pub mod procs;
pub mod scan;
pub mod suggest;
pub mod vdf;

use crate::db::Db;
use crate::paths::DataDir;
use crate::util::{new_id, now_ms};
use rusqlite::{params, OptionalExtension, Row};
use scan::Found;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Bu kadar kısa süren oturumlar (yanlışlıkla açıp kapama, başlatıcı) kaydedilmez.
const MIN_SESSION_SECS: i64 = 30;
const TICK: Duration = Duration::from_secs(5);
pub const TRACK_KEY: &str = "games.track";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub name: String,
    pub source: String,
    pub source_id: String,
    pub install_dir: Option<String>,
    pub exes: Vec<String>,
    pub launch: Option<String>,
    pub steam_appid: Option<i64>,
    pub cover: Option<String>,
    pub header: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<i64>,
    pub description: Option<String>,
    pub genres: Vec<String>,
    pub categories: Vec<String>,
    /// Türkçe etiketler (Roguelike, Co-op, Açık dünya…): Oyun Arşivi süzgeci.
    pub tags: Vec<String>,
    /// Kütüphanede mi (yoksa yalnızca Oyun Arşivi'nde: puanlanmış ama kurulu değil).
    pub in_library: bool,
    pub platform_minutes: i64,
    pub platform_last_played: Option<i64>,
    pub hidden: bool,
    pub added_at: i64,
    /// GMID'in ölçtüğü toplam (saniye; açık oturum dahil).
    pub played_seconds: i64,
    pub sessions: i64,
    /// Son oynama (ms): GMID'in ya da platformun bildiği en yeni.
    pub last_played: Option<i64>,
    /// Şu an açık mı (açıksa ne zamandan beri, ms).
    pub running_since: Option<i64>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub game_id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub seconds: i64,
}

struct Running {
    session: String,
    started: i64,
    saved: i64,
}

/// Algılama için oyunun izleri.
struct Matcher {
    id: String,
    /// Küçük harfli kurulum klasörü (sonu ayraçlı).
    dir: Option<String>,
    exes: Vec<String>,
    /// Pencere başlığıyla tanınanlar: (exe, başlığın başı). Minecraft: `javaw.exe@minecraft`.
    titled: Vec<(String, String)>,
}

#[derive(Default)]
pub struct Games {
    running: Mutex<HashMap<String, Running>>,
    matchers: Mutex<Option<Vec<Matcher>>>,
    meta_busy: AtomicBool,
    suggest: Mutex<SuggestState>,
}

/// "Kütüphaneye ekleyeyim mi?" için çalışma anı durumu.
#[derive(Default)]
struct SuggestState {
    list: Option<std::sync::Arc<suggest::DetectList>>,
    /// Bilinen oyun ilk ne zaman görüldü: yol → (pid, ms).
    seen: HashMap<String, (u32, i64)>,
    /// Hangi açılışında soruldu: yol → pid (aynı açılışta bir daha sorulmaz).
    asked: HashMap<String, u32>,
    /// Ön plandaki tam ekran program: (pid, başlangıç ms).
    fullscreen: Option<(u32, i64)>,
    /// Şu an kartı gösterilen öneriler (yollar).
    showing: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub path: String,
    pub exe: String,
    pub name: String,
    pub reason: String,
    pub seen_at: i64,
}

fn json_list(s: Option<String>) -> Vec<String> {
    s.and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

const COLUMNS: &str = "g.id, g.name, g.source, g.source_id, g.install_dir, g.exes, g.launch,
    g.steam_appid, g.cover, g.header, g.developer, g.publisher, g.year, g.description, g.genres,
    g.categories, g.platform_minutes, g.platform_last_played, g.hidden, g.added_at,
    COALESCE(SUM(s.seconds), 0), COUNT(s.id), MAX(COALESCE(s.ended_at, s.started_at)),
    g.tags, g.in_library";

fn from_row(r: &Row) -> rusqlite::Result<Game> {
    let platform_last: Option<i64> = r.get(17)?;
    let ours: Option<i64> = r.get(22)?;
    Ok(Game {
        id: r.get(0)?,
        name: r.get(1)?,
        source: r.get(2)?,
        source_id: r.get(3)?,
        install_dir: r.get(4)?,
        exes: json_list(r.get(5)?),
        launch: r.get(6)?,
        steam_appid: r.get(7)?,
        cover: r.get(8)?,
        header: r.get(9)?,
        developer: r.get(10)?,
        publisher: r.get(11)?,
        year: r.get(12)?,
        description: r.get(13)?,
        genres: json_list(r.get(14)?),
        categories: json_list(r.get(15)?),
        tags: json_list(r.get(23)?),
        in_library: r.get::<_, i64>(24)? != 0,
        platform_minutes: r.get(16)?,
        platform_last_played: platform_last,
        hidden: r.get::<_, i64>(18)? != 0,
        added_at: r.get(19)?,
        played_seconds: r.get(20)?,
        sessions: r.get(21)?,
        last_played: ours.max(platform_last),
        running_since: None,
    })
}

impl Games {
    pub fn list(&self, db: &Db) -> Result<Vec<Game>, String> {
        // Veritabanı kilidi, oturum kilidinden önce bırakılır (algılama turu ters sırayla alıyor).
        let mut games: Vec<Game> = {
            let conn = db.conn();
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT {COLUMNS} FROM games g LEFT JOIN play_sessions s ON s.game_id = g.id
                     GROUP BY g.id ORDER BY g.name COLLATE NOCASE"
                ))
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], from_row)
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
                .collect();
            rows
        };
        let running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        let now = now_ms();
        for g in &mut games {
            if let Some(r) = running.get(&g.id) {
                g.running_since = Some(r.started);
                // Veritabanındaki süre dakikada bir yazılır; arayüz anlık görsün.
                let saved = (r.saved - r.started) / 1000;
                g.played_seconds += ((now - r.started) / 1000 - saved).max(0);
                g.last_played = Some(now);
            }
        }
        Ok(games)
    }

    fn invalidate(&self) {
        *self.matchers.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Taramada bulunanlar; kütüphanede olanlar işaretli.
    pub fn scan(&self, db: &Db) -> Vec<Found> {
        let have: Vec<(String, String)> = {
            let conn = db.conn();
            let mut stmt =
                match conn.prepare("SELECT source, source_id FROM games WHERE in_library = 1") {
                    Ok(s) => s,
                    Err(_) => return Vec::new(),
                };
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .map(|rows| rows.filter_map(Result::ok).collect())
                .unwrap_or_default()
        };
        scan::all()
            .into_iter()
            .map(|mut f| {
                f.already = have
                    .iter()
                    .any(|(s, i)| *s == f.source && *i == f.source_id);
                f
            })
            .collect()
    }

    /// Seçilenleri kütüphaneye ekler (varsa kurulum bilgisi ve Steam süresi güncellenir).
    pub fn import(&self, app: &AppHandle, found: &[Found]) -> Result<usize, String> {
        let db = app.state::<Db>();
        let mut added = 0;
        {
            let conn = db.conn();
            for f in found {
                let n = conn
                    .execute(
                        "INSERT INTO games (id, name, source, source_id, install_dir, exes, launch,
                            steam_appid, platform_minutes, platform_last_played, added_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                         ON CONFLICT(source, source_id) DO UPDATE SET
                            install_dir = excluded.install_dir, exes = excluded.exes,
                            launch = excluded.launch, hidden = 0, in_library = 1,
                            platform_minutes = MAX(platform_minutes, excluded.platform_minutes),
                            platform_last_played = COALESCE(excluded.platform_last_played, platform_last_played)",
                        params![
                            new_id(),
                            f.name,
                            f.source,
                            f.source_id,
                            f.install_dir,
                            serde_json::to_string(&f.exes).unwrap_or_else(|_| "[]".into()),
                            f.launch,
                            f.steam_appid,
                            f.platform_minutes,
                            f.last_played,
                            now_ms()
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                added += n;
            }
        }
        self.invalidate();
        let _ = app.emit("games-changed", ());
        self.fetch_meta(app);
        Ok(added)
    }

    /// Elle ekleme (çalışan uygulamalardan ya da exe seçerek).
    pub fn add_manual(&self, app: &AppHandle, name: &str, exe: &str) -> Result<(), String> {
        let path = Path::new(exe);
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .filter(|n| n.ends_with(".exe") || !cfg!(windows))
            .ok_or("Bir .exe dosyası seç")?;
        let name = name.trim();
        let name = if name.is_empty() {
            path.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| file.clone())
        } else {
            name.to_string()
        };
        self.import(
            app,
            &[Found {
                source: "manual".into(),
                source_id: exe.to_lowercase(),
                name,
                install_dir: None,
                exes: vec![file],
                launch: Some(exe.to_string()),
                steam_appid: None,
                platform_minutes: 0,
                last_played: None,
                already: false,
            }],
        )
        .map(|_| ())
    }

    /// Oyun Arşivi'ne kütüphanede olmayan bir oyun ekler (konsolda oynanan, silinmiş, istek listesi…).
    /// Steam'de bulunduysa onun kimliğiyle (bilgi ve kapak gelir); yoksa yalnızca adıyla.
    pub fn add_archive(
        &self,
        app: &AppHandle,
        appid: Option<i64>,
        name: &str,
    ) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Oyunun adını yaz".into());
        }
        let id = {
            let db = app.state::<Db>();
            let conn = db.conn();
            let (source, source_id) = match appid {
                Some(a) => ("steam".to_string(), a.to_string()),
                None => ("archive".to_string(), new_id()),
            };
            conn.execute(
                "INSERT INTO games (id, name, source, source_id, steam_appid, in_library, added_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6) ON CONFLICT(source, source_id) DO NOTHING",
                params![new_id(), name, source, source_id, appid, now_ms()],
            )
            .map_err(|e| e.to_string())?;
            conn.query_row(
                "SELECT id FROM games WHERE source = ?1 AND source_id = ?2",
                params![source, source_id],
                |r| r.get::<_, String>(0),
            )
            .map_err(|e| e.to_string())?
        };
        let _ = app.emit("games-changed", ());
        self.fetch_meta(app);
        Ok(id)
    }

    pub fn set_hidden(&self, app: &AppHandle, id: &str, hidden: bool) -> Result<(), String> {
        app.state::<Db>()
            .conn()
            .execute(
                "UPDATE games SET hidden = ?2 WHERE id = ?1",
                params![id, hidden as i64],
            )
            .map_err(|e| e.to_string())?;
        self.invalidate();
        let _ = app.emit("games-changed", ());
        Ok(())
    }

    pub fn rename(&self, app: &AppHandle, id: &str, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Ad boş olamaz".into());
        }
        app.state::<Db>()
            .conn()
            .execute(
                "UPDATE games SET name = ?2 WHERE id = ?1",
                params![id, name],
            )
            .map_err(|e| e.to_string())?;
        let _ = app.emit("games-changed", ());
        Ok(())
    }

    /// Kütüphaneden kaldırır (oturumları da). Oyunun kendisine dokunulmaz. Puanlanmışsa
    /// Oyun Arşivi'nde kalır (puanı ve süreleri durur), yalnızca algılanmaz olur.
    pub fn remove(&self, app: &AppHandle, id: &str) -> Result<(), String> {
        if let Some(r) = self
            .running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id)
        {
            finish(&app.state::<Db>(), &r, now_ms());
        }
        {
            let db = app.state::<Db>();
            let conn = db.conn();
            let reviewed: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM reviews WHERE subject_kind = 'game' AND subject_id = ?1)",
                    [id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if reviewed {
                conn.execute(
                    "UPDATE games SET in_library = 0, install_dir = NULL, exes = '[]', launch = NULL
                     WHERE id = ?1",
                    [id],
                )
            } else {
                conn.execute("DELETE FROM games WHERE id = ?1", [id])
            }
            .map_err(|e| e.to_string())?;
        }
        self.invalidate();
        let _ = app.emit("games-changed", ());
        Ok(())
    }

    /// Bilgisi henüz alınmamış oyunlar için arka planda Steam mağazasına sorar.
    pub fn fetch_meta(&self, app: &AppHandle) {
        if self.meta_busy.swap(true, Ordering::SeqCst) {
            return;
        }
        let app = app.clone();
        std::thread::spawn(move || {
            let games = app.state::<Games>();
            let db = app.state::<Db>();
            let covers = app
                .state::<DataDir>()
                .folder(Some("Kapaklar"))
                .join("Oyunlar");
            loop {
                let next: Option<(String, String, Option<i64>)> = db
                    .conn()
                    .query_row(
                        "SELECT id, name, steam_appid FROM games WHERE meta_at IS NULL LIMIT 1",
                        [],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                    )
                    .optional()
                    .ok()
                    .flatten();
                let Some((id, name, appid)) = next else {
                    break;
                };
                match fill_meta(&db, &covers, &id, &name, appid) {
                    Ok(()) => {
                        let _ = app.emit("games-changed", ());
                    }
                    Err(e) => {
                        // İnternet yoksa sonra yeniden denenir (açılışta ya da yeni taramada).
                        crate::log!("Oyun bilgisi alınamadı ({name}): {e}");
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(1200));
            }
            games.meta_busy.store(false, Ordering::SeqCst);
        });
    }

    /// Bilgileri yeniden alır (yanlış eşleşme ya da güncel kapak için).
    pub fn refresh_meta(&self, app: &AppHandle, id: &str) -> Result<(), String> {
        app.state::<Db>()
            .conn()
            .execute(
                "UPDATE games SET meta_at = NULL, steam_appid = CASE WHEN source = 'steam'
                    THEN steam_appid ELSE NULL END WHERE id = ?1",
                [id],
            )
            .map_err(|e| e.to_string())?;
        self.fetch_meta(app);
        Ok(())
    }

    pub fn sessions(&self, db: &Db, since: i64) -> Result<Vec<Session>, String> {
        let conn = db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.game_id, s.started_at, s.ended_at, s.seconds FROM play_sessions s
                 JOIN games g ON g.id = s.game_id
                 WHERE COALESCE(s.ended_at, s.started_at) >= ?1 ORDER BY s.started_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let mut out: Vec<Session> = stmt
            .query_map([since], |r| {
                Ok(Session {
                    id: r.get(0)?,
                    game_id: r.get(1)?,
                    started_at: r.get(2)?,
                    ended_at: r.get(3)?,
                    seconds: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        let now = now_ms();
        for s in out.iter_mut().filter(|s| s.ended_at.is_none()) {
            s.seconds = (now - s.started_at) / 1000;
        }
        Ok(out)
    }

    fn matchers(&self, db: &Db) -> std::sync::MutexGuard<'_, Option<Vec<Matcher>>> {
        let mut guard = self.matchers.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            let conn = db.conn();
            let list = conn
                .prepare("SELECT id, source, install_dir, exes FROM games WHERE hidden = 0 AND in_library = 1")
                .and_then(|mut stmt| {
                    stmt.query_map([], |r| {
                        let source: String = r.get(1)?;
                        let dir: Option<String> = r.get(2)?;
                        Ok(Matcher {
                            id: r.get(0)?,
                            // Elle eklenenlerde klasör ortak olabilir (ör. D:\Oyunlar): yalnızca exe adı.
                            dir: dir.filter(|d| source != "manual" && d.len() > 3).map(|d| {
                                let d = d.to_lowercase().replace('/', "\\");
                                if d.ends_with('\\') {
                                    d
                                } else {
                                    format!("{d}\\")
                                }
                            }),
                            exes: json_list(r.get(3)?)
                                .into_iter()
                                .filter(|e: &String| !e.contains('@'))
                                .collect(),
                            titled: json_list(r.get(3)?)
                                .into_iter()
                                .filter_map(|e: String| {
                                    e.split_once('@')
                                        .map(|(x, t)| (x.to_string(), t.to_lowercase()))
                                })
                                .collect(),
                        })
                    })
                    .map(|rows| rows.filter_map(Result::ok).collect::<Vec<_>>())
                })
                .unwrap_or_default();
            *guard = Some(list);
        }
        guard
    }

    /// Bir tur: çalışan oyunları bulur, oturumları açar/kapatır.
    fn tick(&self, app: &AppHandle) {
        let procs = procs::list();
        // Pencere başlıkları yalnızca başlıkla tanınan bir oyun varsa okunur.
        let need_titles = self
            .matchers(&app.state::<Db>())
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|m| !m.titled.is_empty());
        let titles: HashMap<u32, String> = if need_titles {
            procs::windowed().into_iter().collect()
        } else {
            HashMap::new()
        };
        self.suggest_tick(app, &procs, &titles);
        if self.step(&app.state::<Db>(), &procs, &titles) {
            let _ = app.emit("games-changed", ());
        }
    }

    /// Algılamanın kendisi; bir oyun açıldı ya da kapandıysa `true`.
    fn step(&self, db: &Db, procs: &[procs::Proc], titles: &HashMap<u32, String>) -> bool {
        let me = std::process::id();
        let now_running: Vec<String> = {
            let guard = self.matchers(db);
            guard
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter(|m| procs.iter().any(|p| p.pid != me && matches(m, p, titles)))
                .map(|m| m.id.clone())
                .collect()
        };
        let now = now_ms();
        let mut changed = false;
        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        for id in &now_running {
            if !running.contains_key(id) {
                let session = new_id();
                let _ = db.conn().execute(
                    "INSERT INTO play_sessions (id, game_id, started_at, seconds) VALUES (?1, ?2, ?3, 0)",
                    params![session, id, now],
                );
                running.insert(
                    id.clone(),
                    Running {
                        session,
                        started: now,
                        saved: now,
                    },
                );
                changed = true;
            }
        }
        let stopped: Vec<String> = running
            .keys()
            .filter(|id| !now_running.contains(id))
            .cloned()
            .collect();
        for id in stopped {
            if let Some(r) = running.remove(&id) {
                finish(db, &r, now);
                changed = true;
            }
        }
        // Çökme ya da kapanmaya karşı süre dakikada bir yazılır.
        for r in running.values_mut() {
            if now - r.saved >= 60_000 {
                let _ = db.conn().execute(
                    "UPDATE play_sessions SET seconds = ?2 WHERE id = ?1",
                    params![r.session, (now - r.started) / 1000],
                );
                r.saved = now;
            }
        }
        changed
    }

    /// Kütüphanede olmayan bir oyun açıldıysa sorar (sağ altta kart + Kütüphanem'de liste).
    fn suggest_tick(&self, app: &AppHandle, procs: &[procs::Proc], titles: &HashMap<u32, String>) {
        let db = app.state::<Db>();
        if !db.setting_json::<bool>(suggest::ASK_KEY).unwrap_or(true) {
            return;
        }
        let me = std::process::id();
        let now = now_ms();
        let mut st = self.suggest.lock().unwrap_or_else(|e| e.into_inner());
        let mut candidates: Vec<(&procs::Proc, String, &'static str)> = Vec::new();
        if let Some(list) = st.list.clone() {
            for p in procs {
                if p.path.is_empty() || suggest::system_path(&p.path) {
                    continue;
                }
                if let Some(name) = list.find(p) {
                    candidates.push((p, name.to_string(), "discord"));
                }
            }
        }
        match procs::foreground_fullscreen() {
            Some((pid, title)) => {
                let since = match st.fullscreen {
                    Some((p, t)) if p == pid => t,
                    _ => {
                        st.fullscreen = Some((pid, now));
                        now
                    }
                };
                if let Some(p) = procs.iter().find(|p| p.pid == pid) {
                    if now - since >= suggest::FULLSCREEN_AFTER_MS
                        && !p.path.is_empty()
                        && !suggest::not_a_game(p)
                        && !candidates.iter().any(|(c, _, _)| c.pid == pid)
                    {
                        candidates.push((p, suggest::clean_title(&title, &p.path), "fullscreen"));
                    }
                }
            }
            None => st.fullscreen = None,
        }
        // Görülmeyenleri unut.
        st.seen
            .retain(|path, _| procs.iter().any(|p| &p.path == path));
        let mut raised = false;
        let guard = self.matchers(&db);
        let matchers = guard.as_deref().unwrap_or_default();
        for (p, name, reason) in candidates {
            if p.pid == me || matchers.iter().any(|m| matches(m, p, titles)) {
                continue;
            }
            let key = p.path.clone();
            if reason == "discord" {
                let first = match st.seen.get(&key) {
                    Some((pid, t)) if *pid == p.pid => *t,
                    _ => {
                        st.seen.insert(key.clone(), (p.pid, now));
                        now
                    }
                };
                if now - first < suggest::KNOWN_AFTER_MS {
                    continue;
                }
            }
            if st.asked.get(&key) == Some(&p.pid) {
                continue;
            }
            st.asked.insert(key.clone(), p.pid);
            let ignored: bool = db
                .conn()
                .query_row(
                    "SELECT state = 'ignored' FROM game_suggestions WHERE path = ?1",
                    [&key],
                    |r| r.get(0),
                )
                .unwrap_or(false);
            if ignored {
                continue;
            }
            let _ = db.conn().execute(
                "INSERT INTO game_suggestions (path, exe, name, reason, state, seen_at)
                 VALUES (?1, ?2, ?3, ?4, 'pending', ?5)
                 ON CONFLICT(path) DO UPDATE SET seen_at = excluded.seen_at",
                params![key, p.name, name, reason, now],
            );
            if !st.showing.contains(&key) {
                st.showing.push(key);
            }
            raised = true;
        }
        drop(guard);
        drop(st);
        if raised {
            crate::overlay::raise_alerts_quiet(app);
            let _ = app.emit("games-suggestions", ());
        }
    }

    fn suggestion_rows(db: &Db, only: Option<&[String]>) -> Vec<Suggestion> {
        let conn = db.conn();
        let mut stmt = match conn.prepare(
            "SELECT path, exe, name, reason, seen_at FROM game_suggestions
             WHERE state = 'pending' ORDER BY seen_at DESC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([], |r| {
            Ok(Suggestion {
                path: r.get(0)?,
                exe: r.get(1)?,
                name: r.get(2)?,
                reason: r.get(3)?,
                seen_at: r.get(4)?,
            })
        })
        .map(|rows| {
            rows.filter_map(Result::ok)
                .filter(|s| match only {
                    None => true,
                    Some(o) => o.contains(&s.path),
                })
                .collect()
        })
        .unwrap_or_default()
    }

    /// Kütüphanem'de gösterilen, henüz cevaplanmamış öneriler.
    pub fn suggestions(&self, db: &Db) -> Vec<Suggestion> {
        Self::suggestion_rows(db, None)
    }

    /// Sağ altta kartı açık olanlar.
    pub fn showing_suggestions(&self, db: &Db) -> Vec<Suggestion> {
        let showing = self
            .suggest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .showing
            .clone();
        Self::suggestion_rows(db, Some(&showing))
    }

    /// Cevap: `add` (kütüphaneye ekle), `later` (şimdi değil), `never` (bir daha sorma).
    pub fn answer_suggestion(
        &self,
        app: &AppHandle,
        path: &str,
        action: &str,
        name: Option<String>,
    ) -> Result<(), String> {
        let db = app.state::<Db>();
        self.suggest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .showing
            .retain(|p| p != path);
        match action {
            "add" => {
                let row = Self::suggestion_rows(&db, Some(&[path.to_string()]))
                    .into_iter()
                    .next()
                    .ok_or("Öneri bulunamadı")?;
                let name = name
                    .map(|n| n.trim().to_string())
                    .filter(|n| !n.is_empty())
                    .unwrap_or(row.name);
                self.add_manual(app, &name, &row.path)?;
                db.conn()
                    .execute("DELETE FROM game_suggestions WHERE path = ?1", [path])
                    .map_err(|e| e.to_string())?;
            }
            "never" => {
                db.conn()
                    .execute(
                        "UPDATE game_suggestions SET state = 'ignored' WHERE path = ?1",
                        [path],
                    )
                    .map_err(|e| e.to_string())?;
            }
            _ => {}
        }
        let _ = app.emit_to(crate::overlay::ALERTS, "alerts-changed", ());
        let _ = app.emit("games-suggestions", ());
        Ok(())
    }

    /// Uygulama kapanırken açık oturumları bitirir.
    /// Şu an açık oyunların kimlikleri (klipleri o anki oyuna bağlamak için).
    pub fn running_ids(&self) -> Vec<String> {
        self.running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    pub fn shutdown(&self, db: &Db) {
        let now = now_ms();
        for (_, r) in self
            .running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain()
        {
            finish(db, &r, now);
        }
    }

    /// Çalışan, pencereli uygulamalar (kütüphanede olmayanlar): "Oyun ekle" listesi.
    pub fn running_apps(&self, db: &Db) -> Vec<RunningApp> {
        let have: Vec<String> = self
            .matchers(db)
            .as_deref()
            .unwrap_or_default()
            .iter()
            .flat_map(|m| m.exes.clone())
            .collect();
        let procs = procs::list();
        let me = std::process::id();
        let mut out: Vec<RunningApp> = procs::windowed()
            .into_iter()
            .filter_map(|(pid, title)| {
                let p = procs.iter().find(|p| p.pid == pid && p.pid != me)?;
                let sys = p.path.starts_with("c:\\windows\\")
                    || [
                        "explorer.exe",
                        "applicationframehost.exe",
                        "textinputhost.exe",
                        "gmid-tool.exe",
                    ]
                    .contains(&p.name.as_str());
                (!sys && !p.path.is_empty() && !have.contains(&p.name)).then(|| RunningApp {
                    name: p.name.clone(),
                    path: p.path.clone(),
                    title,
                })
            })
            .collect();
        out.sort_by_key(|a| a.title.to_lowercase());
        out.dedup_by(|a, b| a.path == b.path);
        out
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RunningApp {
    pub name: String,
    pub path: String,
    /// Pencere başlığı (çoğu oyunda oyunun adı).
    pub title: String,
}

fn matches(m: &Matcher, p: &procs::Proc, titles: &HashMap<u32, String>) -> bool {
    if m.titled.iter().any(|(exe, prefix)| {
        *exe == p.name
            && titles
                .get(&p.pid)
                .is_some_and(|t| t.to_lowercase().starts_with(prefix.as_str()))
    }) {
        return true;
    }
    match (&m.dir, p.path.is_empty()) {
        (Some(dir), false) => p.path.starts_with(dir.as_str()),
        // Korumalı süreçlerin yolu okunamaz: exe adıyla.
        _ => m.exes.contains(&p.name),
    }
}

fn finish(db: &Db, r: &Running, now: i64) {
    let secs = (now - r.started) / 1000;
    if secs < MIN_SESSION_SECS {
        let _ = db
            .conn()
            .execute("DELETE FROM play_sessions WHERE id = ?1", [&r.session]);
    } else {
        let _ = db.conn().execute(
            "UPDATE play_sessions SET ended_at = ?2, seconds = ?3 WHERE id = ?1",
            params![r.session, now, secs],
        );
    }
}

fn fill_meta(
    db: &Db,
    covers: &Path,
    id: &str,
    name: &str,
    appid: Option<i64>,
) -> Result<(), String> {
    let appid = match appid {
        Some(a) => Some(a),
        None => meta::search(name)?,
    };
    let Some(appid) = appid else {
        // Steam'de yok: kapak Wikipedia'dan (VALORANT, LoL, Minecraft…). Elle seçilen kapağa dokunulmaz.
        // Bir daha sorulmasın (elle "Bilgileri yenile" ile tekrar denenir).
        let has_cover: bool = db
            .conn()
            .query_row(
                "SELECT cover IS NOT NULL FROM games WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap_or(false);
        let cover = if has_cover {
            None
        } else {
            match meta::wiki_image(name) {
                Ok(Some(url)) => {
                    let ext = if url.to_lowercase().ends_with(".png") {
                        "png"
                    } else {
                        "jpg"
                    };
                    let dest = covers.join(format!("wiki-{id}.{ext}"));
                    meta::download_image(&url, &dest)
                        .map(|_| dest.to_string_lossy().into_owned())
                        .map_err(|e| crate::log!("Wikipedia kapağı indirilemedi ({name}): {e}"))
                        .ok()
                }
                Ok(None) => None,
                Err(e) => {
                    crate::log!("Wikipedia'ya ulaşılamadı ({name}): {e}");
                    None
                }
            }
        };
        db.conn()
            .execute(
                "UPDATE games SET meta_at = ?2, cover = COALESCE(?3, cover) WHERE id = ?1",
                params![id, now_ms(), cover],
            )
            .map_err(|e| e.to_string())?;
        return Ok(());
    };
    let m = meta::details(appid)?.unwrap_or_default();
    let tags = meta::tr_tags(&meta::store_tags(appid), &m.categories);
    let cover_path: PathBuf = covers.join(format!("{appid}.jpg"));
    let cover = meta::download_cover(appid, m.header.as_deref(), &cover_path)
        .map(|_| cover_path.to_string_lossy().into_owned())
        .ok();
    db.conn()
        .execute(
            "UPDATE games SET steam_appid = ?2, cover = COALESCE(?3, cover), header = ?4,
                developer = ?5, publisher = ?6, year = ?7, description = ?8, genres = ?9,
                categories = ?10, meta_at = ?11, tags = ?12 WHERE id = ?1",
            params![
                id,
                appid,
                cover,
                m.header,
                m.developer,
                m.publisher,
                m.year,
                m.description,
                serde_json::to_string(&m.genres).unwrap_or_else(|_| "[]".into()),
                serde_json::to_string(&m.categories).unwrap_or_else(|_| "[]".into()),
                now_ms(),
                serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into())
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Açılışta: yarım kalan oturumları kapatır, algılamayı ve eksik bilgileri başlatır.
pub fn start(app: &AppHandle) {
    {
        let db = app.state::<Db>();
        let conn = db.conn();
        let _ = conn.execute(
            "UPDATE play_sessions SET ended_at = started_at + seconds * 1000 WHERE ended_at IS NULL",
            [],
        );
        let _ = conn.execute(
            "DELETE FROM play_sessions WHERE seconds < ?1",
            [MIN_SESSION_SECS],
        );
    }
    app.state::<Games>().fetch_meta(app);
    // Bilinen oyunlar listesi (Discord) arka planda; gelene kadar yalnızca tam ekran ipucu çalışır.
    {
        let app = app.clone();
        std::thread::spawn(move || {
            let cache = app
                .state::<DataDir>()
                .root()
                .join("Araclar")
                .join("oyun-listesi.json");
            if let Some(list) = suggest::load_list(&cache) {
                crate::log!("Bilinen oyun listesi hazır ({} exe)", list.len());
                app.state::<Games>()
                    .suggest
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .list = Some(std::sync::Arc::new(list));
            }
        });
    }
    let app = app.clone();
    std::thread::Builder::new()
        .name("oyun-algilama".into())
        .spawn(move || loop {
            std::thread::sleep(TICK);
            let enabled = app
                .state::<Db>()
                .setting_json::<bool>(TRACK_KEY)
                .unwrap_or(true);
            if enabled {
                app.state::<Games>().tick(&app);
            } else {
                // Algılama kapatıldıysa açık oturumlar hemen kapanır.
                app.state::<Games>().shutdown(&app.state::<Db>());
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_open_and_close() {
        let dir = std::env::temp_dir().join(format!("gmid-games-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("gmid.db")).unwrap();
        db.conn()
            .execute(
                "INSERT INTO games (id, name, source, source_id, install_dir, exes, added_at)
                 VALUES ('g', 'Hades II', 'steam', '1', 'D:/Steam/common/Hades II', '[\"hades2.exe\"]', 0)",
                [],
            )
            .unwrap();
        let games = Games::default();
        let game = procs::Proc {
            pid: 99_999,
            name: "hades2.exe".into(),
            path: r"d:\steam\common\hades ii\hades2.exe".into(),
        };
        assert!(
            games.step(&db, std::slice::from_ref(&game), &HashMap::new()),
            "oyun açıldı"
        );
        assert!(
            !games.step(&db, std::slice::from_ref(&game), &HashMap::new()),
            "değişiklik yok"
        );
        assert!(games.list(&db).unwrap()[0].running_since.is_some());
        // Kısa oturum (yanlışlıkla açma) kaydedilmez.
        assert!(games.step(&db, &[], &HashMap::new()));
        assert!(games.sessions(&db, 0).unwrap().is_empty());
        // Uzun oturum: 40 dk önce başlamış gibi.
        games.step(&db, std::slice::from_ref(&game), &HashMap::new());
        games
            .running
            .lock()
            .unwrap()
            .values_mut()
            .for_each(|r| r.started -= 40 * 60_000);
        games.step(&db, &[], &HashMap::new());
        let s = games.sessions(&db, 0).unwrap();
        assert_eq!(s.len(), 1);
        assert!((s[0].seconds - 2400).abs() <= 1, "{:?}", s[0]);
        let g = &games.list(&db).unwrap()[0];
        assert_eq!((g.sessions, g.running_since), (1, None));
        // Gizlenen oyun izlenmez.
        db.conn()
            .execute("UPDATE games SET hidden = 1", [])
            .unwrap();
        games.invalidate();
        assert!(!games.step(&db, std::slice::from_ref(&game), &HashMap::new()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn processes_are_matched_by_folder_or_name() {
        let steam = Matcher {
            id: "a".into(),
            dir: Some(r"d:\steam\steamapps\common\hades ii\".into()),
            exes: vec!["hades2.exe".into()],
            titled: vec![],
        };
        let manual = Matcher {
            id: "b".into(),
            dir: None,
            exes: vec!["valorant-win64-shipping.exe".into()],
            titled: vec![],
        };
        let mc = Matcher {
            id: "c".into(),
            dir: Some(r"c:\users\a\appdata\roaming\.minecraft\".into()),
            exes: vec![],
            titled: vec![("javaw.exe".into(), "minecraft".into())],
        };
        let p = |name: &str, path: &str| procs::Proc {
            pid: 10,
            name: name.into(),
            path: path.into(),
        };
        let none = HashMap::new();
        let m = |m: &Matcher, name: &str, path: &str| matches(m, &p(name, path), &none);
        assert!(m(
            &steam,
            "hades2.exe",
            r"d:\steam\steamapps\common\hades ii\bin\hades2.exe"
        ));
        assert!(m(
            &steam,
            "eac.exe",
            r"d:\steam\steamapps\common\hades ii\eac.exe"
        ));
        assert!(
            !m(&steam, "hades2.exe", r"c:\başka\hades2.exe"),
            "klasör dışı sayılmaz"
        );
        assert!(m(&steam, "hades2.exe", ""), "yol okunamazsa ad");
        assert!(m(&manual, "valorant-win64-shipping.exe", r"c:\riot\x.exe"));
        assert!(!m(&manual, "chrome.exe", r"c:\x\chrome.exe"));
        // Minecraft: hangi Java ile açılırsa açılsın, penceresi "Minecraft…" olan javaw.
        let java = p("javaw.exe", r"c:\program files\java\bin\javaw.exe");
        assert!(!matches(&mc, &java, &none));
        let titles: HashMap<u32, String> =
            [(10, "Minecraft* 1.21.4 - Singleplayer".to_string())].into();
        assert!(matches(&mc, &java, &titles));
        let other: HashMap<u32, String> = [(10, "IntelliJ IDEA".to_string())].into();
        assert!(!matches(&mc, &java, &other));
    }
}
