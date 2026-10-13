//! Discord'da durum (Rich Presence): oyundayken Discord profilinde "GMID Tool · Valorant oynuyor" ve "GMID'e katıl"
//! düğmesi. Sunucu gerekmez: Discord masaüstü uygulamasının bu bilgisayardaki yerel bağlantısı (Windows'ta
//! `\\.\pipe\discord-ipc-0`) kullanılır. Discord, durumu bir "uygulama kimliği" adına gösterir: Discord Developer
//! Portal'da bir kez "GMID Tool" adlı uygulama açılır, kimliği Kontrol Odası'na yapıştırılır.

use crate::db::Db;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

pub const ON_KEY: &str = "discord.presence";
pub const APP_KEY: &str = "discord.appId";
/// Discord Developer Portal → Rich Presence → Art Assets'e bu adla yüklenen görsel.
const LARGE_IMAGE: &str = "gmid";

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;

/// Discord'un yerel bağlantı çerçevesi: işlem kodu ve uzunluk (küçük uçlu), ardından JSON.
pub fn encode(op: u32, payload: &Value) -> Vec<u8> {
    let body = payload.to_string().into_bytes();
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

fn read_frame(r: &mut dyn Read) -> std::io::Result<(u32, Value)> {
    let mut head = [0u8; 8];
    r.read_exact(&mut head)?;
    let op = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    if len > 1 << 20 {
        return Err(std::io::Error::other("çerçeve çok büyük"));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    Ok((op, serde_json::from_slice(&body).unwrap_or(Value::Null)))
}

/// Gösterilecek etkinlik; `None`: durumu temizle.
pub fn activity(game: Option<(&str, i64)>, invite: Option<&str>) -> Option<Value> {
    let (name, started_ms) = game?;
    let mut a = json!({
        "details": format!("{name} oynuyor"),
        "state": "Game Mansion In Discord",
        "timestamps": { "start": started_ms / 1000 },
        "assets": { "large_image": LARGE_IMAGE, "large_text": "GMID Tool" },
    });
    if let Some(url) = invite.filter(|u| u.starts_with("https://")) {
        a["buttons"] = json!([{ "label": "GMID'e katıl", "url": url }]);
    }
    Some(a)
}

trait Pipe: Read + Write + Send {}
impl<T: Read + Write + Send> Pipe for T {}

fn open_pipe() -> Option<Box<dyn Pipe>> {
    for i in 0..10 {
        #[cfg(windows)]
        {
            let path = format!(r"\\.\pipe\discord-ipc-{i}");
            if let Ok(f) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
            {
                return Some(Box::new(f));
            }
        }
        #[cfg(unix)]
        {
            let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
            if let Ok(s) = std::os::unix::net::UnixStream::connect(format!("{dir}/discord-ipc-{i}"))
            {
                return Some(Box::new(s));
            }
        }
    }
    None
}

struct Conn {
    pipe: Box<dyn Pipe>,
    app_id: String,
}

impl Conn {
    fn open(app_id: &str) -> Option<Conn> {
        let mut pipe = open_pipe()?;
        pipe.write_all(&encode(
            OP_HANDSHAKE,
            &json!({ "v": 1, "client_id": app_id }),
        ))
        .ok()?;
        // İlk yanıt READY olmalı; değilse (yanlış kimlik) bağlantı kapanır.
        let (_, ready) = read_frame(&mut pipe).ok()?;
        if ready["evt"] != "READY" {
            crate::log!(
                "Discord durumu: bağlanılamadı ({})",
                ready["data"]["message"]
            );
            return None;
        }
        Some(Conn {
            pipe,
            app_id: app_id.to_string(),
        })
    }

    fn set(&mut self, act: Option<Value>) -> bool {
        let msg = json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": act },
            "nonce": crate::util::new_id(),
        });
        if self.pipe.write_all(&encode(OP_FRAME, &msg)).is_err() {
            return false;
        }
        match read_frame(&mut self.pipe) {
            Ok((_, v)) if v["evt"] == "ERROR" => {
                crate::log!("Discord durumu: {}", v["data"]["message"]);
                true
            }
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

#[derive(Default)]
pub struct Presence {
    conn: Mutex<Option<Conn>>,
    /// Son gönderilen etkinlik (değişmediyse yeniden gönderilmez).
    last: Mutex<Option<Option<Value>>>,
}

impl Presence {
    /// Ayara ve oyuna göre durumu günceller (15 sn'de bir ve ayar değişince).
    pub fn tick(&self, app: &AppHandle) {
        let db = app.state::<Db>();
        let app_id: String = db.setting_json(APP_KEY).unwrap_or_default();
        let on = db.setting_json::<bool>(ON_KEY).unwrap_or(true) && valid_id(&app_id);
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !on {
            if let Some(mut c) = conn.take() {
                c.set(None);
            }
            *self.last.lock().unwrap_or_else(|e| e.into_inner()) = None;
            return;
        }
        let game = app
            .state::<crate::games::Games>()
            .list(&db)
            .unwrap_or_default()
            .into_iter()
            .find_map(|g| g.running_since.map(|t| (g.name, t)));
        let invite = crate::servers::discord_link(&db);
        let want = activity(
            game.as_ref().map(|(n, t)| (n.as_str(), *t)),
            invite.as_deref(),
        );
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        if conn.as_ref().is_some_and(|c| c.app_id != app_id) {
            conn.take();
            *last = None;
        }
        if last.as_ref() == Some(&want) && conn.is_some() {
            return;
        }
        // Discord kapalıysa ve gösterilecek bir şey yoksa bağlanmaya çalışma.
        if conn.is_none() {
            if want.is_none() {
                *last = Some(None);
                return;
            }
            *conn = Conn::open(&app_id);
        }
        let ok = conn.as_mut().is_some_and(|c| c.set(want.clone()));
        if ok {
            *last = Some(want);
        } else {
            conn.take();
            *last = None;
        }
    }
}

/// Discord uygulama kimliği: 17–20 haneli sayı.
pub fn valid_id(id: &str) -> bool {
    (17..=20).contains(&id.len()) && id.chars().all(|c| c.is_ascii_digit())
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        app.state::<Presence>().tick(&app);
        std::thread::sleep(Duration::from_secs(15));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip() {
        let v = json!({ "v": 1, "client_id": "123" });
        let bytes = encode(OP_HANDSHAKE, &v);
        assert_eq!(&bytes[..4], &0u32.to_le_bytes());
        assert_eq!(
            u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize,
            bytes.len() - 8
        );
        let (op, back) = read_frame(&mut &bytes[..]).unwrap();
        assert_eq!((op, back), (OP_HANDSHAKE, v));
    }

    /// Sahte bir Discord: el sıkışmaya READY, SET_ACTIVITY'ye yanıt verir.
    #[cfg(unix)]
    #[test]
    fn talks_to_a_fake_discord() {
        use std::os::unix::net::UnixListener;
        let dir = std::env::temp_dir().join(format!("gmid-ipc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("XDG_RUNTIME_DIR", &dir);
        let listener = UnixListener::bind(dir.join("discord-ipc-0")).unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let (op, hello) = read_frame(&mut s).unwrap();
            assert_eq!(
                (op, hello["client_id"].as_str()),
                (OP_HANDSHAKE, Some("1234567890123456789"))
            );
            s.write_all(&encode(
                OP_FRAME,
                &json!({ "cmd": "DISPATCH", "evt": "READY" }),
            ))
            .unwrap();
            let (_, set) = read_frame(&mut s).unwrap();
            s.write_all(&encode(
                OP_FRAME,
                &json!({ "cmd": "SET_ACTIVITY", "evt": null }),
            ))
            .unwrap();
            set
        });
        let mut c = Conn::open("1234567890123456789").expect("bağlanmalı");
        assert!(c.set(activity(Some(("Valorant", 0)), None)));
        let sent = server.join().unwrap();
        assert_eq!(sent["cmd"], "SET_ACTIVITY");
        assert_eq!(sent["args"]["activity"]["details"], "Valorant oynuyor");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn activity_payload() {
        assert_eq!(activity(None, None), None);
        let a = activity(
            Some(("Valorant", 1_700_000_000_000)),
            Some("https://discord.gg/upu9VQwFrV"),
        )
        .unwrap();
        assert_eq!(a["details"], "Valorant oynuyor");
        assert_eq!(a["timestamps"]["start"], 1_700_000_000);
        assert_eq!(a["buttons"][0]["url"], "https://discord.gg/upu9VQwFrV");
        let b = activity(Some(("CS2", 0)), Some("discord.gg/x")).unwrap();
        assert!(b.get("buttons").is_none());
        assert!(valid_id("1234567890123456789"));
        assert!(!valid_id("abc"));
        assert!(!valid_id(""));
    }
}
