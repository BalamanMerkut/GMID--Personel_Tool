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

// ---- not resimleri ----
//
// Yapıştırılan / eklenen resimler `Notlar\Resimler` klasörüne kopyalanır; notta Markdown olarak
// `![](Resimler/ad.png)` durur. Not dosyaları da `Notlar` klasöründe olduğundan başka bir Markdown
// programında açınca resimler yine görünür.

pub const IMAGES: &str = "Resimler";
const IMAGE_EXTS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];
const MAX_IMAGE: usize = 25 * 1024 * 1024;

/// Dosyanın baştaki baytlarından resim türü (uzantıya güvenilmez).
fn image_kind(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF8") {
        Some("gif")
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// Resmi kaydeder, notta kullanılacak göreli yolu döner ("Resimler/20261014-213005-ab12.png").
pub fn save_image(notes_dir: &Path, bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > MAX_IMAGE {
        return Err("Resim çok büyük (en fazla 25 MB).".into());
    }
    let ext =
        image_kind(bytes).ok_or("Bu dosya bir resim değil (PNG, JPG, GIF ya da WebP olmalı).")?;
    let dir = notes_dir.join(IMAGES);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!(
        "{}-{}.{ext}",
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        crate::util::new_id()
            .chars()
            .rev()
            .take(6)
            .collect::<String>()
    );
    std::fs::write(dir.join(&name), bytes).map_err(|e| format!("Resim kaydedilemedi: {e}"))?;
    Ok(format!("{IMAGES}/{name}"))
}

/// Bilgisayardaki bir resmi nota ekler (kopyalanır; orijinali silinse de notta kalır).
pub fn import_image(notes_dir: &Path, source: &Path) -> Result<String, String> {
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !IMAGE_EXTS.contains(&ext.as_str()) {
        return Err("Resim seç: PNG, JPG, GIF ya da WebP.".into());
    }
    let bytes = std::fs::read(source).map_err(|e| format!("Dosya okunamadı: {e}"))?;
    save_image(notes_dir, &bytes)
}

/// Hiçbir notta (çöp kutusundakiler dahil) geçmeyen, bir günden eski resimler silinir.
pub fn cleanup_images(db: &Db, notes_dir: &Path) -> usize {
    let Ok(rd) = std::fs::read_dir(notes_dir.join(IMAGES)) else {
        return 0;
    };
    let bodies: String = db
        .conn()
        .prepare("SELECT body FROM notes")
        .and_then(|mut st| {
            st.query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .map(|v| v.join("\n"))
        .unwrap_or_default();
    // Notlar okunamadıysa hiçbir şeye dokunma.
    if bodies.is_empty()
        && db
            .conn()
            .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get::<_, i64>(0))
            .unwrap_or(1)
            > 0
    {
        return 0;
    }
    let mut n = 0;
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|a| a.as_secs() > 24 * 3600);
        let ours = name
            .rsplit_once('.')
            .is_some_and(|(_, x)| IMAGE_EXTS.contains(&x.to_lowercase().as_str()));
        if ours && old && !bodies.contains(&name) && std::fs::remove_file(e.path()).is_ok() {
            n += 1;
        }
    }
    n
}

/// Listede görünen kısa önizleme: Markdown işaretleri ayıklanmış ilk satırlar.
pub fn preview(body: &str) -> String {
    let text: String = strip_images(body)
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

/// `![…](…)` resimleri önizlemede "🖼" olur.
fn strip_images(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(i) = rest.find("![") {
        let after = &rest[i..];
        match after
            .find("](")
            .and_then(|j| after[j..].find(')').map(|k| j + k))
        {
            Some(end) => {
                out.push_str(&rest[..i]);
                out.push('🖼');
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
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
        crate::log!("Not dosyası yazılamadı: {e}");
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
    fn images_are_checked_and_saved() {
        let dir = std::env::temp_dir().join(format!("gmid-noteimg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let png = [0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0];
        let rel = save_image(&dir, &png).unwrap();
        assert!(rel.starts_with("Resimler/") && rel.ends_with(".png"));
        assert!(dir.join(&rel).is_file());
        assert!(save_image(&dir, b"MZ not an image").is_err());
        assert_eq!(preview("Bak: ![](Resimler/a.png) güzel"), "Bak: 🖼 güzel");
        let _ = std::fs::remove_dir_all(&dir);
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
