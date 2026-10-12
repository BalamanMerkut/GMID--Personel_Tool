//! Her şeyde tek arama (Ctrl+K): notlar, oyunlar, film/diziler, videolar, klipler, hatırlatıcılar.
//! Türkçe harfler katlanır ("ışık" ~ "isik"), büyük/küçük harf fark etmez. Ekranlar ve ayarlar arayüzde aranır.

use crate::db::Db;
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    /// note | game | title | video | clip | reminder
    pub kind: &'static str,
    pub id: String,
    pub title: String,
    pub sub: String,
    /// Kapak/küçük resim (yerel yol ya da adres).
    pub image: Option<String>,
    /// Sıralama: başta eşleşme önce.
    #[serde(skip)]
    score: u8,
}

/// Küçük harf + Türkçe/aksan katlama.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'İ' | 'I' | 'ı' => vec!['i'],
            'Ş' | 'ş' => vec!['s'],
            'Ğ' | 'ğ' => vec!['g'],
            'Ç' | 'ç' => vec!['c'],
            'Ö' | 'ö' => vec!['o'],
            'Ü' | 'ü' => vec!['u'],
            'Â' | 'â' => vec!['a'],
            'Î' | 'î' => vec!['i'],
            'Û' | 'û' => vec!['u'],
            c => c.to_lowercase().collect(),
        })
        .collect()
}

/// Eşleşme puanı: 3 baştan, 2 bir sözcüğün başından, 1 içinde, 0 yok. Sorgunun bütün sözcükleri geçmeli.
pub fn score(haystack: &str, words: &[String]) -> u8 {
    let h = fold(haystack);
    let mut best = 3;
    for w in words {
        let Some(i) = h.find(w.as_str()) else {
            return 0;
        };
        let s = if i == 0 {
            3
        } else if !h[..i].chars().last().is_some_and(char::is_alphanumeric) {
            2
        } else {
            1
        };
        best = best.min(s);
    }
    best
}

/// Gövdede eşleşen yerin çevresi (tek satır).
fn snippet(body: &str, word: &str) -> String {
    let flat: String = body
        .replace(['\n', '\r', '\t'], " ")
        .chars()
        .filter(|c| !"#*_`>[]".contains(*c))
        .collect();
    let chars: Vec<char> = flat.chars().collect();
    let folded: Vec<String> = chars.iter().map(|c| fold(&c.to_string())).collect();
    let joined: String = folded.concat();
    let pos = joined.find(word).map(|byte| {
        // Katlanmış metindeki konumu karakter sırasına çevir.
        let mut acc = 0;
        folded
            .iter()
            .position(|f| {
                acc += f.len();
                acc > byte
            })
            .unwrap_or(0)
    });
    let start = pos.unwrap_or(0).saturating_sub(30);
    let s: String = chars.iter().skip(start).take(90).collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if start > 0 {
        format!("…{s}")
    } else {
        s
    }
}

const PER_KIND: usize = 6;

pub fn search(db: &Db, query: &str) -> Vec<Hit> {
    let words: Vec<String> = fold(query).split_whitespace().map(String::from).collect();
    if words.is_empty() || words.iter().map(|w| w.chars().count()).sum::<usize>() < 2 {
        return Vec::new();
    }
    let conn = db.conn();
    let mut out: Vec<Hit> = Vec::new();
    let mut push = |mut hits: Vec<Hit>| {
        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.title.cmp(&b.title)));
        out.extend(hits.into_iter().take(PER_KIND));
    };

    // Notlar: başlık ya da içerik.
    if let Ok(mut st) = conn.prepare(
        "SELECT id, title, body, updated_at FROM notes WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT 2000",
    ) {
        let rows = st
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
            .map(|r| r.filter_map(Result::ok).collect::<Vec<_>>())
            .unwrap_or_default();
        push(
            rows.into_iter()
                .filter_map(|(id, title, body)| {
                    let t = score(&title, &words);
                    let b = if t == 0 { score(&body, &words) } else { 0 };
                    (t > 0 || b > 0).then(|| Hit {
                        kind: "note",
                        id,
                        title: if title.trim().is_empty() { "Adsız not".into() } else { title },
                        sub: if t > 0 { snippet(&body, "") } else { snippet(&body, &words[0]) },
                        image: None,
                        score: if t > 0 { t + 1 } else { b },
                    })
                })
                .collect(),
        );
    }

    // Oyunlar (kütüphane ve arşiv)
    if let Ok(mut st) =
        conn.prepare("SELECT id, name, developer, cover, in_library FROM games WHERE hidden = 0")
    {
        let rows = st
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, i64>(4)?,
                ))
            })
            .map(|r| r.filter_map(Result::ok).collect::<Vec<_>>())
            .unwrap_or_default();
        push(
            rows.into_iter()
                .filter_map(|(id, name, dev, cover, lib)| {
                    let s = score(
                        &format!("{name} {}", dev.clone().unwrap_or_default()),
                        &words,
                    );
                    (s > 0).then(|| Hit {
                        kind: "game",
                        id,
                        title: name,
                        sub: [
                            Some(
                                if lib == 1 {
                                    "Kütüphanem"
                                } else {
                                    "Oyun Arşivi"
                                }
                                .to_string(),
                            ),
                            dev,
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" · "),
                        image: cover,
                        score: s,
                    })
                })
                .collect(),
        );
    }

    // Film ve diziler
    if let Ok(mut st) =
        conn.prepare("SELECT id, name, original, year, kind, poster, directors FROM titles")
    {
        let rows = st
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })
            .map(|r| r.filter_map(Result::ok).collect::<Vec<_>>())
            .unwrap_or_default();
        push(
            rows.into_iter()
                .filter_map(|(id, name, original, year, kind, poster, directors)| {
                    let dirs: Vec<String> = serde_json::from_str(&directors).unwrap_or_default();
                    let s = score(
                        &format!("{name} {} {}", original.unwrap_or_default(), dirs.join(" ")),
                        &words,
                    );
                    (s > 0).then(|| Hit {
                        kind: "title",
                        id,
                        title: name,
                        sub: [
                            Some(if kind == "series" { "Dizi" } else { "Film" }.to_string()),
                            year.map(|y| y.to_string()),
                            dirs.first().cloned(),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" · "),
                        image: poster,
                        score: s,
                    })
                })
                .collect(),
        );
    }

    // İndirilen videolar ve müzikler
    if let Ok(mut st) = conn.prepare(
        "SELECT id, title, uploader, thumbnail, audio_only FROM downloads WHERE status = 'done' ORDER BY finished_at DESC",
    ) {
        let rows = st
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, i64>(4)?,
                ))
            })
            .map(|r| r.filter_map(Result::ok).collect::<Vec<_>>())
            .unwrap_or_default();
        push(
            rows.into_iter()
                .filter_map(|(id, title, uploader, thumb, audio)| {
                    let s = score(&format!("{title} {uploader}"), &words);
                    (s > 0).then(|| Hit {
                        kind: "video",
                        id,
                        title,
                        sub: format!("{} · {uploader}", if audio == 1 { "Müzik" } else { "Video" }),
                        image: thumb,
                        score: s,
                    })
                })
                .collect(),
        );
    }

    // Klipler ve ekran görüntüleri
    if let Ok(mut st) = conn.prepare(
        "SELECT c.id, COALESCE(c.title, c.path), c.kind, c.thumb, COALESCE(g.name, c.game_hint, '')
         FROM captures c LEFT JOIN games g ON g.id = c.game_id ORDER BY c.created_at DESC LIMIT 5000",
    ) {
        let rows = st
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })
            .map(|r| r.filter_map(Result::ok).collect::<Vec<_>>())
            .unwrap_or_default();
        push(
            rows.into_iter()
                .filter_map(|(id, title, kind, thumb, game)| {
                    // Yol ise yalnız dosya adı.
                    let name = title
                        .rsplit(['\\', '/'])
                        .next()
                        .unwrap_or(&title)
                        .to_string();
                    let s = score(&format!("{name} {game}"), &words);
                    (s > 0).then(|| Hit {
                        kind: "clip",
                        id,
                        title: name,
                        sub: [
                            if kind == "clip" { "Klip" } else { "Ekran görüntüsü" }.to_string(),
                            game,
                        ]
                        .into_iter()
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(" · "),
                        image: thumb,
                        score: s,
                    })
                })
                .collect(),
        );
    }

    // Hatırlatıcılar (bitmemiş)
    if let Ok(mut st) = conn.prepare("SELECT id, title, note, due_at FROM reminders WHERE done = 0")
    {
        let rows = st
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map(|r| r.filter_map(Result::ok).collect::<Vec<_>>())
            .unwrap_or_default();
        push(
            rows.into_iter()
                .filter_map(|(id, title, note, due)| {
                    let s = score(&format!("{title} {note}"), &words);
                    (s > 0).then(|| Hit {
                        kind: "reminder",
                        id,
                        title,
                        sub: chrono::DateTime::from_timestamp_millis(due)
                            .map(|d| {
                                d.with_timezone(&chrono::Local)
                                    .format("%d.%m.%Y %H:%M")
                                    .to_string()
                            })
                            .unwrap_or_default(),
                        image: None,
                        score: s,
                    })
                })
                .collect(),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_folding_and_scoring() {
        assert_eq!(fold("IŞIK Ğüç İçin"), "isik guc icin");
        let w = |q: &str| {
            fold(q)
                .split_whitespace()
                .map(String::from)
                .collect::<Vec<_>>()
        };
        assert_eq!(score("Valorant", &w("val")), 3);
        assert_eq!(score("Counter-Strike 2", &w("strike")), 2);
        assert_eq!(score("Baldur's Gate 3", &w("dur")), 1);
        assert_eq!(score("Hades II", &w("hades 2")), 0);
        assert_eq!(score("Şehir Işıkları", &w("isik seh")), 2);
    }

    #[test]
    fn searches_database() {
        let dir = std::env::temp_dir().join(format!("gmid-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("gmid.db")).unwrap();
        db.conn()
            .execute_batch(
                "INSERT INTO notes (id, title, body, created_at, updated_at) VALUES
                   ('n1', 'Valorant ayarları', 'hassasiyet 0.35', 1, 1),
                   ('n2', 'Alışveriş', 'süt, ekmek ve **Valorant** skin', 1, 2);
                 INSERT INTO games (id, name, source, source_id, added_at) VALUES ('g1', 'VALORANT', 'riot', 'v', 1);
                 INSERT INTO reminders (id, title, due_at, anchor_at, created_at) VALUES ('r1', 'Valorant turnuvası', 1, 1, 1);",
            )
            .unwrap();
        let hits = search(&db, "valo");
        let kinds: Vec<(&str, &str)> = hits.iter().map(|h| (h.kind, h.id.as_str())).collect();
        // Başlıkta eşleşen not, içerikte eşleşenden önce.
        assert_eq!(
            kinds,
            [
                ("note", "n1"),
                ("note", "n2"),
                ("game", "g1"),
                ("reminder", "r1")
            ]
        );
        assert!(hits[1].sub.contains("Valorant skin"), "{}", hits[1].sub);
        assert!(search(&db, "v").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
