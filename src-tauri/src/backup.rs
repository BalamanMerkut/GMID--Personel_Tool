//! Veritabanının otomatik yedekleri: günde bir kez, son 7 yedek saklanır.

use crate::db::Db;
use chrono::{DateTime, Local};
use serde::Serialize;
use std::path::{Path, PathBuf};

const KEEP: usize = 7;
const PREFIX: &str = "gmid-";

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
    rotate(dir);
    Ok(path)
}

/// Bugün için yedek yoksa bir tane alır.
pub fn daily_if_needed(db: &Db, dir: &Path) -> Result<(), String> {
    let today = format!("{PREFIX}{}", Local::now().format("%Y-%m-%d"));
    if backup_files(dir)
        .iter()
        .any(|p| file_name(p).starts_with(&today))
    {
        return Ok(());
    }
    backup_now(db, dir).map(|_| ())
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

fn rotate(dir: &Path) {
    for old in backup_files(dir).into_iter().skip(KEEP) {
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
    fn keeps_only_the_newest_seven() {
        let dir = std::env::temp_dir().join(format!("gmid-backup-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for d in 1..=9 {
            std::fs::write(dir.join(format!("gmid-2026-01-0{d}_10-00-00.db")), b"x").unwrap();
        }
        std::fs::write(dir.join("baska-dosya.txt"), b"x").unwrap();
        rotate(&dir);
        let names: Vec<String> = list(&dir).into_iter().map(|b| b.name).collect();
        assert_eq!(names.len(), 7);
        assert_eq!(names[0], "gmid-2026-01-09_10-00-00.db");
        assert!(dir.join("baska-dosya.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
