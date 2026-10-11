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
    // 2 — notlar
    "CREATE TABLE notes (
        id         TEXT PRIMARY KEY,
        title      TEXT NOT NULL DEFAULT '',
        body       TEXT NOT NULL DEFAULT '',
        color      TEXT NOT NULL DEFAULT 'default',
        font       TEXT NOT NULL DEFAULT 'inter',
        pinned     INTEGER NOT NULL DEFAULT 0,
        file_name  TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        deleted_at INTEGER
    );
    CREATE INDEX notes_updated ON notes (updated_at);",
    // 3 — hatırlatıcılar
    "CREATE TABLE reminders (
        id         TEXT PRIMARY KEY,
        title      TEXT NOT NULL,
        note       TEXT NOT NULL DEFAULT '',
        due_at     INTEGER NOT NULL,
        -- Tekrarın asıl zamanı; erteleme sadece due_at'i değiştirir.
        anchor_at  INTEGER NOT NULL,
        repeat     TEXT NOT NULL DEFAULT 'none',
        notified   INTEGER NOT NULL DEFAULT 0,
        done       INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL
    );
    CREATE INDEX reminders_due ON reminders (done, notified, due_at);",
    // 4 — oyun sunucuları
    "CREATE TABLE servers (
        id            TEXT PRIMARY KEY,
        name          TEXT NOT NULL,
        game          TEXT NOT NULL,
        host          TEXT NOT NULL,
        port          INTEGER NOT NULL,
        query_port    INTEGER,
        logo          TEXT,
        -- Topluluk listesindeki logo adresi; değişmediyse logo tekrar indirilmez.
        logo_url      TEXT,
        description   TEXT NOT NULL DEFAULT '',
        connect_notes TEXT NOT NULL DEFAULT '',
        remote        INTEGER NOT NULL DEFAULT 0,
        sort          INTEGER NOT NULL DEFAULT 0,
        created_at    INTEGER NOT NULL,
        updated_at    INTEGER NOT NULL
    );",
    // 5 — video indirmeleri (Videolarım)
    "CREATE TABLE downloads (
        id            TEXT PRIMARY KEY,
        url           TEXT NOT NULL,
        video_id      TEXT NOT NULL DEFAULT '',
        title         TEXT NOT NULL DEFAULT '',
        uploader      TEXT NOT NULL DEFAULT '',
        site          TEXT NOT NULL DEFAULT '',
        thumbnail     TEXT,
        duration      REAL,
        quality       TEXT NOT NULL,
        -- yt-dlp biçim seçimi ve parça sayısı (yeniden başlatınca tekrar çözümlemeye gerek kalmasın)
        option_format TEXT NOT NULL,
        parts         INTEGER NOT NULL DEFAULT 1,
        audio_only    INTEGER NOT NULL DEFAULT 0,
        status        TEXT NOT NULL,
        progress      REAL NOT NULL DEFAULT 0,
        size          INTEGER,
        file_path     TEXT,
        error         TEXT,
        source        TEXT NOT NULL DEFAULT 'app',
        created_at    INTEGER NOT NULL,
        finished_at   INTEGER
    );
    CREATE INDEX downloads_status ON downloads (status, created_at);",
    // 6 — çalma listeleri ve eklentinin yakaladığı videolar
    "ALTER TABLE downloads ADD COLUMN playlist TEXT;
    ALTER TABLE downloads ADD COLUMN playlist_index INTEGER;
    -- Videolar\\<Site>\\<alt klasör>: çalma listesinin adı
    ALTER TABLE downloads ADD COLUMN subdir TEXT;
    -- Eklentiden: sayfa adresi, tarayıcı kimliği, sayfa başlığı (JSON)
    ALTER TABLE downloads ADD COLUMN context TEXT;",
    // 7 — videoların yazıları (altyazı / Whisper)
    "CREATE TABLE transcripts (
        download_id TEXT PRIMARY KEY,
        source      TEXT NOT NULL,
        language    TEXT NOT NULL DEFAULT '',
        -- [{start, end, text, words?}] (saniye)
        segments    TEXT NOT NULL,
        created_at  INTEGER NOT NULL
    );",
    // 8 — kullanıcının kendi çalma listeleri (Medya Oynatıcı)
    "CREATE TABLE playlists (
        id         TEXT PRIMARY KEY,
        name       TEXT NOT NULL,
        created_at INTEGER NOT NULL
    );
    CREATE TABLE playlist_items (
        playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
        download_id TEXT NOT NULL REFERENCES downloads(id) ON DELETE CASCADE,
        position    INTEGER NOT NULL,
        PRIMARY KEY (playlist_id, download_id)
    );",
    // 9 — oyun kütüphanesi ve oynama oturumları (Faz 3b)
    "CREATE TABLE games (
        id                   TEXT PRIMARY KEY,
        name                 TEXT NOT NULL,
        -- steam | epic | manual
        source               TEXT NOT NULL,
        -- Steam appid, Epic AppName ya da (elle) exe yolu
        source_id            TEXT NOT NULL,
        install_dir          TEXT,
        -- küçük harfli exe adları (JSON)
        exes                 TEXT NOT NULL DEFAULT '[]',
        -- başlatma: steam://…, com.epicgames.launcher://… ya da exe yolu
        launch               TEXT,
        -- bilgi ve kapak için Steam mağaza kimliği (Epic/elle eklenende de bulunabilir)
        steam_appid          INTEGER,
        cover                TEXT,
        header               TEXT,
        developer            TEXT,
        publisher            TEXT,
        year                 INTEGER,
        description          TEXT,
        genres               TEXT NOT NULL DEFAULT '[]',
        categories           TEXT NOT NULL DEFAULT '[]',
        -- platformun kendi tuttuğu süre (Steam, dakika) ve son oynama
        platform_minutes     INTEGER NOT NULL DEFAULT 0,
        platform_last_played INTEGER,
        hidden               INTEGER NOT NULL DEFAULT 0,
        meta_at              INTEGER,
        added_at             INTEGER NOT NULL,
        UNIQUE (source, source_id)
    );
    CREATE TABLE play_sessions (
        id         TEXT PRIMARY KEY,
        game_id    TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
        started_at INTEGER NOT NULL,
        ended_at   INTEGER,
        seconds    INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX play_sessions_game ON play_sessions (game_id, started_at);
    CREATE INDEX play_sessions_time ON play_sessions (started_at);",
    // 10 — Oyun Arşivi: Türkçe etiketler, arşivde olup kütüphanede olmayan oyunlar, puanlar (Faz 3b)
    "ALTER TABLE games ADD COLUMN tags TEXT NOT NULL DEFAULT '[]';
    ALTER TABLE games ADD COLUMN in_library INTEGER NOT NULL DEFAULT 1;
    -- etiketler için bilgiler bir kez yeniden alınır
    UPDATE games SET meta_at = NULL;
    CREATE TABLE reviews (
        id           TEXT PRIMARY KEY,
        -- şimdilik hep 'me'; ortak arşivde arkadaşlar
        author       TEXT NOT NULL DEFAULT 'me',
        -- game (ileride film/dizi)
        subject_kind TEXT NOT NULL,
        subject_id   TEXT NOT NULL,
        -- soru → puan ya da null ('Yok') (JSON)
        scores       TEXT NOT NULL,
        adjust       REAL NOT NULL DEFAULT 0,
        overall      REAL NOT NULL,
        one_liner    TEXT,
        notes        TEXT,
        -- playing | finished | completed | dropped | wishlist
        status       TEXT,
        -- yes | maybe | no
        recommend    TEXT,
        finished_at  INTEGER,
        created_at   INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL,
        UNIQUE (author, subject_kind, subject_id)
    );",
    // 11 — Kliplerim: kayıt programlarının klip ve ekran görüntüleri (dosyalar yerinde kalır)
    "CREATE TABLE captures (
        id          TEXT PRIMARY KEY,
        path        TEXT NOT NULL UNIQUE,
        -- clip | shot
        kind        TEXT NOT NULL,
        -- nvidia | xbox | amd | obs | steam | medal | folder
        source      TEXT NOT NULL,
        game_id     TEXT REFERENCES games(id) ON DELETE SET NULL,
        -- klasör ya da dosya adındaki oyun adı
        game_hint   TEXT,
        steam_appid INTEGER,
        -- elle verilen ad (dosyanın kendisi yeniden adlandırılmaz)
        title       TEXT,
        size        INTEGER NOT NULL,
        duration    REAL,
        width       INTEGER,
        height      INTEGER,
        thumb       TEXT,
        thumb_at    INTEGER,
        favorite    INTEGER NOT NULL DEFAULT 0,
        -- oyun elle seçildiyse kendiliğinden değiştirilmez
        manual_game INTEGER NOT NULL DEFAULT 0,
        created_at  INTEGER NOT NULL,
        added_at    INTEGER NOT NULL
    );
    CREATE INDEX captures_time ON captures (created_at);",
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

    /// JSON olarak saklanan bir ayarı okur; yoksa ya da bozuksa `None`.
    pub fn setting_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.setting(key)
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str(&v).ok())
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
        assert_eq!(db.setting_json::<bool>("ui.splash"), Some(true));
        let v: i64 = db
            .conn()
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
