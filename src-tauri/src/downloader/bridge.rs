//! Tarayıcı eklentisi ile uygulama arasındaki köprü: sadece bu bilgisayardan (127.0.0.1) erişilen
//! küçük bir HTTP sunucusu.
//!
//! Güvenlik:
//! - Eklenti bir kez, uygulamada gösterilen 6 haneli kodla eşleşir ve gizli bir anahtar alır;
//!   sonraki her istek bu anahtarı taşır. Kod 10 dakika geçerli, en fazla 5 deneme.
//! - Web sayfalarından gelen istekler (`Origin: https://…`) reddedilir; sadece eklentiler konuşabilir.

use super::ytdlp::Context;
use super::Downloader;
use crate::db::Db;
use crate::util::now_ms;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::Read;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Request, Response, Server};

/// Eklentinin sırayla denediği portlar (ilki doluysa sonrakiler).
pub const PORTS: std::ops::RangeInclusive<u16> = 47913..=47917;
const TOKEN_KEY: &str = "videos.bridgeToken";
const PAIR_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_BODY: u64 = 1024 * 1024;

#[derive(Default)]
pub struct Bridge {
    port: AtomicU16,
    pairing: Mutex<Option<Pairing>>,
    seen: Mutex<Option<(i64, String)>>,
}

struct Pairing {
    code: String,
    created: Instant,
    attempts: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatus {
    pub port: u16,
    pub paired: bool,
    pub last_seen: Option<i64>,
    pub browser: Option<String>,
}

fn random_u64() -> u64 {
    // RandomState her seferinde işletim sisteminden gelen rastgele anahtarlarla başlar.
    let mut h = RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    h.finish()
}

impl Bridge {
    pub fn status(&self, db: &Db) -> BridgeStatus {
        let seen = self.seen.lock().ok().and_then(|s| s.clone());
        BridgeStatus {
            port: self.port.load(Ordering::SeqCst),
            paired: db.setting(TOKEN_KEY).ok().flatten().is_some(),
            last_seen: seen.as_ref().map(|s| s.0),
            browser: seen.map(|s| s.1),
        }
    }

    /// Yeni bir eşleştirme kodu üretir (10 dk geçerli).
    pub fn start_pairing(&self) -> String {
        let code = format!("{:06}", random_u64() % 1_000_000);
        *self.pairing.lock().unwrap_or_else(|e| e.into_inner()) = Some(Pairing {
            code: code.clone(),
            created: Instant::now(),
            attempts: 0,
        });
        code
    }

    pub fn unpair(&self, db: &Db) {
        let _ = db
            .conn()
            .execute("DELETE FROM settings WHERE key = ?1", [TOKEN_KEY]);
        *self.seen.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    fn try_pair(&self, db: &Db, code: &str) -> Result<String, &'static str> {
        let mut guard = self.pairing.lock().unwrap_or_else(|e| e.into_inner());
        let Some(p) = guard.as_mut() else {
            return Err("Önce GMID Tool'da Video İndirici → Tarayıcı eklentisi → Eşleştir'e bas.");
        };
        if p.created.elapsed() > PAIR_TTL || p.attempts >= 5 {
            *guard = None;
            return Err("Kodun süresi doldu. Uygulamada yeni kod al.");
        }
        p.attempts += 1;
        if p.code != code.trim() {
            return Err("Kod yanlış.");
        }
        *guard = None;
        let token = format!("{:016x}{:016x}", random_u64(), random_u64());
        db.set_setting(TOKEN_KEY, &token)
            .map_err(|_| "Kaydedilemedi")?;
        Ok(token)
    }
}

/// Köprüyü arka planda başlatır.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let server = PORTS
            .clone()
            .find_map(|p| Server::http(("127.0.0.1", p)).ok().map(|s| (p, s)));
        let Some((port, server)) = server else {
            crate::log!("Eklenti köprüsü başlatılamadı: portlar dolu");
            return;
        };
        app.state::<Bridge>().port.store(port, Ordering::SeqCst);
        for req in server.incoming_requests() {
            let app = app.clone();
            // Çözümleme saniyeler sürebilir; diğer istekler beklemesin.
            std::thread::spawn(move || handle(&app, req));
        }
    });
}

fn header<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    req.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

fn reply(req: Request, status: u16, body: serde_json::Value) {
    let mut resp = Response::from_string(body.to_string()).with_status_code(status);
    for (k, v) in [
        ("Content-Type", "application/json; charset=utf-8"),
        ("Cache-Control", "no-store"),
    ] {
        if let Ok(h) = Header::from_bytes(k, v) {
            resp.add_header(h);
        }
    }
    let _ = req.respond(resp);
}

fn browser_name(ua: &str) -> String {
    if ua.contains("OPR/") || ua.contains("Opera") {
        "Opera"
    } else if ua.contains("Edg/") {
        "Edge"
    } else if ua.contains("Firefox/") {
        "Firefox"
    } else {
        "Chrome"
    }
    .to_string()
}

#[derive(Deserialize)]
struct PairBody {
    code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProbeBody {
    url: String,
    #[serde(default)]
    context: Context,
    #[serde(default)]
    whole_list: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadBody {
    url: String,
    option: String,
    cookies: Option<String>,
    #[serde(default)]
    context: Context,
}

#[derive(Deserialize)]
struct PlaylistBody {
    url: String,
    option: String,
    ids: Option<Vec<String>>,
}

const TOOLS_MISSING: &str = "GMID Tool'da Video İndirici'yi bir kez açıp araçları kur.";

fn handle(app: &AppHandle, mut req: Request) {
    // Web sayfaları buraya istek gönderemesin: sadece eklenti kökenleri (ya da kökensiz) kabul.
    if let Some(origin) = header(&req, "Origin") {
        if !(origin.starts_with("chrome-extension://") || origin.starts_with("moz-extension://")) {
            return reply(req, 403, json!({ "error": "forbidden" }));
        }
    }
    let path = req.url().split('?').next().unwrap_or("").to_string();
    let method = req.method().clone();
    let db = app.state::<Db>();
    let bridge = app.state::<Bridge>();

    let token = db.setting(TOKEN_KEY).ok().flatten();
    let authorized = match (&token, header(&req, "Authorization")) {
        (Some(t), Some(h)) => h.strip_prefix("Bearer ").is_some_and(|v| v == t),
        _ => false,
    };
    if authorized {
        let ua = header(&req, "User-Agent").unwrap_or("").to_string();
        *bridge.seen.lock().unwrap_or_else(|e| e.into_inner()) =
            Some((now_ms(), browser_name(&ua)));
    }

    let mut body = String::new();
    if method == Method::Post {
        if req.body_length().is_some_and(|l| l as u64 > MAX_BODY) {
            return reply(req, 413, json!({ "error": "Çok büyük" }));
        }
        let _ = req.as_reader().take(MAX_BODY).read_to_string(&mut body);
    }

    match (method, path.as_str()) {
        (Method::Get, "/v1/ping") => reply(
            req,
            200,
            json!({ "app": "GMID Tool", "version": app.package_info().version.to_string(), "paired": authorized }),
        ),
        (Method::Post, "/v1/pair") => {
            let Ok(b) = serde_json::from_str::<PairBody>(&body) else {
                return reply(req, 400, json!({ "error": "Kod eksik" }));
            };
            match bridge.try_pair(&db, &b.code) {
                Ok(token) => {
                    let _ = tauri::Emitter::emit(app, "bridge-changed", ());
                    reply(req, 200, json!({ "token": token }))
                }
                Err(e) => reply(req, 403, json!({ "error": e })),
            }
        }
        _ if !authorized => reply(req, 401, json!({ "error": "Eşleştirme gerekli" })),
        (Method::Post, "/v1/probe") => {
            let Ok(b) = serde_json::from_str::<ProbeBody>(&body) else {
                return reply(req, 400, json!({ "error": "Bağlantı eksik" }));
            };
            let root = app.state::<crate::paths::DataDir>().root().to_path_buf();
            match app
                .state::<Downloader>()
                .probe(&root, &b.url, &b.context, b.whole_list)
            {
                Ok(p) => reply(req, 200, json!(p)),
                Err(e) if e == "TOOLS_MISSING" => {
                    reply(req, 409, json!({ "error": TOOLS_MISSING }))
                }
                Err(e) => reply(req, 422, json!({ "error": e })),
            }
        }
        (Method::Post, "/v1/download") => {
            let Ok(b) = serde_json::from_str::<DownloadBody>(&body) else {
                return reply(req, 400, json!({ "error": "Eksik bilgi" }));
            };
            match app.state::<Downloader>().enqueue(
                app,
                &b.url,
                &b.option,
                b.cookies,
                "extension",
                &b.context,
            ) {
                Ok(d) => reply(req, 200, json!(d)),
                Err(e) if e == "TOOLS_MISSING" => {
                    reply(req, 409, json!({ "error": TOOLS_MISSING }))
                }
                Err(e) => reply(req, 422, json!({ "error": e })),
            }
        }
        (Method::Post, "/v1/playlist") => {
            let Ok(b) = serde_json::from_str::<PlaylistBody>(&body) else {
                return reply(req, 400, json!({ "error": "Eksik bilgi" }));
            };
            match app.state::<Downloader>().enqueue_playlist(
                app,
                &b.url,
                &b.option,
                b.ids,
                "extension",
            ) {
                Ok(n) => reply(req, 200, json!({ "added": n })),
                Err(e) => reply(req, 422, json!({ "error": e })),
            }
        }
        (Method::Get, "/v1/jobs") => {
            let all = super::list(&db).unwrap_or_default();
            let recent: Vec<_> = all
                .into_iter()
                .filter(|d| {
                    d.status != "done" || d.finished_at.is_some_and(|t| now_ms() - t < 10 * 60_000)
                })
                .take(8)
                .collect();
            reply(
                req,
                200,
                json!({ "downloads": recent, "live": app.state::<Downloader>().live() }),
            )
        }
        (Method::Post, "/v1/open") => {
            crate::window::show_main_at(app, "videos".into());
            reply(req, 200, json!({ "ok": true }))
        }
        _ => reply(req, 404, json!({ "error": "Bulunamadı" })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_codes_and_tokens() {
        let dir = std::env::temp_dir().join(format!("gmid-br-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("t.db")).unwrap();
        let b = Bridge::default();
        assert!(
            b.try_pair(&db, "123456").is_err(),
            "eşleştirme başlatılmadan kabul edilmez"
        );
        let code = b.start_pairing();
        assert_eq!(code.len(), 6);
        let wrong = if code == "000000" { "111111" } else { "000000" };
        assert!(b.try_pair(&db, wrong).is_err());
        let token = b.try_pair(&db, &code).unwrap();
        assert_eq!(token.len(), 32);
        assert!(b.try_pair(&db, &code).is_err(), "kod tek kullanımlık");
        assert!(b.status(&db).paired);
        b.unpair(&db);
        assert!(!b.status(&db).paired);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn browsers() {
        assert_eq!(
            browser_name("Mozilla/5.0 ... Chrome/130 Safari/537.36 OPR/115"),
            "Opera"
        );
        assert_eq!(
            browser_name("Mozilla/5.0 ... Chrome/130 Safari/537.36"),
            "Chrome"
        );
    }
}
