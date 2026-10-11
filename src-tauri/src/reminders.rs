//! Hatırlatıcılar ve tekrar kuralları.

use crate::db::Db;
use crate::util::{new_id, now_ms};
use chrono::{Datelike, Duration, Local, Months, NaiveDateTime, TimeZone, Weekday};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    None,
    Daily,
    Weekdays,
    Weekly,
    Monthly,
}

impl Repeat {
    fn as_str(self) -> &'static str {
        match self {
            Repeat::None => "none",
            Repeat::Daily => "daily",
            Repeat::Weekdays => "weekdays",
            Repeat::Weekly => "weekly",
            Repeat::Monthly => "monthly",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "daily" => Repeat::Daily,
            "weekdays" => Repeat::Weekdays,
            "weekly" => Repeat::Weekly,
            "monthly" => Repeat::Monthly,
            _ => Repeat::None,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub note: String,
    pub due_at: i64,
    pub repeat: Repeat,
    /// Zamanı geldi ve kullanıcıya gösterildi, henüz "Tamam" denmedi.
    pub notified: bool,
    pub done: bool,
    pub created_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderInput {
    pub id: Option<String>,
    pub title: String,
    pub note: String,
    pub due_at: i64,
    pub repeat: Repeat,
}

const COLUMNS: &str = "id, title, note, due_at, repeat, notified, done, created_at";

fn from_row(r: &Row) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: r.get(0)?,
        title: r.get(1)?,
        note: r.get(2)?,
        due_at: r.get(3)?,
        repeat: Repeat::parse(&r.get::<_, String>(4)?),
        notified: r.get(5)?,
        done: r.get(6)?,
        created_at: r.get(7)?,
    })
}

fn query(db: &Db, where_order: &str, p: impl rusqlite::Params) -> rusqlite::Result<Vec<Reminder>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM reminders {where_order}"))?;
    let rows = stmt.query_map(p, from_row)?;
    rows.collect()
}

/// Bekleyenler önce (zamanına göre), tamamlananlar sonra (yeniden eskiye), en fazla 50 tamamlanan.
pub fn list(db: &Db) -> rusqlite::Result<Vec<Reminder>> {
    let mut open = query(db, "WHERE done = 0 ORDER BY due_at", [])?;
    open.extend(query(
        db,
        "WHERE done = 1 ORDER BY due_at DESC LIMIT 50",
        [],
    )?);
    Ok(open)
}

pub fn get(db: &Db, id: &str) -> rusqlite::Result<Option<Reminder>> {
    db.conn()
        .query_row(
            &format!("SELECT {COLUMNS} FROM reminders WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()
}

pub fn save(db: &Db, input: ReminderInput) -> rusqlite::Result<Reminder> {
    let id = input.id.unwrap_or_else(new_id);
    db.conn().execute(
        "INSERT INTO reminders (id, title, note, due_at, anchor_at, repeat, notified, done, created_at)
         VALUES (?1, ?2, ?3, ?4, ?4, ?5, 0, 0, ?6)
         ON CONFLICT(id) DO UPDATE SET
            title = excluded.title, note = excluded.note, due_at = excluded.due_at,
            anchor_at = excluded.anchor_at, repeat = excluded.repeat, notified = 0, done = 0",
        params![
            id,
            input.title.trim(),
            input.note.trim(),
            input.due_at,
            input.repeat.as_str(),
            now_ms()
        ],
    )?;
    get(db, &id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn delete(db: &Db, id: &str) -> rusqlite::Result<()> {
    db.conn()
        .execute("DELETE FROM reminders WHERE id = ?1", [id])?;
    Ok(())
}

/// Zamanı gelmiş ama henüz gösterilmemiş hatırlatıcıları "gösterildi" yapar ve döndürür.
pub fn take_due(db: &Db, now: i64) -> rusqlite::Result<Vec<Reminder>> {
    let due = query(
        db,
        "WHERE done = 0 AND notified = 0 AND due_at <= ?1 ORDER BY due_at",
        [now],
    )?;
    if !due.is_empty() {
        db.conn().execute(
            "UPDATE reminders SET notified = 1 WHERE done = 0 AND notified = 0 AND due_at <= ?1",
            [now],
        )?;
    }
    Ok(due)
}

/// Uyarı penceresinde duran (gösterilmiş, "Tamam" denmemiş) hatırlatıcılar.
pub fn pending_alerts(db: &Db) -> rusqlite::Result<Vec<Reminder>> {
    query(db, "WHERE done = 0 AND notified = 1 ORDER BY due_at", [])
}

pub fn next_due(db: &Db) -> Option<i64> {
    db.conn()
        .query_row(
            "SELECT MIN(due_at) FROM reminders WHERE done = 0 AND notified = 0",
            [],
            |r| r.get(0),
        )
        .ok()
        .flatten()
}

/// "Tamam": tekrarlamayan biter; tekrarlayan asıl zamanından (ertelemeden değil) bir sonrakine kurulur.
pub fn complete(db: &Db, id: &str, now: i64) -> rusqlite::Result<()> {
    let Some(r) = get(db, id)? else { return Ok(()) };
    let anchor: i64 = db.conn().query_row(
        "SELECT anchor_at FROM reminders WHERE id = ?1",
        [id],
        |row| row.get(0),
    )?;
    match next_occurrence(anchor, r.repeat, now) {
        Some(next) => db.conn().execute(
            "UPDATE reminders SET due_at = ?2, anchor_at = ?2, notified = 0, done = 0 WHERE id = ?1",
            params![id, next],
        )?,
        None => db.conn().execute(
            "UPDATE reminders SET done = 1, notified = 0 WHERE id = ?1",
            [id],
        )?,
    };
    Ok(())
}

pub fn snooze(db: &Db, id: &str, minutes: i64, now: i64) -> rusqlite::Result<()> {
    db.conn().execute(
        "UPDATE reminders SET due_at = ?2, notified = 0, done = 0 WHERE id = ?1",
        params![id, now + minutes * 60_000],
    )?;
    Ok(())
}

fn local(ms: i64) -> Option<NaiveDateTime> {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.naive_local())
}

fn to_ms(t: NaiveDateTime) -> Option<i64> {
    Local
        .from_local_datetime(&t)
        .earliest()
        .map(|d| d.timestamp_millis())
}

/// Tekrar kuralına göre `now`dan sonraki ilk zaman. Saat (ör. 09:00) yaz/kış saatinden etkilenmez.
pub fn next_occurrence(due: i64, repeat: Repeat, now: i64) -> Option<i64> {
    let start = local(due)?;
    let mut k: u32 = 0;
    loop {
        k += 1;
        let t = match repeat {
            Repeat::None => return None,
            Repeat::Daily => start + Duration::days(k as i64),
            Repeat::Weekly => start + Duration::weeks(k as i64),
            // Ayın 31'i gibi günlerde her ay asıl güne göre hesaplanır (31 Oca → 28 Şub → 31 Mar).
            Repeat::Monthly => start.checked_add_months(Months::new(k))?,
            Repeat::Weekdays => {
                let mut t = start + Duration::days(k as i64);
                while matches!(t.weekday(), Weekday::Sat | Weekday::Sun) {
                    k += 1;
                    t = start + Duration::days(k as i64);
                }
                t
            }
        };
        let ms = to_ms(t)?;
        if ms > now {
            return Some(ms);
        }
        if k > 100_000 {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(y: i32, m: u32, d: u32, h: u32) -> i64 {
        to_ms(
            NaiveDate::from_ymd_opt(y, m, d)
                .unwrap()
                .and_hms_opt(h, 0, 0)
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn weekdays_skip_the_weekend() {
        // 9 Ekim 2026 Cuma 09:00 → 12 Ekim Pazartesi 09:00
        let fri = at(2026, 10, 9, 9);
        assert_eq!(
            next_occurrence(fri, Repeat::Weekdays, fri),
            Some(at(2026, 10, 12, 9))
        );
    }

    #[test]
    fn monthly_keeps_the_original_day() {
        let jan31 = at(2026, 1, 31, 9);
        assert_eq!(
            next_occurrence(jan31, Repeat::Monthly, jan31),
            Some(at(2026, 2, 28, 9))
        );
        assert_eq!(
            next_occurrence(jan31, Repeat::Monthly, at(2026, 3, 1, 0)),
            Some(at(2026, 3, 31, 9))
        );
    }

    #[test]
    fn daily_catches_up_after_missed_days() {
        let d = at(2026, 10, 1, 8);
        assert_eq!(
            next_occurrence(d, Repeat::Daily, at(2026, 10, 11, 10)),
            Some(at(2026, 10, 12, 8))
        );
        assert_eq!(next_occurrence(d, Repeat::None, d), None);
    }

    #[test]
    fn due_snooze_and_complete_flow() {
        let dir = std::env::temp_dir().join(format!("gmid-rem-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("t.db")).unwrap();
        let t0 = at(2026, 10, 11, 9);
        let r = save(
            &db,
            ReminderInput {
                id: None,
                title: "Çay".into(),
                note: String::new(),
                due_at: t0,
                repeat: Repeat::Daily,
            },
        )
        .unwrap();
        assert!(take_due(&db, t0 - 1).unwrap().is_empty());
        assert_eq!(take_due(&db, t0).unwrap().len(), 1);
        assert!(
            take_due(&db, t0).unwrap().is_empty(),
            "bir kez gösterilmeli"
        );
        assert_eq!(pending_alerts(&db).unwrap().len(), 1);
        snooze(&db, &r.id, 10, t0).unwrap();
        assert_eq!(next_due(&db), Some(t0 + 600_000));
        complete(&db, &r.id, t0).unwrap();
        let after = get(&db, &r.id).unwrap().unwrap();
        assert!(!after.done);
        assert_eq!(after.due_at, at(2026, 10, 12, 9));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
