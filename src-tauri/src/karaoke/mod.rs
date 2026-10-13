//! Karaoke: şarkının sesini (vokal) ayırıp yalnız müziği çalar; sözler akar, mikrofonla söylenen kaydedilip
//! müzikle karıştırılır. Ayırma Meta'nın Demucs modeliyle (HTDemucs, ONNX) bu bilgisayarda yapılır:
//! ekran kartı varsa DirectML ile (NVIDIA, AMD, Intel), yoksa işlemciyle. Şarkı başına bir kez yapılır,
//! sonuç `Belgeler\GMID Tool\Karaoke\<kayıt>` içinde saklanır.
//!
//! İlk kullanımda inenler (`Araclar\karaoke`): ONNX Runtime + DirectML (PyPI'deki resmî paketten, ~25 MB) ve
//! model (npm'deki `demucs` paketinden, ~98 MB). İkisi de sabit sürüm ve SHA-256 ile doğrulanır.

pub mod demucs;
pub mod dsp;

use crate::downloader::tools;
use crate::util::{hide_console, new_id};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

const RUNTIME_URL: &str = "https://files.pythonhosted.org/packages/88/ea/33814eb0ec96775eda4c1d30b0d86e91d7d2cd0d84c66d3915aef0e06fa3/onnxruntime_directml-1.24.4-cp312-cp312-win_amd64.whl";
const RUNTIME_SHA256: &str = "f2ecb68b7b7b259d2ef3112ae760149f9b5a1e7c0fbb73d539da6250a648a614";
const RUNTIME_FILES: [&str; 3] = [
    "onnxruntime.dll",
    "DirectML.dll",
    "onnxruntime_providers_shared.dll",
];
const MODEL_URL: &str = "https://registry.npmjs.org/demucs/-/demucs-1.0.0.tgz";
const MODEL_SHA256: &str = "83aadd00d48cd2a0c2e81e2a35616f4fa41a41f5c07efcc4a5c1959377b4ec42";
const MODEL_FILE: &str = "htdemucs.onnx";
/// Hazır şarkının dosyaları.
const MUSIC: &str = "muzik.m4a";
const VOCALS: &str = "vokal.m4a";
/// Ayırma yöntemi değişince eski (bozuk olabilecek) sonuçlar yeniden hazırlanır.
const VERSION: &str = "2";
const VERSION_FILE: &str = "surum.txt";

#[derive(Default)]
pub struct Karaoke {
    /// Aynı anda tek ayırma (bellek ve işlemci için).
    busy: Mutex<Option<String>>,
    cancel: AtomicBool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Bu şarkı hazır mı (müzik ve vokal dosyaları).
    pub ready: bool,
    pub music: Option<String>,
    pub vocals: Option<String>,
    /// İlk kullanım indirmesi gerekiyor mu (yaklaşık MB).
    pub download_mb: u32,
    /// Şu an ayrılan şarkı.
    pub busy: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    id: &'a str,
    /// runtime | model | decode | separate | save
    stage: &'a str,
    progress: f64,
    /// Ayırma ekran kartıyla mı yapılıyor.
    gpu: bool,
}

pub fn tools_dir(root: &Path) -> PathBuf {
    tools::dir(root).join("karaoke")
}

pub fn song_dir(root: &Path, id: &str) -> PathBuf {
    root.join("Karaoke").join(id)
}

pub fn recordings_dir(root: &Path) -> PathBuf {
    root.join("Karaoke").join("Kayıtlar")
}

fn runtime_ready(root: &Path) -> bool {
    let d = tools_dir(root);
    if cfg!(windows) {
        RUNTIME_FILES.iter().all(|f| d.join(f).is_file())
    } else {
        std::env::var_os("ORT_DYLIB_PATH").is_some()
    }
}

fn model_ready(root: &Path) -> bool {
    tools_dir(root).join(MODEL_FILE).is_file()
}

impl Karaoke {
    pub fn status(&self, root: &Path, id: &str) -> Status {
        let d = song_dir(root, id);
        let file = |n: &str| {
            let p = d.join(n);
            p.is_file().then(|| p.to_string_lossy().into_owned())
        };
        let current =
            std::fs::read_to_string(d.join(VERSION_FILE)).is_ok_and(|v| v.trim() == VERSION);
        let (music, vocals) = if current {
            (file(MUSIC), file(VOCALS))
        } else {
            (None, None)
        };
        let mut mb = 0;
        if !runtime_ready(root) {
            mb += 25;
        }
        if !model_ready(root) {
            mb += 98;
        }
        Status {
            ready: music.is_some() && vocals.is_some(),
            music,
            vocals,
            download_mb: mb,
            busy: self.busy.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Şarkıyı ayırır (gerekirse önce indirir). `cpu`: ekran kartını hiç deneme. Bitince `status`.
    pub fn prepare(
        &self,
        app: &AppHandle,
        root: &Path,
        id: &str,
        file: &Path,
        cpu: bool,
    ) -> Result<Status, String> {
        {
            let mut busy = self.busy.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(other) = busy.as_ref() {
                return Err(if other == id {
                    "Bu şarkı zaten hazırlanıyor.".into()
                } else {
                    "Başka bir şarkı hazırlanıyor; bitince dene.".into()
                });
            }
            *busy = Some(id.to_string());
        }
        self.cancel.store(false, Ordering::SeqCst);
        let r = self.run(app, root, id, file, cpu);
        *self.busy.lock().unwrap_or_else(|e| e.into_inner()) = None;
        if let Err(e) = &r {
            crate::log!("Karaoke hazırlanamadı: {e}");
        }
        r.map(|_| self.status(root, id))
    }

    fn run(
        &self,
        app: &AppHandle,
        root: &Path,
        id: &str,
        file: &Path,
        cpu: bool,
    ) -> Result<(), String> {
        let gpu = std::cell::Cell::new(false);
        let emit = |stage: &str, progress: f64| {
            let _ = app.emit(
                "karaoke-progress",
                Progress {
                    id,
                    stage,
                    progress,
                    gpu: gpu.get(),
                },
            );
        };
        install(root, &|stage, p| emit(stage, p))?;
        let dir = song_dir(root, id);
        let _ = std::fs::remove_dir_all(&dir);

        emit("decode", 0.0);
        let mix = decode(root, file)?;
        if mix[0].len() < demucs::RATE {
            return Err("Ses çok kısa.".into());
        }

        // Ekran kartı (DirectML) bazı kartlarda/sürücülerde yanlış sonuç verebiliyor: önce şarkının ortasından
        // kısa bir parça hem ekran kartıyla hem işlemciyle ayrılır; sonuçlar tutmazsa işlemciyle devam edilir.
        let mut chosen = None;
        if cfg!(windows) && !cpu && std::env::var_os("GMID_KARAOKE_CPU").is_none() {
            match session(root, true) {
                Ok(mut g) => {
                    emit("check", 0.0);
                    let probe = excerpt(&mix);
                    let on_gpu = demucs::separate(&mut g, &probe, &[demucs::VOCALS], |_, _| true);
                    emit("check", 0.5);
                    let mut c = session(root, false)?;
                    let on_cpu = demucs::separate(&mut c, &probe, &[demucs::VOCALS], |_, _| true)?;
                    let verdict = match on_gpu {
                        Ok(v) => agreement(&on_cpu[0], &v[0]),
                        Err(e) => {
                            crate::log!("Karaoke: ekran kartında hata: {e}");
                            -1.0
                        }
                    };
                    crate::log!("Karaoke: ekran kartı ↔ işlemci uyumu {verdict:.1} dB");
                    chosen = Some(if verdict >= 20.0 {
                        drop(c);
                        (g, true)
                    } else {
                        crate::log!("Karaoke: ekran kartının sonucu tutmadı, işlemciyle ayrılıyor");
                        drop(g);
                        (c, false)
                    });
                }
                Err(e) => crate::log!("Karaoke: ekran kartı kullanılamadı, işlemciyle: {e}"),
            }
        }
        let (mut session, on_gpu) = match chosen {
            Some(s) => s,
            None => (session(root, false)?, false),
        };
        gpu.set(on_gpu);
        emit("separate", 0.0);
        let vocals = demucs::separate(&mut session, &mix, &[demucs::VOCALS], |done, total| {
            emit("separate", done as f64 / total as f64);
            !self.cancel.load(Ordering::SeqCst)
        })?
        .remove(0);
        drop(session);
        sane(&mix, &vocals)?;

        emit("save", 0.0);
        // Müzik = karışım − vokal (Demucs'un iki kaynaklı çıktısıyla aynı; kalan her şey müzikte kalır).
        let music: demucs::Stereo =
            [0, 1].map(|c| mix[c].iter().zip(&vocals[c]).map(|(m, v)| m - v).collect());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        encode(root, &vocals, &dir.join(VOCALS))?;
        encode(root, &music, &dir.join(MUSIC))?;
        std::fs::write(dir.join(VERSION_FILE), VERSION).map_err(|e| e.to_string())?;
        crate::log!(
            "Karaoke hazır ({})",
            if on_gpu { "ekran kartı" } else { "işlemci" }
        );
        emit("save", 1.0);
        Ok(())
    }
}

/// Doğrulama için şarkının ortasından tek parçalık kesit.
fn excerpt(mix: &demucs::Stereo) -> demucs::Stereo {
    let n = mix[0].len();
    let len = (demucs::SEGMENT * 3 / 4).min(n);
    let start = (n - len) / 2;
    [0, 1].map(|c| mix[c][start..start + len].to_vec())
}

/// İki ayırma sonucunun birbirine yakınlığı (dB; yüksek = aynı). Ses yoksa (sessiz kesit) tam uyum sayılır.
fn agreement(reference: &demucs::Stereo, other: &demucs::Stereo) -> f64 {
    let (mut sig, mut err) = (0f64, 0f64);
    for c in 0..2 {
        for (a, b) in reference[c].iter().zip(&other[c]) {
            if !b.is_finite() {
                return -1.0;
            }
            sig += (*a as f64).powi(2);
            err += ((a - b) as f64).powi(2);
        }
    }
    if sig < 1e-6 {
        return if err < 1e-6 { 99.0 } else { -1.0 };
    }
    10.0 * (sig / err.max(1e-12)).log10()
}

/// Bozuk sonucu (NaN, karışımdan çok güçlü "vokal") kaydetmeden durdurur: kulak sağlığı için.
fn sane(mix: &demucs::Stereo, vocals: &demucs::Stereo) -> Result<(), String> {
    let energy = |s: &demucs::Stereo| s.iter().flatten().map(|x| (*x as f64).powi(2)).sum::<f64>();
    let bad = vocals.iter().flatten().any(|x| !x.is_finite());
    if bad || energy(vocals) > 1.5 * energy(mix) + 1e-3 {
        return Err(
            "Ses ayırma bozuk sonuç verdi; kaydedilmedi. \"İşlemciyle hazırla\" ile yeniden dene."
                .into(),
        );
    }
    Ok(())
}

/// ONNX Runtime ve modeli (yoksa) indirir.
fn install(root: &Path, emit: &dyn Fn(&str, f64)) -> Result<(), String> {
    let dir = tools_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if !runtime_ready(root) {
        if !cfg!(windows) {
            return Err("Karaoke şimdilik yalnızca Windows'ta.".into());
        }
        let tmp = dir.join("onnxruntime.indiriliyor");
        let r = fetch(RUNTIME_URL, RUNTIME_SHA256, &tmp, &|p| emit("runtime", p))
            .and_then(|_| tools::extract(&tmp, &dir, &RUNTIME_FILES));
        let _ = std::fs::remove_file(&tmp);
        r.map_err(|e| format!("Ses ayırma programı indirilemedi: {e}"))?;
    }
    if !model_ready(root) {
        let tmp = dir.join("demucs.indiriliyor");
        let r = fetch(MODEL_URL, MODEL_SHA256, &tmp, &|p| emit("model", p))
            .and_then(|_| untar_one(&tmp, MODEL_FILE, &dir.join(MODEL_FILE)));
        let _ = std::fs::remove_file(&tmp);
        r.map_err(|e| format!("Ses ayırma modeli indirilemedi: {e}"))?;
    }
    Ok(())
}

fn fetch(url: &str, sha: &str, dest: &Path, progress: &dyn Fn(f64)) -> Result<(), String> {
    tools::download(url, dest, |d, t| {
        progress(if t > 0 { d as f64 / t as f64 } else { 0.0 })
    })?;
    let mut f = std::fs::File::open(dest).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let got: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if got != sha {
        return Err("İnen dosya bozuk (doğrulama tutmadı); yeniden dene.".into());
    }
    Ok(())
}

/// `.tgz` içinden adı `name` ile biten tek dosyayı çıkarır.
fn untar_one(tgz: &Path, name: &str, dest: &Path) -> Result<(), String> {
    let f = std::fs::File::open(tgz).map_err(|e| e.to_string())?;
    let mut ar = tar::Archive::new(flate2::read::GzDecoder::new(std::io::BufReader::new(f)));
    for entry in ar.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry
            .path()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into_owned();
        if path.ends_with(&format!("/{name}")) || path == name {
            let tmp = dest.with_extension("yaziliyor");
            let mut out = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
            drop(out);
            return tools::replace(&tmp, dest);
        }
    }
    Err("Arşivde model yok".into())
}

/// ONNX Runtime'ı yükler (bir kez) ve oturum açar: `gpu` ise ekran kartıyla (DirectML), değilse işlemciyle.
fn session(root: &Path, gpu: bool) -> Result<ort::session::Session, String> {
    use ort::session::Session;
    static INIT: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    let dir = tools_dir(root);
    INIT.get_or_init(|| {
        let lib = if cfg!(windows) {
            preload(&dir.join("DirectML.dll"));
            dir.join("onnxruntime.dll").to_string_lossy().into_owned()
        } else {
            std::env::var("ORT_DYLIB_PATH").unwrap_or_default()
        };
        ort::init_from(lib)
            .with_name("gmid-karaoke")
            .commit()
            .map(|_| ())
            .map_err(|e| format!("Ses ayırma programı açılamadı: {e}"))
    })
    .clone()?;
    let model = dir.join(MODEL_FILE);
    if gpu {
        use ort::execution_providers::DirectMLExecutionProvider;
        return Session::builder()
            .and_then(|b| b.with_memory_pattern(false))
            .and_then(|b| b.with_parallel_execution(false))
            .and_then(|b| {
                b.with_execution_providers([DirectMLExecutionProvider::default()
                    .build()
                    .error_on_failure()])
            })
            .and_then(|b| b.commit_from_file(&model))
            .map_err(|e| e.to_string());
    }
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    Session::builder()
        .and_then(|b| b.with_intra_threads(threads))
        .and_then(|b| b.commit_from_file(&model))
        .map_err(|e| format!("Model açılamadı: {e}"))
}

/// Sistemdeki eski DirectML.dll yerine indirileni kullanması için önceden yüklenir.
fn preload(_path: &Path) {
    #[cfg(windows)]
    unsafe {
        use windows::core::HSTRING;
        let _ =
            windows::Win32::System::LibraryLoader::LoadLibraryW(&HSTRING::from(_path.as_os_str()));
    }
}

fn ffmpeg(root: &Path) -> Result<PathBuf, String> {
    tools::ffmpeg_dir(root)
        .map(|d| {
            d.join(if cfg!(windows) {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            })
        })
        .ok_or_else(|| "FFmpeg yok: Video İndirici'nin araçlarını kur.".to_string())
}

/// Dosyayı 44,1 kHz stereo `f32` olarak okur.
fn decode(root: &Path, file: &Path) -> Result<demucs::Stereo, String> {
    let mut cmd = Command::new(ffmpeg(root)?);
    cmd.args(["-nostdin", "-loglevel", "error", "-i"])
        .arg(file)
        .args(["-vn", "-ac", "2", "-ar", "44100", "-f", "f32le", "-"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("FFmpeg başlatılamadı: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "Ses okunamadı: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("")
        ));
    }
    let n = out.stdout.len() / 8;
    let mut st: demucs::Stereo = [Vec::with_capacity(n), Vec::with_capacity(n)];
    for fr in out.stdout.chunks_exact(8) {
        st[0].push(f32::from_le_bytes([fr[0], fr[1], fr[2], fr[3]]));
        st[1].push(f32::from_le_bytes([fr[4], fr[5], fr[6], fr[7]]));
    }
    Ok(st)
}

/// Stereo sesi AAC (.m4a) olarak yazar.
fn encode(root: &Path, st: &demucs::Stereo, dest: &Path) -> Result<(), String> {
    let tmp = dest.with_extension("yaziliyor.m4a");
    let mut cmd = Command::new(ffmpeg(root)?);
    cmd.args([
        "-nostdin",
        "-y",
        "-loglevel",
        "error",
        "-f",
        "f32le",
        "-ar",
        "44100",
        "-ac",
        "2",
        "-i",
        "-",
    ])
    .args(["-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart"])
    .arg(&tmp)
    .stdin(Stdio::piped())
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    hide_console(&mut cmd);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("FFmpeg başlatılamadı: {e}"))?;
    {
        let mut stdin = child.stdin.take().ok_or("FFmpeg girişi yok")?;
        let mut buf = Vec::with_capacity(1 << 20);
        // Taşan örnekler kırpılır (patlama sesi olmasın), sayı olmayanlar sessizlik.
        let safe = |x: f32| {
            if x.is_finite() {
                x.clamp(-1.0, 1.0)
            } else {
                0.0
            }
        };
        for (l, r) in st[0].iter().zip(&st[1]) {
            buf.extend_from_slice(&safe(*l).to_le_bytes());
            buf.extend_from_slice(&safe(*r).to_le_bytes());
            if buf.len() >= 1 << 20 {
                stdin.write_all(&buf).map_err(|e| e.to_string())?;
                buf.clear();
            }
        }
        stdin.write_all(&buf).map_err(|e| e.to_string())?;
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "Ses kaydedilemedi: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("")
        ));
    }
    tools::replace(&tmp, dest)
}

/// Mikrofon kaydını saklar (sonra farklı gecikmeyle yeniden karıştırılabilsin diye).
pub fn store_voice(root: &Path, bytes: &[u8], ext: &str) -> Result<String, String> {
    let dir = recordings_dir(root).join(".ham");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ext: String = ext
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(5)
        .collect();
    let path = dir.join(format!(
        "{}.{}",
        new_id(),
        if ext.is_empty() { "webm" } else { &ext }
    ));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// Karışım ayarları.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mix {
    /// Ham ses kaydı (`store_voice`).
    pub voice: String,
    /// Kayıt başladığında şarkının saniyesi.
    pub song_start: f64,
    /// Sesin gecikmesi (sn): kayıt bu kadar öne alınır (kulaklık + mikrofon gecikmesi).
    pub latency: f64,
    /// Ses ve müzik seviyesi (1 = olduğu gibi).
    pub voice_gain: f64,
    pub music_gain: f64,
    /// Hafif yankı.
    pub echo: bool,
    /// Önceki karışımın üzerine yaz (yeniden ayar).
    pub output: Option<String>,
    pub title: String,
}

/// Müziği ve sesi karıştırıp `Karaoke\Kayıtlar` içine .m4a yazar; yolunu döner.
pub fn mix(root: &Path, id: &str, m: &Mix) -> Result<String, String> {
    let music = song_dir(root, id).join(MUSIC);
    if !music.is_file() {
        return Err("Şarkının müziği hazır değil.".into());
    }
    let voice = PathBuf::from(&m.voice);
    if !voice.starts_with(recordings_dir(root)) || !voice.is_file() {
        return Err("Kayıt bulunamadı.".into());
    }
    let out = match &m.output {
        Some(o) if Path::new(o).starts_with(recordings_dir(root)) => PathBuf::from(o),
        _ => {
            let stamp = chrono::Local::now().format("%Y-%m-%d %H.%M");
            let clean: String = m
                .title
                .chars()
                .map(|c| {
                    if "\\/:*?\"<>|".contains(c) || c.is_control() {
                        ' '
                    } else {
                        c
                    }
                })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let name: String = clean.chars().take(80).collect();
            recordings_dir(root).join(format!(
                "{} - {stamp}.m4a",
                if name.is_empty() { "Karaoke" } else { &name }
            ))
        }
    };
    let filter = filter_graph(m);
    let tmp = out.with_extension("yaziliyor.m4a");
    let mut cmd = Command::new(ffmpeg(root)?);
    cmd.args(["-nostdin", "-y", "-loglevel", "error", "-ss"])
        .arg(format!("{:.3}", m.song_start.max(0.0)))
        .arg("-i")
        .arg(&music)
        .arg("-i")
        .arg(&voice)
        .args([
            "-filter_complex",
            &filter,
            "-map",
            "[out]",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-movflags",
            "+faststart",
        ])
        .arg(&tmp)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_console(&mut cmd);
    let o = cmd
        .output()
        .map_err(|e| format!("FFmpeg başlatılamadı: {e}"))?;
    if !o.status.success() {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "Karıştırılamadı: {}",
            String::from_utf8_lossy(&o.stderr)
                .lines()
                .last()
                .unwrap_or("")
        ));
    }
    tools::replace(&tmp, &out)?;
    Ok(out.to_string_lossy().into_owned())
}

/// ffmpeg süzgeci: ses gecikmesi kadar baştan kırpılır, seviyeler, isteğe bağlı yankı; müzik sesin süresi kadar.
fn filter_graph(m: &Mix) -> String {
    let lat = m.latency.clamp(0.0, 1.0);
    let vg = m.voice_gain.clamp(0.0, 4.0);
    let mg = m.music_gain.clamp(0.0, 2.0);
    let echo = if m.echo {
        ",aecho=0.8:0.6:70|140:0.22|0.12"
    } else {
        ""
    };
    format!(
        "[1:a]aresample=44100,aformat=channel_layouts=stereo,atrim=start={lat:.3},asetpts=PTS-STARTPTS,\
         volume={vg:.2}{echo}[v];[0:a]volume={mg:.2}[m];[m][v]amix=inputs=2:duration=shortest:normalize=0,\
         alimiter=limit=0.95[out]"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agreement_and_sanity() {
        let a: demucs::Stereo = [vec![0.5, -0.5, 0.25], vec![0.1, 0.2, -0.3]];
        let close: demucs::Stereo = [vec![0.5, -0.49, 0.25], vec![0.1, 0.2, -0.3]];
        let noise: demucs::Stereo = [vec![-0.4, 0.6, 0.9], vec![0.7, -0.8, 0.1]];
        assert!(agreement(&a, &a) > 90.0);
        assert!(agreement(&a, &close) > 30.0);
        assert!(agreement(&a, &noise) < 5.0);
        let nan: demucs::Stereo = [vec![f32::NAN, 0.0, 0.0], vec![0.0; 3]];
        assert!(agreement(&a, &nan) < 0.0);
        assert!(sane(&a, &close).is_ok());
        let loud: demucs::Stereo = [vec![5.0, 5.0, 5.0], vec![5.0; 3]];
        assert!(sane(&a, &loud).is_err());
        assert!(sane(&a, &nan).is_err());
    }

    #[test]
    fn filter_is_clamped() {
        let m = Mix {
            voice: String::new(),
            song_start: 3.0,
            latency: 5.0,
            voice_gain: 9.0,
            music_gain: 0.8,
            echo: true,
            output: None,
            title: String::new(),
        };
        let f = filter_graph(&m);
        assert!(f.contains("atrim=start=1.000"));
        assert!(f.contains("volume=4.00"));
        assert!(f.contains("aecho"));
        assert!(f.ends_with("[out]"));
    }

    /// Gerçek model ile uçtan uca: `ORT_DYLIB_PATH` (libonnxruntime) ve `GMID_KARAOKE_MODEL` verilirse çalışır.
    /// Sınama: ayrılan dört kaynağın toplamı karışıma çok yakın olmalı (STFT/ISTFT ve parçalama doğruysa).
    #[test]
    #[ignore]
    #[allow(clippy::needless_range_loop)]
    fn demucs_sources_add_up() {
        let model = std::env::var("GMID_KARAOKE_MODEL").expect("GMID_KARAOKE_MODEL");
        let root = std::env::temp_dir().join("gmid-karaoke-test");
        std::fs::create_dir_all(tools_dir(&root)).unwrap();
        let dest = tools_dir(&root).join(MODEL_FILE);
        if !dest.exists() {
            std::os::unix::fs::symlink(&model, &dest).unwrap();
        }
        let mut s = session(&root, false).unwrap();
        // 10 sn: akorlar (iki yanda farklı) + ortada şarkı gibi çınlayan bir ses.
        let n = 10 * demucs::RATE;
        let mut mix: demucs::Stereo = [vec![0.0; n], vec![0.0; n]];
        for i in 0..n {
            let t = i as f32 / 44100.0;
            let chord = [110.0f32, 138.6, 164.8]
                .iter()
                .map(|f| (t * f * std::f32::consts::TAU).sin())
                .sum::<f32>()
                * 0.12;
            let vib = 220.0
                * (1.0 + 0.01 * (t * 5.5 * std::f32::consts::TAU).sin())
                * if (t as u32) % 2 == 0 { 1.0 } else { 1.26 };
            let voice = ((t * vib * std::f32::consts::TAU).sin()
                + 0.5 * (t * vib * 2.0 * std::f32::consts::TAU).sin())
                * 0.2;
            mix[0][i] = chord * 1.2 + voice;
            mix[1][i] = chord * 0.8 + voice;
        }
        let all = demucs::separate(&mut s, &mix, &[0, 1, 2, 3], |d, t| {
            eprintln!("{d}/{t}");
            true
        })
        .unwrap();
        let (mut err, mut pow) = (0f64, 0f64);
        for c in 0..2 {
            for i in 0..n {
                let sum: f32 = all.iter().map(|s| s[c][i]).sum();
                err += ((sum - mix[c][i]) as f64).powi(2);
                pow += (mix[c][i] as f64).powi(2);
            }
        }
        let snr = 10.0 * (pow / err).log10();
        let energy =
            |s: &demucs::Stereo| s.iter().flatten().map(|x| (*x as f64).powi(2)).sum::<f64>() / pow;
        eprintln!(
            "SNR {snr:.1} dB; kaynak enerjileri {:?}",
            all.iter().map(energy).collect::<Vec<_>>()
        );
        assert!(
            snr > 15.0,
            "kaynakların toplamı karışımı tutmuyor: {snr:.1} dB"
        );
    }

    /// Uçtan uca (Linux'ta ffmpeg ve ONNX Runtime ile): çöz → ayır → kaydet → sesle karıştır.
    #[test]
    #[ignore]
    fn pipeline_end_to_end() {
        let model = std::env::var("GMID_KARAOKE_MODEL").expect("GMID_KARAOKE_MODEL");
        let root = std::env::temp_dir().join("gmid-karaoke-e2e");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(tools_dir(&root)).unwrap();
        std::fs::create_dir_all(tools::dir(&root).join("ffmpeg")).unwrap();
        std::os::unix::fs::symlink("/usr/bin/ffmpeg", tools::dir(&root).join("ffmpeg/ffmpeg"))
            .unwrap();
        std::os::unix::fs::symlink(&model, tools_dir(&root).join(MODEL_FILE)).unwrap();
        let ff = |args: &[&str]| {
            assert!(Command::new("ffmpeg")
                .args(["-y", "-loglevel", "error"])
                .args(args)
                .status()
                .unwrap()
                .success())
        };
        let song = root.join("sarki.mp3");
        ff(&[
            "-f",
            "lavfi",
            "-i",
            "sine=f=330:d=12",
            "-f",
            "lavfi",
            "-i",
            "sine=f=110:d=12",
            "-filter_complex",
            "[0][1]amerge=inputs=2",
            "-ac",
            "2",
            song.to_str().unwrap(),
        ]);
        let mix = decode(&root, &song).unwrap();
        assert!(
            (mix[0].len() as i64 - 12 * 44100).abs() < 4096,
            "{}",
            mix[0].len()
        );
        let mut s = session(&root, false).unwrap();
        let v = demucs::separate(&mut s, &mix, &[demucs::VOCALS], |_, _| true)
            .unwrap()
            .remove(0);
        let dir = song_dir(&root, "x");
        std::fs::create_dir_all(&dir).unwrap();
        let music: demucs::Stereo =
            [0, 1].map(|c| mix[c].iter().zip(&v[c]).map(|(m, x)| m - x).collect());
        encode(&root, &v, &dir.join(VOCALS)).unwrap();
        encode(&root, &music, &dir.join(MUSIC)).unwrap();
        let voice_src = root.join("ses.webm");
        ff(&[
            "-f",
            "lavfi",
            "-i",
            "sine=f=500:d=5",
            "-c:a",
            "libopus",
            voice_src.to_str().unwrap(),
        ]);
        let voice = store_voice(&root, &std::fs::read(&voice_src).unwrap(), "webm").unwrap();
        let m = Mix {
            voice,
            song_start: 2.0,
            latency: 0.12,
            voice_gain: 1.0,
            music_gain: 0.9,
            echo: true,
            output: None,
            title: "Deneme: şarkı / 1".into(),
        };
        let out = super::mix(&root, "x", &m).unwrap();
        assert!(
            out.contains("Deneme  şarkı   1") || out.contains("Deneme şarkı 1"),
            "{out}"
        );
        let back = decode(&root, Path::new(&out)).unwrap();
        let secs = back[0].len() as f64 / 44100.0;
        assert!((secs - 4.88).abs() < 0.2, "karışım {secs:.2} sn");
    }

    /// İndirilen npm paketinden modelin çıkarılması (`GMID_KARAOKE_TGZ`: demucs-1.0.0.tgz).
    #[test]
    #[ignore]
    fn model_from_npm_package() {
        let tgz = std::env::var("GMID_KARAOKE_TGZ").expect("GMID_KARAOKE_TGZ");
        let dest = std::env::temp_dir().join("gmid-htdemucs.onnx");
        untar_one(Path::new(&tgz), MODEL_FILE, &dest).unwrap();
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), 174_263_526);
        let _ = std::fs::remove_file(dest);
    }

    /// Gerçeğe yakın sınama: konuşan bir ses (flite) + davul/bas/akor müziği karıştırılır; ayrılan vokal gerçek
    /// sese, kalan müzik gerçek müziğe benzemeli. (Linux'ta ffmpeg'in flite süzgeci ve ONNX Runtime ile.)
    #[test]
    #[ignore]
    fn voice_is_separated() {
        let model = std::env::var("GMID_KARAOKE_MODEL").expect("GMID_KARAOKE_MODEL");
        let root = std::env::temp_dir().join("gmid-karaoke-voice");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(tools_dir(&root)).unwrap();
        std::fs::create_dir_all(tools::dir(&root).join("ffmpeg")).unwrap();
        std::os::unix::fs::symlink("/usr/bin/ffmpeg", tools::dir(&root).join("ffmpeg/ffmpeg"))
            .unwrap();
        std::os::unix::fs::symlink(&model, tools_dir(&root).join(MODEL_FILE)).unwrap();
        let ff = |args: &[&str]| {
            let st = Command::new("ffmpeg")
                .args(["-y", "-loglevel", "error"])
                .args(args)
                .status()
                .unwrap();
            assert!(st.success());
        };
        let voice_f = root.join("ses.wav");
        ff(&["-f", "lavfi", "-i",
            "flite=text='Gece yine uzun yol yine sessiz, this is the night we sing together, hold on to me and never let me go, we are the champions my friend':voice=slt",
            "-af", "aresample=44100,apad=pad_dur=1,aloop=loop=2:size=2e6,atrim=0:20,volume=1.6", "-ac", "2", voice_f.to_str().unwrap()]);
        let music_f = root.join("muzik.wav");
        ff(&["-f", "lavfi", "-i",
            "aevalsrc='0.25*sin(2*PI*55*t)*(1+0.3*sin(2*PI*2*t))+0.08*sin(2*PI*220*t)+0.06*sin(2*PI*277*t)+0.05*sin(2*PI*330*t)|0.25*sin(2*PI*55*t)*(1+0.3*sin(2*PI*2*t))+0.08*sin(2*PI*262*t)+0.06*sin(2*PI*330*t)+0.05*sin(2*PI*392*t)':s=44100:d=20",
            "-f", "lavfi", "-i", "anoisesrc=d=20:c=pink:a=0.5:r=44100",
            "-filter_complex", "[1]volume='if(lt(mod(t,0.5),0.06),1,0)':eval=frame,highpass=f=2000[hat];[0][hat]amix=inputs=2:normalize=0,aformat=channel_layouts=stereo",
            music_f.to_str().unwrap()]);
        let voice = decode(&root, &voice_f).unwrap();
        let music = decode(&root, &music_f).unwrap();
        let n = voice[0].len().min(music[0].len());
        let mix: demucs::Stereo =
            [0, 1].map(|c| (0..n).map(|i| voice[c][i] + music[c][i]).collect());
        let mut s = session(&root, false).unwrap();
        let est = demucs::separate(&mut s, &mix, &[demucs::VOCALS], |_, _| true)
            .unwrap()
            .remove(0);
        let truth_v: demucs::Stereo = [0, 1].map(|c| voice[c][..n].to_vec());
        let truth_m: demucs::Stereo = [0, 1].map(|c| music[c][..n].to_vec());
        let est_m: demucs::Stereo = [0, 1].map(|c| (0..n).map(|i| mix[c][i] - est[c][i]).collect());
        let (sv, sm) = (agreement(&truth_v, &est), agreement(&truth_m, &est_m));
        // Karşılaştırma: hiç ayırmasaydık (vokal = 0) vokal SNR'ı 0 dB olurdu.
        eprintln!("vokal SNR {sv:.1} dB, müzik SNR {sm:.1} dB");
        let out = root.join("tahmin-vokal.wav");
        encode(&root, &est, &out.with_extension("m4a")).unwrap();
        assert!(
            sv > 4.0 && sm > 4.0,
            "ayırma zayıf: vokal {sv:.1} dB, müzik {sm:.1} dB"
        );
    }
}
