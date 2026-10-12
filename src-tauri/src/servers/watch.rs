//! Sunucuda hareket bildirimi: "sunucu açılınca" ve "en az N kişi olunca" haber ver. 2 dakikada bir yalnız izlenen
//! sunucular sorgulanır. Aynı şey için tekrar tekrar bildirim gelmez: kişi sayısı eşiğin altına inip yeniden çıkınca
//! (ve en erken 45 dk sonra) yeniden haber verilir. İlk bakışta (uygulama açılırken) zaten dolu olanlar sessiz.

use super::{list, Server, Servers};
use crate::db::Db;
use crate::util::now_ms;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

pub const KEY: &str = "servers.watch";
const EVERY: Duration = Duration::from_secs(120);
const COOLDOWN_MS: i64 = 45 * 60 * 1000;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Rule {
    /// Kapalıyken açılınca haber ver.
    pub online: bool,
    /// En az bu kadar oyuncu olunca haber ver.
    pub min_players: Option<u32>,
}

impl Rule {
    pub fn active(&self) -> bool {
        self.online || self.min_players.is_some_and(|n| n > 0)
    }
}

pub fn rules(db: &Db) -> HashMap<String, Rule> {
    db.setting_json(KEY).unwrap_or_default()
}

pub fn set_rule(db: &Db, id: &str, rule: Rule) -> Result<(), String> {
    let mut all = rules(db);
    if rule.active() {
        all.insert(id.to_string(), rule);
    } else {
        all.remove(id);
    }
    let j = serde_json::to_string(&all).map_err(|e| e.to_string())?;
    db.set_setting(KEY, &j).map_err(|e| e.to_string())
}

/// Bir sunucunun son görülen hali.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seen {
    pub online: bool,
    pub players: u32,
    /// Kişi eşiği için bildirim verildi (eşiğin altına inince sıfırlanır).
    pub crowd_sent: bool,
    pub last_sent: i64,
}

/// Bir gözlemden sonra verilecek bildirim (varsa) ve yeni hal. `first`: ilk gözlem, sessiz.
pub fn decide(
    rule: &Rule,
    prev: Option<&Seen>,
    online: bool,
    players: u32,
    now: i64,
) -> (Option<&'static str>, Seen) {
    let mut next = Seen {
        online,
        players,
        crowd_sent: prev.is_some_and(|p| p.crowd_sent),
        last_sent: prev.map_or(0, |p| p.last_sent),
    };
    let Some(prev) = prev else {
        // İlk bakış: dolu olan için "gönderildi" say, uygulama açılırken bildirim yağmasın.
        next.crowd_sent = rule
            .min_players
            .is_some_and(|n| n > 0 && online && players >= n);
        return (None, next);
    };
    let cooled = now - prev.last_sent >= COOLDOWN_MS;
    let mut event = None;
    if let Some(n) = rule.min_players.filter(|n| *n > 0) {
        if online && players >= n && !next.crowd_sent && cooled {
            event = Some("crowd");
            next.crowd_sent = true;
        } else if !online || players + 1 < n {
            // Eşiğin bir altına inince yeniden kurulur (sınırda gidip gelince bildirim yağmasın).
            next.crowd_sent = false;
        }
    }
    if event.is_none() && rule.online && online && !prev.online && cooled {
        event = Some("online");
    }
    if event.is_some() {
        next.last_sent = now;
    }
    (event, next)
}

#[derive(Default)]
pub struct Watcher {
    seen: Mutex<HashMap<String, Seen>>,
}

fn tick(app: &AppHandle) {
    let db = app.state::<Db>();
    let rules = rules(&db);
    if rules.is_empty() {
        return;
    }
    let servers: Vec<Server> = list(&db)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| rules.contains_key(&s.id))
        .collect();
    if servers.is_empty() {
        return;
    }
    let statuses = app.state::<Servers>().query(&servers);
    let now = now_ms();
    let watcher = app.state::<Watcher>();
    let mut seen = watcher.seen.lock().unwrap_or_else(|e| e.into_inner());
    for st in statuses {
        if st.state == "unknown" {
            continue;
        }
        let (Some(rule), Some(srv)) = (rules.get(&st.id), servers.iter().find(|s| s.id == st.id))
        else {
            continue;
        };
        let online = st.state == "online";
        let players = st.info.as_ref().map_or(0, |i| i.players);
        let (event, next) = decide(rule, seen.get(&st.id), online, players, now);
        seen.insert(st.id.clone(), next);
        let map = st
            .info
            .as_ref()
            .map(|i| i.map.trim().to_string())
            .filter(|m| !m.is_empty());
        let body = match event {
            Some("crowd") => format!(
                "{players} kişi oynuyor{}. Gel!",
                map.map(|m| format!(" · {m}")).unwrap_or_default()
            ),
            Some("online") => format!(
                "Sunucu açıldı{}.",
                map.map(|m| format!(" · {m}")).unwrap_or_default()
            ),
            _ => continue,
        };
        crate::overlay::system_notification(app, "servers.notify", &srv.name, &body);
    }
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(45));
        loop {
            tick(&app);
            std::thread::sleep(EVERY);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crowd_notifies_once_until_it_drops() {
        let rule = Rule {
            online: false,
            min_players: Some(5),
        };
        let t = 10 * COOLDOWN_MS;
        // İlk bakış: zaten 6 kişi → sessiz, gönderildi sayılır.
        let (e, s) = decide(&rule, None, true, 6, t);
        assert_eq!(e, None);
        assert!(s.crowd_sent);
        // Hâlâ dolu: yine sessiz.
        let (e, s) = decide(&rule, Some(&s), true, 7, t + 1);
        assert_eq!(e, None);
        // 3'e düştü (eşiğin 1 altından da aşağı): yeniden kurulur.
        let (e, s) = decide(&rule, Some(&s), true, 3, t + 2);
        assert_eq!(e, None);
        assert!(!s.crowd_sent);
        // 5'e çıktı: bildirim.
        let (e, s) = decide(&rule, Some(&s), true, 5, t + 3);
        assert_eq!(e, Some("crowd"));
        // 4 ↔ 5 arasında gidip gelmek bildirim yağdırmaz.
        let (_, s) = decide(&rule, Some(&s), true, 4, t + 4);
        let (e, _) = decide(&rule, Some(&s), true, 5, t + 5);
        assert_eq!(e, None);
    }

    #[test]
    fn online_notifies_on_transition_with_cooldown() {
        let rule = Rule {
            online: true,
            min_players: None,
        };
        let t = 10 * COOLDOWN_MS;
        let (e, s) = decide(&rule, None, false, 0, t);
        assert_eq!(e, None);
        let (e, s) = decide(&rule, Some(&s), true, 0, t + 1);
        assert_eq!(e, Some("online"));
        let (_, s) = decide(&rule, Some(&s), false, 0, t + 2);
        // Kısa sürede yeniden açıldı: bekleme süresi dolmadı, sessiz.
        let (e, _) = decide(&rule, Some(&s), true, 0, t + 3);
        assert_eq!(e, None);
        assert!(!Rule::default().active());
    }
}
