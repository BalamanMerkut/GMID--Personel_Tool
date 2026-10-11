//! Kullanıcının kendi çalma listeleri: Medya Oynatıcı'da "+" ile oluşturulur, şarkılar seçilir.
//! Şarkı silinince listeden de düşer (veritabanı bağlantısı).

use crate::db::Db;
use crate::util::{new_id, now_ms};
use rusqlite::params;
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    /// Sıralı indirme kimlikleri.
    pub items: Vec<String>,
}

fn clean_name(name: &str) -> Result<String, String> {
    let n: String = name.trim().chars().take(80).collect();
    if n.is_empty() {
        Err("Listeye bir ad ver".into())
    } else {
        Ok(n)
    }
}

pub fn list(db: &Db) -> Result<Vec<Playlist>, String> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare("SELECT id, name, created_at FROM playlists ORDER BY created_at")
        .map_err(|e| e.to_string())?;
    let mut lists: Vec<Playlist> = stmt
        .query_map([], |r| {
            Ok(Playlist {
                id: r.get(0)?,
                name: r.get(1)?,
                created_at: r.get(2)?,
                items: Vec::new(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    let mut items = conn
        .prepare("SELECT download_id FROM playlist_items WHERE playlist_id = ?1 ORDER BY position")
        .map_err(|e| e.to_string())?;
    for p in &mut lists {
        p.items = items
            .query_map([&p.id], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
    }
    Ok(lists)
}

fn set_items(db: &Db, id: &str, items: &[String]) -> Result<(), String> {
    let conn = db.conn();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM playlist_items WHERE playlist_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    let mut seen = std::collections::HashSet::new();
    for (i, d) in items.iter().filter(|d| seen.insert(d.as_str())).enumerate() {
        // Silinmiş bir kayıt seçiliyse sessizce atlanır.
        let _ = tx.execute(
            "INSERT INTO playlist_items (playlist_id, download_id, position)
             SELECT ?1, ?2, ?3 WHERE EXISTS (SELECT 1 FROM downloads WHERE id = ?2)",
            params![id, d, i as i64],
        );
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn create(db: &Db, name: &str, items: &[String]) -> Result<Playlist, String> {
    let name = clean_name(name)?;
    let id = new_id();
    db.conn()
        .execute(
            "INSERT INTO playlists (id, name, created_at) VALUES (?1, ?2, ?3)",
            params![id, name, now_ms()],
        )
        .map_err(|e| e.to_string())?;
    set_items(db, &id, items)?;
    get(db, &id)
}

pub fn update(db: &Db, id: &str, name: &str, items: &[String]) -> Result<Playlist, String> {
    let name = clean_name(name)?;
    let changed = db
        .conn()
        .execute(
            "UPDATE playlists SET name = ?2 WHERE id = ?1",
            params![id, name],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Liste bulunamadı".into());
    }
    set_items(db, id, items)?;
    get(db, id)
}

pub fn delete(db: &Db, id: &str) -> Result<(), String> {
    db.conn()
        .execute("DELETE FROM playlists WHERE id = ?1", [id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn get(db: &Db, id: &str) -> Result<Playlist, String> {
    list(db)?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "Liste bulunamadı".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_download(db: &Db, id: &str) {
        db.conn()
            .execute(
                "INSERT INTO downloads (id, url, video_id, title, uploader, site, quality, option_format,
                    parts, audio_only, status, progress, source, created_at)
                 VALUES (?1, 'u', 'v', 't', 'a', 's', 'MP3', 'ba', 1, 1, 'done', 1, 'app', 0)",
                [id],
            )
            .unwrap();
    }

    #[test]
    fn playlists_round_trip() {
        let dir = std::env::temp_dir().join(format!("gmid-pl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("gmid.db")).unwrap();
        for id in ["a", "b", "c"] {
            insert_download(&db, id);
        }
        assert!(create(&db, "  ", &[]).is_err());
        let p = create(
            &db,
            "Yol şarkıları",
            &["b".into(), "a".into(), "b".into(), "yok".into()],
        )
        .unwrap();
        assert_eq!(
            p.items,
            ["b", "a"],
            "sıra korunur, tekrar ve olmayan atlanır"
        );
        let p = update(&db, &p.id, "Gece", &["c".into(), "a".into()]).unwrap();
        assert_eq!(
            (p.name.as_str(), p.items.clone()),
            ("Gece", vec!["c".to_string(), "a".to_string()])
        );
        // Şarkı silinince listeden düşer.
        db.conn()
            .execute("DELETE FROM downloads WHERE id = 'c'", [])
            .unwrap();
        assert_eq!(list(&db).unwrap()[0].items, ["a"]);
        delete(&db, &p.id).unwrap();
        assert!(list(&db).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
