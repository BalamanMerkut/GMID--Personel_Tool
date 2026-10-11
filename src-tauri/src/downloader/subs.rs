//! Film ve dizi altyazıları: video indikten sonra seçilen altyazılar (sitenin kendi altyazıları ya
//! da eklentinin sayfada yakaladığı dosyalar) videonun yanına `<video adı>.<dil>.vtt` olarak iner.
//! VLC gibi oynatıcılar bunları kendiliğinden bulur; uygulamanın oynatıcısı da listeler.

use super::tools;
use super::ytdlp::{self, Context, ExtraSub};
use crate::util::hide_console;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

/// Videonun yanındaki altyazı dosyası.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sidecar {
    /// Dosya adı (klasörsüz).
    pub file: String,
    /// "tr", "en"…; uygulamanın yazıya döktüğü dosyada boş.
    pub lang: String,
    pub label: String,
}

/// Video indikten sonra altyazıları indirir (arka planda; hata indirmeyi bozmaz).
pub fn fetch_after(root: PathBuf, url: String, video: PathBuf, ctx: Context) {
    if ctx.sub_langs.is_empty() && ctx.extra_subs.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        if !ctx.sub_langs.is_empty() {
            let mut cmd = tools::ytdlp(&root);
            cmd.args(ytdlp::subtitle_args(&url, &video, &ctx.sub_langs, &ctx))
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .stdin(Stdio::null());
            hide_console(&mut cmd);
            match cmd.output() {
                Ok(o) if !o.status.success() => crate::log!(
                    "Altyazı indirilemedi: {}",
                    String::from_utf8_lossy(&o.stderr)
                        .lines()
                        .last()
                        .unwrap_or("")
                ),
                Err(e) => crate::log!("Altyazı indirilemedi: {e}"),
                _ => {}
            }
        }
        for (i, sub) in ctx.extra_subs.iter().take(6).enumerate() {
            if let Err(e) = fetch_extra(&video, sub, i, &ctx) {
                crate::log!("Yakalanan altyazı indirilemedi ({}): {e}", sub.url);
            }
        }
    });
}

/// Eklentinin yakaladığı altyazı dosyasını sayfanın bilgileriyle (Referer) indirir.
fn fetch_extra(video: &Path, sub: &ExtraSub, index: usize, ctx: &Context) -> Result<(), String> {
    if !sub.url.starts_with("http://") && !sub.url.starts_with("https://") {
        return Err("geçersiz adres".into());
    }
    let agent = crate::net::agent(Duration::from_secs(10), Duration::from_secs(30));
    let mut req = agent.get(&sub.url);
    if let Some(r) = ctx.referer.as_deref() {
        req = req.header("Referer", r);
    }
    if let Some(ua) = ctx.user_agent.as_deref() {
        req = req.header("User-Agent", ua);
    }
    let mut resp = req.call().map_err(|e| e.to_string())?;
    if resp.status() != 200 {
        return Err(format!("{} döndü", resp.status()));
    }
    let text = resp
        .body_mut()
        .with_config()
        .limit(5 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let ext = if text.trim_start_matches('\u{feff}').starts_with("WEBVTT") {
        "vtt"
    } else if text.contains("-->") {
        "srt"
    } else {
        return Err("altyazı değil".into());
    };
    let lang = Some(sub.lang.as_str())
        .filter(|l| ytdlp::valid_code(l))
        .map(ytdlp::lang_base)
        .unwrap_or_else(|| "und".into());
    let mut path = with_suffix(video, &format!("{lang}.{ext}"));
    if path.exists() {
        path = with_suffix(video, &format!("{lang}{}.{ext}", index + 2));
    }
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

/// "Film [x] 1080p.mp4" + "tr.vtt" → "Film [x] 1080p.tr.vtt"
fn with_suffix(video: &Path, suffix: &str) -> PathBuf {
    let stem = video
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    video.with_file_name(format!("{stem}.{suffix}"))
}

/// Videonun yanındaki altyazı dosyaları: önce Türkçe, sonra İngilizce, sonra diğerleri.
pub fn sidecars(video: &Path) -> Vec<Sidecar> {
    let (Some(dir), Some(stem)) = (video.parent(), video.file_stem()) else {
        return Vec::new();
    };
    let stem = stem.to_string_lossy().into_owned();
    let mut out: Vec<Sidecar> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let rest = name.strip_prefix(&stem)?;
            let middle = rest
                .strip_suffix(".vtt")
                .or_else(|| rest.strip_suffix(".srt"))?;
            let lang = match middle {
                "" => String::new(),
                m => m.strip_prefix('.')?.to_string(),
            };
            if lang.contains('.') || (!lang.is_empty() && !ytdlp::valid_code(&lang)) {
                return None;
            }
            let label = if lang.is_empty() {
                "GMID yazısı".to_string()
            } else {
                ytdlp::lang_label(lang.trim_end_matches(|c: char| c.is_ascii_digit()))
            };
            Some(Sidecar {
                file: name,
                lang,
                label,
            })
        })
        .collect();
    let rank = |s: &Sidecar| match ytdlp::lang_base(&s.lang).as_str() {
        "tr" => 0,
        "en" => 1,
        "" => 3,
        _ => 2,
    };
    out.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.file.cmp(&b.file)));
    out
}

/// Videonun yanındaki altyazı dosyasının yolu; başka bir dosyaya erişilemesin diye adı denetlenir.
pub fn sidecar_path(video: &Path, file: &str) -> Option<PathBuf> {
    sidecars(video)
        .into_iter()
        .find(|s| s.file == file)
        .and_then(|s| Some(video.parent()?.join(s.file)))
}

/// Video silinirken yanındaki altyazılar da silinir.
pub fn remove_all(video: &Path) {
    for s in sidecars(video) {
        if let Some(dir) = video.parent() {
            let _ = std::fs::remove_file(dir.join(s.file));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecars_are_found_and_sorted() {
        let dir = std::env::temp_dir().join(format!("gmid-sidecar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let video = dir.join("Film [abc] 1080p.mp4");
        for f in [
            "Film [abc] 1080p.mp4",
            "Film [abc] 1080p.en.vtt",
            "Film [abc] 1080p.tr.vtt",
            "Film [abc] 1080p.srt",
            "Film [abc] 1080p.de.srt",
            "Film [abc] 720p.tr.vtt",
            "Film [abc] 1080p.tr.vtt.part",
        ] {
            std::fs::write(dir.join(f), "WEBVTT").unwrap();
        }
        let got: Vec<(String, String)> = sidecars(&video)
            .into_iter()
            .map(|s| (s.lang, s.label))
            .collect();
        assert_eq!(
            got,
            vec![
                ("tr".into(), "Türkçe".into()),
                ("en".into(), "İngilizce".into()),
                ("de".into(), "Almanca".into()),
                ("".into(), "GMID yazısı".into()),
            ]
        );
        assert!(sidecar_path(&video, "Film [abc] 1080p.tr.vtt").is_some());
        assert!(sidecar_path(&video, "../gizli.vtt").is_none());
        remove_all(&video);
        assert!(!dir.join("Film [abc] 1080p.tr.vtt").exists());
        assert!(dir.join("Film [abc] 720p.tr.vtt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn suffix_names() {
        assert_eq!(
            with_suffix(Path::new("/v/Film [x] 1080p.mp4"), "tr.vtt"),
            Path::new("/v/Film [x] 1080p.tr.vtt")
        );
    }
}
