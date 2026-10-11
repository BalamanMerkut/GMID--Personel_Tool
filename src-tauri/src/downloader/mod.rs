//! Video İndirici: bağlantıyı çözümler, kaliteyi seçtirir, sırayla indirir; bitenler "Videolarım"da.
//!
//! - İndirilenler `Videolar\<Site>\` altına, küçük resimler `Kapaklar\Videolar\` altına iner.
//! - Aynı anda en fazla `videos.parallel` (varsayılan 2) indirme yapılır, gerisi sırada bekler.
//! - Duraklatılan indirme kaldığı yerden devam eder; uygulama kapanırken inenler açılışta "duraklatıldı" olur.

pub mod bridge;
pub mod extension;
pub mod subs;
pub mod thumbs;
pub mod tools;
pub mod ytdlp;

use crate::db::Db;
use crate::paths::DataDir;
use crate::util::{kill_tree, new_id, now_ms};
use rusqlite::{params, OptionalExtension, Row};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use ytdlp::{Context, Langs, Line, Parsed, PlaylistProbe, Probe, QualityOption};

const PROBE_TIMEOUT: Duration = Duration::from_secs(90);
/// Çözümlenen videolar bu süre boyunca hatırlanır (kalite seçilene kadar tekrar sorulmasın).
const PROBE_CACHE: Duration = Duration::from_secs(30 * 60);

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    pub id: String,
    pub url: String,
    pub video_id: String,
    pub title: String,
    pub uploader: String,
    pub site: String,
    /// Yerel küçük resim ya da (inerken) sitenin resim adresi.
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    /// "1080p", "MP3"…
    pub quality: String,
    pub audio_only: bool,
    /// queued | downloading | processing | paused | done | error
    pub status: String,
    pub progress: f64,
    pub size: Option<u64>,
    pub file_path: Option<String>,
    pub error: Option<String>,
    /// "app" | "extension"
    pub source: String,
    pub created_at: i64,
    pub finished_at: Option<i64>,
    /// Çalma listesinden geldiyse listenin adı.
    pub playlist: Option<String>,
    #[serde(skip)]
    pub subdir: Option<String>,
}

/// İnmekte olan bir işin anlık durumu (veritabanına yazılmaz).
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Live {
    pub id: String,
    pub progress: f64,
    pub speed: Option<f64>,
    pub eta: Option<u64>,
    pub downloaded: u64,
    pub total: Option<u64>,
    /// "download" | "processing"
    pub phase: &'static str,
}

const COLUMNS: &str =
    "id, url, video_id, title, uploader, site, thumbnail, duration, quality, audio_only,
    status, progress, size, file_path, error, source, created_at, finished_at, playlist, subdir";

fn from_row(r: &Row) -> rusqlite::Result<Download> {
    Ok(Download {
        id: r.get(0)?,
        url: r.get(1)?,
        video_id: r.get(2)?,
        title: r.get(3)?,
        uploader: r.get(4)?,
        site: r.get(5)?,
        thumbnail: r.get(6)?,
        duration: r.get(7)?,
        quality: r.get(8)?,
        audio_only: r.get(9)?,
        status: r.get(10)?,
        progress: r.get(11)?,
        size: r.get::<_, Option<i64>>(12)?.map(|v| v as u64),
        file_path: r.get(13)?,
        error: r.get(14)?,
        source: r.get(15)?,
        created_at: r.get(16)?,
        finished_at: r.get(17)?,
        playlist: r.get(18)?,
        subdir: r.get(19)?,
    })
}

pub fn list(db: &Db) -> rusqlite::Result<Vec<Download>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM downloads ORDER BY COALESCE(finished_at, created_at) DESC LIMIT 1000"
    ))?;
    let rows = stmt.query_map([], from_row)?;
    rows.collect()
}

pub fn get(db: &Db, id: &str) -> rusqlite::Result<Option<Download>> {
    db.conn()
        .query_row(
            &format!("SELECT {COLUMNS} FROM downloads WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()
}

fn set_status(db: &Db, id: &str, status: &str, error: Option<&str>) {
    let _ = db.conn().execute(
        "UPDATE downloads SET status = ?2, error = ?3 WHERE id = ?1",
        params![id, status, error],
    );
}

/// Uygulama kapanırken inmekte olanlar açılışta "duraklatıldı" olur.
pub fn recover(db: &Db) {
    let _ = db.conn().execute(
        "UPDATE downloads SET status = CASE status WHEN 'retrying' THEN 'queued' ELSE 'paused' END
         WHERE status IN ('downloading', 'processing', 'retrying')",
        [],
    );
}

#[derive(Clone, Copy, PartialEq)]
enum Stop {
    Pause,
    Cancel,
}

struct Running {
    pid: u32,
    stop: Option<Stop>,
    live: Live,
}

#[derive(Default)]
pub struct Downloader {
    running: Mutex<HashMap<String, Running>>,
    probes: Mutex<Vec<(Instant, String, Parsed)>>,
    /// İş kimliği → eklentiden gelen geçici çerez dosyası.
    cookies: Mutex<HashMap<String, PathBuf>>,
    /// İş kimliği → kaçıncı kendiliğinden yeniden deneme.
    attempts: Mutex<HashMap<String, u8>>,
}

/// Geçici hatalarda en fazla bu kadar kez kendiliğinden yeniden denenir.
const AUTO_RETRIES: u8 = 3;

fn emit_changed(app: &AppHandle) {
    let _ = app.emit("downloads-changed", ());
}

/// İşin kayıtlı sayfa bilgisi (Referer, dublaj, altyazılar).
pub fn context_of(db: &Db, id: &str) -> Context {
    db.conn()
        .query_row("SELECT context FROM downloads WHERE id = ?1", [id], |r| {
            r.get::<_, Option<String>>(0)
        })
        .ok()
        .flatten()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// Ayarlardaki tercih: ses dili ("tr" | "orig"), altyazılar ("tr" | "tr,en" | "none").
fn preferred_langs(db: &Db, probe: &Probe) -> Langs {
    let audio: String = db
        .setting_json("videos.audioLang")
        .unwrap_or_else(|| "tr".into());
    let subs: String = db
        .setting_json("videos.subLangs")
        .unwrap_or_else(|| "tr".into());
    let pick = |tracks: &[ytdlp::Track], base: &str| {
        tracks
            .iter()
            .find(|t| ytdlp::lang_base(&t.code) == base)
            .map(|t| t.code.clone())
    };
    Langs {
        audio: (audio != "orig")
            .then(|| pick(&probe.audio_langs, &audio))
            .flatten(),
        subs: subs
            .split(',')
            .filter(|b| *b != "none" && !b.is_empty())
            .filter_map(|b| pick(&probe.subtitles, b))
            .collect(),
    }
}

/// Yalnızca bu videoda gerçekten olan diller kalır.
fn resolve_langs(probe: &Probe, langs: Langs) -> Langs {
    Langs {
        audio: langs
            .audio
            .filter(|a| probe.audio_langs.iter().any(|t| &t.code == a)),
        subs: langs
            .subs
            .into_iter()
            .filter(|s| probe.subtitles.iter().any(|t| &t.code == s))
            .take(8)
            .collect(),
    }
}

fn valid_url(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() <= 2048
        && !url.chars().any(char::is_whitespace)
}

fn videos_dir(dir: &DataDir) -> PathBuf {
    dir.folder(Some("Videolar"))
}

/// İndirmenin klasörü: `Videolar\<Site>\[<Çalma listesi>]`.
fn out_dir(dir: &DataDir, site: &str, subdir: Option<&str>) -> PathBuf {
    let base = videos_dir(dir).join(ytdlp::site_folder(site));
    match subdir.filter(|s| !s.is_empty()) {
        Some(s) => base.join(ytdlp::safe_name(s, 80)),
        None => base,
    }
}

fn thumbs_dir(dir: &DataDir) -> PathBuf {
    dir.folder(Some("Kapaklar")).join("Videolar")
}

impl Downloader {
    pub fn live(&self) -> Vec<Live> {
        let running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        running.values().map(|r| r.live.clone()).collect()
    }

    fn cached_probe(&self, key: &str) -> Option<Parsed> {
        let mut probes = self.probes.lock().unwrap_or_else(|e| e.into_inner());
        probes.retain(|(t, _, _)| t.elapsed() < PROBE_CACHE);
        probes
            .iter()
            .find(|(_, k, _)| k == key)
            .map(|(_, _, p)| p.clone())
    }

    /// Bağlantıyı çözümler: tek videoysa başlık ve kalite seçenekleri, çalma listesiyse videoları.
    /// `whole_list`: `watch?v=…&list=…` gibi bağlantılarda videonun değil listenin tamamını getir.
    pub fn probe(
        &self,
        root: &Path,
        url: &str,
        ctx: &Context,
        whole_list: bool,
    ) -> Result<Parsed, String> {
        let url = url.trim();
        if !valid_url(url) {
            return Err("Geçerli bir bağlantı yapıştır (https://… ile başlamalı).".into());
        }
        let key = format!(
            "{whole_list}|{url}|{}",
            ctx.referer.as_deref().unwrap_or("")
        );
        if let Some(p) = self.cached_probe(&key) {
            return Ok(p);
        }
        if !tools::ytdlp_path(root).is_file() {
            return Err("TOOLS_MISSING".into());
        }
        // Geçici engellerde (403, kopan bağlantı) iki kez daha dene.
        let mut attempt = 0;
        let out = loop {
            attempt += 1;
            let mut cmd = tools::ytdlp(root);
            cmd.args([
                "-J",
                if whole_list {
                    "--yes-playlist"
                } else {
                    "--no-playlist"
                },
                "--flat-playlist",
                "--no-warnings",
            ])
            .args(ctx.args())
            .arg("--")
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
            let child = cmd
                .spawn()
                .map_err(|e| format!("yt-dlp başlatılamadı: {e}"))?;
            let out = wait_with_timeout(child, PROBE_TIMEOUT)?;
            if out.status.success() && !out.stdout.is_empty() {
                break out;
            }
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            if attempt >= 3 || !ytdlp::is_transient(&stderr) {
                crate::log!(
                    "Çözümlenemedi ({url}): {}",
                    stderr.lines().rev().take(3).collect::<Vec<_>>().join(" | ")
                );
                return Err(ytdlp::friendly_error(&stderr));
            }
            std::thread::sleep(Duration::from_millis(1500 * attempt));
        };
        let mut parsed = ytdlp::parse_any(&out.stdout, url)?;
        // Site videoyu tanımıyorsa (eklentinin yakaladığı doğrudan dosya) başlık sayfadan gelir.
        if let (Parsed::Video(p), Some(t)) = (&mut parsed, ctx.title.as_deref()) {
            if p.site.eq_ignore_ascii_case("generic") && !t.trim().is_empty() {
                p.title = t.trim().chars().take(200).collect();
            }
        }
        let mut probes = self.probes.lock().unwrap_or_else(|e| e.into_inner());
        probes.push((Instant::now(), key, parsed.clone()));
        if probes.len() > 30 {
            probes.remove(0);
        }
        Ok(parsed)
    }

    /// Seçilen kaliteyi sıraya ekler. `cookies`: eklentinin verdiği (Netscape biçimli) çerezler.
    #[allow(clippy::too_many_arguments)]
    pub fn enqueue(
        &self,
        app: &AppHandle,
        url: &str,
        option_id: &str,
        cookies: Option<String>,
        source: &str,
        ctx: &Context,
        langs: Option<Langs>,
    ) -> Result<Download, String> {
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        let probe = match self.probe(dir.root(), url, ctx, false)? {
            Parsed::Video(p) => p,
            Parsed::Playlist(_) => return Err("PLAYLIST".into()),
        };
        let option = ytdlp::choose(&probe.options, option_id).ok_or("Kalite bulunamadı")?;
        // Dublaj ve altyazı: arayüzde seçildiyse o, değilse (eklenti) ayarlardaki tercih.
        let langs = resolve_langs(
            &probe,
            langs.unwrap_or_else(|| preferred_langs(&db, &probe)),
        );
        let mut ctx = Context {
            audio_lang: langs.audio,
            sub_langs: if option.audio_only {
                Vec::new()
            } else {
                langs.subs
            },
            ..ctx.clone()
        };
        if option.audio_only {
            ctx.extra_subs.clear();
        }
        ctx.extra_subs
            .retain(|s| s.url.starts_with("http://") || s.url.starts_with("https://"));
        ctx.extra_subs.truncate(6);
        // Sayfa bilgisi sadece site videoyu tanımıyorsa saklanır (dosya adı ve erişim için).
        let ctx = if probe.site.eq_ignore_ascii_case("generic") {
            Context {
                title: Some(probe.title.clone()),
                ..ctx
            }
        } else {
            Context { title: None, ..ctx }
        };
        let context = (ctx != Context::default())
            .then(|| serde_json::to_string(&ctx).ok())
            .flatten();

        let id = new_id();
        let quality = if option.audio_only {
            "MP3".to_string()
        } else if option.id.parse::<u32>().is_ok() {
            format!("{}p", option.id)
        } else {
            "En iyi".to_string()
        };
        db.conn()
            .execute(
                "INSERT INTO downloads (id, url, video_id, title, uploader, site, thumbnail, duration,
                    quality, option_format, parts, audio_only, status, progress, size, source, created_at, context)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'queued', 0, ?13, ?14, ?15, ?16)",
                params![
                    id,
                    probe.url,
                    probe.video_id,
                    probe.title,
                    probe.uploader,
                    probe.site,
                    probe.thumbnail,
                    probe.duration,
                    quality,
                    option.format,
                    option.parts,
                    option.audio_only,
                    option.size.map(|s| s as i64),
                    source,
                    now_ms(),
                    context
                ],
            )
            .map_err(|e| e.to_string())?;

        if let Some(c) = cookies.filter(|c| !c.trim().is_empty()) {
            let path = tools::dir(dir.root()).join(format!("cerez-{id}.txt"));
            if std::fs::write(&path, c).is_ok() {
                self.cookies
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(id.clone(), path);
            }
        }
        emit_changed(app);
        self.pump(app);
        get(&db, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Eklenemedi".into())
    }

    /// Çalma listesindeki videoları (ya da seçilenleri) sıraya ekler. Her video ayrıca çözümlenmez;
    /// kalite "en fazla 1080p" gibi genel bir seçimle iner. Dosyalar listenin adıyla bir klasöre gider.
    pub fn enqueue_playlist(
        &self,
        app: &AppHandle,
        url: &str,
        option_id: &str,
        ids: Option<Vec<String>>,
        source: &str,
    ) -> Result<usize, String> {
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        let list: PlaylistProbe = match self.probe(dir.root(), url, &Context::default(), true)? {
            Parsed::Playlist(p) => p,
            Parsed::Video(_) => return Err("Bu bağlantı bir çalma listesi değil.".into()),
        };
        let option = ytdlp::generic_option(option_id);
        let quality = if option.audio_only {
            "MP3".to_string()
        } else {
            format!("{}p", option.id)
        };
        let folder = ytdlp::safe_name(&list.title, 80);
        let site = if list.site.to_ascii_lowercase().starts_with("youtube") {
            "Youtube".to_string()
        } else {
            list.site.clone()
        };
        let now = now_ms();
        let mut added = 0usize;
        {
            let mut conn = db.conn();
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            for (i, e) in list.entries.iter().enumerate() {
                if ids.as_ref().is_some_and(|ids| !ids.contains(&e.id)) {
                    continue;
                }
                tx.execute(
                    "INSERT INTO downloads (id, url, video_id, title, uploader, site, thumbnail, duration,
                        quality, option_format, parts, audio_only, status, progress, source, created_at,
                        playlist, subdir, playlist_index)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'queued', 0, ?13, ?14, ?15, ?16, ?17)",
                    params![
                        new_id(),
                        e.url,
                        e.id,
                        e.title,
                        e.uploader,
                        site,
                        e.thumbnail,
                        e.duration,
                        quality,
                        option.format,
                        option.parts,
                        option.audio_only,
                        source,
                        // Sıra korunsun diye her biri bir milisaniye sonra.
                        now + i as i64,
                        list.title,
                        folder,
                        i as i64 + 1
                    ],
                )
                .map_err(|e| e.to_string())?;
                added += 1;
            }
            tx.commit().map_err(|e| e.to_string())?;
        }
        if added == 0 {
            return Err("Hiç video seçilmedi.".into());
        }
        emit_changed(app);
        self.pump(app);
        Ok(added)
    }

    /// Boşta yer varsa sıradakileri başlatır.
    pub fn pump(&self, app: &AppHandle) {
        let db = app.state::<Db>();
        let dir = app.state::<DataDir>();
        if !tools::ytdlp_path(dir.root()).is_file() {
            return;
        }
        let max: usize = db
            .setting_json::<usize>("videos.parallel")
            .unwrap_or(2)
            .clamp(1, 4);
        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        if running.len() >= max {
            return;
        }
        type Row = (String, String, String, u8, bool, Option<String>);
        let queued: Vec<Row> = {
            let conn = db.conn();
            let Ok(mut stmt) = conn.prepare(
                "SELECT id, url, option_format, parts, audio_only, context FROM downloads
                 WHERE status = 'queued' ORDER BY created_at LIMIT 8",
            ) else {
                return;
            };
            stmt.query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })
            .map(|rows| rows.filter_map(Result::ok).collect())
            .unwrap_or_default()
        };
        for (id, url, format, parts, audio_only, context) in queued {
            let ctx: Context = context
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default();
            if running.len() >= max {
                break;
            }
            if running.contains_key(&id) {
                continue;
            }
            let option = QualityOption {
                id: String::new(),
                label: String::new(),
                detail: String::new(),
                size: None,
                audio_only,
                recommended: false,
                format,
                parts,
            };
            match self.start(app, &id, &url, &option, &ctx) {
                Ok(pid) => {
                    running.insert(
                        id.clone(),
                        Running {
                            pid,
                            stop: None,
                            live: Live {
                                id: id.clone(),
                                phase: "download",
                                ..Default::default()
                            },
                        },
                    );
                    set_status(&db, &id, "downloading", None);
                }
                Err(e) => {
                    crate::log!("İndirme başlatılamadı: {e}");
                    set_status(&db, &id, "error", Some(&e));
                }
            }
        }
        drop(running);
        emit_changed(app);
    }

    fn start(
        &self,
        app: &AppHandle,
        id: &str,
        url: &str,
        option: &QualityOption,
        ctx: &Context,
    ) -> Result<u32, String> {
        let dir = app.state::<DataDir>();
        let db = app.state::<Db>();
        let (site, subdir): (String, Option<String>) = db
            .conn()
            .query_row(
                "SELECT site, subdir FROM downloads WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or_default();
        let out_dir = out_dir(&dir, &site, subdir.as_deref());
        let thumbs = thumbs_dir(&dir);
        let _ = std::fs::create_dir_all(&out_dir);
        let _ = std::fs::create_dir_all(&thumbs);
        let cookie = self
            .cookies
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned();

        let mut cmd = tools::ytdlp(dir.root());
        cmd.args(ytdlp::download_args(
            url,
            option,
            &out_dir,
            &thumbs,
            cookie.as_deref(),
            ctx,
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("yt-dlp başlatılamadı: {e}"))?;
        let pid = child.id();
        let stdout = child.stdout.take().ok_or("Çıktı okunamadı")?;
        let stderr = child.stderr.take().ok_or("Çıktı okunamadı")?;

        // Hata satırlarını ayrı bir iş parçacığı toplar (yoksa tampon dolar, yt-dlp takılır).
        let err_buf = std::sync::Arc::new(Mutex::new(String::new()));
        let eb = err_buf.clone();
        std::thread::spawn(move || {
            for line in lossy_lines(stderr) {
                let mut b = eb.lock().unwrap_or_else(|e| e.into_inner());
                if b.len() > 16_000 {
                    let cut = b.len() - 8_000;
                    let cut = (cut..b.len()).find(|i| b.is_char_boundary(*i)).unwrap_or(0);
                    b.drain(..cut);
                }
                b.push_str(&line);
                b.push('\n');
            }
        });

        let (app, id, parts) = (app.clone(), id.to_string(), option.parts.max(1));
        std::thread::spawn(move || {
            let mut file: Option<String> = None;
            let mut formats: Vec<String> = Vec::new();
            let mut last_emit = Instant::now() - Duration::from_secs(1);
            for line in lossy_lines(stdout) {
                let update = match ytdlp::parse_line(&line) {
                    Line::Progress {
                        downloaded,
                        total,
                        speed,
                        eta,
                        format_id,
                    } => {
                        if !formats.contains(&format_id) {
                            formats.push(format_id.clone());
                        }
                        let part = formats.iter().position(|f| *f == format_id).unwrap_or(0);
                        let frac = total
                            .filter(|t| *t > 0)
                            .map_or(0.0, |t| (downloaded as f64 / t as f64).min(1.0));
                        // Görüntü genelde toplamın büyük kısmı; ses kalanı.
                        let progress = if parts >= 2 {
                            if part == 0 {
                                frac * 0.85
                            } else {
                                0.85 + frac * 0.12
                            }
                        } else {
                            frac * 0.97
                        };
                        Some((progress, speed, eta, downloaded, total, "download"))
                    }
                    Line::Processing(_) => Some((0.98, None, None, 0, None, "processing")),
                    Line::File(f) => {
                        file = Some(f);
                        None
                    }
                    Line::Other => None,
                };
                if let Some((progress, speed, eta, downloaded, total, phase)) = update {
                    let downloader = app.state::<Downloader>();
                    let mut running = downloader.running.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(r) = running.get_mut(&id) {
                        let phase_changed = r.live.phase != phase;
                        r.live = Live {
                            id: id.clone(),
                            progress: progress.max(r.live.progress),
                            speed,
                            eta,
                            downloaded,
                            total,
                            phase,
                        };
                        drop(running);
                        if phase_changed {
                            set_status(&app.state::<Db>(), &id, "processing", None);
                            emit_changed(&app);
                        }
                        if phase_changed || last_emit.elapsed() > Duration::from_millis(400) {
                            last_emit = Instant::now();
                            let _ = app.emit("downloads-progress", downloader.live());
                        }
                    }
                }
            }
            let status = child.wait();
            let stderr = err_buf.lock().map(|b| b.clone()).unwrap_or_default();
            app.state::<Downloader>().finish(
                &app,
                &id,
                status.map(|s| s.success()).unwrap_or(false),
                file,
                &stderr,
            );
        });
        Ok(pid)
    }

    fn finish(&self, app: &AppHandle, id: &str, ok: bool, file: Option<String>, stderr: &str) {
        let db = app.state::<Db>();
        let dir = app.state::<DataDir>();
        let entry = self
            .running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        let stop = entry.as_ref().and_then(|r| r.stop);
        let progress = entry.as_ref().map_or(0.0, |r| r.live.progress);
        let Ok(Some(d)) = get(&db, id) else {
            self.pump(app);
            return;
        };

        match stop {
            Some(Stop::Pause) => {
                let _ = db.conn().execute(
                    "UPDATE downloads SET status = 'paused', progress = ?2 WHERE id = ?1",
                    params![id, progress],
                );
            }
            Some(Stop::Cancel) => {
                remove_partials(&out_dir(&dir, &d.site, d.subdir.as_deref()), &d.video_id);
                let _ = db
                    .conn()
                    .execute("DELETE FROM downloads WHERE id = ?1", [id]);
                self.drop_cookies(id);
            }
            None => {
                // Yazılan ad okunamadıysa dosyayı video kimliğiyle bul.
                let file = file.filter(|f| Path::new(f).is_file()).or_else(|| {
                    find_finished(
                        &out_dir(&dir, &d.site, d.subdir.as_deref()),
                        &d.video_id,
                        d.audio_only,
                    )
                });
                if let (true, Some(f)) = (ok, file) {
                    let size = std::fs::metadata(&f).map(|m| m.len() as i64).ok();
                    let thumb = thumbs_dir(&dir).join(format!("{}.jpg", d.video_id));
                    let thumb = thumb
                        .is_file()
                        .then(|| thumb.to_string_lossy().into_owned())
                        .or(d.thumbnail.clone());
                    let _ = db.conn().execute(
                        "UPDATE downloads SET status = 'done', progress = 1, file_path = ?2, size = ?3,
                            thumbnail = ?4, error = NULL, finished_at = ?5 WHERE id = ?1",
                        params![id, f, size, thumb, now_ms()],
                    );
                    self.drop_cookies(id);
                    // Yerel resim yazılamadıysa (ya da site vermediyse) arka planda onar.
                    {
                        let app = app.clone();
                        let id = id.to_string();
                        std::thread::spawn(move || {
                            thumbs::repair(&app, Some(&id));
                        });
                    }
                    self.attempts
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .remove(id);
                    subs::fetch_after(
                        dir.root().to_path_buf(),
                        d.url.clone(),
                        PathBuf::from(&f),
                        context_of(&db, id),
                    );
                    crate::overlay::system_notification(
                        app,
                        "videos.notify",
                        "İndirme tamamlandı",
                        &d.title,
                    );
                } else {
                    let msg = if ok {
                        "İndirilen dosya bulunamadı.".to_string()
                    } else {
                        ytdlp::friendly_error(stderr)
                    };
                    crate::log!(
                        "İndirme başarısız ({}): {}",
                        d.url,
                        stderr.lines().rev().take(4).collect::<Vec<_>>().join(" | ")
                    );
                    let attempt = {
                        let mut a = self.attempts.lock().unwrap_or_else(|e| e.into_inner());
                        let n = a.entry(id.to_string()).or_insert(0);
                        *n += 1;
                        *n
                    };
                    if !ok && attempt <= AUTO_RETRIES && ytdlp::is_transient(stderr) {
                        // Biraz bekleyip kendiliğinden yeniden dene (5 sn, 10 sn, 20 sn).
                        let _ = db.conn().execute(
                            "UPDATE downloads SET status = 'retrying', error = ?2, progress = ?3 WHERE id = ?1",
                            params![id, format!("Tekrar deneniyor ({attempt}/{AUTO_RETRIES}): {msg}"), progress],
                        );
                        let (app2, id2) = (app.clone(), id.to_string());
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_secs(5 << (attempt - 1)));
                            let db = app2.state::<Db>();
                            let _ = db.conn().execute(
                                "UPDATE downloads SET status = 'queued' WHERE id = ?1 AND status = 'retrying'",
                                [&id2],
                            );
                            emit_changed(&app2);
                            app2.state::<Downloader>().pump(&app2);
                        });
                    } else {
                        self.drop_cookies(id);
                        self.attempts
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .remove(id);
                        let _ = db.conn().execute(
                            "UPDATE downloads SET status = 'error', error = ?2, progress = ?3 WHERE id = ?1",
                            params![id, msg, progress],
                        );
                    }
                }
            }
        }
        emit_changed(app);
        let _ = app.emit("downloads-progress", self.live());
        self.pump(app);
    }

    fn drop_cookies(&self, id: &str) {
        if let Some(p) = self
            .cookies
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id)
        {
            let _ = std::fs::remove_file(p);
        }
    }

    fn stop(&self, id: &str, how: Stop) -> bool {
        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = running.get_mut(id) {
            r.stop = Some(how);
            let pid = r.pid;
            drop(running);
            kill_tree(pid);
            true
        } else {
            false
        }
    }

    pub fn pause(&self, app: &AppHandle, id: &str) {
        if !self.stop(id, Stop::Pause) {
            // Sırada bekliyorsa sadece durumunu değiştir.
            let db = app.state::<Db>();
            let _ = db.conn().execute(
                "UPDATE downloads SET status = 'paused' WHERE id = ?1 AND status IN ('queued', 'retrying')",
                [id],
            );
            emit_changed(app);
        }
    }

    pub fn resume(&self, app: &AppHandle, id: &str) {
        let db = app.state::<Db>();
        let _ = db.conn().execute(
            "UPDATE downloads SET status = 'queued', error = NULL, created_at = ?2
             WHERE id = ?1 AND status IN ('paused', 'error', 'retrying')",
            params![id, now_ms()],
        );
        emit_changed(app);
        self.pump(app);
    }

    pub fn cancel(&self, app: &AppHandle, id: &str) {
        if self.stop(id, Stop::Cancel) {
            return;
        }
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        if let Ok(Some(d)) = get(&db, id) {
            if d.status != "done" {
                remove_partials(&out_dir(&dir, &d.site, d.subdir.as_deref()), &d.video_id);
                let _ = db
                    .conn()
                    .execute("DELETE FROM downloads WHERE id = ?1", [id]);
                self.drop_cookies(id);
            }
        }
        emit_changed(app);
    }

    /// Videolarım'dan kaldırır; `delete_file` ise dosyayı da siler.
    pub fn remove(&self, app: &AppHandle, id: &str, delete_file: bool) -> Result<(), String> {
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        let Some(d) = get(&db, id).map_err(|e| e.to_string())? else {
            return Ok(());
        };
        if d.status != "done" {
            self.cancel(app, id);
            return Ok(());
        }
        // Aynı dosyayı gösteren başka bir kayıt varsa (aynı video iki kez indirildiyse) dosya kalsın.
        let shared: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM downloads WHERE file_path = ?1 AND id != ?2",
                params![d.file_path, id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if delete_file && shared == 0 {
            if let Some(f) = d
                .file_path
                .as_deref()
                .filter(|f| inside(f, &videos_dir(&dir)))
            {
                std::fs::remove_file(f)
                    .or_else(|e| {
                        if Path::new(f).exists() {
                            Err(e)
                        } else {
                            Ok(())
                        }
                    })
                    .map_err(|e| format!("Dosya silinemedi (açık olabilir): {e}"))?;
                // Yanındaki yazı dosyaları (Whisper / altyazılar) da gitsin.
                subs::remove_all(Path::new(f));
                let _ = std::fs::remove_file(Path::new(f).with_extension("lrc"));
            }
        }
        crate::transcript::delete(&db, id);
        // Aynı videonun başka bir indirmesi aynı küçük resmi kullanıyorsa silinmez
        // (müzik + video olarak iki kez inen videolarda öteki kartın resmi kayboluyordu).
        if let Some(t) = d
            .thumbnail
            .as_deref()
            .filter(|t| inside(t, &thumbs_dir(&dir)))
            .filter(|t| !thumbs::shared(&db, t, id))
        {
            let _ = std::fs::remove_file(t);
        }
        db.conn()
            .execute("DELETE FROM downloads WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        emit_changed(app);
        Ok(())
    }

    /// Uygulama kapanırken: inen işleri durdur (açılışta kaldığı yerden devam edilebilir).
    pub fn shutdown(&self) {
        let running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        for r in running.values() {
            kill_tree(r.pid);
        }
    }
}

/// Satır satır okur; geçersiz UTF-8 olsa bile okumayı bırakmaz (yoksa yt-dlp'nin çıktısı birikir
/// ve indirme takılır).
fn lossy_lines(r: impl Read) -> impl Iterator<Item = String> {
    let mut reader = BufReader::new(r);
    std::iter::from_fn(move || {
        let mut buf = Vec::new();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(
                String::from_utf8_lossy(&buf)
                    .trim_end_matches(['\r', '\n'])
                    .to_string(),
            ),
        }
    })
}

fn inside(path: &str, dir: &Path) -> bool {
    Path::new(path).starts_with(dir)
}

/// Klasörde `[kimlik]` taşıyan, bitmiş (yarım olmayan) en yeni dosya.
fn find_finished(dir: &Path, video_id: &str, audio_only: bool) -> Option<String> {
    if video_id.is_empty() {
        return None;
    }
    let tag = format!("[{video_id}]");
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let ext = Path::new(&name)
                .extension()
                .map(|x| x.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let wanted = if audio_only {
                ext == "mp3"
            } else {
                matches!(ext.as_str(), "mp4" | "mkv" | "webm" | "mov")
            };
            // Birleştirilmemiş ara parçalar: "… [kimlik].f137.mp4", ".temp.mp4"
            let intermediate = name.contains(".temp.")
                || name.split('.').any(|seg| {
                    seg.len() > 1
                        && seg.starts_with('f')
                        && seg[1..].bytes().all(|b| b.is_ascii_digit())
                });
            let ok = name.contains(&tag) && wanted && !intermediate;
            let modified = e.metadata().and_then(|m| m.modified()).ok()?;
            ok.then(|| (modified, e.path().to_string_lossy().into_owned()))
        })
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

/// İptal edilen indirmenin yarım parçalarını siler (`… [kimlik].f137.mp4.part` gibi).
fn remove_partials(dir: &Path, video_id: &str) {
    if video_id.is_empty() {
        return;
    }
    let tag = format!("[{video_id}]");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let partial = name.ends_with(".part")
            || name.ends_with(".ytdl")
            || name.contains(".part-Frag")
            || name.contains(".temp.");
        if name.contains(&tag) && partial {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

fn wait_with_timeout(
    mut child: std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out_t = std::thread::spawn(move || {
        let mut v = Vec::new();
        if let Some(s) = stdout.as_mut() {
            let _ = s.read_to_end(&mut v);
        }
        v
    });
    let err_t = std::thread::spawn(move || {
        let mut v = Vec::new();
        if let Some(s) = stderr.as_mut() {
            let _ = s.read_to_end(&mut v);
        }
        v
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() > timeout => {
                kill_tree(child.id());
                let _ = child.kill();
                return Err("Site çok geç yanıt verdi. Tekrar dene.".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => return Err(e.to_string()),
        }
    };
    Ok(std::process::Output {
        status,
        stdout: out_t.join().unwrap_or_default(),
        stderr: err_t.join().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partials_are_removed_but_finished_files_kept() {
        let dir = std::env::temp_dir().join(format!("gmid-dl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in [
            "Video [abc].f137.mp4.part",
            "Video [abc].f140.m4a.ytdl",
            "Video [abc].mp4",
            "Başka [xyz].mp4.part",
        ] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        remove_partials(&dir, "abc");
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, ["Başka [xyz].mp4.part", "Video [abc].mp4"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_utf8_does_not_stop_reading() {
        let data: &[u8] =
            b"GMIDP 1|2|NA|NA|NA|137\r\nbozuk \xff\xfe sat\xc4\n\nGMIDFILE C:\\a.mp4\n";
        let lines: Vec<String> = lossy_lines(data).collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "GMIDP 1|2|NA|NA|NA|137");
        assert_eq!(lines[3], "GMIDFILE C:\\a.mp4");
    }

    #[test]
    fn finished_file_is_found_by_id() {
        let dir = std::env::temp_dir().join(format!("gmid-find-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in [
            "Goodfellas x GTA IV Theme［EDIT］ [abc] 1080p.mp4",
            "Goodfellas [abc].f137.mp4.part",
            "Goodfellas [abc].f137.mp4",
            "Goodfellas [abc] (ses).mp3",
            "Başka [xyz] 720p.mp4",
        ] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let v = find_finished(&dir, "abc", false).unwrap();
        assert!(
            v.ends_with("Goodfellas x GTA IV Theme［EDIT］ [abc] 1080p.mp4"),
            "{v}"
        );
        assert!(find_finished(&dir, "abc", true)
            .unwrap()
            .ends_with("(ses).mp3"));
        assert!(find_finished(&dir, "yok", false).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn url_validation() {
        assert!(valid_url("https://www.youtube.com/watch?v=abc"));
        assert!(!valid_url("file:///C:/x"));
        assert!(!valid_url("https://a b"));
        assert!(!valid_url("--exec calc"));
    }
}
