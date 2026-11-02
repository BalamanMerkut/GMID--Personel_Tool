//! Birlikte izle: video **inerken izlenir**. yt-dlp videonun ve sesin adreslerini bulur; ffmpeg ikisini inerken
//! birleştirip parça parça (fragmented MP4) geçici dosyaya yazar. Arayüz dosyanın inen kısmını okuyup oynatıcıya
//! ekler (MediaSource), video birkaç saniye içinde başlar, indirme arkada sürer. İndirme bitince dosya
//! tamamlanmış kopya olarak kalır (aynı videoya yeniden girmek anında olur).
//!
//! Bu yol olmazsa (ffmpeg yok, site parça listesi veriyor, codec uygun değil) `AKIS_YOK` döner; arayüz eski yola
//! (önce tamamını indir) geçer.

use crate::paths::DataDir;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

type CmdResult<T> = Result<T, String>;

/// Akış kurulamadı; arayüz önce tamamını indirme yoluna geçer.
const AKIS_YOK: &str = "AKIS_YOK";
/// Bir okumada en çok bu kadar bayt.
const PARCA: u64 = 4 * 1024 * 1024;

struct Durum {
    /// İnerken `<ad>.parca.mp4`, bitince `<ad>.mp4`.
    yol: PathBuf,
    bitti: bool,
    hata: Option<String>,
    mime: String,
}

fn kayit() -> &'static Mutex<HashMap<String, Durum>> {
    static K: OnceLock<Mutex<HashMap<String, Durum>>> = OnceLock::new();
    K.get_or_init(Default::default)
}

fn kilit() -> std::sync::MutexGuard<'static, HashMap<String, Durum>> {
    kayit().lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Baslangic {
    /// Akışın kimliği (okuma ve durum için).
    id: String,
    /// Video zaten tam inmişse dosya yolu: doğrudan dosyadan oynatılır.
    hazir: Option<String>,
    /// MediaSource için tür, ör. `video/mp4; codecs="avc1.64001f,mp4a.40.2"`.
    mime: String,
}

#[derive(Serialize)]
pub struct AkisDurum {
    /// Şu ana kadar yazılan bayt.
    boyut: u64,
    bitti: bool,
    hata: Option<String>,
    /// Bittiyse tam dosyanın yolu.
    yol: Option<String>,
}

/// Bağlantının önbellekteki adı (önce tamamını indirme yoluyla aynı: ikisi aynı dosyayı paylaşır).
fn ad(url: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    true.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn klasor() -> PathBuf {
    std::env::temp_dir().join("gmid-birlikte")
}

/// yt-dlp'nin codec adını MediaSource'un anlayacağı biçime çevirir; desteklenmiyorsa `None`.
fn codec(c: Option<&str>, video: bool) -> Option<String> {
    let c = c.unwrap_or("").trim();
    if c.is_empty() || c == "none" {
        return Some(if video { "avc1.64001f" } else { "mp4a.40.2" }.into());
    }
    let k = c.to_ascii_lowercase();
    if video {
        if k.starts_with("avc1") {
            Some(c.to_string())
        } else if k.starts_with("h264") || k.starts_with("avc") {
            Some("avc1.64001f".into())
        } else {
            None
        }
    } else if k.starts_with("mp4a") {
        Some(c.to_string())
    } else if k.starts_with("aac") {
        Some("mp4a.40.2".into())
    } else {
        None
    }
}

struct Girdi {
    url: String,
    basliklar: String,
}

/// yt-dlp'nin `-j` çıktısından girdiler (video [+ ses]) ve MediaSource türü.
fn coz(j: &Value) -> Option<(Vec<Girdi>, String)> {
    let parcalar: Vec<&Value> = match j["requested_formats"].as_array() {
        Some(l) if !l.is_empty() => l.iter().collect(),
        _ => vec![j],
    };
    let mut girdiler = Vec::new();
    let mut vc = None;
    let mut ac = None;
    for p in &parcalar {
        let proto = p["protocol"].as_str().unwrap_or("");
        // Parça listeli biçimler (DASH, ISM, F4M) ffmpeg'e tek adres olarak verilemez.
        if proto.contains("dash") || proto.contains("ism") || proto.contains("f4m") {
            return None;
        }
        let url = p["url"].as_str()?.to_string();
        let mut basliklar = String::new();
        if let Some(h) = p["http_headers"].as_object() {
            for (k, v) in h {
                if let Some(v) = v.as_str() {
                    basliklar.push_str(&format!("{k}: {v}\r\n"));
                }
            }
        }
        let pv = p["vcodec"].as_str();
        let pa = p["acodec"].as_str();
        if pv.is_some_and(|v| v != "none") || parcalar.len() == 1 {
            vc = Some(codec(pv, true)?);
        }
        if pa.is_some_and(|a| a != "none") || parcalar.len() == 1 {
            ac = Some(codec(pa, false)?);
        }
        girdiler.push(Girdi { url, basliklar });
    }
    let vc = vc?;
    let mime = match ac {
        Some(a) => format!("video/mp4; codecs=\"{vc},{a}\""),
        None => format!("video/mp4; codecs=\"{vc}\""),
    };
    Some((girdiler, mime))
}

/// Akışı başlatır (zaten inmişse dosyayı, iniyorsa süren akışı verir).
#[tauri::command]
pub async fn oda_akis_baslat(
    app: AppHandle,
    url: String,
    sayfa: Option<String>,
) -> CmdResult<Baslangic> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Geçersiz bağlantı".into());
    }
    let sayfa = sayfa.filter(|s| s.starts_with("https://") || s.starts_with("http://"));
    let root = app.state::<DataDir>().root().to_path_buf();
    let dir = klasor();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let _ = app.asset_protocol_scope().allow_directory(&dir, false);
    let ad = ad(&url);
    // Tam inmiş kopya.
    let tam = dir.join(format!("{ad}.mp4"));
    if tam.is_file() {
        return Ok(Baslangic {
            id: ad,
            hazir: Some(tam.to_string_lossy().into_owned()),
            mime: String::new(),
        });
    }
    // Süren akış.
    if let Some(d) = kilit().get(&ad).filter(|d| d.hata.is_none()) {
        return Ok(Baslangic {
            id: ad.clone(),
            hazir: None,
            mime: d.mime.clone(),
        });
    }
    let ffmpeg = crate::downloader::tools::ffmpeg_dir(&root)
        .map(|d| {
            d.join(if cfg!(windows) {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            })
        })
        .ok_or(AKIS_YOK)?;
    tauri::async_runtime::spawn_blocking(move || {
        // Adresler: MediaSource'un çalabileceği biçimler (H.264 + AAC) önce.
        let mut cmd = crate::downloader::tools::ytdlp(&root);
        cmd.args(["--no-playlist", "--no-warnings", "-j", "-f"]).arg(
            "bv*[height<=720][vcodec^=avc1]+ba[acodec^=mp4a]/b[height<=720][vcodec^=avc1]/b[height<=720]",
        );
        if let Some(s) = &sayfa {
            cmd.args(["--referer", s]);
        }
        cmd.args(["--", &url]);
        let out = cmd
            .output()
            .map_err(|_| "yt-dlp bulunamadı; Video İndirici'den araçları kur".to_string())?;
        let j: Value = serde_json::from_slice(&out.stdout).map_err(|_| AKIS_YOK.to_string())?;
        let (girdiler, mime) = coz(&j).ok_or(AKIS_YOK)?;
        let parca = dir.join(format!("{ad}.parca.mp4"));
        let _ = std::fs::remove_file(&parca);
        let mut ff = Command::new(&ffmpeg);
        ff.args(["-hide_banner", "-loglevel", "error", "-y"]);
        for g in &girdiler {
            if !g.basliklar.is_empty() {
                ff.arg("-headers").arg(&g.basliklar);
            }
            ff.arg("-i").arg(&g.url);
        }
        if girdiler.len() > 1 {
            ff.args(["-map", "0:v:0", "-map", "1:a:0"]);
        } else {
            ff.args(["-map", "0:v:0", "-map", "0:a:0?"]);
        }
        ff.args([
            "-c",
            "copy",
            "-movflags",
            "frag_keyframe+empty_moov+default_base_moof",
            "-f",
            "mp4",
        ])
        .arg(&parca)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
        crate::util::hide_console(&mut ff);
        let mut cocuk = ff.spawn().map_err(|_| AKIS_YOK.to_string())?;
        kilit().insert(
            ad.clone(),
            Durum {
                yol: parca.clone(),
                bitti: false,
                hata: None,
                mime: mime.clone(),
            },
        );
        let id = ad.clone();
        std::thread::spawn(move || {
            let mut hata = String::new();
            if let Some(mut e) = cocuk.stderr.take() {
                let _ = e.read_to_string(&mut hata);
            }
            let tamam = cocuk.wait().is_ok_and(|s| s.success());
            let mut k = kilit();
            if let Some(d) = k.get_mut(&id) {
                if tamam && std::fs::rename(&parca, &tam).is_ok() {
                    d.yol = tam;
                    d.bitti = true;
                } else {
                    d.hata = Some(if hata.trim().is_empty() {
                        "Video inerken kesildi".into()
                    } else {
                        hata.lines().last().unwrap_or("").chars().take(200).collect()
                    });
                }
            }
        });
        Ok(Baslangic {
            id: ad,
            hazir: None,
            mime,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn oda_akis_durum(id: String) -> CmdResult<AkisDurum> {
    let k = kilit();
    let d = k.get(&id).ok_or("Akış bulunamadı")?;
    Ok(AkisDurum {
        boyut: std::fs::metadata(&d.yol).map(|m| m.len()).unwrap_or(0),
        bitti: d.bitti,
        hata: d.hata.clone(),
        yol: d.bitti.then(|| d.yol.to_string_lossy().into_owned()),
    })
}

/// Dosyanın `konum`dan sonraki kısmı (en çok 4 MB).
#[tauri::command]
pub fn oda_akis_oku(id: String, konum: u64) -> CmdResult<tauri::ipc::Response> {
    let yol = kilit()
        .get(&id)
        .map(|d| d.yol.clone())
        .ok_or("Akış bulunamadı")?;
    let mut f = std::fs::File::open(&yol).map_err(|e| e.to_string())?;
    f.seek(SeekFrom::Start(konum)).map_err(|e| e.to_string())?;
    let mut bayt = Vec::new();
    f.take(PARCA)
        .read_to_end(&mut bayt)
        .map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bayt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn birlesik_ve_tek() {
        let j = json!({ "requested_formats": [
            { "url": "https://v", "protocol": "https", "vcodec": "avc1.64001F", "acodec": "none",
              "http_headers": { "Referer": "https://site/" } },
            { "url": "https://a", "protocol": "https", "vcodec": "none", "acodec": "mp4a.40.2" },
        ]});
        let (g, mime) = coz(&j).unwrap();
        assert_eq!(g.len(), 2);
        assert!(g[0].basliklar.contains("Referer: https://site/\r\n"));
        assert_eq!(mime, "video/mp4; codecs=\"avc1.64001F,mp4a.40.2\"");
        // HLS (tek adres, codec bilinmiyor): varsayılan H.264 + AAC.
        let (g, mime) =
            coz(&json!({ "url": "https://x/m.m3u8", "protocol": "m3u8_native" })).unwrap();
        assert_eq!(g.len(), 1);
        assert!(mime.contains("avc1.64001f"));
        // VP9 ve DASH olmaz.
        assert!(coz(
            &json!({ "url": "https://x", "protocol": "https", "vcodec": "vp9", "acodec": "opus" })
        )
        .is_none());
        assert!(coz(
            &json!({ "url": "https://x", "protocol": "http_dash_segments", "vcodec": "avc1" })
        )
        .is_none());
    }
}
