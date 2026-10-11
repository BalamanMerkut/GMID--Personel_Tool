//! ffmpeg ile küçük resim ve süre; "Discord'a sığdır" (10 MB'a sığan kopya).

use crate::util::hide_console;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

/// "Duration: 00:01:02.50," → 62.5
pub fn parse_duration(stderr: &str) -> Option<f64> {
    let i = stderr.find("Duration: ")?;
    let t = stderr[i + 10..].split(',').next()?.trim();
    let mut parts = t.split(':');
    let h: f64 = parts.next()?.parse().ok()?;
    let m: f64 = parts.next()?.parse().ok()?;
    let s: f64 = parts.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

/// "Stream #0:0: Video: h264 …, 1920x1080 [SAR …" → (1920, 1080)
pub fn parse_size(stderr: &str) -> Option<(i64, i64)> {
    let line = stderr.lines().find(|l| l.contains("Video:"))?;
    line.split([',', ' '])
        .filter_map(|tok| {
            let (w, h) = tok.split_once('x')?;
            Some((w.parse::<i64>().ok()?, h.parse::<i64>().ok()?))
        })
        .find(|(w, h)| *w >= 16 && *h >= 16)
}

pub struct Probe {
    pub duration: Option<f64>,
    pub size: Option<(i64, i64)>,
}

/// Küçük resmi (480 px genişlik) yazar; süre ve çözünürlüğü döndürür.
pub fn thumbnail(ffmpeg: &Path, input: &Path, out: &Path, video: bool) -> Result<Probe, String> {
    if let Some(d) = out.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-y"]);
    if video {
        // İlk karelerde siyah ekran olabiliyor: 1. saniyeden (kısa kliplerde baştan).
        cmd.args(["-ss", "1"]);
    }
    cmd.arg("-i")
        .arg(input)
        .args(["-frames:v", "1", "-vf", "scale=480:-2", "-q:v", "5"])
        .arg(out)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_console(&mut cmd);
    let o = cmd.output().map_err(|e| e.to_string())?;
    let err = String::from_utf8_lossy(&o.stderr);
    let probe = Probe {
        duration: if video { parse_duration(&err) } else { None },
        size: parse_size(&err),
    };
    if !out.is_file() && video {
        // Klip 1 saniyeden kısaysa baştan dene.
        let mut cmd = Command::new(ffmpeg);
        cmd.args(["-hide_banner", "-y", "-i"])
            .arg(input)
            .args(["-frames:v", "1", "-vf", "scale=480:-2", "-q:v", "5"])
            .arg(out)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_console(&mut cmd);
        let _ = cmd.status();
    }
    if out.is_file() {
        Ok(probe)
    } else {
        Err(err
            .lines()
            .last()
            .unwrap_or("küçük resim çıkarılamadı")
            .to_string())
    }
}

/// Discord'un ücretsiz yükleme sınırı.
pub const DISCORD_LIMIT: u64 = 10 * 1000 * 1000;

/// Hedef boyuta göre bit hızı ve yükseklik.
pub fn plan(duration: f64, limit: u64, factor: f64) -> (u32, u32, u32) {
    let total_kbps = (limit as f64 * 0.92 * 8.0 / 1000.0 / duration.max(1.0) * factor) as u32;
    let audio = if total_kbps < 500 { 64 } else { 96 };
    let video = total_kbps.saturating_sub(audio).max(80);
    let height = match video {
        v if v >= 3500 => 1080,
        v if v >= 1500 => 720,
        v if v >= 600 => 480,
        _ => 360,
    };
    (video, audio, height)
}

/// Discord'a sığan bir kopya üretir. `progress(0..1)`.
pub fn fit(
    ffmpeg: &Path,
    input: &Path,
    out: &Path,
    duration: f64,
    progress: &dyn Fn(f32),
) -> Result<(), String> {
    if let Some(d) = out.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let mut factor = 1.0;
    for attempt in 0..3 {
        let (video, audio, height) = plan(duration, DISCORD_LIMIT, factor);
        let mut cmd = Command::new(ffmpeg);
        cmd.args(["-hide_banner", "-y", "-i"])
            .arg(input)
            .args([
                "-map", "0:v:0", "-map", "0:a:0?", "-c:v", "libx264", "-preset", "veryfast",
                "-pix_fmt", "yuv420p",
            ])
            .args(["-b:v", &format!("{video}k")])
            .args(["-maxrate", &format!("{}k", video * 5 / 4)])
            .args(["-bufsize", &format!("{}k", video * 2)])
            .args(["-vf", &{
                // Düşük bit hızında 60 kare yerine 30: görüntü bulanıklaşmasın.
                let fps = if height <= 480 { ",fps=30" } else { "" };
                format!("scale=-2:'min(ih,{height})'{fps}")
            }])
            .args(["-c:a", "aac", "-b:a", &format!("{audio}k"), "-ac", "2"])
            .args(["-movflags", "+faststart", "-progress", "pipe:1", "-nostats"])
            .arg(out)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console(&mut cmd);
        let mut child = cmd.spawn().map_err(|e| format!("ffmpeg açılamadı: {e}"))?;
        // stderr dolup ffmpeg takılmasın.
        let stderr = child.stderr.take();
        let err_reader = std::thread::spawn(move || {
            let mut s = String::new();
            if let Some(mut e) = stderr {
                let _ = std::io::Read::read_to_string(&mut e, &mut s);
            }
            s
        });
        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(us) = line.strip_prefix("out_time_us=") {
                    if let Ok(us) = us.trim().parse::<f64>() {
                        let done = (us / 1e6 / duration.max(0.1)).clamp(0.0, 1.0) as f32;
                        progress((attempt as f32 + done) / (attempt as f32 + 1.0));
                    }
                }
            }
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        let err = err_reader.join().unwrap_or_default();
        if !status.success() {
            return Err(err
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("ffmpeg hata verdi")
                .to_string());
        }
        let size = std::fs::metadata(out).map(|m| m.len()).unwrap_or(u64::MAX);
        if size <= DISCORD_LIMIT {
            return Ok(());
        }
        // Biraz taştı: bit hızını oranla düşürüp yeniden.
        factor *= DISCORD_LIMIT as f64 / size as f64 * 0.95;
    }
    Err("Klip 10 MB'a sığdırılamadı (çok uzun olabilir)".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffmpeg_output_is_parsed() {
        let err = "Input #0, mov,mp4, from 'a.mp4':\n  Duration: 00:01:02.50, start: 0.000000, bitrate: 50000 kb/s\n  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, bt709), 2560x1440 [SAR 1:1 DAR 16:9], 49000 kb/s, 60 fps";
        assert_eq!(parse_duration(err), Some(62.5));
        assert_eq!(parse_size(err), Some((2560, 1440)));
    }

    #[test]
    fn plan_scales_down_long_clips() {
        // 30 sn → ~2.3 Mbit: 720p
        let (v, a, h) = plan(30.0, DISCORD_LIMIT, 1.0);
        assert_eq!((a, h), (96, 720));
        assert!((2000..2500).contains(&v), "{v}");
        assert_eq!(plan(90.0, DISCORD_LIMIT, 1.0).2, 480);
        assert_eq!(plan(120.0, DISCORD_LIMIT, 1.0).2, 360);
        assert_eq!(plan(600.0, DISCORD_LIMIT, 1.0).2, 360);
    }
}

#[cfg(test)]
mod real_ffmpeg {
    use super::*;

    /// `cargo test -- --ignored real_ffmpeg` (sistemde ffmpeg varsa): büyük bir klibi gerçekten sığdırır.
    #[test]
    #[ignore]
    fn fits_a_big_clip() {
        let ff = Path::new("ffmpeg");
        let dir = std::env::temp_dir().join("gmid-fit");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("Valorant 2024.10.11 - 21.33.12.02.DVR.mp4");
        let st = Command::new(ff)
            .args([
                "-y",
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
            ])
            .arg("testsrc2=size=1920x1080:rate=60:duration=40,noise=alls=40:allf=t")
            .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=40"])
            .args([
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-b:v",
                "12M",
                "-c:a",
                "aac",
                "-shortest",
            ])
            .arg(&src)
            .status()
            .unwrap();
        assert!(st.success());
        assert!(std::fs::metadata(&src).unwrap().len() > DISCORD_LIMIT);
        let p = thumbnail(ff, &src, &dir.join("t.jpg"), true).unwrap();
        assert_eq!(p.size, Some((1920, 1080)));
        let d = p.duration.unwrap();
        assert!((39.0..41.0).contains(&d), "{d}");
        let out = dir.join("out (Discord).mp4");
        let last = std::sync::Mutex::new(0.0f32);
        fit(ff, &src, &out, d, &|x| *last.lock().unwrap() = x).unwrap();
        let size = std::fs::metadata(&out).unwrap().len();
        println!(
            "sığdırılan: {size} bayt, son ilerleme {}",
            last.lock().unwrap()
        );
        assert!(size <= DISCORD_LIMIT && size > DISCORD_LIMIT / 3);
    }
}
