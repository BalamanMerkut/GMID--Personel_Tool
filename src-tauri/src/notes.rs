//! Notlar. Asıl kayıt veritabanında; her notun bir de `Notlar\` klasöründe düz `.md` kopyası
//! tutulur, böylece uygulama olmadan da Not Defteri'yle okunabilir.

use crate::db::Db;
use crate::util::{new_id, now_ms};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Çöp kutusundaki notlar bu süreden sonra kalıcı olarak silinir.
pub const TRASH_DAYS: i64 = 30;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    pub color: String,
    pub font: String,
    pub pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub color: String,
    pub pinned: bool,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteInput {
    pub id: Option<String>,
    pub title: String,
    pub body: String,
    pub color: String,
    pub font: String,
    pub pinned: bool,
}

const COLUMNS: &str = "id, title, body, color, font, pinned, created_at, updated_at, deleted_at";

fn from_row(r: &Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: r.get(0)?,
        title: r.get(1)?,
        body: r.get(2)?,
        color: r.get(3)?,
        font: r.get(4)?,
        pinned: r.get(5)?,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
        deleted_at: r.get(8)?,
    })
}

pub fn list(db: &Db, trash: bool) -> rusqlite::Result<Vec<NoteSummary>> {
    let conn = db.conn();
    let filter = if trash { "IS NOT NULL" } else { "IS NULL" };
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM notes WHERE deleted_at {filter} ORDER BY pinned DESC, updated_at DESC"
    ))?;
    let rows = stmt.query_map([], from_row)?;
    rows.map(|n| {
        n.map(|n| NoteSummary {
            preview: preview(&n.body),
            id: n.id,
            title: n.title,
            color: n.color,
            pinned: n.pinned,
            updated_at: n.updated_at,
            deleted_at: n.deleted_at,
        })
    })
    .collect()
}

pub fn get(db: &Db, id: &str) -> rusqlite::Result<Option<Note>> {
    db.conn()
        .query_row(
            &format!("SELECT {COLUMNS} FROM notes WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()
}

pub fn save(db: &Db, dir: &Path, input: NoteInput) -> rusqlite::Result<Note> {
    let now = now_ms();
    let id = input.id.unwrap_or_else(new_id);
    db.conn().execute(
        "INSERT INTO notes (id, title, body, color, font, pinned, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
         ON CONFLICT(id) DO UPDATE SET
            title = excluded.title, body = excluded.body, color = excluded.color,
            font = excluded.font, pinned = excluded.pinned, updated_at = excluded.updated_at",
        params![
            id,
            input.title,
            input.body,
            input.color,
            input.font,
            input.pinned,
            now
        ],
    )?;
    let note = get(db, &id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    if note.deleted_at.is_none() {
        write_mirror(db, dir, &note);
    }
    Ok(note)
}

pub fn trash(db: &Db, dir: &Path, id: &str) -> rusqlite::Result<()> {
    remove_mirror(db, dir, id);
    db.conn().execute(
        "UPDATE notes SET deleted_at = ?2 WHERE id = ?1",
        params![id, now_ms()],
    )?;
    Ok(())
}

pub fn restore(db: &Db, dir: &Path, id: &str) -> rusqlite::Result<()> {
    db.conn()
        .execute("UPDATE notes SET deleted_at = NULL WHERE id = ?1", [id])?;
    if let Some(note) = get(db, id)? {
        write_mirror(db, dir, &note);
    }
    Ok(())
}

pub fn delete(db: &Db, dir: &Path, id: &str) -> rusqlite::Result<()> {
    remove_mirror(db, dir, id);
    db.conn().execute("DELETE FROM notes WHERE id = ?1", [id])?;
    Ok(())
}

/// Çöpte 30 günden uzun duran notları kalıcı siler.
pub fn purge_trash(db: &Db) -> rusqlite::Result<usize> {
    let limit = now_ms() - TRASH_DAYS * 24 * 60 * 60 * 1000;
    db.conn().execute(
        "DELETE FROM notes WHERE deleted_at IS NOT NULL AND deleted_at < ?1",
        [limit],
    )
}

/// Listede görünen kısa önizleme: Markdown işaretleri ayıklanmış ilk satırlar.
pub fn preview(body: &str) -> String {
    let text: String = body
        .lines()
        .map(|l| {
            l.trim()
                .trim_start_matches(['#', '>', '-', '*', '+'])
                .trim_start()
                .trim_start_matches("[ ]")
                .trim_start_matches("[x]")
                .trim()
                .replace(['*', '_', '`'], "")
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    text.chars().take(140).collect()
}

/// Windows'ta dosya adında kullanılamayan karakterleri ayıklar.
pub fn slug(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    let joined = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let short: String = joined.chars().take(60).collect();
    let trimmed = short.trim_end_matches(['.', ' ']).to_string();
    if trimmed.is_empty() {
        "Başlıksız not".into()
    } else {
        trimmed
    }
}

fn write_mirror(db: &Db, dir: &Path, note: &Note) {
    let base = slug(&note.title);
    let taken = |name: &str| -> bool {
        db.conn()
            .query_row(
                "SELECT 1 FROM notes WHERE file_name = ?1 AND id <> ?2",
                params![name, note.id],
                |_| Ok(()),
            )
            .optional()
            .ok()
            .flatten()
            .is_some()
    };
    let mut name = format!("{base}.md");
    if taken(&name) {
        name = format!(
            "{base} ({}).md",
            &note.id[note.id.len().saturating_sub(4)..]
        );
    }
    let content = if note.title.trim().is_empty() {
        format!("{}\n", note.body.trim_end())
    } else {
        format!("# {}\n\n{}\n", note.title.trim(), note.body.trim_end())
    };
    if let Err(e) = std::fs::write(dir.join(&name), content) {
        eprintln!("Not dosyası yazılamadı: {e}");
        return;
    }
    let old: Option<String> = db
        .conn()
        .query_row(
            "SELECT file_name FROM notes WHERE id = ?1",
            [&note.id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    if let Some(old) = old.filter(|o| *o != name) {
        let _ = std::fs::remove_file(dir.join(old));
    }
    let _ = db.conn().execute(
        "UPDATE notes SET file_name = ?2 WHERE id = ?1",
        params![note.id, name],
    );
}

fn remove_mirror(db: &Db, dir: &Path, id: &str) {
    let name: Option<String> = db
        .conn()
        .query_row("SELECT file_name FROM notes WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .ok()
        .flatten();
    if let Some(name) = name {
        let _ = std::fs::remove_file(dir.join(name));
        let _ = db
            .conn()
            .execute("UPDATE notes SET file_name = NULL WHERE id = ?1", [id]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_removes_forbidden_characters() {
        assert_eq!(slug("Hades II: Nyx build?"), "Hades II Nyx build");
        assert_eq!(slug("   "), "Başlıksız not");
        assert_eq!(slug("a/b\\c"), "a b c");
        assert_eq!(slug("son nokta..."), "son nokta");
    }

    #[test]
    fn preview_strips_markdown() {
        let body = "# Başlık\n\n- [ ] çay al\n- [x] **ekmek**\n";
        assert_eq!(preview(body), "Başlık · çay al · ekmek");
    }

    #[test]
    fn save_writes_and_renames_mirror_file() {
        let dir = std::env::temp_dir().join(format!("gmid-notes-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("t.db")).unwrap();
        let input = |id: Option<String>, title: &str| NoteInput {
            id,
            title: title.into(),
            body: "- [ ] dene".into(),
            color: "default".into(),
            font: "inter".into(),
            pinned: false,
        };
        let n = save(&db, &dir, input(None, "Market")).unwrap();
        assert!(dir.join("Market.md").exists());
        save(&db, &dir, input(Some(n.id.clone()), "Alışveriş")).unwrap();
        assert!(!dir.join("Market.md").exists());
        assert!(dir.join("Alışveriş.md").exists());
        // Aynı başlıklı ikinci not kendi dosyasını almalı.
        let m = save(&db, &dir, input(None, "Alışveriş")).unwrap();
        assert_ne!(m.id, n.id);
        assert_eq!(
            std::fs::read_dir(&dir)
                .unwrap()
                .filter(|e| e
                    .as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|x| x == "md"))
                .count(),
            2
        );
        trash(&db, &dir, &n.id).unwrap();
        assert!(!dir.join("Alışveriş.md").exists());
        assert_eq!(list(&db, true).unwrap().len(), 1);
        restore(&db, &dir, &n.id).unwrap();
        assert_eq!(list(&db, false).unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
