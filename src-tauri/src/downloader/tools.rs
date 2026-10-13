//! İndirmeyi yapan araçlar: yt-dlp (videoyu bulur ve indirir), ffmpeg (görüntü ile sesi birleştirir,
//! MP3'e çevirir) ve deno (YouTube'un istediği JavaScript çalıştırıcısı).
//!
//! Kurulum dosyası küçük kalsın diye uygulamayla gelmezler; ilk kullanımda resmî GitHub
//! sürümlerinden `GMID Tool\Araclar` klasörüne indirilirler. yt-dlp kendini günde en fazla bir kez günceller.

use crate::util::{hide_console, now_ms};
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// yt-dlp en fazla bu sıklıkla güncellenir.
const UPDATE_EVERY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
enum Kind {
    /// Tek dosya, olduğu gibi kaydedilir.
    File,
    /// Zip; içinden sadece istenen dosyalar çıkarılır.
    Zip(&'static [&'static str]),
}

struct Tool {
    id: &'static str,
    name: &'static str,
    purpose: &'static str,
    url: &'static str,
    /// `Araclar` klasörüne göre kurulu dosya (var mı diye buna bakılır).
    file: &'static str,
    /// Zip'ten çıkan dosyaların gideceği alt klasör.
    subdir: &'static str,
    kind: Kind,
    /// Arayüzde gösterilen yaklaşık indirme boyutu (MB).
    size_mb: u32,
}

#[cfg(windows)]
const TOOLS: &[Tool] = &[
    Tool {
        id: "yt-dlp",
        name: "yt-dlp",
        purpose: "Videoyu bulur ve indirir",
        url: "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
        file: "yt-dlp.exe",
        subdir: "",
        kind: Kind::File,
        size_mb: 18,
    },
    Tool {
        id: "ffmpeg",
        name: "FFmpeg",
        purpose: "Görüntü ile sesi birleştirir, MP3'e çevirir",
        url: "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl-shared.zip",
        file: "ffmpeg/ffmpeg.exe",
        subdir: "ffmpeg",
        // ffplay gerekmiyor (18 MB).
        kind: Kind::Zip(&["/bin/ffmpeg.exe", "/bin/ffprobe.exe", "/bin/*.dll"]),
        size_mb: 89,
    },
    Tool {
        id: "deno",
        name: "Deno",
        purpose: "YouTube'un doğrulamasını çözer",
        url: "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip",
        file: "deno.exe",
        subdir: "",
        kind: Kind::Zip(&["deno.exe"]),
        size_mb: 43,
    },
];

/// Geliştirme (Linux): sadece yt-dlp indirilir; ffmpeg ve deno sistemde varsa kullanılır.
#[cfg(not(windows))]
const TOOLS: &[Tool] = &[Tool {
    id: "yt-dlp",
    name: "yt-dlp",
    purpose: "Videoyu bulur ve indirir",
    url: "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux",
    file: "yt-dlp",
    subdir: "",
    kind: Kind::File,
    size_mb: 40,
}];

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    id: &'static str,
    name: &'static str,
    purpose: &'static str,
    installed: bool,
    size_mb: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolsState {
    pub tools: Vec<ToolStatus>,
    pub ready: bool,
    pub installing: bool,
    pub version: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    tool: &'a str,
    name: &'a str,
    /// "download" | "extract" | "done" | "error"
    stage: &'a str,
    done: u64,
    total: u64,
    error: Option<String>,
}

#[derive(Default)]
pub struct Tools {
    installing: AtomicBool,
}

pub fn dir(root: &Path) -> PathBuf {
    root.join("Araclar")
}

pub fn ytdlp_path(root: &Path) -> PathBuf {
    dir(root).join(TOOLS[0].file)
}

pub fn ffmpeg_dir(root: &Path) -> Option<PathBuf> {
    let d = dir(root).join("ffmpeg");
    d.join(if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    })
    .is_file()
    .then_some(d)
}

impl Tools {
    pub fn state(&self, root: &Path, version: Option<String>) -> ToolsState {
        let tools: Vec<ToolStatus> = TOOLS
            .iter()
            .map(|t| ToolStatus {
                id: t.id,
                name: t.name,
                purpose: t.purpose,
                installed: dir(root).join(t.file).is_file(),
                size_mb: t.size_mb,
            })
            .collect();
        ToolsState {
            ready: tools.iter().all(|t| t.installed),
            tools,
            installing: self.installing.load(Ordering::SeqCst),
            version,
        }
    }

    /// Eksik araçları (ya da `force` ise hepsini) indirir. İlerleme `tools-progress` olayıyla gider.
    pub fn install(&self, app: &AppHandle, root: &Path, force: bool) -> Result<(), String> {
        self.install_some(app, root, force, None)
    }

    /// Yalnızca bir aracı indirir (ör. klip kapakları için ffmpeg); kuruluysa bir şey yapmaz.
    pub fn install_only(&self, app: &AppHandle, root: &Path, id: &str) -> Result<(), String> {
        self.install_some(app, root, false, Some(id))
    }

    fn install_some(
        &self,
        app: &AppHandle,
        root: &Path,
        force: bool,
        only: Option<&str>,
    ) -> Result<(), String> {
        if self.installing.swap(true, Ordering::SeqCst) {
            return Err("Araçlar zaten indiriliyor.".into());
        }
        let result = (|| {
            let base = dir(root);
            std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
            for t in TOOLS {
                if only.is_some_and(|id| id != t.id) || (!force && base.join(t.file).is_file()) {
                    continue;
                }
                let emit = |stage: &str, done: u64, total: u64, error: Option<String>| {
                    let _ = app.emit(
                        "tools-progress",
                        Progress {
                            tool: t.id,
                            name: t.name,
                            stage,
                            done,
                            total,
                            error,
                        },
                    );
                };
                if let Err(e) = install_one(t, &base, &emit) {
                    crate::log!("{} kurulamadı: {e}", t.name);
                    emit("error", 0, 0, Some(e.clone()));
                    return Err(format!("{} indirilemedi: {e}", t.name));
                }
                emit("done", 0, 0, None);
            }
            Ok(())
        })();
        self.installing.store(false, Ordering::SeqCst);
        result
    }
}

/// İlerleme bildirimi: aşama, inen, toplam, hata.
type Emit<'a> = &'a dyn Fn(&str, u64, u64, Option<String>);

fn install_one(t: &Tool, base: &Path, emit: Emit) -> Result<(), String> {
    let tmp = base.join(format!("{}.indiriliyor", t.id));
    download(t.url, &tmp, |done, total| {
        emit("download", done, total, None)
    })?;
    let result = match t.kind {
        Kind::File => {
            let dest = base.join(t.file);
            replace(&tmp, &dest)?;
            make_executable(&dest);
            Ok(())
        }
        Kind::Zip(wanted) => {
            emit("extract", 0, 0, None);
            let r = extract(&tmp, &base.join(t.subdir), wanted);
            let _ = std::fs::remove_file(&tmp);
            r
        }
    };
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

pub(crate) fn download(url: &str, dest: &Path, progress: impl Fn(u64, u64)) -> Result<(), String> {
    // Gövde süresi dosyanın tamamı için: birkaç GB'lık modeller yavaş bağlantıda saatler sürebilir.
    let agent = crate::net::agent(Duration::from_secs(20), Duration::from_secs(6 * 3600));
    let resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("İnternete ulaşılamadı ({e})"))?;
    if resp.status() != 200 {
        return Err(format!("Sunucu {} döndü", resp.status()));
    }
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let mut reader = resp.into_body().into_reader();
    let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 256 * 1024];
    let (mut done, mut last) = (0u64, 0i64);
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("İndirme yarıda kesildi ({e})"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if now_ms() - last > 150 {
            last = now_ms();
            progress(done, total);
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    if total > 0 && done != total {
        return Err("İndirme yarıda kesildi".into());
    }
    progress(done, total.max(done));
    Ok(())
}

/// Çalışan bir programın üzerine yazılamaz (Windows); önce eskisini kenara al.
pub(crate) fn replace(src: &Path, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        let old = dest.with_extension("eski");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(dest, &old).map_err(|e| format!("Eski sürüm kaldırılamadı: {e}"))?;
    }
    std::fs::rename(src, dest).map_err(|e| e.to_string())?;
    // Eski sürüm çalışmıyorsa hemen silinir; çalışıyorsa bir sonraki güncellemede.
    let _ = std::fs::remove_file(dest.with_extension("eski"));
    Ok(())
}

fn matches(name: &str, pattern: &str) -> bool {
    match pattern.split_once('*') {
        Some((pre, post)) => {
            // "/bin/*.dll": klasör yolu herhangi bir üst klasörle başlayabilir.
            let (dir, file) = name.rsplit_once('/').unwrap_or(("", name));
            let pdir = pre.trim_end_matches('/');
            (pdir.is_empty() || format!("/{dir}").ends_with(pdir)) && file.ends_with(post)
        }
        None if pattern.starts_with('/') => name.ends_with(pattern),
        None => name == pattern || name.ends_with(&format!("/{pattern}")),
    }
}

pub(crate) fn extract(zip_path: &Path, dest: &Path, wanted: &[&str]) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Arşiv açılamadı: {e}"))?;
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let mut found = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if entry.is_dir() || !wanted.iter().any(|p| matches(&name, p)) {
            continue;
        }
        let file_name = name.rsplit('/').next().unwrap_or(&name);
        let tmp = dest.join(format!("{file_name}.indiriliyor"));
        let mut out = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        drop(out);
        replace(&tmp, &dest.join(file_name))?;
        make_executable(&dest.join(file_name));
        found += 1;
    }
    if found == 0 {
        return Err("Arşivde beklenen dosya yok".into());
    }
    Ok(())
}

fn make_executable(_path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(_path, std::fs::Permissions::from_mode(0o755));
    }
}

/// yt-dlp komutu: araç klasörü PATH'in başında (deno oradan bulunur), konsol penceresi açılmaz,
/// çıktı UTF-8.
pub fn ytdlp(root: &Path) -> Command {
    let tools = dir(root);
    let mut cmd = Command::new(ytdlp_path(root));
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![tools.clone()];
    paths.extend(std::env::split_paths(&path));
    if let Ok(joined) = std::env::join_paths(paths) {
        cmd.env("PATH", joined);
    }
    cmd.env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .current_dir(&tools)
        .arg("--ignore-config")
        .arg("--no-color")
        // Türkçe Windows'ta yt-dlp çıktıyı cp1254 ile yazar ve bu tabloda olmayan karakterleri
        // (［］, ｜ gibi) sessizce atar; dosya adları bozulmasın diye çıktı her zaman UTF-8.
        // (PYTHONUTF8 paketlenmiş yt-dlp.exe'de işe yaramıyor.)
        .args(["--encoding", "utf-8"]);
    if let Some(ff) = ffmpeg_dir(root) {
        cmd.arg("--ffmpeg-location").arg(ff);
    }
    hide_console(&mut cmd);
    cmd
}

pub fn version(root: &Path) -> Option<String> {
    if !ytdlp_path(root).is_file() {
        return None;
    }
    let out = ytdlp(root).arg("--version").output().ok()?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!v.is_empty()).then_some(v)
}

/// yt-dlp'yi günceller (gün içinde zaten denendiyse `force` olmadan atlar). Yeni sürümü döner.
pub fn update_ytdlp(db: &crate::db::Db, root: &Path, force: bool) -> Option<String> {
    const KEY: &str = "videos.ytdlpCheckedAt";
    let last: i64 = db.setting_json(KEY).unwrap_or(0);
    if !force && now_ms() - last < UPDATE_EVERY_MS {
        return None;
    }
    if !ytdlp_path(root).is_file() {
        return None;
    }
    let _ = db.set_setting(KEY, &now_ms().to_string());
    let out = ytdlp(root).arg("-U").output();
    match out {
        Ok(o) if !o.status.success() => crate::log!(
            "yt-dlp güncellenemedi: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        ),
        Err(e) => crate::log!("yt-dlp güncellenemedi: {e}"),
        _ => {}
    }
    version(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_wanted_files() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("gmid-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("a.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        for name in [
            "ff/bin/ffmpeg.exe",
            "ff/bin/avcodec-63.dll",
            "ff/bin/ffplay.exe",
            "ff/doc/x.txt",
        ] {
            z.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(name.as_bytes()).unwrap();
        }
        z.finish().unwrap();
        let out = dir.join("ffmpeg");
        extract(&zip_path, &out, &["/bin/ffmpeg.exe", "/bin/*.dll"]).unwrap();
        let mut got: Vec<String> = std::fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        got.sort();
        assert_eq!(got, ["avcodec-63.dll", "ffmpeg.exe"]);
        // İkinci kez kurulum (güncelleme) eski dosyaların üzerine yazabilmeli.
        extract(&zip_path, &out, &["/bin/ffmpeg.exe"]).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Ağ gerektirir: `cargo test -- --ignored real_download`
    #[test]
    #[ignore]
    fn real_download() {
        let dir = std::env::temp_dir().join(format!("gmid-tools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        install_one(&TOOLS[0], &dir, &|stage, done, total, _| {
            if stage != "download" || done == total {
                println!("{stage} {done}/{total}");
            }
        })
        .unwrap();
        assert!(dir.join(TOOLS[0].file).is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zip_patterns() {
        let p = "ffmpeg-master-latest-win64-gpl-shared/bin/avcodec-63.dll";
        assert!(matches(p, "/bin/*.dll"));
        assert!(matches("x/bin/ffmpeg.exe", "/bin/ffmpeg.exe"));
        assert!(!matches("x/bin/ffplay.exe", "/bin/ffmpeg.exe"));
        assert!(!matches("x/doc/readme.dll.txt", "/bin/*.dll"));
        assert!(!matches("x/lib/a.dll", "/bin/*.dll"));
        assert!(matches("deno.exe", "deno.exe"));
    }
}
