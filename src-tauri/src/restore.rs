//! GMID'in tam yedeği (`GMID-Yedek.zip`) ve geri yüklenmesi.
//!
//! Yedek: veritabanı (ayarlar, notlar, hatırlatıcılar, kütüphane, oturumlar, puanlar…), notların .md
//! kopyaları, arka planlar, kapaklar, sunucu logoları ve eklenti; isteğe bağlı indirilen videolar ve
//! araçlar/modeller. Format Kiti'ndeki betik bu zip'i `Belgeler\GMID Tool` içine açar; uygulama içinden
//! geri yüklemede önce bir ara klasöre açılır, uygulama yeniden başlarken veritabanı kapalıyken yerine konur.
//!
//! Veritabanı dosya yollarını tam yol olarak tutar (ör. `C:\Users\Ali\Documents\GMID Tool\Videolar\…`).
//! Kullanıcı adı ya da Belgeler klasörü formattan sonra değişirse ilk açılışta yollar yeni yere çevrilir.

use crate::db::Db;
use crate::util::now_ms;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub const MANIFEST: &str = "gmid-yedek.json";
pub const DB_NAME: &str = "gmid.db";
/// Uygulama içinden geri yüklemede zip'in açıldığı ara klasör.
const STAGING: &str = ".geri-yukleme";
/// Kullanıcı "Geri yükle"yi onaylayınca ara klasöre konan işaret; yoksa yarım kalmış hazırlık silinir.
const CONFIRMED: &str = ".onaylandi";
/// Her zaman yedeklenen klasörler.
const FOLDERS: &[&str] = &["Notlar", "Arkaplanlar", "Kapaklar", "Sunucular", "Eklenti"];
const VIDEOS: &str = "Videolar";
const TOOLS: &str = "Araclar";
/// Zaten sıkıştırılmış dosyalar sıkıştırılmadan konur (hızlı, aynı boyut).
const STORED: &[&str] = &[
    "mp4", "mkv", "webm", "mov", "m4a", "mp3", "opus", "ogg", "flac", "jpg", "jpeg", "png", "webp",
    "gif", "zip", "7z", "exe", "dll", "bin", "gguf",
];

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Manifest {
    pub app: String,
    pub version: String,
    pub created: i64,
    /// Yedeğin alındığı veri klasörü ve kullanıcı klasörü (yollar yeni yere çevrilsin diye).
    pub root: String,
    pub home: String,
    pub folders: Vec<String>,
}

#[derive(Deserialize, Clone, Copy, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    pub videos: bool,
    pub tools: bool,
}

/// Yedeğe girecek klasörler.
pub fn folders(opts: Options) -> Vec<&'static str> {
    let mut v = FOLDERS.to_vec();
    if opts.videos {
        v.push(VIDEOS);
    }
    if opts.tools {
        v.push(TOOLS);
    }
    v
}

fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        match e.file_type() {
            Ok(t) if t.is_dir() => files_under(&p, out),
            Ok(t) if t.is_file() => out.push(p),
            _ => {}
        }
    }
}

/// Yedeğin aşağı yukarı boyutu (veritabanı + klasörler), arayüzde göstermek için.
pub fn estimate(root: &Path, opts: Options) -> u64 {
    let mut files = vec![root.join(DB_NAME)];
    for f in folders(opts) {
        files_under(&root.join(f), &mut files);
    }
    files
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum()
}

fn zip_name(rel: &Path) -> String {
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// `GMID-Yedek.zip`'i oluşturur. `progress(yazılan, toplam)` bayt cinsinden.
pub fn archive(
    db: &Db,
    root: &Path,
    home: &Path,
    out: &Path,
    opts: Options,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<u64, String> {
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;

    // Çalışan veritabanının tutarlı bir kopyası.
    let snap =
        std::env::temp_dir().join(format!("gmid-yedek-{}-{}.db", std::process::id(), now_ms()));
    let _ = std::fs::remove_file(&snap);
    db.conn()
        .execute("VACUUM INTO ?1", [snap.to_string_lossy()])
        .map_err(|e| format!("Veritabanı kopyalanamadı: {e}"))?;

    let mut files: Vec<(PathBuf, String)> = vec![(snap.clone(), DB_NAME.into())];
    let used = folders(opts);
    for f in &used {
        let mut list = Vec::new();
        files_under(&root.join(f), &mut list);
        for p in list {
            if let Ok(rel) = p.strip_prefix(root) {
                files.push((p.clone(), zip_name(rel)));
            }
        }
    }
    let total: u64 = files
        .iter()
        .filter_map(|(p, _)| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum();

    let result = (|| -> Result<u64, String> {
        let tmp = out.with_extension("zip.part");
        let file = std::fs::File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
        let mut z = zip::ZipWriter::new(std::io::BufWriter::new(file));
        let mut done = 0u64;
        let mut buf = vec![0u8; 256 * 1024];
        progress(0, total);
        for (path, name) in &files {
            let Ok(mut src) = std::fs::File::open(path) else {
                continue; // Bu arada silinmiş olabilir.
            };
            let size = src.metadata().map(|m| m.len()).unwrap_or(0);
            let ext = Path::new(name)
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let method = if STORED.contains(&ext.as_str()) {
                CompressionMethod::Stored
            } else {
                CompressionMethod::Deflated
            };
            let o = SimpleFileOptions::default()
                .compression_method(method)
                .large_file(size >= u32::MAX as u64);
            z.start_file(name.as_str(), o).map_err(|e| e.to_string())?;
            loop {
                let n = src.read(&mut buf).map_err(|e| format!("{name}: {e}"))?;
                if n == 0 {
                    break;
                }
                z.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                done += n as u64;
                progress(done, total);
            }
        }
        let manifest = Manifest {
            app: "GMID Tool".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            created: now_ms(),
            root: root.to_string_lossy().into_owned(),
            home: home.to_string_lossy().into_owned(),
            folders: used.iter().map(|s| s.to_string()).collect(),
        };
        z.start_file(MANIFEST, SimpleFileOptions::default())
            .map_err(|e| e.to_string())?;
        z.write_all(
            serde_json::to_string_pretty(&manifest)
                .unwrap_or_default()
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        let mut w = z.finish().map_err(|e| e.to_string())?;
        w.flush().map_err(|e| e.to_string())?;
        drop(w);
        let _ = std::fs::remove_file(out);
        std::fs::rename(&tmp, out).map_err(|e| e.to_string())?;
        Ok(std::fs::metadata(out).map(|m| m.len()).unwrap_or(0))
    })();
    let _ = std::fs::remove_file(&snap);
    result
}

/// Zip içindeki bir yol geri yüklenebilir mi: veritabanı, bildirge ya da bilinen klasörlerden biri.
fn allowed(rel: &Path) -> bool {
    let mut comps = rel.components();
    let Some(Component::Normal(first)) = comps.next() else {
        return false;
    };
    let first = first.to_string_lossy();
    let rest = comps.next().is_some();
    if !rest {
        return first == DB_NAME || first == MANIFEST;
    }
    FOLDERS.contains(&first.as_ref()) || first == VIDEOS || first == TOOLS
}

/// Uygulama içinden geri yükleme, 1. adım: zip'i doğrular ve ara klasöre açar.
/// Asıl değiştirme uygulama yeniden başlarken (`apply_pending`) yapılır.
pub fn stage(zip_path: &Path, root: &Path) -> Result<Manifest, String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut z =
        zip::ZipArchive::new(file).map_err(|_| "Bu dosya bir GMID yedeği değil".to_string())?;
    let manifest: Manifest = {
        let mut m = z
            .by_name(MANIFEST)
            .map_err(|_| "Bu dosya bir GMID yedeği değil (gmid-yedek.json yok)".to_string())?;
        let mut s = String::new();
        m.read_to_string(&mut s).map_err(|e| e.to_string())?;
        serde_json::from_str(&s).map_err(|_| "Yedek bilgisi okunamadı".to_string())?
    };
    {
        let mut db = z
            .by_name(DB_NAME)
            .map_err(|_| "Yedekte veritabanı yok".to_string())?;
        let mut head = [0u8; 16];
        db.read_exact(&mut head).map_err(|e| e.to_string())?;
        if &head != b"SQLite format 3\0" {
            return Err("Yedekteki veritabanı bozuk".into());
        }
    }
    let dest = root.join(STAGING);
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        // enclosed_name: "../" ya da mutlak yol içeren adlar dışarı yazamasın.
        let Some(rel) = f.enclosed_name() else {
            continue;
        };
        if f.is_dir() || !allowed(&rel) {
            continue;
        }
        let target = dest.join(&rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut f, &mut out).map_err(|e| e.to_string())?;
    }
    if !dest.join(DB_NAME).is_file() {
        let _ = std::fs::remove_dir_all(&dest);
        return Err("Yedek açılamadı".into());
    }
    Ok(manifest)
}

pub fn cancel_staged(root: &Path) {
    let _ = std::fs::remove_dir_all(root.join(STAGING));
}

/// Hazırlanan yedek bir sonraki açılışta uygulansın.
pub fn confirm(root: &Path) -> Result<(), String> {
    let staged = root.join(STAGING);
    if !staged.join(DB_NAME).is_file() {
        return Err("Hazırlanmış bir yedek yok".into());
    }
    std::fs::write(staged.join(CONFIRMED), b"").map_err(|e| e.to_string())
}

fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(p) = to.parent() {
        std::fs::create_dir_all(p)?;
    }
    let _ = std::fs::remove_file(to);
    std::fs::rename(from, to).or_else(|_| {
        std::fs::copy(from, to)?;
        std::fs::remove_file(from)
    })
}

/// Uygulama içinden geri yükleme, 2. adım (açılışta, veritabanı açılmadan): bekleyen yedek varsa
/// mevcut veritabanını `Yedekler\geri-yukleme-oncesi-….db` olarak saklar ve yedeği yerine koyar.
pub fn apply_pending(root: &Path) -> Result<bool, String> {
    let staged = root.join(STAGING);
    if !staged.join(CONFIRMED).is_file() || !staged.join(DB_NAME).is_file() {
        // Onaylanmadan kalmış hazırlık (uygulama "Vazgeç" denmeden kapandı): uygulanmaz.
        if staged.exists() {
            let _ = std::fs::remove_dir_all(&staged);
        }
        return Ok(false);
    }
    let _ = std::fs::remove_file(staged.join(CONFIRMED));
    let keep = root.join("Yedekler").join(format!(
        "geri-yukleme-oncesi-{}.db",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));
    let current = root.join(DB_NAME);
    if current.is_file() {
        move_file(&current, &keep).map_err(|e| format!("Mevcut veritabanı saklanamadı: {e}"))?;
        for ext in ["-wal", "-shm"] {
            let side = PathBuf::from(format!("{}{ext}", current.display()));
            if side.is_file() {
                let _ = move_file(&side, &PathBuf::from(format!("{}{ext}", keep.display())));
            }
        }
    }
    let mut files = Vec::new();
    files_under(&staged, &mut files);
    for f in files {
        if let Ok(rel) = f.strip_prefix(&staged) {
            move_file(&f, &root.join(rel)).map_err(|e| format!("{}: {e}", rel.display()))?;
        }
    }
    let _ = std::fs::remove_dir_all(&staged);
    Ok(true)
}

/// JSON metninde ters bölü iki kat yazılır (`C:\\Users\\…`).
fn json_escaped(s: &str) -> String {
    s.replace('\\', "\\\\")
}

/// Veritabanındaki bütün metin sütunlarında `from` → `to` (düz ve JSON içindeki hâliyle). Değişen hücre sayısı.
pub fn replace_paths(conn: &rusqlite::Connection, from: &str, to: &str) -> rusqlite::Result<usize> {
    if from.is_empty() || from == to {
        return Ok(0);
    }
    let tables: Vec<String> = {
        let mut st = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             AND sql NOT LIKE 'CREATE VIRTUAL%'",
        )?;
        let rows = st.query_map([], |r| r.get(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let pairs = [
        (from.to_string(), to.to_string()),
        (json_escaped(from), json_escaped(to)),
    ];
    let mut changed = 0;
    let tx = conn.unchecked_transaction()?;
    for t in &tables {
        let cols: Vec<String> = {
            let mut st = tx.prepare(&format!("SELECT name, type FROM pragma_table_info('{t}')"))?;
            let rows =
                st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            rows.filter_map(Result::ok)
                .filter(|(_, ty)| {
                    let ty = ty.to_uppercase();
                    ty.is_empty() || ty.contains("TEXT") || ty.contains("CHAR")
                })
                .map(|(n, _)| n)
                .collect()
        };
        for c in &cols {
            for (a, b) in &pairs {
                changed += tx.execute(
                    &format!(
                        "UPDATE \"{t}\" SET \"{c}\" = REPLACE(\"{c}\", ?1, ?2)
                         WHERE typeof(\"{c}\") = 'text' AND instr(\"{c}\", ?1) > 0"
                    ),
                    [a, b],
                )?;
            }
        }
    }
    tx.commit()?;
    Ok(changed)
}

/// Yedekten açıldıktan sonraki ilk açılışta: yollar yeni klasörlere çevrilir, bildirge silinir.
pub fn finish(db: &Db, root: &Path, home: &Path) {
    let path = root.join(MANIFEST);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let m: Manifest = serde_json::from_str(&text).unwrap_or_default();
    // Sonuna ayraç eklenir: "C:\Users\Ali" → "C:\Users\Veli" çevrilirken "C:\Users\Alican" bozulmasın.
    let sep = |p: &str| {
        if p.is_empty() || p.ends_with(['\\', '/']) {
            p.to_string()
        } else {
            format!("{p}{}", std::path::MAIN_SEPARATOR)
        }
    };
    let (old_root, old_home) = (sep(&m.root), sep(&m.home));
    let new_root = sep(&root.to_string_lossy());
    let new_home = sep(&home.to_string_lossy());
    let conn = db.conn();
    // Önce veri klasörü (Belgeler başka diske taşınmış olabilir), sonra kullanıcı klasörü.
    let mut n = 0;
    for (from, to) in [(&old_root, &new_root), (&old_home, &new_home)] {
        match replace_paths(&conn, from, to) {
            Ok(k) => n += k,
            Err(e) => crate::log!("Geri yükleme: yollar çevrilemedi: {e}"),
        }
    }
    drop(conn);
    crate::log!(
        "Yedekten geri yüklendi ({}, {} yol güncellendi)",
        chrono::DateTime::from_timestamp_millis(m.created)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        n
    );
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gmid-restore-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn paths_are_rewritten_plain_and_in_json() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE videos (id TEXT, path TEXT, size INTEGER);
             INSERT INTO settings VALUES ('theme.bg.path', '\"C:\\\\Users\\\\Eski\\\\Documents\\\\GMID Tool\\\\Arkaplanlar\\\\a.jpg\"');
             INSERT INTO videos VALUES ('1', 'C:\\Users\\Eski\\Documents\\GMID Tool\\Videolar\\v.mp4', 5);
             INSERT INTO videos VALUES ('2', 'D:\\Baska\\v.mp4', 5);",
        )
        .unwrap();
        let n = replace_paths(&conn, "C:\\Users\\Eski", "C:\\Users\\Yeni").unwrap();
        assert_eq!(n, 2);
        let bg: String = conn
            .query_row("SELECT value FROM settings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            bg,
            "\"C:\\\\Users\\\\Yeni\\\\Documents\\\\GMID Tool\\\\Arkaplanlar\\\\a.jpg\""
        );
        let v: Vec<String> = conn
            .prepare("SELECT path FROM videos ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            v,
            [
                "C:\\Users\\Yeni\\Documents\\GMID Tool\\Videolar\\v.mp4",
                "D:\\Baska\\v.mp4"
            ]
        );
    }

    #[test]
    fn home_rename_does_not_touch_similar_names() {
        let d = tmp("sep");
        let db = Db::open(&d.join(DB_NAME)).unwrap();
        db.set_setting("a", "/home/ali/Belgeler/x.mp4").unwrap();
        db.set_setting("b", "/home/alican/y.mp4").unwrap();
        let m = Manifest {
            root: "/eski/GMID Tool".into(),
            home: "/home/ali".into(),
            ..Default::default()
        };
        std::fs::write(d.join(MANIFEST), serde_json::to_string(&m).unwrap()).unwrap();
        finish(&db, &d, Path::new("/home/veli"));
        assert_eq!(
            db.setting("a").unwrap().as_deref(),
            Some("/home/veli/Belgeler/x.mp4")
        );
        assert_eq!(
            db.setting("b").unwrap().as_deref(),
            Some("/home/alican/y.mp4")
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn archive_stage_and_apply_roundtrip() {
        let src = tmp("src");
        std::fs::create_dir_all(src.join("Notlar")).unwrap();
        std::fs::create_dir_all(src.join("Videolar")).unwrap();
        std::fs::create_dir_all(src.join("Yedekler")).unwrap();
        std::fs::write(src.join("Notlar").join("not.md"), "merhaba").unwrap();
        std::fs::write(src.join("Videolar").join("v.mp4"), "video").unwrap();
        std::fs::write(src.join("gmid.log"), "log").unwrap();
        let db = Db::open(&src.join(DB_NAME)).unwrap();
        db.set_setting("ornek", "\"1\"").unwrap();

        let out = src.join("GMID-Yedek.zip");
        let mut last = (0, 0);
        archive(
            &db,
            &src,
            Path::new("C:\\Users\\Eski"),
            &out,
            Options::default(),
            &mut |a, b| last = (a, b),
        )
        .unwrap();
        assert!(last.1 > 0 && last.0 == last.1);

        // Yeni bilgisayar: boş klasör, yeni bir veritabanı.
        let dst = tmp("dst");
        std::fs::create_dir_all(dst.join("Yedekler")).unwrap();
        drop(Db::open(&dst.join(DB_NAME)).unwrap());
        let m = stage(&out, &dst).unwrap();
        assert_eq!(m.app, "GMID Tool");
        assert_eq!(m.folders, FOLDERS);
        // Onaylanmadan kapanırsa yedek uygulanmaz, hazırlık silinir.
        assert!(!apply_pending(&dst).unwrap());
        assert!(!dst.join(STAGING).exists());
        assert!(confirm(&dst).is_err());
        stage(&out, &dst).unwrap();
        confirm(&dst).unwrap();
        assert!(apply_pending(&dst).unwrap());
        assert!(!apply_pending(&dst).unwrap());
        assert_eq!(
            std::fs::read_to_string(dst.join("Notlar").join("not.md")).unwrap(),
            "merhaba"
        );
        // Videolar istenmedi, günlük dosyası hiç yedeklenmez.
        assert!(!dst.join("Videolar").join("v.mp4").exists());
        assert!(!dst.join("gmid.log").exists());
        let kept = std::fs::read_dir(dst.join("Yedekler")).unwrap().count();
        assert_eq!(kept, 1);
        let db2 = Db::open(&dst.join(DB_NAME)).unwrap();
        assert_eq!(db2.setting("ornek").unwrap().as_deref(), Some("\"1\""));
        finish(&db2, &dst, Path::new("C:\\Users\\Yeni"));
        assert!(!dst.join(MANIFEST).exists());

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&dst);
    }

    #[test]
    fn foreign_zip_is_rejected_and_paths_are_confined() {
        let d = tmp("bad");
        let out = d.join("x.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&out).unwrap());
        z.start_file("baska.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(b"x").unwrap();
        z.finish().unwrap();
        assert!(stage(&out, &d).is_err());
        assert!(allowed(Path::new("Notlar/a.md")));
        assert!(allowed(Path::new("gmid.db")));
        assert!(!allowed(Path::new("gmid.log")));
        assert!(!allowed(Path::new("Yedekler/x.db")));
        assert!(!allowed(Path::new("Windows/System32/x.dll")));
        let _ = std::fs::remove_dir_all(&d);
    }
}
