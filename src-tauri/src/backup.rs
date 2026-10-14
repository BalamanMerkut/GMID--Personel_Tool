//! Veritabanının otomatik yedekleri. Yalnız `gmid.db` kopyalanır (ayarlar, notlar, kütüphane listesi, geçmiş…);
//! videolar, müzikler, klipler gibi dosyalar yedeğe girmez. Kaç yedek tutulacağı ve ne sıklıkla alınacağı ayardır:
//! sınır dolunca en yeni yedek en eskisinin yerini alır.

use crate::db::Db;
use chrono::{DateTime, Local};
use serde::Serialize;
use std::path::{Path, PathBuf};

const PREFIX: &str = "gmid-";
/// Saklanacak yedek sayısı (1–10, varsayılan 3).
pub const KEEP_KEY: &str = "backup.keep";
/// Otomatik yedek sıklığı: "day" | "week" | "off" (varsayılan "day").
pub const EVERY_KEY: &str = "backup.every";
const DEFAULT_KEEP: usize = 3;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    pub keep: usize,
    pub every: String,
}

impl Policy {
    pub fn of(db: &Db) -> Policy {
        Policy::new(
            db.setting_json::<usize>(KEEP_KEY),
            db.setting_json::<String>(EVERY_KEY),
        )
    }
    fn new(keep: Option<usize>, every: Option<String>) -> Policy {
        let every = every
            .filter(|e| matches!(e.as_str(), "day" | "week" | "off"))
            .unwrap_or_else(|| "day".into());
        Policy {
            keep: keep.unwrap_or(DEFAULT_KEEP).clamp(1, 10),
            every,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub name: String,
    pub size: u64,
    /// Yerel saatle "2026-10-11 21:04"
    pub created: String,
}

/// Hemen bir yedek alır ve eskileri temizler.
pub fn backup_now(db: &Db, dir: &Path) -> Result<PathBuf, String> {
    let path = dir.join(format!(
        "{PREFIX}{}.db",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));
    db.conn()
        .execute("VACUUM INTO ?1", [path.to_string_lossy()])
        .map_err(|e| format!("Yedek alınamadı: {e}"))?;
    rotate(dir, Policy::of(db).keep);
    Ok(path)
}

/// Sırası geldiyse (günlükte bugün, haftalıkta son 7 günde yedek yoksa) bir yedek alır.
pub fn auto_if_needed(db: &Db, dir: &Path) -> Result<(), String> {
    let policy = Policy::of(db);
    let days = match policy.every.as_str() {
        "off" => return Ok(()),
        "week" => 7,
        _ => 1,
    };
    let newest = backup_files(dir).first().map(|p| file_name(p));
    if !due(newest.as_deref(), Local::now().date_naive(), days) {
        return Ok(());
    }
    backup_now(db, dir).map(|_| ())
}

/// En yeni yedeğin adındaki tarihten bu yana `days` gün geçti mi (ya da hiç yedek yok mu).
fn due(newest: Option<&str>, today: chrono::NaiveDate, days: i64) -> bool {
    let Some(date) = newest
        .and_then(|n| n.strip_prefix(PREFIX))
        .and_then(|n| n.get(..10))
        .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
    else {
        return true;
    };
    (today - date).num_days() >= days
}

/// Ayarı değiştirir ve fazla yedekleri hemen siler.
pub fn set_policy(db: &Db, dir: &Path, keep: usize, every: &str) -> Result<Policy, String> {
    let p = Policy::new(Some(keep), Some(every.to_string()));
    db.set_setting(KEEP_KEY, &p.keep.to_string())
        .and_then(|_| db.set_setting(EVERY_KEY, &format!("\"{}\"", p.every)))
        .map_err(|e| e.to_string())?;
    rotate(dir, p.keep);
    Ok(p)
}

/// Yedekler, en yenisi başta.
pub fn list(dir: &Path) -> Vec<BackupInfo> {
    backup_files(dir)
        .into_iter()
        .filter_map(|p| {
            let meta = std::fs::metadata(&p).ok()?;
            let created: DateTime<Local> = meta.modified().ok()?.into();
            Some(BackupInfo {
                name: file_name(&p),
                size: meta.len(),
                created: created.format("%Y-%m-%d %H:%M").to_string(),
            })
        })
        .collect()
}

fn rotate(dir: &Path, keep: usize) {
    for old in backup_files(dir).into_iter().skip(keep.max(1)) {
        let _ = std::fs::remove_file(old);
    }
}

/// `gmid-*.db` dosyaları; ad tarih içerdiği için ada göre ters sıra = en yeni başta.
fn backup_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    let n = file_name(p);
                    n.starts_with(PREFIX) && n.ends_with(".db")
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort_by_key(|p| std::cmp::Reverse(file_name(p)));
    files
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_newest() {
        let dir = std::env::temp_dir().join(format!("gmid-backup-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for d in 1..=9 {
            std::fs::write(dir.join(format!("gmid-2026-01-0{d}_10-00-00.db")), b"x").unwrap();
        }
        std::fs::write(dir.join("baska-dosya.txt"), b"x").unwrap();
        rotate(&dir, 7);
        let names: Vec<String> = list(&dir).into_iter().map(|b| b.name).collect();
        assert_eq!(names.len(), 7);
        assert_eq!(names[0], "gmid-2026-01-09_10-00-00.db");
        rotate(&dir, 3);
        let names: Vec<String> = list(&dir).into_iter().map(|b| b.name).collect();
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "gmid-2026-01-09_10-00-00.db");
        assert!(dir.join("baska-dosya.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn schedule_and_policy() {
        let d = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        assert!(due(None, d("2026-10-14"), 1));
        assert!(!due(
            Some("gmid-2026-10-14_00-10-00.db"),
            d("2026-10-14"),
            1
        ));
        assert!(due(Some("gmid-2026-10-13_23-59-00.db"), d("2026-10-14"), 1));
        assert!(!due(
            Some("gmid-2026-10-10_10-00-00.db"),
            d("2026-10-14"),
            7
        ));
        assert!(due(Some("gmid-2026-10-07_10-00-00.db"), d("2026-10-14"), 7));
        assert_eq!(
            Policy::new(None, None),
            Policy {
                keep: 3,
                every: "day".into()
            }
        );
        assert_eq!(Policy::new(Some(0), Some("x".into())).keep, 1);
        assert_eq!(
            Policy::new(Some(50), Some("week".into())),
            Policy {
                keep: 10,
                every: "week".into()
            }
        );
    }
}
