//! Karaoke için internetten temiz müzik: şarkının resmî "instrumental" / "karaoke" sürümü YouTube'da aranır
//! (yt-dlp), seçilen indirilir ve şarkıya hizalanır (iki kaydın ortak müziği üzerinden çapraz ilinti), böylece
//! sözlerin zamanlaması aynen tutar. Yapay zekâyla ayırmanın boğuklaşması olmaz; müzik stüdyodaki gibi temizdir.

use super::demucs::{Stereo, RATE};
use crate::downloader::tools;
use crate::util::hide_console;
use rustfft::num_complex::Complex32;
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Hizalanmış müziğin adı `muzik-internet-<n>.m4a` (her indirmede yeni ad: arayüz eskisini önbellekten çalmasın).
const PREFIX: &str = "muzik-internet";
const INFO: &str = "muzik-internet.json";
/// Hizalamada kullanılan örnekleme (44,1 kHz / 4) ve en çok kayma.
const DECIM: usize = 4;
const MAX_SHIFT_SECS: f64 = 20.0;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    /// Klasördeki dosya adı.
    pub file: String,
    pub title: String,
    pub url: String,
    /// Bulunan kayma (sn; artı: indirilen sürüm şarkıdan geç başlıyordu).
    pub offset: f64,
    /// Benzerlik 0..1 (düşükse bu kayıt şarkıyla tam uyuşmuyor olabilir).
    pub score: f64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub url: String,
    pub title: String,
    pub channel: String,
    pub duration: Option<f64>,
}

pub fn info(dir: &Path) -> Option<Info> {
    let i: Info = serde_json::from_str(&std::fs::read_to_string(dir.join(INFO)).ok()?).ok()?;
    dir.join(&i.file).is_file().then_some(i)
}

/// Kullanılacak müzik dosyası (internetten bulunan varsa).
pub fn music_file(dir: &Path) -> Option<std::path::PathBuf> {
    info(dir).map(|i| dir.join(i.file))
}

pub fn remove(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(INFO));
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_name().to_string_lossy().starts_with(PREFIX) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// YouTube'da arar (ilk 8 sonuç).
pub fn search(root: &Path, query: &str) -> Result<Vec<Hit>, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("Aranacak bir şey yaz.".into());
    }
    let mut cmd = tools::ytdlp(root);
    cmd.args(["-J", "--flat-playlist", "--no-warnings"])
        .arg(format!("ytsearch8:{q}"));
    hide_console(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("yt-dlp başlatılamadı: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "Arama yapılamadı: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("")
        ));
    }
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("Arama sonucu okunamadı: {e}"))?;
    Ok(parse_hits(&v))
}

fn parse_hits(v: &serde_json::Value) -> Vec<Hit> {
    v["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let id = e["id"].as_str()?;
            let url = e["url"]
                .as_str()
                .filter(|u| u.starts_with("http"))
                .map(str::to_string)
                .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}"));
            Some(Hit {
                url,
                title: e["title"].as_str().unwrap_or_default().to_string(),
                channel: e["channel"]
                    .as_str()
                    .or(e["uploader"].as_str())
                    .unwrap_or_default()
                    .to_string(),
                duration: e["duration"].as_f64(),
            })
        })
        .collect()
}

/// İndirir, şarkıya hizalar, `FILE` olarak kaydeder.
pub fn fetch(
    root: &Path,
    dir: &Path,
    original: &Path,
    url: &str,
    title: &str,
    emit: &dyn Fn(&str, f64),
) -> Result<Info, String> {
    if !url.starts_with("https://") {
        return Err("Geçersiz bağlantı".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    emit("download", 0.0);
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_name().to_string_lossy().starts_with("indirilen.") {
            let _ = std::fs::remove_file(e.path());
        }
    }
    let mut cmd = tools::ytdlp(root);
    cmd.args([
        "-f",
        "bestaudio/best",
        "--no-playlist",
        "--no-warnings",
        "-o",
    ])
    .arg(dir.join("indirilen.%(ext)s"))
    .arg(url);
    hide_console(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("yt-dlp başlatılamadı: {e}"))?;
    let downloaded = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name().is_some_and(|n| {
                n.to_string_lossy().starts_with("indirilen.")
                    && !n.to_string_lossy().ends_with(".part")
            })
        });
    let Some(downloaded) = downloaded.filter(|_| out.status.success()) else {
        return Err(format!(
            "İndirilemedi: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("")
        ));
    };

    emit("align", 0.0);
    let song = super::decode(root, original)?;
    let inst = super::decode(root, &downloaded);
    let _ = std::fs::remove_file(&downloaded);
    let inst = inst?;
    let (lag, score) = align(&song, &inst);
    emit("align", 0.6);
    let aligned = shift(&inst, lag, song[0].len());
    remove(dir);
    let file = format!("{PREFIX}-{}.m4a", crate::util::now_ms());
    super::encode(root, &aligned, &dir.join(&file))?;
    let info = Info {
        file,
        title: title.to_string(),
        url: url.to_string(),
        offset: lag as f64 / RATE as f64,
        score,
    };
    std::fs::write(
        dir.join(INFO),
        serde_json::to_string(&info).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    crate::log!(
        "Karaoke: internetten müzik hizalandı (kayma {:.2} sn, benzerlik {:.2})",
        info.offset,
        score
    );
    emit("align", 1.0);
    Ok(info)
}

/// Tek kanallı, seyreltilmiş (11 kHz) ilk 2 dakika.
fn envelope(st: &Stereo) -> Vec<f32> {
    let n = st[0].len().min(120 * RATE);
    (0..n / DECIM)
        .map(|i| {
            let mut sum = 0.0;
            for j in 0..DECIM {
                sum += st[0][i * DECIM + j] + st[1][i * DECIM + j];
            }
            sum / (2 * DECIM) as f32
        })
        .collect()
}

/// `inst` şarkıya göre kaç örnek kaymış (44,1 kHz) ve benzerlik (0..1).
/// Artı kayma: `inst` geç başlıyor (baştan kırpılmalı)… yani `inst[t + lag] ≈ song[t]`.
pub fn align(song: &Stereo, inst: &Stereo) -> (isize, f64) {
    let x = envelope(song);
    let y = envelope(inst);
    if x.is_empty() || y.is_empty() {
        return (0, 0.0);
    }
    let n = (x.len() + y.len()).next_power_of_two();
    let mut planner = FftPlanner::<f32>::new();
    let fwd = planner.plan_fft_forward(n);
    let inv = planner.plan_fft_inverse(n);
    let pad = |v: &[f32]| {
        let mut b: Vec<Complex32> = v.iter().map(|&a| Complex32::new(a, 0.0)).collect();
        b.resize(n, Complex32::new(0.0, 0.0));
        b
    };
    let (mut fx, mut fy) = (pad(&x), pad(&y));
    fwd.process(&mut fx);
    fwd.process(&mut fy);
    // r[k] = Σ y[t + k] · x[t]
    let mut r: Vec<Complex32> = fy.iter().zip(&fx).map(|(a, b)| a * b.conj()).collect();
    inv.process(&mut r);
    let max = (MAX_SHIFT_SECS * RATE as f64 / DECIM as f64) as isize;
    let (mut best, mut best_k) = (f32::MIN, 0isize);
    for k in -max..=max {
        let idx = if k >= 0 {
            k as usize
        } else {
            n - (-k) as usize
        };
        if idx < n && r[idx].re > best {
            best = r[idx].re;
            best_k = k;
        }
    }
    let ex: f64 = x.iter().map(|a| (*a as f64).powi(2)).sum();
    let ey: f64 = y.iter().map(|a| (*a as f64).powi(2)).sum();
    let score = (best as f64 / n as f64) / (ex * ey).sqrt().max(1e-12);
    (best_k * DECIM as isize, score.clamp(0.0, 1.0))
}

/// `inst`i `lag` kadar kaydırıp şarkının uzunluğuna getirir (eksik yer sessiz).
pub fn shift(inst: &Stereo, lag: isize, len: usize) -> Stereo {
    [0, 1].map(|c| {
        (0..len as isize)
            .map(|t| {
                let j = t + lag;
                if j >= 0 && (j as usize) < inst[c].len() {
                    inst[c][j as usize]
                } else {
                    0.0
                }
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(n: usize, seed: u32) -> Vec<f32> {
        // Müzik gibi: birkaç frekans + ritmik vuruşlar (tekrarlamayan).
        let mut s = seed;
        (0..n)
            .map(|i| {
                s = s.wrapping_mul(1_103_515_245).wrapping_add(12345);
                let t = i as f32 / RATE as f32;
                let beat = if (i / 5000) % 3 == 0 {
                    ((s >> 16) as f32 / 65536.0 - 0.5) * 0.6
                } else {
                    0.0
                };
                (t * 110.0 * std::f32::consts::TAU).sin() * 0.2
                    + (t * 233.0 * std::f32::consts::TAU).sin() * 0.1
                    + beat
            })
            .collect()
    }

    #[test]
    fn finds_offset_between_song_and_instrumental() {
        let n = 30 * RATE;
        let music = tone(n + 3 * RATE, 7);
        let voice = tone(n, 99);
        // Şarkı = müzik + ses. İnternetteki sürüm 1,37 sn geç başlıyor ve biraz kısık.
        let song: Stereo = [0, 1].map(|_| (0..n).map(|i| music[i] + 0.7 * voice[i]).collect());
        let lead = (1.37 * RATE as f64) as usize;
        let inst: Stereo = [0, 1].map(|_| {
            let mut v = vec![0.0; lead];
            v.extend(music[..n].iter().map(|m| m * 0.8));
            v
        });
        let (lag, score) = align(&song, &inst);
        assert!(
            (lag - lead as isize).abs() <= DECIM as isize,
            "kayma {lag}, beklenen {lead}"
        );
        assert!(score > 0.3, "benzerlik {score}");
        let back = shift(&inst, lag, n);
        assert_eq!(back[0].len(), n);
        assert!((back[0][RATE] - music[RATE] * 0.8).abs() < 0.05);
        // Erken başlayan sürüm (eksi kayma).
        let early: Stereo = [0, 1].map(|_| music[RATE..RATE + n].to_vec());
        let song2: Stereo = [0, 1].map(|_| music[..n].to_vec());
        let (lag2, _) = align(&song2, &early);
        assert!(
            (lag2 + RATE as isize).abs() <= DECIM as isize,
            "kayma {lag2}"
        );
    }

    #[test]
    fn parses_search_results() {
        let v = serde_json::json!({ "entries": [
            { "id": "abc", "title": "Şarkı (Instrumental)", "channel": "Kanal", "duration": 201.0, "url": "https://www.youtube.com/watch?v=abc" },
            { "id": "def", "title": "Karaoke", "uploader": "Biri" },
            { "title": "kimliksiz" }
        ]});
        let h = parse_hits(&v);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].duration, Some(201.0));
        assert_eq!(h[1].url, "https://www.youtube.com/watch?v=def");
        assert_eq!(h[1].channel, "Biri");
    }
}
