//! GMID Wrapped: bir dönemin (ay/yıl) özeti — oyun saatleri, en çok oynananlar, alışkanlıklar, rozetler,
//! puanlar, izlenenler, klipler. Arayüz bunu paylaşılabilir bir görsele (PNG) çizer.

use crate::db::Db;
use base64::Engine;
use chrono::{Datelike, Local, TimeZone, Timelike};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TopGame {
    pub name: String,
    pub seconds: i64,
    pub sessions: i64,
    /// Kapak (data: adresi; görsele çizilebilsin diye gömülü).
    pub cover: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Longest {
    pub game: String,
    pub seconds: i64,
    pub at: i64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Badge {
    pub id: &'static str,
    pub title: &'static str,
    pub desc: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Wrapped {
    pub from: i64,
    pub to: i64,
    pub total_seconds: i64,
    pub sessions: i64,
    pub days_played: usize,
    pub distinct_games: usize,
    pub top: Vec<TopGame>,
    pub longest: Option<Longest>,
    /// 0 = Pazartesi.
    pub best_weekday: Option<u32>,
    /// Başlangıç saatine göre oynama (saniye), yerel saat.
    pub hours: Vec<i64>,
    pub night_share: f64,
    pub weekend_share: f64,
    pub new_games: i64,
    pub rated_games: i64,
    pub best_game: Option<(String, f64)>,
    pub titles_watched: i64,
    pub best_title: Option<(String, f64)>,
    pub clips: i64,
    pub shots: i64,
    pub downloads: i64,
    pub notes: i64,
    pub badges: Vec<Badge>,
}

/// Bir oturum: (oyun, başlangıç ms, saniye).
pub type Session = (String, i64, i64);

/// Oturumlardan oyun istatistikleri (saf; test edilebilir).
pub fn play_stats(sessions: &[Session], names: &HashMap<String, String>) -> Wrapped {
    let mut w = Wrapped {
        hours: vec![0; 24],
        ..Default::default()
    };
    let mut per_game: HashMap<&str, (i64, i64)> = HashMap::new();
    let mut days = HashSet::new();
    let mut weekday = [0i64; 7];
    let (mut night, mut weekend) = (0i64, 0i64);
    for (game, start, secs) in sessions {
        let secs = (*secs).max(0);
        w.total_seconds += secs;
        w.sessions += 1;
        let e = per_game.entry(game.as_str()).or_default();
        e.0 += secs;
        e.1 += 1;
        if let Some(t) = Local.timestamp_millis_opt(*start).single() {
            days.insert(t.date_naive());
            let wd = t.weekday().num_days_from_monday() as usize;
            weekday[wd] += secs;
            w.hours[t.hour() as usize] += secs;
            if t.hour() < 5 {
                night += secs;
            }
            if wd >= 5 {
                weekend += secs;
            }
        }
        let name = names.get(game).cloned().unwrap_or_else(|| game.clone());
        if w.longest.as_ref().map_or(true, |l| secs > l.seconds) {
            w.longest = Some(Longest {
                game: name,
                seconds: secs,
                at: *start,
            });
        }
    }
    w.days_played = days.len();
    w.distinct_games = per_game.len();
    let mut top: Vec<(&str, (i64, i64))> = per_game.into_iter().collect();
    top.sort_by_key(|(n, (s, _))| (std::cmp::Reverse(*s), n.to_string()));
    w.top = top
        .into_iter()
        .take(5)
        .map(|(g, (s, n))| TopGame {
            name: names.get(g).cloned().unwrap_or_else(|| g.to_string()),
            seconds: s,
            sessions: n,
            cover: None,
        })
        .collect();
    w.best_weekday = (w.total_seconds > 0)
        .then(|| (0..7u32).max_by_key(|d| weekday[*d as usize]))
        .flatten();
    if w.total_seconds > 0 {
        w.night_share = night as f64 / w.total_seconds as f64;
        w.weekend_share = weekend as f64 / w.total_seconds as f64;
    }
    w
}

/// En fazla üç rozet, en "dikkat çekici"den başlayarak.
pub fn badges(w: &Wrapped) -> Vec<Badge> {
    let hours = w.total_seconds as f64 / 3600.0;
    let mut out = Vec::new();
    if let Some(t) = w.top.first() {
        let share = t.seconds as f64 / w.total_seconds.max(1) as f64;
        if share >= 0.6 && hours >= 5.0 {
            out.push(Badge {
                id: "loyal",
                title: "Sadık",
                desc: format!("Oyun saatlerinin %{}'i {}", (share * 100.0).round(), t.name),
            });
        }
    }
    if w.night_share >= 0.3 && hours >= 3.0 {
        out.push(Badge {
            id: "night",
            title: "Gece Kuşu",
            desc: format!(
                "Oyunlarının %{}'i gece 00:00–05:00 arası",
                (w.night_share * 100.0).round()
            ),
        });
    }
    if w.longest.as_ref().is_some_and(|l| l.seconds >= 4 * 3600) {
        let l = w.longest.as_ref().unwrap();
        out.push(Badge {
            id: "marathon",
            title: "Maratoncu",
            desc: format!("{} ile {} saatlik tek oturum", l.game, l.seconds / 3600),
        });
    }
    if w.weekend_share >= 0.6 && hours >= 3.0 {
        out.push(Badge {
            id: "weekend",
            title: "Hafta Sonu Savaşçısı",
            desc: format!(
                "Oyunlarının %{}'i hafta sonu",
                (w.weekend_share * 100.0).round()
            ),
        });
    }
    if w.distinct_games >= 8 {
        out.push(Badge {
            id: "explorer",
            title: "Kâşif",
            desc: format!("{} farklı oyun", w.distinct_games),
        });
    }
    if w.titles_watched >= 5 {
        out.push(Badge {
            id: "cinephile",
            title: "Sinefil",
            desc: format!("{} film/dizi bitirdin", w.titles_watched),
        });
    }
    if w.rated_games >= 5 {
        out.push(Badge {
            id: "critic",
            title: "Eleştirmen",
            desc: format!("{} oyun puanladın", w.rated_games),
        });
    }
    if w.clips >= 10 {
        out.push(Badge {
            id: "clipper",
            title: "Klip Avcısı",
            desc: format!("{} klip", w.clips),
        });
    }
    out.truncate(3);
    out
}

fn data_url(path: &str) -> Option<String> {
    let p = Path::new(path);
    let meta = std::fs::metadata(p).ok()?;
    if !meta.is_file() || meta.len() > 3 * 1024 * 1024 {
        return None;
    }
    let mime = match p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/jpeg",
    };
    let bytes = std::fs::read(p).ok()?;
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn count(db: &Db, sql: &str, from: i64, to: i64) -> i64 {
    db.conn()
        .query_row(sql, [from, to], |r| r.get(0))
        .unwrap_or(0)
}

fn best(db: &Db, sql: &str, from: i64, to: i64) -> Option<(String, f64)> {
    db.conn()
        .query_row(sql, [from, to], |r| Ok((r.get(0)?, r.get(1)?)))
        .ok()
}

pub fn compute(db: &Db, from: i64, to: i64) -> Result<Wrapped, String> {
    let (sessions, names, covers) = {
        let conn = db.conn();
        let mut st = conn
            .prepare(
                "SELECT game_id, started_at, seconds FROM play_sessions
                 WHERE started_at >= ?1 AND started_at < ?2 AND seconds > 0",
            )
            .map_err(|e| e.to_string())?;
        let sessions: Vec<Session> = st
            .query_map([from, to], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        let mut st = conn
            .prepare("SELECT id, name, cover FROM games")
            .map_err(|e| e.to_string())?;
        let rows: Vec<(String, String, Option<String>)> = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        let names: HashMap<String, String> = rows
            .iter()
            .map(|(i, n, _)| (i.clone(), n.clone()))
            .collect();
        let covers: HashMap<String, String> = rows
            .into_iter()
            .filter_map(|(_, n, c)| Some((n, c?)))
            .collect();
        (sessions, names, covers)
    };
    let mut w = play_stats(&sessions, &names);
    w.from = from;
    w.to = to;
    for t in &mut w.top {
        t.cover = covers.get(&t.name).and_then(|c| data_url(c));
    }
    w.new_games = count(
        db,
        "SELECT COUNT(*) FROM games WHERE in_library = 1 AND added_at >= ?1 AND added_at < ?2",
        from,
        to,
    );
    w.rated_games = count(
        db,
        "SELECT COUNT(*) FROM reviews WHERE subject_kind = 'game' AND overall IS NOT NULL
         AND updated_at >= ?1 AND updated_at < ?2",
        from,
        to,
    );
    w.best_game = best(
        db,
        "SELECT g.name, r.overall FROM reviews r JOIN games g ON g.id = r.subject_id
         WHERE r.subject_kind = 'game' AND r.overall IS NOT NULL AND r.updated_at >= ?1 AND r.updated_at < ?2
         ORDER BY r.overall DESC LIMIT 1",
        from,
        to,
    );
    w.titles_watched = count(
        db,
        "SELECT COUNT(*) FROM reviews WHERE subject_kind = 'title' AND status IN ('finished', 'completed')
         AND updated_at >= ?1 AND updated_at < ?2",
        from,
        to,
    );
    w.best_title = best(
        db,
        "SELECT t.name, r.overall FROM reviews r JOIN titles t ON t.id = r.subject_id
         WHERE r.subject_kind = 'title' AND r.overall IS NOT NULL AND r.updated_at >= ?1 AND r.updated_at < ?2
         ORDER BY r.overall DESC LIMIT 1",
        from,
        to,
    );
    w.clips = count(
        db,
        "SELECT COUNT(*) FROM captures WHERE kind = 'clip' AND created_at >= ?1 AND created_at < ?2",
        from,
        to,
    );
    w.shots = count(
        db,
        "SELECT COUNT(*) FROM captures WHERE kind = 'shot' AND created_at >= ?1 AND created_at < ?2",
        from,
        to,
    );
    w.downloads = count(
        db,
        "SELECT COUNT(*) FROM downloads WHERE status = 'done' AND finished_at >= ?1 AND finished_at < ?2",
        from,
        to,
    );
    w.notes = count(
        db,
        "SELECT COUNT(*) FROM notes WHERE deleted_at IS NULL AND created_at >= ?1 AND created_at < ?2",
        from,
        to,
    );
    w.badges = badges(&w);
    Ok(w)
}

/// Görseli `Ozetler` klasörüne yazar.
pub fn save(dir: &Path, name: &str, png: &[u8]) -> Result<String, String> {
    if png.len() < 8 || &png[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("Görsel oluşturulamadı".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .take(60)
        .collect();
    let path = dir.join(format!("GMID Wrapped - {}.png", safe.trim()));
    std::fs::write(&path, png).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, m: u32, d: u32, h: u32) -> i64 {
        Local
            .with_ymd_and_hms(y, m, d, h, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn play_stats_and_badges() {
        let names: HashMap<String, String> = [("v", "VALORANT"), ("h", "Hades II"), ("c", "CS2")]
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        let s = vec![
            // Cumartesi gece: 5 saat Valorant (maraton + gece)
            ("v".to_string(), at(2026, 10, 3, 1), 5 * 3600),
            ("v".to_string(), at(2026, 10, 4, 2), 3 * 3600),
            ("h".to_string(), at(2026, 10, 6, 20), 3600),
            ("c".to_string(), at(2026, 10, 7, 21), 1800),
        ];
        let w = play_stats(&s, &names);
        assert_eq!(w.total_seconds, 9 * 3600 + 1800);
        assert_eq!(w.sessions, 4);
        assert_eq!(w.days_played, 4);
        assert_eq!(w.distinct_games, 3);
        assert_eq!(w.top[0].name, "VALORANT");
        assert_eq!(w.top[0].seconds, 8 * 3600);
        assert_eq!(w.longest.as_ref().unwrap().game, "VALORANT");
        assert_eq!(w.best_weekday, Some(5)); // Cumartesi
        assert!(w.night_share > 0.8);
        let b: Vec<&str> = badges(&w).iter().map(|b| b.id).collect();
        assert_eq!(b, ["loyal", "night", "marathon"]);
        // Boş dönem
        let e = play_stats(&[], &names);
        assert_eq!((e.total_seconds, e.best_weekday, e.top.len()), (0, None, 0));
        assert!(badges(&e).is_empty());
    }

    #[test]
    fn compute_reads_database() {
        let dir = std::env::temp_dir().join(format!("gmid-wrapped-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("gmid.db")).unwrap();
        let t = at(2026, 10, 10, 20);
        db.conn()
            .execute_batch(&format!(
                "INSERT INTO games (id, name, source, source_id, added_at) VALUES ('g1', 'Hades II', 'manual', 'x', {t});
                 INSERT INTO play_sessions (id, game_id, started_at, ended_at, seconds) VALUES ('s1', 'g1', {t}, {t}, 7200);
                 INSERT INTO play_sessions (id, game_id, started_at, ended_at, seconds) VALUES ('s0', 'g1', 1000, 1000, 9999);"
            ))
            .unwrap();
        let w = compute(&db, at(2026, 10, 1, 0), at(2026, 11, 1, 0)).unwrap();
        assert_eq!(w.total_seconds, 7200);
        assert_eq!(w.new_games, 1);
        assert_eq!(w.top[0].name, "Hades II");
        let p = save(&dir.join("Ozetler"), "Ekim 2026", b"\x89PNG\r\n\x1a\nxxxx").unwrap();
        assert!(p.ends_with("GMID Wrapped - Ekim 2026.png"));
        assert!(save(&dir, "x", b"not png").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
