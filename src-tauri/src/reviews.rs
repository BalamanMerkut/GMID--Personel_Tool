//! Oyun Arşivi puanları: 5 kısa soru → otomatik genel puan (+ istersen ±1), tek cümle, notlar, durum, tavsiye.
//! Veri modeli baştan çok kişili (`author`); şimdilik hep "me". Film ve diziler de ileride aynı tabloya gelir.

use crate::db::Db;
use crate::util::{new_id, now_ms};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

const ME: &str = "me";

/// Soru başına puan (1–10); `None` = "Yok" (ör. hikâyesi olmayan çok oyunculu oyun).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Scores {
    pub gameplay: Option<f64>,
    pub story: Option<f64>,
    pub enjoyment: Option<f64>,
    pub visuals: Option<f64>,
    pub performance: Option<f64>,
}

/// Genel puana etkisi (yüzde): oynanış 35, hikâye 20, keyif 20, görsellik 15, performans 10.
const WEIGHTS: [f64; 5] = [35.0, 20.0, 20.0, 15.0, 10.0];

impl Scores {
    fn values(&self) -> [Option<f64>; 5] {
        [
            self.gameplay,
            self.story,
            self.enjoyment,
            self.visuals,
            self.performance,
        ]
    }

    /// "Yok" denen soruların etkisi diğerlerine oransal dağılır (puanı düşürmez).
    pub fn auto(&self) -> Option<f64> {
        let (mut sum, mut weight) = (0.0, 0.0);
        for (v, w) in self.values().iter().zip(WEIGHTS) {
            if let Some(v) = v {
                sum += v.clamp(1.0, 10.0) * w;
                weight += w;
            }
        }
        (weight > 0.0).then(|| round1(sum / weight))
    }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: String,
    pub author: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub scores: Scores,
    pub adjust: f64,
    pub overall: f64,
    pub one_liner: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub recommend: Option<String>,
    pub finished_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReviewInput {
    pub subject_kind: String,
    pub subject_id: String,
    pub scores: Scores,
    #[serde(default)]
    pub adjust: f64,
    pub one_liner: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub recommend: Option<String>,
}

const STATUSES: &[&str] = &["playing", "finished", "completed", "dropped", "wishlist"];
const RECOMMENDS: &[&str] = &["yes", "maybe", "no"];

const COLUMNS: &str = "id, author, subject_kind, subject_id, scores, adjust, overall, one_liner,
    notes, status, recommend, finished_at, created_at, updated_at";

fn from_row(r: &Row) -> rusqlite::Result<Review> {
    let scores: String = r.get(4)?;
    Ok(Review {
        id: r.get(0)?,
        author: r.get(1)?,
        subject_kind: r.get(2)?,
        subject_id: r.get(3)?,
        scores: serde_json::from_str(&scores).unwrap_or_default(),
        adjust: r.get(5)?,
        overall: r.get(6)?,
        one_liner: r.get(7)?,
        notes: r.get(8)?,
        status: r.get(9)?,
        recommend: r.get(10)?,
        finished_at: r.get(11)?,
        created_at: r.get(12)?,
        updated_at: r.get(13)?,
    })
}

pub fn list(db: &Db, kind: &str) -> Result<Vec<Review>, String> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM reviews WHERE subject_kind = ?1 ORDER BY updated_at DESC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([kind], from_row)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

fn clean(s: Option<String>, max: usize) -> Option<String> {
    s.map(|s| s.trim().chars().take(max).collect::<String>())
        .filter(|s| !s.is_empty())
}

/// Aynı oyunu ikinci kez puanlamak öncekini günceller.
pub fn save(db: &Db, input: ReviewInput) -> Result<Review, String> {
    let auto = input.scores.auto().ok_or("En az bir soruya puan ver")?;
    let adjust = input.adjust.clamp(-1.0, 1.0);
    let overall = round1((auto + adjust).clamp(1.0, 10.0));
    let status = input.status.filter(|s| STATUSES.contains(&s.as_str()));
    let recommend = input.recommend.filter(|s| RECOMMENDS.contains(&s.as_str()));
    let one_liner = clean(input.one_liner, 200);
    let notes = clean(input.notes, 20_000);
    let scores = serde_json::to_string(&input.scores).map_err(|e| e.to_string())?;
    let now = now_ms();
    let conn = db.conn();
    let prev: Option<(String, Option<i64>)> = conn
        .query_row(
            "SELECT id, finished_at FROM reviews WHERE author = ?1 AND subject_kind = ?2 AND subject_id = ?3",
            params![ME, input.subject_kind, input.subject_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    // Bitirme tarihi: "Bitirdim" / "%100" ilk seçildiğinde.
    let done = matches!(status.as_deref(), Some("finished" | "completed"));
    let finished_at = match (&prev, done) {
        (Some((_, Some(t))), true) => Some(*t),
        (_, true) => Some(now),
        (_, false) => None,
    };
    let id = match prev {
        Some((id, _)) => {
            conn.execute(
                "UPDATE reviews SET scores = ?2, adjust = ?3, overall = ?4, one_liner = ?5, notes = ?6,
                    status = ?7, recommend = ?8, finished_at = ?9, updated_at = ?10 WHERE id = ?1",
                params![id, scores, adjust, overall, one_liner, notes, status, recommend, finished_at, now],
            )
            .map_err(|e| e.to_string())?;
            id
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO reviews (id, author, subject_kind, subject_id, scores, adjust, overall,
                    one_liner, notes, status, recommend, finished_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
                params![
                    id,
                    ME,
                    input.subject_kind,
                    input.subject_id,
                    scores,
                    adjust,
                    overall,
                    one_liner,
                    notes,
                    status,
                    recommend,
                    finished_at,
                    now
                ],
            )
            .map_err(|e| e.to_string())?;
            id
        }
    };
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM reviews WHERE id = ?1"),
        [&id],
        from_row,
    )
    .map_err(|e| e.to_string())
}

/// Puanı siler. Oyun yalnızca arşiv için eklenmişse (kütüphanede değilse) o da gider.
pub fn delete(db: &Db, id: &str) -> Result<(), String> {
    let conn = db.conn();
    let subject: Option<(String, String)> = conn
        .query_row(
            "SELECT subject_kind, subject_id FROM reviews WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM reviews WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    if let Some((kind, sid)) = subject {
        if kind == "game" {
            conn.execute("DELETE FROM games WHERE id = ?1 AND in_library = 0", [&sid])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: [Option<f64>; 5]) -> Scores {
        Scores {
            gameplay: v[0],
            story: v[1],
            enjoyment: v[2],
            visuals: v[3],
            performance: v[4],
        }
    }

    #[test]
    fn overall_is_weighted_and_none_is_redistributed() {
        // Tasarımdaki örnek: 9 · 8 · 10 · 9 · 9 → 9.0
        assert_eq!(
            s([Some(9.0), Some(8.0), Some(10.0), Some(9.0), Some(9.0)]).auto(),
            Some(9.0)
        );
        // Hepsi 7 → 7, hikâye "Yok" olsa da 7 (puanı düşürmez).
        assert_eq!(s([Some(7.0); 5]).auto(), Some(7.0));
        assert_eq!(
            s([Some(7.0), None, Some(7.0), Some(7.0), Some(7.0)]).auto(),
            Some(7.0)
        );
        // Hikâye yokken oynanışın etkisi büyür: 35/80.
        assert_eq!(
            s([Some(10.0), None, Some(6.0), Some(6.0), Some(6.0)]).auto(),
            Some(7.8)
        );
        assert_eq!(s([None; 5]).auto(), None);
    }

    #[test]
    fn reviews_round_trip() {
        let dir = std::env::temp_dir().join(format!("gmid-rv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(dir.join("gmid.db"));
        let db = Db::open(&dir.join("gmid.db")).unwrap();
        db.conn()
            .execute(
                "INSERT INTO games (id, name, source, source_id, in_library, added_at)
                 VALUES ('g1', 'Hades II', 'archive', 'x', 0, 0)",
                [],
            )
            .unwrap();
        let input = |status: &str, adjust: f64| ReviewInput {
            subject_kind: "game".into(),
            subject_id: "g1".into(),
            scores: s([Some(9.0), Some(8.0), Some(10.0), Some(9.0), Some(9.0)]),
            adjust,
            one_liner: Some("  Bir tur daha derken sabah oldu.  ".into()),
            notes: None,
            status: Some(status.into()),
            recommend: Some("yes".into()),
        };
        assert!(save(
            &db,
            ReviewInput {
                scores: Scores::default(),
                ..input("playing", 0.0)
            }
        )
        .is_err());
        let a = save(&db, input("playing", 5.0)).unwrap();
        assert_eq!(a.overall, 10.0);
        assert_eq!(a.adjust, 1.0);
        assert_eq!(
            a.one_liner.as_deref(),
            Some("Bir tur daha derken sabah oldu.")
        );
        assert_eq!(a.finished_at, None);
        let b = save(&db, input("finished", -0.5)).unwrap();
        assert_eq!(b.id, a.id);
        assert_eq!(b.overall, 8.5);
        assert!(b.finished_at.is_some());
        let c = save(&db, input("bogus", 0.0)).unwrap();
        assert_eq!(c.status, None);
        assert_eq!(list(&db, "game").unwrap().len(), 1);
        delete(&db, &a.id).unwrap();
        assert!(list(&db, "game").unwrap().is_empty());
        // Yalnızca arşivde olan oyun da gitti.
        let n: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM games", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }
}
