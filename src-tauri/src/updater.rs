//! Uygulama içinden güncelleme.
//!
//! Her derleme GitHub'da `build-<numara>` etiketli bir sürüm olarak yayınlanır (bkz. `.github/workflows/build.yml`):
//! `GMID-Tool-Kurulum.exe` ve `GMID-Tool-Kurulum.exe.sha256`. Uygulama kendi derleme numarasını derlenirken
//! (`GMID_BUILD`) öğrenir; açılıştan biraz sonra ve 6 saatte bir en son sürüme bakar. "Güncelle" denince kurulum
//! dosyası indirilir, SHA-256 ile doğrulanır, uygulama kapanır, kurulum sessizce çalışır ve GMID yeniden açılır.

use crate::util::now_ms;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const API: &str = "https://api.github.com/repos/BalamanMerkut/GMID--Personel_Tool/releases/latest";
const ASSET: &str = "GMID-Tool-Kurulum.exe";
const TAG_PREFIX: &str = "build-";

/// Bu kopyanın derleme numarası (yerel geliştirme derlemesinde yok).
pub fn current_build() -> Option<u32> {
    option_env!("GMID_BUILD").and_then(|s| s.trim().parse().ok())
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub build: u32,
    pub title: String,
    pub notes: String,
    pub published: String,
    pub size: u64,
    #[serde(skip)]
    url: String,
    #[serde(skip)]
    sha_url: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub current: Option<u32>,
    pub latest: Option<Release>,
    pub available: bool,
    pub checked_at: Option<i64>,
    pub error: Option<String>,
    /// İndirilirken 0..1.
    pub progress: Option<f32>,
}

#[derive(Default)]
pub struct Updater {
    status: Mutex<Status>,
    /// Bildirimi bir kez göster.
    notified: Mutex<Option<u32>>,
}

/// GitHub'ın "son sürüm" yanıtından bilgiler; etiket `build-N` değilse `None`.
pub fn parse_release(json: &Value) -> Option<Release> {
    let build: u32 = json["tag_name"]
        .as_str()?
        .strip_prefix(TAG_PREFIX)?
        .parse()
        .ok()?;
    let assets = json["assets"].as_array()?;
    let find = |name: &str| assets.iter().find(|a| a["name"].as_str() == Some(name));
    let exe = find(ASSET)?;
    let body = json["body"].as_str().unwrap_or_default().trim().to_string();
    // Gövde: commit mesajı. İlk satır başlık, gerisi ayrıntı (ortak yazar satırları atılır).
    let mut lines = body.lines();
    let title = lines.next().unwrap_or_default().trim().to_string();
    let notes = lines
        .filter(|l| {
            let l = l.trim();
            !l.starts_with("Co-Authored-By") && !l.starts_with("Claude-Session")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    Some(Release {
        build,
        title: if title.is_empty() {
            json["name"].as_str().unwrap_or_default().to_string()
        } else {
            title
        },
        notes,
        published: json["published_at"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        size: exe["size"].as_u64().unwrap_or(0),
        url: exe["browser_download_url"].as_str()?.to_string(),
        sha_url: find(&format!("{ASSET}.sha256"))
            .and_then(|a| a["browser_download_url"].as_str())
            .map(str::to_string),
    })
}

fn agent() -> ureq::Agent {
    crate::net::agent(Duration::from_secs(15), Duration::from_secs(30 * 60))
}

impl Updater {
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn set(&self, app: &AppHandle, f: impl FnOnce(&mut Status)) {
        let s = {
            let mut s = self.status.lock().unwrap_or_else(|e| e.into_inner());
            f(&mut s);
            s.clone()
        };
        let _ = app.emit("update", &s);
    }

    pub fn check(&self, app: &AppHandle) -> Status {
        let current = current_build();
        let result = (|| -> Result<Release, String> {
            let mut r = agent()
                .get(API)
                .header("Accept", "application/vnd.github+json")
                .call()
                .map_err(|e| format!("İnternete ulaşılamadı ({e})"))?;
            if r.status() != 200 {
                return Err(format!("GitHub {} döndü", r.status()));
            }
            let text = r
                .body_mut()
                .with_config()
                .limit(2 * 1024 * 1024)
                .read_to_string()
                .map_err(|e| e.to_string())?;
            let json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            parse_release(&json).ok_or_else(|| "Yayınlanmış bir sürüm bulunamadı".to_string())
        })();
        self.set(app, |s| {
            s.current = current;
            s.checked_at = Some(now_ms());
            match result {
                Ok(rel) => {
                    // Geliştirme derlemesinde (numarasız) güncelleme önerilmez.
                    s.available = current.is_some_and(|c| rel.build > c);
                    s.latest = Some(rel);
                    s.error = None;
                }
                Err(e) => s.error = Some(e),
            }
        });
        let s = self.status();
        if s.available {
            let build = s.latest.as_ref().map(|r| r.build);
            let mut n = self.notified.lock().unwrap_or_else(|e| e.into_inner());
            if *n != build {
                *n = build;
                crate::overlay::system_notification(
                    app,
                    "update.notify",
                    "GMID Tool güncellemesi var",
                    s.latest.as_ref().map(|r| r.title.as_str()).unwrap_or(""),
                );
            }
        }
        s
    }

    /// İndirir, doğrular, kurulumu başlatır ve uygulamayı kapatır.
    pub fn install(&self, app: &AppHandle) -> Result<(), String> {
        let rel = self
            .status()
            .latest
            .filter(|_| self.status().available)
            .ok_or("Yeni bir sürüm yok")?;
        let dest = std::env::temp_dir().join(format!("GMID-Tool-Kurulum-{}.exe", rel.build));
        self.set(app, |s| s.progress = Some(0.0));
        let res = download(&rel, &dest, |p| self.set(app, |s| s.progress = Some(p)));
        if let Err(e) = res {
            self.set(app, |s| {
                s.progress = None;
                s.error = Some(e.clone());
            });
            let _ = std::fs::remove_file(&dest);
            return Err(e);
        }
        crate::log!("Güncelleme kuruluyor: derleme {}", rel.build);
        launch_installer(&dest)?;
        // Kurulum, uygulama kapanınca başlar (bkz. launch_installer); kapanışta Oyun Modu vb. geri alınır.
        app.exit(0);
        Ok(())
    }
}

fn download(rel: &Release, dest: &Path, progress: impl Fn(f32)) -> Result<(), String> {
    let expected = match &rel.sha_url {
        Some(u) => {
            let mut r = agent().get(u).call().map_err(|e| e.to_string())?;
            let t = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
            Some(
                t.split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_lowercase(),
            )
        }
        None => None,
    };
    let resp = agent()
        .get(&rel.url)
        .call()
        .map_err(|e| format!("İndirilemedi ({e})"))?;
    if resp.status() != 200 {
        return Err(format!("İndirilemedi (GitHub {} döndü)", resp.status()));
    }
    let mut reader = resp.into_body().into_reader();
    let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let (mut done, mut last) = (0u64, 0i64);
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("İndirme yarıda kesildi ({e})"))?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n]).map_err(|e| e.to_string())?;
        hasher.update(&buf[..n]);
        done += n as u64;
        if now_ms() - last > 150 && rel.size > 0 {
            last = now_ms();
            progress((done as f64 / rel.size as f64).min(1.0) as f32);
        }
    }
    drop(file);
    if rel.size > 0 && done != rel.size {
        return Err("İndirme eksik kaldı".into());
    }
    let got: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if let Some(exp) = expected {
        if exp != got {
            return Err("İndirilen dosya doğrulanamadı (SHA-256 tutmuyor)".into());
        }
    }
    progress(1.0);
    Ok(())
}

/// `cmd /S /C "…"` ile çalışacak komut: uygulama kapanana dek bekle (görünmez pencerede `timeout` çalışmaz,
/// `ping` ile beklenir), kurulumu sessizce çalıştır, bitince GMID'i yeniden aç.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn relaunch_script(setup: &Path, exe: &Path) -> String {
    format!(
        "ping -n 3 127.0.0.1 >nul & \"{}\" /S & start \"\" \"{}\"",
        setup.display(),
        exe.display()
    )
}

#[cfg(windows)]
fn launch_installer(setup: &Path) -> Result<(), String> {
    use crate::util::hide_console;
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new("cmd");
    // Rust'ın tırnaklama kuralları cmd.exe'ninkinden farklı: komut olduğu gibi verilir.
    cmd.raw_arg(format!("/S /C \"{}\"", relaunch_script(setup, &exe)));
    hide_console(&mut cmd);
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Kurulum başlatılamadı: {e}"))
}

#[cfg(not(windows))]
fn launch_installer(_setup: &Path) -> Result<(), String> {
    Err("Güncelleme yalnızca Windows'ta kurulur".into())
}

/// Açılıştan 30 sn sonra ve 6 saatte bir bakar (ayardan kapatılabilir).
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(30));
        loop {
            let auto = app
                .state::<crate::db::Db>()
                .setting_json::<bool>("update.auto")
                .unwrap_or(true);
            if auto {
                app.state::<Updater>().check(&app);
            }
            std::thread::sleep(Duration::from_secs(6 * 3600));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_is_parsed_from_github_json() {
        let json: Value = serde_json::from_str(
            r#"{"tag_name":"build-42","name":"GMID Tool derleme 42","published_at":"2026-10-12T01:00:00Z",
                "body":"Klip kırpma ve daha net Discord\n\n- Kırp ve hazırla penceresi\n\nCo-Authored-By: X <x@y>\nClaude-Session: https://z",
                "assets":[
                  {"name":"GMID-Tool-Kurulum.exe","size":9000000,"browser_download_url":"https://github.com/o/r/releases/download/build-42/GMID-Tool-Kurulum.exe"},
                  {"name":"GMID-Tool-Kurulum.exe.sha256","size":64,"browser_download_url":"https://github.com/o/r/releases/download/build-42/GMID-Tool-Kurulum.exe.sha256"}
                ]}"#,
        )
        .unwrap();
        let r = parse_release(&json).unwrap();
        assert_eq!(r.build, 42);
        assert_eq!(r.title, "Klip kırpma ve daha net Discord");
        assert_eq!(r.notes, "- Kırp ve hazırla penceresi");
        assert_eq!(r.size, 9_000_000);
        assert!(r.sha_url.is_some());
        // Başka biçimde etiket ya da kurulum dosyası yoksa sürüm sayılmaz.
        let other: Value = serde_json::from_str(r#"{"tag_name":"v1.0","assets":[]}"#).unwrap();
        assert!(parse_release(&other).is_none());
    }

    #[test]
    fn relaunch_script_quotes_paths_with_spaces() {
        let s = relaunch_script(
            Path::new(r"C:\Users\Ali Veli\AppData\Local\Temp\GMID-Tool-Kurulum-42.exe"),
            Path::new(r"C:\Users\Ali Veli\AppData\Local\GMID Tool\gmid-tool.exe"),
        );
        assert_eq!(
            s,
            r#"ping -n 3 127.0.0.1 >nul & "C:\Users\Ali Veli\AppData\Local\Temp\GMID-Tool-Kurulum-42.exe" /S & start "" "C:\Users\Ali Veli\AppData\Local\GMID Tool\gmid-tool.exe""#
        );
    }
}
