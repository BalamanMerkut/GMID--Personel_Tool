//! Videolarım küçük resimlerinin onarımı. Küçük resim kaybolabiliyordu:
//! - aynı video iki kez indirildiyse (müzik + video) ikisi aynı dosyayı paylaşıyordu, biri silinince öteki boş kalıyordu;
//! - yerel kopya yazılamadıysa sitenin resim adresi tutuluyordu (bazı sitelerde adres süresi dolunca bozuluyor);
//! - bazı sitelerden (eklentiyle yakalanan akışlar) hiç resim gelmiyordu.
//!
//! Onarım: site adresi varsa resim indirilir; yoksa ya da indirilemezse resim dosyanın kendisinden çıkarılır
//! (videoda bir kare, müzikte gömülü kapak). Her indirmenin kendi dosyası olur (`<kimlik>.jpg`).

use super::{thumbs_dir, tools};
use crate::db::Db;
use crate::paths::DataDir;
use crate::util::hide_console;
use rusqlite::params;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tauri::{AppHandle, Manager};

struct Row {
    id: String,
    thumbnail: Option<String>,
    file_path: Option<String>,
    audio_only: bool,
    duration: Option<f64>,
}

fn is_remote(t: &str) -> bool {
    t.starts_with("http://") || t.starts_with("https://")
}

/// Yerelde duran, açılabilen bir küçük resmi var mı.
fn healthy(t: Option<&str>) -> bool {
    t.is_some_and(|t| !is_remote(t) && Path::new(t).is_file())
}

fn rows(db: &Db, only: Option<&str>) -> Vec<Row> {
    let conn = db.conn();
    let sql = "SELECT id, thumbnail, file_path, audio_only, duration FROM downloads
               WHERE status = 'done' AND (?1 IS NULL OR id = ?1)";
    let Ok(mut stmt) = conn.prepare(sql) else {
        return Vec::new();
    };
    stmt.query_map([only], |r| {
        Ok(Row {
            id: r.get(0)?,
            thumbnail: r.get(1)?,
            file_path: r.get(2)?,
            audio_only: r.get::<_, i64>(3)? != 0,
            duration: r.get(4)?,
        })
    })
    .map(|rows| rows.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    let mut r = crate::net::agent(Duration::from_secs(10), Duration::from_secs(30))
        .get(url)
        .call()
        .map_err(|e| e.to_string())?;
    let bytes = r
        .body_mut()
        .with_config()
        .limit(10 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    if bytes.len() < 1024 {
        return Err("resim çok küçük".into());
    }
    std::fs::write(dest, bytes).map_err(|e| e.to_string())
}

/// Videodan bir kare (başlardaki siyah ekranı atlamak için sürenin %10'u, en çok 30. sn) ya da
/// müzik dosyasının gömülü kapağı.
fn extract(ffmpeg: &Path, file: &Path, dest: &Path, audio: bool, duration: Option<f64>) -> bool {
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-loglevel", "error", "-y"]);
    if !audio {
        let at = duration.map_or(3.0, |d| (d * 0.1).min(30.0));
        cmd.args(["-ss", &format!("{at:.2}")]);
    }
    cmd.arg("-i").arg(file);
    if audio {
        cmd.args(["-an", "-map", "0:v:0"]);
    }
    cmd.args(["-frames:v", "1", "-vf", "scale=640:-2", "-q:v", "4"])
        .arg(dest)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_console(&mut cmd);
    cmd.status().is_ok_and(|s| s.success()) && dest.is_file()
}

fn ffmpeg(root: &Path) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    tools::ffmpeg_dir(root).map(|d| d.join(exe))
}

/// Küçük resmi eksik ya da bozuk olanları onarır (`only`: tek bir indirme). Onarılan sayısı.
pub fn repair(app: &AppHandle, only: Option<&str>) -> usize {
    let db = app.state::<Db>();
    let dir = app.state::<DataDir>();
    let out = thumbs_dir(&dir);
    let _ = std::fs::create_dir_all(&out);
    let ff = ffmpeg(dir.root());
    let mut fixed = 0;
    for r in rows(&db, only) {
        if healthy(r.thumbnail.as_deref()) {
            continue;
        }
        let dest = out.join(format!("{}.jpg", r.id));
        let mut ok = dest.is_file();
        if !ok {
            if let Some(url) = r.thumbnail.as_deref().filter(|t| is_remote(t)) {
                ok = download(url, &dest).is_ok();
            }
        }
        if !ok {
            if let (Some(ff), Some(file)) = (&ff, r.file_path.as_deref()) {
                if Path::new(file).is_file() {
                    ok = extract(ff, Path::new(file), &dest, r.audio_only, r.duration);
                }
            }
        }
        if ok {
            let _ = db.conn().execute(
                "UPDATE downloads SET thumbnail = ?2 WHERE id = ?1",
                params![r.id, dest.to_string_lossy()],
            );
            fixed += 1;
        }
    }
    if fixed > 0 {
        super::emit_changed(app);
    }
    fixed
}

/// Bu küçük resmi başka bir indirme de kullanıyor mu (aynı video iki kez indirildiyse).
pub fn shared(db: &Db, thumb: &str, except: &str) -> bool {
    db.conn()
        .query_row(
            "SELECT COUNT(*) FROM downloads WHERE thumbnail = ?1 AND id != ?2",
            params![thumb, except],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_and_extraction() {
        assert!(!healthy(None));
        assert!(!healthy(Some("https://i.ytimg.com/vi/x/hq.jpg")));
        assert!(!healthy(Some("/yok/boyle/bir/dosya.jpg")));
        let dir = std::env::temp_dir().join(format!("gmid-th-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.jpg");
        std::fs::write(&f, b"x").unwrap();
        assert!(healthy(Some(f.to_str().unwrap())));
        // Sistemde ffmpeg varsa gerçek bir videodan kare çıkar.
        let ff = Path::new("ffmpeg");
        let video = dir.join("v.mp4");
        let made = Command::new(ff)
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=320x240:rate=10:duration=4",
            ])
            .arg(&video)
            .status()
            .is_ok_and(|s| s.success());
        if made {
            let dest = dir.join("t.jpg");
            let _ = std::fs::remove_file(&dest);
            assert!(extract(ff, &video, &dest, false, Some(4.0)));
            let dest2 = dir.join("t2.jpg");
            assert!(!extract(
                ff,
                &video.with_extension("yok"),
                &dest2,
                false,
                None
            ));
        }
    }
}
