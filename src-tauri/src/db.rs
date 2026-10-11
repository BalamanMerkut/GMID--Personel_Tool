//! Tek dosyalık SQLite veritabanı (`gmid.db`) ve sürüm sürüm ilerleyen şema.

use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

/// Her eleman bir şema sürümü. Sıra asla değişmez; yeni sürüm sona eklenir.
const MIGRATIONS: &[&str] = &[
    // 1 — ayarlar
    "CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );",
];

pub struct Db(Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        migrate(&conn)?;
        Ok(Self(Mutex::new(conn)))
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        // Bir iş parçacığı panik yaparsa bile veritabanı kullanılabilir kalsın.
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn all_settings(&self) -> rusqlite::Result<HashMap<String, String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect()
    }

    pub fn setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current.max(0) as usize) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_migrations_are_idempotent() {
        let dir = std::env::temp_dir().join(format!("gmid-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path).unwrap();
        assert_eq!(db.setting("ui.splash").unwrap(), None);
        db.set_setting("ui.splash", "false").unwrap();
        db.set_setting("ui.splash", "true").unwrap();
        assert_eq!(db.setting("ui.splash").unwrap().as_deref(), Some("true"));
        drop(db);

        // İkinci açılışta göçler tekrar çalışmamalı, veri durmalı.
        let db = Db::open(&path).unwrap();
        assert_eq!(db.all_settings().unwrap().len(), 1);
        let v: i64 = db
            .conn()
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
