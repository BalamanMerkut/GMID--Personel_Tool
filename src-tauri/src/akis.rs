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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
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
    /// MediaSource türü; dosyanın başı (moov) inince dosyadan okunur.
    mime: Option<String>,
    /// İzleyen kalmadı: ffmpeg durdurulsun.
    durdur: Arc<AtomicBool>,
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
}

#[derive(Serialize)]
pub struct AkisDurum {
    /// Şu ana kadar yazılan bayt.
    boyut: u64,
    bitti: bool,
    hata: Option<String>,
    /// Bittiyse tam dosyanın yolu.
    yol: Option<String>,
    /// MediaSource türü (dosyanın başı inince), ör. `video/mp4; codecs="avc1.64001f,mp4a.40.2"`.
    mime: Option<String>,
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

/// Geçici klasörde en çok bu kadar video/şarkı kalır (üçüncüsü açılınca en eskisi silinir).
const SAKLA: usize = 2;
/// Bundan eski olanlar her durumda silinir.
const OMUR: Duration = Duration::from_secs(24 * 60 * 60);

/// Geçici klasörü toplar: şimdi açılan (`simdiki`, dosya adının noktaya kadarki kısmı) korunur ve en yeni sayılır;
/// en yeni [`SAKLA`] taneden fazlası ve 1 günden eskiler silinir. Çalınmakta olan dosya silinemezse sonraki sefere kalır.
pub fn onbellek_topla(dir: &std::path::Path, simdiki: &str) {
    let Ok(d) = std::fs::read_dir(dir) else {
        return;
    };
    let simdi = std::time::SystemTime::now();
    // Aynı videonun dosyaları (ör. `ad.mp4`, `ad.parca.mp4`, `ad.m4a.part`) birlikte sayılır.
    let mut gruplar: HashMap<String, (std::time::SystemTime, Vec<PathBuf>)> = HashMap::new();
    for e in d.filter_map(Result::ok) {
        let ad = e
            .file_name()
            .to_string_lossy()
            .split('.')
            .next()
            .unwrap_or("")
            .to_string();
        let zaman = e.metadata().and_then(|m| m.modified()).unwrap_or(simdi);
        let g = gruplar
            .entry(ad)
            .or_insert((std::time::UNIX_EPOCH, Vec::new()));
        g.0 = g.0.max(zaman);
        g.1.push(e.path());
    }
    let mut l: Vec<_> = gruplar.into_iter().collect();
    l.sort_by_key(|(ad, (zaman, _))| (ad != simdiki, std::cmp::Reverse(*zaman)));
    for (i, (ad, (zaman, dosyalar))) in l.into_iter().enumerate() {
        let eski = simdi.duration_since(zaman).is_ok_and(|g| g > OMUR);
        if ad != simdiki && (i >= SAKLA || eski) {
            for f in dosyalar {
                let _ = std::fs::remove_file(f);
            }
        }
    }
}

/// Oynatıcının çalamayacağı belli olan codec (VP9, AV1, HEVC, Opus…): akış denenmez.
fn uygunsuz(c: Option<&str>) -> bool {
    let k = c.unwrap_or("").to_ascii_lowercase();
    [
        "vp9", "vp09", "vp8", "av01", "av1", "hev", "hvc", "h265", "opus", "vorbis", "ac-3", "ec-3",
    ]
    .iter()
    .any(|x| k.starts_with(x))
}

struct Girdi {
    url: String,
    basliklar: String,
    /// Bu girdide görüntü var (bilinmiyorsa var sayılır).
    video: bool,
}

/// yt-dlp'nin `-j` çıktısından ffmpeg girdileri (video [+ ses]).
fn coz(j: &Value) -> Option<Vec<Girdi>> {
    let parcalar: Vec<&Value> = match j["requested_formats"].as_array() {
        Some(l) if !l.is_empty() => l.iter().collect(),
        _ => vec![j],
    };
    let mut girdiler = Vec::new();
    for p in &parcalar {
        let proto = p["protocol"].as_str().unwrap_or("");
        // Parça listeli biçimler (DASH, ISM, F4M) ffmpeg'e tek adres olarak verilemez.
        if proto.contains("dash") || proto.contains("ism") || proto.contains("f4m") {
            return None;
        }
        let (pv, pa) = (p["vcodec"].as_str(), p["acodec"].as_str());
        if uygunsuz(pv) || uygunsuz(pa) {
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
        girdiler.push(Girdi {
            url,
            basliklar,
            video: pv != Some("none"),
        });
    }
    girdiler.iter().any(|g| g.video).then_some(girdiler)
}

fn bul(veri: &[u8], ne: &[u8]) -> Option<usize> {
    veri.windows(ne.len()).position(|w| w == ne)
}

/// Dosyanın başındaki `moov` kutusundan MediaSource türü. `Ok(None)`: henüz inmedi; `Err`: oynatıcı çalamaz.
fn tur_oku(bas: &[u8]) -> Result<Option<String>, ()> {
    let Some(m) = bul(bas, b"moov").filter(|m| *m >= 4) else {
        return Ok(None);
    };
    let boy = u32::from_be_bytes([bas[m - 4], bas[m - 3], bas[m - 2], bas[m - 1]]) as usize;
    let Some(moov) = bas.get(m - 4..m - 4 + boy) else {
        return Ok(None);
    };
    // avcC: sürüm, profil, uyumluluk, seviye.
    let video = match bul(moov, b"avcC") {
        Some(i) if moov.len() >= i + 8 => format!(
            "avc1.{:02x}{:02x}{:02x}",
            moov[i + 5],
            moov[i + 6],
            moov[i + 7]
        ),
        _ => return Err(()),
    };
    if bul(moov, b"mp4a").is_some() {
        Ok(Some(format!("video/mp4; codecs=\"{video},mp4a.40.2\"")))
    } else if [&b"Opus"[..], b"ac-3", b"ec-3", b"fLaC", b".mp3"]
        .iter()
        .any(|t| bul(moov, t).is_some())
    {
        Err(())
    } else {
        Ok(Some(format!("video/mp4; codecs=\"{video}\"")))
    }
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
    onbellek_topla(&dir, &ad);
    if tam.is_file() {
        // Yeniden izlendi: en yeni sayılsın (1 gün buradan sayılır).
        if let Ok(f) = std::fs::File::options().write(true).open(&tam) {
            let _ = f.set_modified(std::time::SystemTime::now());
        }
        return Ok(Baslangic {
            id: ad,
            hazir: Some(tam.to_string_lossy().into_owned()),
        });
    }
    // Süren akış.
    if let Some(d) = kilit().get(&ad).filter(|d| d.hata.is_none() && !d.bitti) {
        d.durdur.store(false, Ordering::Relaxed);
        return Ok(Baslangic {
            id: ad.clone(),
            hazir: None,
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
        let girdiler = coz(&j).ok_or(AKIS_YOK)?;
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
        // Görüntü görüntülü girdiden, ses (varsa) diğerinden.
        let v = girdiler.iter().position(|g| g.video).unwrap_or(0);
        let a = if girdiler.len() > 1 { 1 - v.min(1) } else { 0 };
        ff.args(["-map", &format!("{v}:v:0"), "-map", &format!("{a}:a:0?")]);
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
        let durdur = Arc::new(AtomicBool::new(false));
        kilit().insert(
            ad.clone(),
            Durum {
                yol: parca.clone(),
                bitti: false,
                hata: None,
                mime: None,
                durdur: durdur.clone(),
            },
        );
        let id = ad.clone();
        let hata_oku = cocuk.stderr.take().map(|mut e| {
            std::thread::spawn(move || {
                let mut s = String::new();
                let _ = e.read_to_string(&mut s);
                s
            })
        });
        std::thread::spawn(move || {
            // Kimse izlemiyorsa (oda değişti, şarkı geçildi) indirme durdurulur.
            let durum = loop {
                match cocuk.try_wait() {
                    Ok(Some(s)) => break Some(s),
                    Ok(None) if durdur.load(Ordering::Relaxed) => {
                        let _ = cocuk.kill();
                        let _ = cocuk.wait();
                        break None;
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(300)),
                    Err(_) => break None,
                }
            };
            let hata = hata_oku.and_then(|t| t.join().ok()).unwrap_or_default();
            let mut k = kilit();
            if durum.is_none() {
                k.remove(&id);
                let _ = std::fs::remove_file(&parca);
                return;
            }
            if let Some(d) = k.get_mut(&id) {
                if durum.is_some_and(|s| s.success()) && std::fs::rename(&parca, &tam).is_ok() {
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
        Ok(Baslangic { id: ad, hazir: None })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn oda_akis_durum(id: String) -> CmdResult<AkisDurum> {
    let mut k = kilit();
    let d = k.get_mut(&id).ok_or("Akış bulunamadı")?;
    let boyut = std::fs::metadata(&d.yol).map(|m| m.len()).unwrap_or(0);
    if d.mime.is_none() && boyut > 0 {
        let mut bas = Vec::new();
        if let Ok(f) = std::fs::File::open(&d.yol) {
            let _ = f.take(1024 * 1024).read_to_end(&mut bas);
        }
        match tur_oku(&bas) {
            Ok(m) => d.mime = m,
            // Oynatıcının çalamadığı biçim: arayüz önce tamamını indirme yoluna geçer.
            Err(()) => {
                d.durdur.store(true, Ordering::Relaxed);
                return Err(AKIS_YOK.into());
            }
        }
    }
    Ok(AkisDurum {
        boyut: if d.mime.is_some() { boyut } else { 0 },
        bitti: d.bitti,
        hata: d.hata.clone(),
        yol: d.bitti.then(|| d.yol.to_string_lossy().into_owned()),
        mime: d.mime.clone(),
    })
}

/// İzleme bıraktı (şarkı değişti, odadan çıkıldı): inmekte olan video durdurulur.
#[tauri::command]
pub fn oda_akis_birak(id: String) {
    if let Some(d) = kilit().get(&id).filter(|d| !d.bitti) {
        d.durdur.store(true, Ordering::Relaxed);
    }
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
        let g = coz(&j).unwrap();
        assert_eq!(g.len(), 2);
        assert!(g[0].video && !g[1].video);
        assert!(g[0].basliklar.contains("Referer: https://site/\r\n"));
        // HLS (tek adres, codec bilinmiyor): denenir, tür dosyadan okunur.
        assert_eq!(
            coz(&json!({ "url": "https://x/m.m3u8", "protocol": "m3u8_native" }))
                .unwrap()
                .len(),
            1
        );
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

    fn kutu(tur: &[u8], ic: &[u8]) -> Vec<u8> {
        let mut v = ((ic.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(tur);
        v.extend_from_slice(ic);
        v
    }

    #[test]
    fn toplama() {
        let dir = std::env::temp_dir().join(format!("gmid-akis-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let yaz = |ad: &str, once: u64| {
            let f = std::fs::File::create(dir.join(ad)).unwrap();
            f.set_modified(std::time::SystemTime::now() - Duration::from_secs(once))
                .unwrap();
        };
        yaz("a.mp4", 300);
        yaz("b.mp4", 200);
        yaz("b.m4a.part", 200);
        yaz("c.parca.mp4", 0);
        yaz("eski.mp4", 2 * 24 * 3600);
        // c açıldı: c ve en yeni bir tane (b) kalır; a ve 1 günden eski silinir.
        onbellek_topla(&dir, "c");
        let mut kalan: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        kalan.sort();
        assert_eq!(kalan, ["b.m4a.part", "b.mp4", "c.parca.mp4"]);
        // Eski bir videoya dönülse de o korunur.
        onbellek_topla(&dir, "a");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tur() {
        let ftyp = kutu(b"ftyp", b"isom");
        let avcc = kutu(b"avcC", &[1, 0x64, 0x00, 0x1f, 0xff]);
        let ses = kutu(b"mp4a", &[0; 8]);
        let tam = [ftyp.clone(), kutu(b"moov", &[avcc.clone(), ses].concat())].concat();
        assert_eq!(
            tur_oku(&tam).unwrap().unwrap(),
            "video/mp4; codecs=\"avc1.64001f,mp4a.40.2\""
        );
        // Sessiz video.
        let sessiz = [ftyp.clone(), kutu(b"moov", &avcc)].concat();
        assert_eq!(
            tur_oku(&sessiz).unwrap().unwrap(),
            "video/mp4; codecs=\"avc1.64001f\""
        );
        // Yarım inmiş baş: bekle.
        assert_eq!(tur_oku(&tam[..tam.len() - 3]), Ok(None));
        // HEVC: olmaz.
        let hevc = [ftyp, kutu(b"moov", &kutu(b"hvcC", &[1; 8]))].concat();
        assert!(tur_oku(&hevc).is_err());
    }
}
