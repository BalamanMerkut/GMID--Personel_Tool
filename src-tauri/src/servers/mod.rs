//! Sunucular: topluluğun (GMID) ve kendi eklediğin oyun sunucuları, canlı durumları ve "Bağlan".
//!
//! - Topluluk listesi bir JSON dosyasından gelir (varsayılan: bu projenin GitHub'daki
//!   `topluluk/sunucular.json` dosyası). Liste değişince herkesin uygulamasında kendiliğinden güncellenir.
//! - Kendi eklediğin sunucular sadece bu bilgisayarda durur.

pub mod games;
pub mod query;
pub mod watch;

use crate::db::Db;
use crate::util::{new_id, now_ms};
use games::{Join, Proto};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Topluluk listesinin varsayılan adresi (`HEAD` = deponun ana dalı).
pub const DEFAULT_LIST_URL: &str =
    "https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/topluluk/sunucular.json";
const LIST_URL_KEY: &str = "servers.listUrl";
const LIST_CACHE_KEY: &str = "servers.listCache";
/// Liste en fazla bu sıklıkla indirilir (elle yenileme hariç).
const SYNC_EVERY: Duration = Duration::from_secs(120);
const LOGO_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];
const MAX_LOGO_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    pub name: String,
    pub game: String,
    pub host: String,
    pub port: u16,
    pub query_port: Option<u16>,
    /// `Sunucular` klasöründeki logo dosyası.
    pub logo: Option<String>,
    pub description: String,
    pub connect_notes: String,
    /// Topluluk listesinden geldiyse `true` (düzenlenemez, silinemez).
    pub remote: bool,
    pub created_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInput {
    pub id: Option<String>,
    pub name: String,
    pub game: String,
    pub host: String,
    pub port: Option<u16>,
    pub query_port: Option<u16>,
    pub logo: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub connect_notes: String,
}

const COLUMNS: &str =
    "id, name, game, host, port, query_port, logo, description, connect_notes, remote, created_at";

fn from_row(r: &Row) -> rusqlite::Result<Server> {
    Ok(Server {
        id: r.get(0)?,
        name: r.get(1)?,
        game: r.get(2)?,
        host: r.get(3)?,
        port: r.get(4)?,
        query_port: r.get(5)?,
        logo: r.get(6)?,
        description: r.get(7)?,
        connect_notes: r.get(8)?,
        remote: r.get(9)?,
        created_at: r.get(10)?,
    })
}

/// Önce topluluk sunucuları (listedeki sırayla), sonra kendi eklediklerin.
pub fn list(db: &Db) -> rusqlite::Result<Vec<Server>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM servers ORDER BY remote DESC, sort, created_at"
    ))?;
    let rows = stmt.query_map([], from_row)?;
    rows.collect()
}

pub fn get(db: &Db, id: &str) -> rusqlite::Result<Option<Server>> {
    db.conn()
        .query_row(
            &format!("SELECT {COLUMNS} FROM servers WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()
}

/// Adresi `host:port` biçiminde de kabul eder ("185.1.2.3:28961").
fn split_address(host: &str, port: Option<u16>) -> (String, Option<u16>) {
    let host = host.trim();
    if let Some((h, p)) = host.rsplit_once(':') {
        if let Ok(p) = p.parse::<u16>() {
            return (h.to_string(), Some(p));
        }
    }
    (host.to_string(), port)
}

pub fn save(db: &Db, logos: &Path, input: ServerInput) -> Result<Server, String> {
    let game = games::find(&input.game).ok_or("Bilinmeyen oyun")?;
    let (host, typed_port) = split_address(&input.host, input.port);
    let host = host.to_ascii_lowercase();
    if !games::valid_host(&host) {
        return Err("Adres geçersiz. Örnek: 185.12.34.56 ya da cod2.gmid.gg".into());
    }
    let port = typed_port
        .or(input.port)
        .filter(|p| *p > 0)
        .or(Some(game.default_port).filter(|p| *p > 0))
        .ok_or("Port gerekli")?;
    let name = match input.name.trim() {
        "" => format!("{} sunucusu", game.name),
        n => n.chars().take(80).collect(),
    };
    // Logo sadece kendi klasörümüzden olabilir.
    let logo = input
        .logo
        .filter(|l| Path::new(l).parent() == Some(logos) && Path::new(l).is_file());

    if let Some(id) = &input.id {
        let old = get(db, id).map_err(|e| e.to_string())?;
        match old {
            Some(o) if o.remote => return Err("Topluluk sunucuları düzenlenemez.".into()),
            Some(o) if o.logo != logo => remove_logo(logos, o.logo.as_deref()),
            _ => {}
        }
    }

    let id = input.id.unwrap_or_else(new_id);
    let now = now_ms();
    db.conn()
        .execute(
            "INSERT INTO servers (id, name, game, host, port, query_port, logo, description,
                                  connect_notes, remote, sort, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, 0, ?10, ?10)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name, game = excluded.game, host = excluded.host,
                port = excluded.port, query_port = excluded.query_port, logo = excluded.logo,
                description = excluded.description, connect_notes = excluded.connect_notes,
                updated_at = excluded.updated_at",
            params![
                id,
                name,
                game.id,
                host,
                port,
                input.query_port,
                logo,
                input.description.trim(),
                input.connect_notes.trim(),
                now
            ],
        )
        .map_err(|e| e.to_string())?;
    get(db, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Kaydedilemedi".into())
}

pub fn delete(db: &Db, logos: &Path, id: &str) -> Result<(), String> {
    let Some(server) = get(db, id).map_err(|e| e.to_string())? else {
        return Ok(());
    };
    if server.remote {
        return Err("Topluluk sunucuları listeden gelir, silinemez.".into());
    }
    db.conn()
        .execute("DELETE FROM servers WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    remove_logo(logos, server.logo.as_deref());
    Ok(())
}

fn remove_logo(logos: &Path, logo: Option<&str>) {
    if let Some(l) = logo.map(Path::new).filter(|l| l.parent() == Some(logos)) {
        let _ = std::fs::remove_file(l);
    }
}

/// Seçilen resmi `Sunucular` klasörüne kopyalar.
pub fn import_logo(logos: &Path, source: &str) -> Result<String, String> {
    let src = Path::new(source);
    let ext = extension(source).ok_or("Logo PNG, JPG, WebP ya da GIF olmalı.")?;
    let size = std::fs::metadata(src).map_err(|e| e.to_string())?.len();
    if size > MAX_LOGO_BYTES {
        return Err("Logo en fazla 2 MB olabilir.".into());
    }
    let dest = logos.join(format!("{}.{ext}", new_id()));
    std::fs::copy(src, &dest).map_err(|e| format!("Logo kopyalanamadı: {e}"))?;
    Ok(dest.to_string_lossy().into_owned())
}

fn extension(path: &str) -> Option<&'static str> {
    let path = path.split(['?', '#']).next().unwrap_or(path);
    let ext = Path::new(path)
        .extension()?
        .to_string_lossy()
        .to_lowercase();
    LOGO_EXTENSIONS.iter().find(|e| **e == ext).copied()
}

// ---------------------------------------------------------------- topluluk listesi

#[derive(Deserialize)]
struct RemoteList {
    #[serde(default)]
    servers: Vec<RemoteServer>,
    /// GMID Discord davet bağlantısı (uygulamada "Discord'a katıl"). Boş: gösterilmez.
    #[serde(default)]
    discord: Option<String>,
}

/// Topluluk listesinde bağlantı yoksa (ya da liste hiç indirilemediyse) kullanılan davet.
pub const DEFAULT_DISCORD: &str = "https://discord.gg/upu9VQwFrV";
const DISCORD_KEY: &str = "community.discord";

/// Yalnız Discord davet bağlantıları kabul edilir.
pub fn valid_invite(url: &str) -> bool {
    let rest = url
        .strip_prefix("https://discord.gg/")
        .or_else(|| url.strip_prefix("https://discord.com/invite/"));
    rest.is_some_and(|code| {
        !code.is_empty()
            && code.len() <= 64
            && code
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

/// Uygulamada gösterilecek davet bağlantısı.
pub fn discord_link(db: &Db) -> Option<String> {
    match db.setting(DISCORD_KEY).ok().flatten() {
        Some(v) if valid_invite(&v) => Some(v),
        // Liste bilerek boş bıraktıysa gösterilmez.
        Some(v) if v.is_empty() => None,
        _ => valid_invite(DEFAULT_DISCORD).then(|| DEFAULT_DISCORD.to_string()),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteServer {
    id: String,
    name: String,
    game: String,
    host: String,
    port: Option<u16>,
    query_port: Option<u16>,
    logo: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    connect: String,
}

#[derive(Serialize, Deserialize, Default)]
struct ListCache {
    url: String,
    etag: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub servers: Vec<Server>,
    /// Liste indirilemediyse neden (eldeki liste gösterilmeye devam eder).
    pub error: Option<String>,
    pub url: String,
    pub default_url: &'static str,
}

pub fn list_url(db: &Db) -> String {
    db.setting(LIST_URL_KEY)
        .ok()
        .flatten()
        .unwrap_or_else(|| DEFAULT_LIST_URL.to_string())
}

fn agent() -> ureq::Agent {
    crate::net::agent(Duration::from_secs(10), Duration::from_secs(10))
}

/// Topluluk listesini indirir ve veritabanına işler. `force` değilse kısa sürede tekrar indirmez.
pub fn sync(db: &Db, logos: &Path, state: &Servers, force: bool) -> SyncResult {
    let url = list_url(db);
    let mut error = None;
    let due = {
        let mut last = state.last_sync.lock().unwrap_or_else(|e| e.into_inner());
        let due = force || last.map_or(true, |t| t.elapsed() >= SYNC_EVERY);
        if due {
            *last = Some(Instant::now());
        }
        due
    };
    if due {
        if let Err(e) = download_list(db, logos, &url) {
            error = Some(e);
        }
    }
    SyncResult {
        servers: list(db).unwrap_or_default(),
        error,
        url,
        default_url: DEFAULT_LIST_URL,
    }
}

fn download_list(db: &Db, logos: &Path, url: &str) -> Result<(), String> {
    if url.trim().is_empty() {
        // Topluluk listesi kapatıldı.
        return apply_list(db, logos, url, Vec::new());
    }
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("Liste adresi http(s):// ile başlamalı.".into());
    }
    let cache: ListCache = db.setting_json(LIST_CACHE_KEY).unwrap_or_default();
    let mut req = agent().get(url);
    if cache.url == url {
        if let Some(etag) = &cache.etag {
            req = req.header("If-None-Match", etag);
        }
    }
    let mut resp = req
        .call()
        .map_err(|_| "İnternete ulaşılamadı".to_string())?;
    match resp.status().as_u16() {
        304 => return Ok(()),
        200 => {}
        404 => return Err("Liste bulunamadı (404). Adresi kontrol et.".into()),
        s => return Err(format!("Liste indirilemedi (HTTP {s})")),
    }
    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let body = resp
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_string()
        .map_err(|_| "Liste indirilemedi".to_string())?;
    let parsed: RemoteList =
        serde_json::from_str(&body).map_err(|e| format!("Liste okunamadı: {e}"))?;
    if let Some(link) = parsed.discord.as_deref().map(str::trim) {
        if link.is_empty() || valid_invite(link) {
            let _ = db.set_setting(DISCORD_KEY, link);
        }
    }
    apply_list(db, logos, url, parsed.servers)?;
    let cache = ListCache {
        url: url.to_string(),
        etag,
    };
    if let Ok(json) = serde_json::to_string(&cache) {
        let _ = db.set_setting(LIST_CACHE_KEY, &json);
    }
    Ok(())
}

/// Listedeki sunucuları ekler/günceller, listeden çıkanları siler.
fn apply_list(db: &Db, logos: &Path, url: &str, items: Vec<RemoteServer>) -> Result<(), String> {
    let old: HashMap<String, (Option<String>, Option<String>)> = {
        let conn = db.conn();
        let mut stmt = conn
            .prepare("SELECT id, logo, logo_url FROM servers WHERE remote = 1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))
            .map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };

    let mut keep = Vec::new();
    let mut rows = Vec::new();
    for (sort, s) in items.into_iter().take(200).enumerate() {
        let slug: String =
            s.id.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .take(40)
                .collect();
        let game = games::find(&s.game).unwrap_or(&games::GAMES[games::GAMES.len() - 1]);
        let (host, port) = split_address(&s.host, s.port);
        let port = port.unwrap_or(game.default_port);
        if slug.is_empty() || !games::valid_host(&host) || port == 0 {
            continue;
        }
        let id = format!("r-{slug}");
        if keep.contains(&id) {
            continue;
        }

        // Logo: listeye göre göreli ya da tam adres. Değişmediyse tekrar indirilmez.
        let logo_url = s.logo.as_deref().map(|l| resolve_url(url, l));
        let (old_logo, old_url) = old.get(&id).cloned().unwrap_or_default();
        let logo = match &logo_url {
            Some(u)
                if old_url.as_ref() == Some(u)
                    && old_logo.as_deref().is_some_and(|p| Path::new(p).is_file()) =>
            {
                old_logo
            }
            Some(u) => download_logo(logos, &id, u).or(old_logo),
            None => {
                remove_logo(logos, old_logo.as_deref());
                None
            }
        };
        keep.push(id.clone());
        rows.push((id, s, game.id, host, port, logo, logo_url, sort as i64));
    }

    let now = now_ms();
    let mut conn = db.conn();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (id, s, game, host, port, logo, logo_url, sort) in rows {
        tx.execute(
            "INSERT INTO servers (id, name, game, host, port, query_port, logo, logo_url, description,
                                  connect_notes, remote, sort, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11, ?12, ?12)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name, game = excluded.game, host = excluded.host, port = excluded.port,
                query_port = excluded.query_port, logo = excluded.logo, logo_url = excluded.logo_url,
                description = excluded.description, connect_notes = excluded.connect_notes,
                remote = 1, sort = excluded.sort, updated_at = excluded.updated_at",
            params![
                id,
                s.name.trim().chars().take(80).collect::<String>(),
                game,
                host,
                port,
                s.query_port,
                logo,
                logo_url,
                s.description.trim(),
                s.connect.trim(),
                sort,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    for (id, (logo, _)) in &old {
        if !keep.contains(id) {
            tx.execute("DELETE FROM servers WHERE id = ?1", [id])
                .map_err(|e| e.to_string())?;
            remove_logo(logos, logo.as_deref());
        }
    }
    tx.commit().map_err(|e| e.to_string())
}

fn resolve_url(base: &str, path: &str) -> String {
    if path.starts_with("https://") || path.starts_with("http://") {
        return path.to_string();
    }
    let dir = base.rsplit_once('/').map_or(base, |(d, _)| d);
    format!("{dir}/{}", path.trim_start_matches("./"))
}

fn download_logo(logos: &Path, id: &str, url: &str) -> Option<String> {
    let ext = extension(url)?;
    let mut resp = agent().get(url).call().ok()?;
    if resp.status() != 200 {
        return None;
    }
    let bytes = resp
        .body_mut()
        .with_config()
        .limit(MAX_LOGO_BYTES)
        .read_to_vec()
        .ok()?;
    // Eski logolar (başka uzantıyla) kalmasın.
    for e in LOGO_EXTENSIONS {
        let _ = std::fs::remove_file(logos.join(format!("{id}.{e}")));
    }
    let dest = logos.join(format!("{id}.{ext}"));
    std::fs::write(&dest, bytes).ok()?;
    Some(dest.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------- canlı durum

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub id: String,
    /// "online" | "offline" | "unknown" (sorgulanamayan oyun)
    pub state: &'static str,
    pub info: Option<query::Info>,
    pub error: Option<String>,
    pub checked_at: i64,
    /// Son sorgu başarısız oldu, bir önceki bilgi gösteriliyor.
    pub stale: bool,
}

struct Cached {
    status: Status,
    fails: u8,
}

#[derive(Default)]
pub struct Servers {
    cache: Mutex<HashMap<String, Cached>>,
    last_sync: Mutex<Option<Instant>>,
}

impl Servers {
    pub fn cached(&self) -> Vec<Status> {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.values().map(|c| c.status.clone()).collect()
    }

    /// Sunucuları aynı anda sorgular. Tek bir kaçırılan yanıt "kapalı" saydırmaz.
    pub fn query(&self, servers: &[Server]) -> Vec<Status> {
        let results: Vec<(String, Result<query::Info, String>, bool)> = std::thread::scope(|s| {
            let handles: Vec<_> = servers
                .iter()
                .map(|srv| {
                    s.spawn(move || {
                        let game = games::find(&srv.game);
                        let Some(game) = game.filter(|g| g.proto != Proto::None) else {
                            return (srv.id.clone(), Err(String::new()), false);
                        };
                        let port = srv.query_port.unwrap_or(srv.port + game.query_offset);
                        (
                            srv.id.clone(),
                            query::query(game.proto, &srv.host, port),
                            true,
                        )
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });

        let now = now_ms();
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        results
            .into_iter()
            .map(|(id, result, queryable)| {
                let status = match (result, queryable) {
                    (_, false) => Status {
                        id: id.clone(),
                        state: "unknown",
                        info: None,
                        error: None,
                        checked_at: now,
                        stale: false,
                    },
                    (Ok(info), _) => {
                        cache.insert(
                            id.clone(),
                            Cached {
                                status: Status {
                                    id: id.clone(),
                                    state: "online",
                                    info: Some(info),
                                    error: None,
                                    checked_at: now,
                                    stale: false,
                                },
                                fails: 0,
                            },
                        );
                        return cache[&id].status.clone();
                    }
                    (Err(e), _) => {
                        let prev = cache.get_mut(&id);
                        match prev {
                            Some(c) if c.status.state == "online" && c.fails == 0 => {
                                c.fails = 1;
                                c.status.stale = true;
                                return c.status.clone();
                            }
                            _ => Status {
                                id: id.clone(),
                                state: "offline",
                                info: None,
                                error: Some(e),
                                checked_at: now,
                                stale: false,
                            },
                        }
                    }
                };
                cache.insert(
                    id,
                    Cached {
                        status: status.clone(),
                        fails: 2,
                    },
                );
                status
            })
            .collect()
    }
}

// ---------------------------------------------------------------- Bağlan

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinResult {
    /// "launched": oyun açılıyor; "copy": adres panoya kopyalanmalı.
    pub kind: &'static str,
    pub address: String,
}

/// Oyunun .exe'sinin kayıtlı olduğu ayar.
pub fn exe_key(game: &str) -> String {
    format!("servers.exe.{game}")
}

/// Bağlanmak için açılacak adres ya da çalıştırılacak program.
pub enum Launch {
    Url(String),
    Exe { path: PathBuf, address: String },
    Copy(String),
}

pub fn launch_plan(db: &Db, server: &Server) -> Result<Launch, String> {
    let game = games::find(&server.game).ok_or("Bilinmeyen oyun")?;
    if !games::valid_host(&server.host) {
        return Err("Sunucu adresi geçersiz.".into());
    }
    let address = format!("{}:{}", server.host, server.port);
    Ok(match game.join {
        Join::Steam => Launch::Url(format!("steam://connect/{address}")),
        Join::Uri(t) => Launch::Url(
            t.replace("{host}", &server.host)
                .replace("{port}", &server.port.to_string())
                .replace("{name}", &url_encode(&server.name)),
        ),
        Join::Copy => Launch::Copy(address),
        Join::Exe(exe) => {
            let saved: Option<PathBuf> = db
                .setting(&exe_key(game.id))
                .ok()
                .flatten()
                .map(PathBuf::from)
                .filter(|p| p.is_file());
            let path = match saved.or_else(|| find_exe(game.id, exe)) {
                Some(p) => p,
                None => return Err(format!("EXE_NEEDED:{}", game.id)),
            };
            let _ = db.set_setting(&exe_key(game.id), &path.to_string_lossy());
            Launch::Exe { path, address }
        }
    })
}

/// Oyunu `+connect adres` ile başlatır.
pub fn run_exe(path: &Path, address: &str) -> Result<(), String> {
    let mut cmd = std::process::Command::new(path);
    cmd.args(["+connect", address]);
    if let Some(dir) = path.parent() {
        // Eski oyunlar dosyalarını çalışma klasöründe arar.
        cmd.current_dir(dir);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Oyun başlatılamadı: {e}"))
}

pub fn set_exe(db: &Db, game: &str, path: &str) -> Result<(), String> {
    games::find(game).ok_or("Bilinmeyen oyun")?;
    let p = Path::new(path);
    let is_exe = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    if !is_exe || !p.is_file() {
        return Err("Oyunun .exe dosyasını seç.".into());
    }
    db.set_setting(&exe_key(game), path)
        .map_err(|e| e.to_string())
}

/// Oyunun kurulu olabileceği klasör adları.
fn install_folders(game: &str) -> &'static [&'static str] {
    match game {
        "cod2" => &["Call of Duty 2"],
        "cod4" => &["Call of Duty 4", "Call of Duty 4 - Modern Warfare"],
        "coduo" | "cod1" => &["Call of Duty", "Call of Duty United Offensive"],
        _ => &[],
    }
}

/// Steam kütüphanelerinde ve Activision'ın varsayılan klasöründe oyunu arar.
fn find_exe(game: &str, exe: &str) -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for steam in [r"C:\Program Files (x86)\Steam", r"C:\Program Files\Steam"] {
        let steam = PathBuf::from(steam);
        roots.push(steam.join("steamapps").join("common"));
        // Diğer disklerdeki Steam kütüphaneleri.
        if let Ok(vdf) = std::fs::read_to_string(steam.join("steamapps").join("libraryfolders.vdf"))
        {
            for line in vdf.lines() {
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() >= 4 && parts[1] == "path" {
                    let lib = parts[3].replace("\\\\", "\\");
                    roots.push(PathBuf::from(lib).join("steamapps").join("common"));
                }
            }
        }
    }
    roots.push(PathBuf::from(r"C:\Program Files (x86)\Activision"));
    roots.push(PathBuf::from(r"C:\Program Files\Activision"));

    roots.iter().find_map(|root| {
        install_folders(game)
            .iter()
            .map(|f| root.join(f).join(exe))
            .find(|p| p.is_file())
    })
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {

    #[test]
    fn only_discord_invites_are_accepted() {
        assert!(valid_invite("https://discord.gg/gmid"));
        assert!(valid_invite("https://discord.com/invite/AbC-12_x"));
        assert!(!valid_invite("http://discord.gg/gmid"));
        assert!(valid_invite(DEFAULT_DISCORD));
        assert!(!valid_invite("https://discord.gg/"));
        assert!(!valid_invite("https://evil.com/discord.gg/x"));
        assert!(!valid_invite("https://discord.gg/a b"));
    }

    use super::*;

    fn temp_db(name: &str) -> (Db, PathBuf) {
        let dir = std::env::temp_dir().join(format!("gmid-srv-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        (Db::open(&dir.join("t.db")).unwrap(), dir)
    }

    fn remote(id: &str, host: &str) -> RemoteServer {
        RemoteServer {
            id: id.into(),
            name: format!("GMID {id}"),
            game: "cod2".into(),
            host: host.into(),
            port: None,
            query_port: None,
            logo: None,
            description: String::new(),
            connect: String::new(),
        }
    }

    #[test]
    fn save_parses_address_and_defaults() {
        let (db, dir) = temp_db("save");
        let s = save(
            &db,
            &dir,
            ServerInput {
                id: None,
                name: " ".into(),
                game: "cod2".into(),
                host: "Cod2.GMID.gg:28961".into(),
                port: None,
                query_port: None,
                logo: None,
                description: String::new(),
                connect_notes: String::new(),
            },
        )
        .unwrap();
        assert_eq!((s.host.as_str(), s.port), ("cod2.gmid.gg", 28961));
        assert_eq!(s.name, "Call of Duty 2 sunucusu");
        assert!(save(
            &db,
            &dir,
            ServerInput {
                id: None,
                name: "x".into(),
                game: "cod2".into(),
                host: "1.2.3.4 +exec x".into(),
                port: None,
                query_port: None,
                logo: None,
                description: String::new(),
                connect_notes: String::new(),
            },
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn community_list_is_replaced_not_duplicated() {
        let (db, dir) = temp_db("sync");
        let url = "https://example.com/sunucular.json";
        apply_list(
            &db,
            &dir,
            url,
            vec![
                remote("cod2", "1.2.3.4"),
                remote("mc", "5.6.7.8"),
                remote("bad", "x y"),
            ],
        )
        .unwrap();
        let ids: Vec<_> = list(&db).unwrap().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, ["r-cod2", "r-mc"]);

        apply_list(&db, &dir, url, vec![remote("mc", "9.9.9.9")]).unwrap();
        let all = list(&db).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].host, "9.9.9.9");
        assert!(all[0].remote);
        assert!(delete(&db, &dir, "r-mc").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn join_plans() {
        let (db, dir) = temp_db("join");
        let mut s = Server {
            id: "a".into(),
            name: "GMID & Dostlar".into(),
            game: "minecraft-bedrock".into(),
            host: "mc.gmid.gg".into(),
            port: 19132,
            query_port: None,
            logo: None,
            description: String::new(),
            connect_notes: String::new(),
            remote: false,
            created_at: 0,
        };
        match launch_plan(&db, &s).unwrap() {
            Launch::Url(u) => assert_eq!(
                u,
                "minecraft://?addExternalServer=GMID%20%26%20Dostlar|mc.gmid.gg:19132"
            ),
            _ => panic!(),
        }
        s.game = "cs16".into();
        s.port = 27015;
        assert!(
            matches!(launch_plan(&db, &s).unwrap(), Launch::Url(u) if u == "steam://connect/mc.gmid.gg:27015")
        );
        s.game = "cod2".into();
        assert!(matches!(launch_plan(&db, &s), Err(e) if e == "EXE_NEEDED:cod2"));
        assert_eq!(
            resolve_url("https://a.b/x/list.json", "logolar/c.png"),
            "https://a.b/x/logolar/c.png"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
