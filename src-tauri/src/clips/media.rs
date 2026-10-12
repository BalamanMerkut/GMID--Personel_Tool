//! ffmpeg ile küçük resim ve süre; klip kırpma ve "Discord'a sığdır" (seçilen boyuta sığan kopya).

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

/// Discord'un ücretsiz yükleme sınırı (testlerde kullanılır; asıl sınır ayardan gelir).
#[cfg(test)]
pub const DISCORD_LIMIT: u64 = 10 * 1000 * 1000;

/// "…, 59.94 fps, …" → 59.94
pub fn parse_fps(stderr: &str) -> Option<f64> {
    let line = stderr.lines().find(|l| l.contains("Video:"))?;
    line.split(',')
        .map(str::trim)
        .find_map(|t| t.strip_suffix(" fps")?.trim().parse::<f64>().ok())
        .filter(|f| *f > 1.0)
}

/// Süre, çözünürlük ve kare hızı (`ffmpeg -i` çıktısından; dosyaya bir şey yazılmaz).
pub fn probe(ffmpeg: &Path, input: &Path) -> (Option<f64>, Option<(i64, i64)>, Option<f64>) {
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-i"])
        .arg(input)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_console(&mut cmd);
    let err = cmd
        .output()
        .map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
        .unwrap_or_default();
    (parse_duration(&err), parse_size(&err), parse_fps(&err))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Codec {
    /// Aynı boyutta belirgin şekilde daha net (Discord masaüstü, tarayıcı, yeni telefonlar oynatır).
    #[default]
    Av1,
    /// Her yerde oynar.
    H264,
}

impl Codec {
    /// Bu kodek için "iyi görünür" en düşük piksel başına bit (ölçümle seçildi: aynı boyutta AV1 1080p,
    /// H.264 480p'den daha net çıktı).
    fn min_bpp(self) -> f64 {
        match self {
            Codec::Av1 => 0.022,
            Codec::H264 => 0.065,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub video: u32,
    pub audio: u32,
    pub height: u32,
    pub fps: u32,
    /// great | good | ok | poor
    pub quality: &'static str,
}

const HEIGHTS: &[u32] = &[1440, 1080, 900, 720, 540, 480, 360];

/// Hedef boyuta göre bit hızı, çözünürlük ve kare hızı. Önce netlik: 720p60 yerine 1080p30 seçilir.
#[cfg(test)]
pub fn plan(
    duration: f64,
    limit: u64,
    src: (u32, u32),
    src_fps: f64,
    codec: Codec,
    factor: f64,
) -> Plan {
    plan_with(duration, limit, src, src_fps, codec, factor, false)
}

/// `smooth`: kaynağın kare hızı (ör. 60) korunur, gerekirse çözünürlük düşer ("akıcı").
pub fn plan_with(
    duration: f64,
    limit: u64,
    src: (u32, u32),
    src_fps: f64,
    codec: Codec,
    factor: f64,
    smooth: bool,
) -> Plan {
    let total_kbps = (limit as f64 * 0.94 * 8.0 / 1000.0 / duration.max(1.0) * factor) as u32;
    let audio = match total_kbps {
        k if k < 600 => 64,
        k if k < 3000 => 96,
        _ => 128,
    };
    let video = total_kbps.saturating_sub(audio).max(60);
    let (sw, sh) = (src.0.max(16), src.1.max(16));
    let aspect = sw as f64 / sh as f64;
    let src_fps = src_fps.clamp(1.0, 60.0).round() as u32;
    let bpp =
        |h: u32, fps: u32| video as f64 * 1000.0 / (h as f64 * aspect * h as f64 * fps as f64);
    let mut pick = None;
    'outer: for &h in HEIGHTS.iter().filter(|&&h| h <= sh) {
        let rates = if smooth {
            vec![src_fps]
        } else {
            vec![src_fps, src_fps.min(30)]
        };
        for fps in rates {
            if bpp(h, fps) >= codec.min_bpp() {
                pick = Some((h, fps));
                break 'outer;
            }
        }
    }
    let (height, fps) =
        pick.unwrap_or((sh.min(360), if smooth { src_fps } else { src_fps.min(30) }));
    let quality = match (pick.is_some(), height) {
        (false, _) => "poor",
        (true, h) if h >= 1080 || h >= sh => "great",
        (true, h) if h >= 720 => "good",
        (true, h) if h >= 480 => "ok",
        _ => "poor",
    };
    Plan {
        video,
        audio,
        height,
        fps,
        quality,
    }
}

pub struct Job {
    pub start: f64,
    pub end: f64,
    /// `None`: boyut sınırı yok, kırpılmış yüksek kaliteli kopya.
    pub limit: Option<u64>,
    pub codec: Codec,
    pub src: (u32, u32),
    pub src_fps: f64,
    /// Kare hızını koru (akıcı).
    pub smooth: bool,
}

fn null_sink() -> &'static str {
    if cfg!(windows) {
        "NUL"
    } else {
        "/dev/null"
    }
}

/// ffmpeg'i çalıştırır; `-progress` çıktısından `done(0..1)`.
fn run(mut cmd: Command, duration: f64, done: &dyn Fn(f32)) -> Result<(), String> {
    cmd.args(["-progress", "pipe:1", "-nostats"])
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
                    done((us / 1e6 / duration.max(0.1)).clamp(0.0, 1.0) as f32);
                }
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let err = err_reader.join().unwrap_or_default();
    if status.success() {
        Ok(())
    } else {
        Err(err
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty() && !l.starts_with("Svt[info]"))
            .unwrap_or("ffmpeg hata verdi")
            .to_string())
    }
}

fn base(ffmpeg: &Path, input: &Path, job: &Job) -> Command {
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-y", "-loglevel", "error"]);
    // Girişten önce -ss: hızlı ve (yeniden kodlandığı için) kare doğruluğunda.
    if job.start > 0.05 {
        cmd.args(["-ss", &format!("{:.3}", job.start)]);
    }
    cmd.arg("-i").arg(input);
    cmd.args(["-t", &format!("{:.3}", (job.end - job.start).max(0.1))]);
    cmd.args(["-map", "0:v:0", "-map", "0:a:0?", "-pix_fmt", "yuv420p"]);
    cmd
}

/// Klibi kırpar ve (sınır varsa) o boyuta sığdırır. `progress(0..1)`.
pub fn encode(
    ffmpeg: &Path,
    input: &Path,
    out: &Path,
    job: &Job,
    progress: &dyn Fn(f32),
) -> Result<Plan, String> {
    if let Some(d) = out.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let duration = (job.end - job.start).max(0.1);
    let Some(limit) = job.limit else {
        // Yalnız kırpma: neredeyse kayıpsız.
        let mut cmd = base(ffmpeg, input, job);
        cmd.args(["-c:v", "libx264", "-preset", "fast", "-crf", "18"])
            .args(["-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart"])
            .arg(out);
        run(cmd, duration, progress)?;
        return Ok(Plan {
            video: 0,
            audio: 192,
            height: job.src.1,
            fps: job.src_fps.round() as u32,
            quality: "great",
        });
    };
    let passlog = std::env::temp_dir().join(format!("gmid-pass-{}", std::process::id()));
    // SVT-AV1'in bit hızı denetimi biraz taşabiliyor: baştan küçük bir pay (yeniden kodlamayı önler).
    let mut factor = if job.codec == Codec::Av1 { 0.93 } else { 1.0 };
    let mut last = None;
    for attempt in 0..3 {
        let p = plan_with(
            duration,
            limit,
            job.src,
            job.src_fps,
            job.codec,
            factor,
            job.smooth,
        );
        last = Some(p);
        let vf = format!("scale=-2:{},fps={}", p.height, p.fps);
        let rate = |cmd: &mut Command| {
            cmd.args(["-b:v", &format!("{}k", p.video)]);
        };
        // Her deneme ilerlemenin bir dilimi; deneme tekrarlanırsa çubuk geri gitmesin.
        let slice = |a: f32, b: f32| {
            let lo = attempt as f32 / (attempt as f32 + 1.0);
            move |x: f32| progress(lo + (1.0 - lo) * (a + (b - a) * x))
        };
        match job.codec {
            Codec::H264 => {
                // İki geçiş: aynı boyutta tek geçişten daha net, hedef boyuta da daha sadık.
                let mut p1 = base(ffmpeg, input, job);
                p1.args(["-c:v", "libx264", "-preset", "medium", "-vf", &vf]);
                rate(&mut p1);
                p1.args(["-pass", "1", "-passlogfile"]).arg(&passlog).args([
                    "-an",
                    "-f",
                    "null",
                    null_sink(),
                ]);
                run(p1, duration, &slice(0.0, 0.45))?;
                let mut p2 = base(ffmpeg, input, job);
                p2.args(["-c:v", "libx264", "-preset", "medium", "-vf", &vf]);
                rate(&mut p2);
                p2.args(["-pass", "2", "-passlogfile"]).arg(&passlog);
                p2.args(["-c:a", "aac", "-b:a", &format!("{}k", p.audio), "-ac", "2"])
                    .args(["-movflags", "+faststart"])
                    .arg(out);
                run(p2, duration, &slice(0.45, 1.0))?;
            }
            Codec::Av1 => {
                let mut c = base(ffmpeg, input, job);
                // Uzun kliplerde biraz daha hızlı ön ayar.
                let preset = if duration > 180.0 { "9" } else { "7" };
                c.args([
                    "-c:v",
                    "libsvtav1",
                    "-preset",
                    preset,
                    "-svtav1-params",
                    "rc=1",
                ])
                .args(["-vf", &vf]);
                rate(&mut c);
                c.args(["-c:a", "aac", "-b:a", &format!("{}k", p.audio), "-ac", "2"])
                    .args(["-movflags", "+faststart"])
                    .arg(out);
                run(c, duration, &slice(0.0, 1.0))?;
            }
        }
        for ext in ["-0.log", "-0.log.mbtree", ".log", ".log.mbtree"] {
            let _ = std::fs::remove_file(format!("{}{ext}", passlog.display()));
        }
        let size = std::fs::metadata(out).map(|m| m.len()).unwrap_or(u64::MAX);
        if size <= limit {
            progress(1.0);
            return Ok(p);
        }
        // Biraz taştı: bit hızını oranla düşürüp yeniden.
        factor *= limit as f64 / size as f64 * 0.95;
    }
    let _ = last;
    Err(format!(
        "Klip {} MB'a sığdırılamadı; seçimi kısaltmayı dene",
        limit / 1_000_000
    ))
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
    fn smooth_keeps_frame_rate() {
        let sharp = plan_with(
            60.0,
            DISCORD_LIMIT,
            (1920, 1080),
            60.0,
            Codec::Av1,
            1.0,
            false,
        );
        let smooth = plan_with(
            60.0,
            DISCORD_LIMIT,
            (1920, 1080),
            60.0,
            Codec::Av1,
            1.0,
            true,
        );
        assert_eq!(sharp.fps, 30, "{sharp:?}");
        assert_eq!(smooth.fps, 60, "{smooth:?}");
        assert!(smooth.height < sharp.height, "{smooth:?} {sharp:?}");
    }

    #[test]
    fn plan_prefers_sharpness_and_av1_keeps_resolution() {
        let src = (2560, 1440);
        // 30 sn, 10 MB: AV1 1080p'yi korur, H.264 küçültür.
        let av1 = plan(30.0, DISCORD_LIMIT, src, 60.0, Codec::Av1, 1.0);
        let h264 = plan(30.0, DISCORD_LIMIT, src, 60.0, Codec::H264, 1.0);
        assert!(av1.height >= 1080, "{av1:?}");
        assert!(h264.height < av1.height, "{h264:?}");
        assert!((2000..2600).contains(&(av1.video + av1.audio)), "{av1:?}");
        // Kısa klip: 60 kare korunur.
        let short = plan(8.0, DISCORD_LIMIT, (1920, 1080), 60.0, Codec::Av1, 1.0);
        assert_eq!(
            (short.height, short.fps, short.quality),
            (1080, 60, "great")
        );
        // Çok uzun klip 10 MB'a: bulanık uyarısı.
        let long = plan(600.0, DISCORD_LIMIT, (1920, 1080), 60.0, Codec::H264, 1.0);
        assert_eq!(long.quality, "poor");
        // Nitro (500 MB): 5 dakikalık klip bile 1080p60.
        let nitro = plan(
            300.0,
            500 * 1000 * 1000,
            (1920, 1080),
            60.0,
            Codec::H264,
            1.0,
        );
        assert_eq!((nitro.height, nitro.fps), (1080, 60));
        // Kaynaktan büyük çözünürlüğe çıkılmaz.
        assert!(plan(5.0, DISCORD_LIMIT, (1280, 720), 30.0, Codec::Av1, 1.0).height <= 720);
    }

    #[test]
    fn fps_is_parsed() {
        let err = "  Stream #0:0: Video: h264 (High), yuv420p, 1920x1080, 49000 kb/s, 59.94 fps, 59.94 tbr";
        assert_eq!(parse_fps(err), Some(59.94));
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
        for codec in [Codec::H264, Codec::Av1] {
            let out = dir.join(format!("out {codec:?} (Discord).mp4"));
            let last = std::sync::Mutex::new(0.0f32);
            let job = Job {
                start: 5.0,
                end: d,
                limit: Some(DISCORD_LIMIT),
                codec,
                src: (1920, 1080),
                src_fps: 60.0,
                smooth: false,
            };
            let p = encode(ff, &src, &out, &job, &|x| *last.lock().unwrap() = x).unwrap();
            let size = std::fs::metadata(&out).unwrap().len();
            println!(
                "{codec:?}: {size} bayt, {p:?}, son ilerleme {}",
                last.lock().unwrap()
            );
            assert!(size <= DISCORD_LIMIT && size > DISCORD_LIMIT / 3);
            let (dur, _, _) = probe(ff, &out);
            assert!((34.0..36.0).contains(&dur.unwrap()), "{dur:?}");
        }
    }
}
