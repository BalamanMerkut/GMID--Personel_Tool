//! Whisper (whisper.cpp): videodaki konuşmayı ya da şarkı sözünü tamamen bilgisayarda yazıya döker.
//! Program ve model ilk kullanımda `Araclar\whisper` klasörüne iner (internetteki bir servise ses gönderilmez).

use super::parse::{parse_whisper, Segment};
use crate::downloader::tools;
use crate::util::{hide_console, new_id};
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub struct Model {
    pub id: &'static str,
    pub file: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
    pub size_mb: u32,
}

pub const MODELS: &[Model] = &[
    Model {
        id: "turbo",
        file: "ggml-large-v3-turbo-q5_0.bin",
        label: "Dengeli (önerilen)",
        detail: "large-v3-turbo · en iyi doğruluğa çok yakın, hızlı",
        size_mb: 550,
    },
    Model {
        id: "small",
        file: "ggml-small-q5_1.bin",
        label: "Hızlı",
        detail: "small · eski bilgisayarlar için, daha az doğru",
        size_mb: 190,
    },
];

const MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/";
/// whisper.cpp'nin Windows sürümü bazı sürümlerde boş geliyor; son sürümler arasında bu dosyayı taşıyan aranır.
const RELEASES_API: &str = "https://api.github.com/repos/ggml-org/whisper.cpp/releases?per_page=20";
const WINDOWS_ASSET: &str = "whisper-bin-x64.zip";
pub const MODEL_KEY: &str = "transcript.model";

pub(crate) fn dir(root: &Path) -> PathBuf {
    tools::dir(root).join("whisper")
}

/// NVIDIA ekran kartıyla çalışan (CUDA) sürüm ayrı klasörde durur; DLL'leri işlemci sürümüyle karışmasın.
pub(crate) fn cuda_dir(root: &Path) -> PathBuf {
    tools::dir(root).join("whisper-cuda")
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn cli(root: &Path) -> PathBuf {
    dir(root).join(exe("whisper-cli"))
}

/// Kalıcı çalışan sunucu (modeli bir kez yükler): sesle yazma için.
pub(crate) fn server_exe(root: &Path, gpu: bool) -> PathBuf {
    if gpu {
        cuda_dir(root).join(exe("whisper-server"))
    } else {
        dir(root).join(exe("whisper-server"))
    }
}

pub(crate) fn cuda_ready(root: &Path) -> bool {
    server_exe(root, true).is_file() && cuda_dir(root).join(exe("whisper-cli")).is_file()
}

pub(crate) fn model_path(root: &Path, m: &Model) -> PathBuf {
    dir(root).join("models").join(m.file)
}

pub fn find_model(id: &str) -> &'static Model {
    MODELS.iter().find(|m| m.id == id).unwrap_or(&MODELS[0])
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    id: &'static str,
    label: &'static str,
    detail: &'static str,
    size_mb: u32,
    installed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperStatus {
    pub program: bool,
    pub models: Vec<ModelStatus>,
    pub selected: &'static str,
    /// Seçili modelle kullanıma hazır mı?
    pub ready: bool,
}

pub fn status(root: &Path, selected: &str) -> WhisperStatus {
    let selected = find_model(selected);
    let program = cli(root).is_file();
    WhisperStatus {
        program,
        ready: program && model_path(root, selected).is_file(),
        selected: selected.id,
        models: MODELS
            .iter()
            .map(|m| ModelStatus {
                id: m.id,
                label: m.label,
                detail: m.detail,
                size_mb: m.size_mb,
                installed: model_path(root, m).is_file(),
            })
            .collect(),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    tool: &'a str,
    name: &'a str,
    stage: &'a str,
    done: u64,
    total: u64,
    error: Option<String>,
}

/// Programı (yoksa) ve seçilen modeli (yoksa) indirir. İlerleme `tools-progress` olayıyla gider.
pub fn install(app: &AppHandle, root: &Path, model: &str) -> Result<(), String> {
    let base = dir(root);
    std::fs::create_dir_all(base.join("models")).map_err(|e| e.to_string())?;
    let emit =
        |tool: &str, name: &str, stage: &str, done: u64, total: u64, error: Option<String>| {
            let _ = app.emit(
                "tools-progress",
                Progress {
                    tool,
                    name,
                    stage,
                    done,
                    total,
                    error,
                },
            );
        };

    // Sunucu (sesle yazma) eski kurulumlarda yoktu: eksikse program yeniden iner (~8 MB).
    if !cli(root).is_file() || !server_exe(root, false).is_file() {
        if !cfg!(windows) {
            return Err(
                "Whisper programı şimdilik sadece Windows için kendiliğinden kurulabiliyor.".into(),
            );
        }
        let url = release_asset(RELEASES_API, |n| n == WINDOWS_ASSET)?.0;
        let tmp = base.join("whisper.indiriliyor");
        let r = tools::download(&url, &tmp, |d, t| {
            emit("whisper", "Whisper", "download", d, t, None)
        })
        .and_then(|_| {
            emit("whisper", "Whisper", "extract", 0, 0, None);
            tools::extract(
                &tmp,
                &base,
                &["whisper-cli.exe", "whisper-server.exe", "*.dll"],
            )
        });
        let _ = std::fs::remove_file(&tmp);
        if let Err(e) = r {
            emit("whisper", "Whisper", "error", 0, 0, Some(e.clone()));
            return Err(format!("Whisper indirilemedi: {e}"));
        }
        emit("whisper", "Whisper", "done", 0, 0, None);
    }

    let m = find_model(model);
    let dest = model_path(root, m);
    if !dest.is_file() {
        let tmp = dest.with_extension("indiriliyor");
        let r = tools::download(&format!("{MODEL_URL}{}", m.file), &tmp, |d, t| {
            emit("model", m.label, "download", d, t, None)
        })
        .and_then(|_| tools::replace(&tmp, &dest));
        if let Err(e) = r {
            let _ = std::fs::remove_file(&tmp);
            emit("model", m.label, "error", 0, 0, Some(e.clone()));
            return Err(format!("Model indirilemedi: {e}"));
        }
        emit("model", m.label, "done", 0, 0, None);
    }
    Ok(())
}

/// GitHub'daki son sürümler arasında adı uyan ilk dosyanın adresi ve boyutu. whisper.cpp ve
/// llama.cpp Windows dosyalarını her sürümde yayımlamıyor; son birkaç sürüme bakılır.
pub(crate) fn release_asset(
    api: &str,
    wanted: impl Fn(&str) -> bool,
) -> Result<(String, u64), String> {
    let agent = crate::net::agent(Duration::from_secs(15), Duration::from_secs(60));
    let mut resp = agent
        .get(api)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("GitHub'a ulaşılamadı ({e})"))?;
    if resp.status() != 200 {
        return Err(format!("GitHub {} döndü", resp.status()));
    }
    let body = resp
        .body_mut()
        .with_config()
        .limit(32 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let releases: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    releases
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|r| r["assets"].as_array().cloned().unwrap_or_default())
        .find(|a| a["name"].as_str().is_some_and(&wanted))
        .and_then(|a| {
            Some((
                a["browser_download_url"].as_str()?.to_string(),
                a["size"].as_u64().unwrap_or(0),
            ))
        })
        .ok_or_else(|| "İndirilecek Windows sürümü bulunamadı.".into())
}

/// whisper.cpp'nin NVIDIA (CUDA 12) sürümü: "whisper-bin-win-cuda-12.4.0-x64.zip" ya da eski adıyla
/// "whisper-cublas-12.4.0-bin-x64.zip".
pub(crate) fn is_cuda_asset(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.starts_with("whisper")
        && n.ends_with("x64.zip")
        && (n.contains("cuda-12") || n.contains("cublas-12"))
}

/// NVIDIA ekran kartı için whisper.cpp'yi indirir (~650 MB; CUDA kütüphaneleri içinde).
pub fn install_cuda(app: &AppHandle, root: &Path) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("NVIDIA hızlandırma yalnızca Windows'ta kurulabiliyor.".into());
    }
    let emit = |stage: &str, done: u64, total: u64, error: Option<String>| {
        let _ = app.emit(
            "tools-progress",
            Progress {
                tool: "whisper-cuda",
                name: "Whisper (NVIDIA)",
                stage,
                done,
                total,
                error,
            },
        );
    };
    let dest = cuda_dir(root);
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    let (url, _) = release_asset(RELEASES_API, is_cuda_asset)?;
    let tmp = dest.join("whisper-cuda.indiriliyor");
    let r = tools::download(&url, &tmp, |d, t| emit("download", d, t, None)).and_then(|_| {
        emit("extract", 0, 0, None);
        tools::extract(
            &tmp,
            &dest,
            &["whisper-cli.exe", "whisper-server.exe", "*.dll"],
        )
    });
    let _ = std::fs::remove_file(&tmp);
    match r {
        Ok(()) => {
            emit("done", 0, 0, None);
            Ok(())
        }
        Err(e) => {
            emit("error", 0, 0, Some(e.clone()));
            Err(format!("NVIDIA sürümü indirilemedi: {e}"))
        }
    }
}

fn ffmpeg(root: &Path) -> PathBuf {
    let exe = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    tools::ffmpeg_dir(root)
        .map(|d| d.join(exe))
        .unwrap_or_else(|| PathBuf::from(exe))
}

/// Dosyayı yazıya döker. `lang`: "auto" (ya da "orig"), "tr", "en"… `prompt`: başlık gibi ipucu
/// (sanatçı ve özel adlar doğru yazılsın). `progress`: 0..1
pub fn transcribe(
    root: &Path,
    file: &Path,
    lang: &str,
    prompt: &str,
    model: &str,
    progress: impl Fn(&'static str, f64),
) -> Result<Vec<Segment>, String> {
    let m = find_model(model);
    let model_file = model_path(root, m);
    if !cli(root).is_file() || !model_file.is_file() {
        return Err("WHISPER_MISSING".into());
    }
    // NVIDIA sürümü kuruluysa önce o (çok daha hızlı); çalışmazsa işlemci sürümü.
    let mut clis = Vec::new();
    if cuda_ready(root) {
        clis.push(cuda_dir(root).join(exe("whisper-cli")));
    }
    clis.push(cli(root));
    let tmp = tools::dir(root).join("tmp");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let name = new_id();
    let wav = tmp.join(format!("{name}.wav"));
    let out_base = tmp.join(&name);
    let json = tmp.join(format!("{name}.json"));
    let cleanup = || {
        let _ = std::fs::remove_file(&wav);
        let _ = std::fs::remove_file(&json);
    };

    // 1) Sesi Whisper'ın istediği biçime çevir (16 kHz, tek kanal).
    progress("audio", 0.0);
    let mut cmd = Command::new(ffmpeg(root));
    cmd.args(["-nostdin", "-y", "-loglevel", "error", "-i"])
        .arg(file)
        .args(["-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"])
        .arg(&wav)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hide_console(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("FFmpeg başlatılamadı: {e}"))?;
    if !out.status.success() || !wav.is_file() {
        cleanup();
        return Err(format!(
            "Ses çıkarılamadı: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("")
        ));
    }

    // 2) Whisper
    progress("whisper", 0.0);
    let threads = std::thread::available_parallelism()
        .map(|n| n.get().clamp(1, 8))
        .unwrap_or(4)
        .to_string();
    let detected;
    let lang = match lang {
        "tr" | "en" => lang,
        _ => {
            // Whisper dili ilk 30 saniyeden anlar; şarkıların girişi çoğu zaman yalnızca müzik
            // olduğundan yanlış dil seçip uydurabiliyor. Dil, kaydın ortasındaki parçadan bulunur.
            detected = detect_language(
                root,
                &model_file,
                &wav,
                &tmp.join(format!("{name}-dil.wav")),
                &threads,
            );
            detected.as_deref().unwrap_or("auto")
        }
    };
    let mut result = Err("Whisper çalışırken hata oluştu.".to_string());
    for (i, program) in clis.iter().enumerate() {
        let _ = std::fs::remove_file(&json);
        result = run_cli(
            program,
            &model_file,
            &wav,
            lang,
            &threads,
            prompt,
            &out_base,
            &json,
            &progress,
        );
        match &result {
            Ok(_) => break,
            Err(e) if i + 1 < clis.len() => {
                crate::log!("Whisper (NVIDIA) çalışmadı, işlemciyle deneniyor: {e}")
            }
            Err(_) => {}
        }
    }
    cleanup();
    result
}

#[allow(clippy::too_many_arguments)]
fn run_cli(
    program: &Path,
    model_file: &Path,
    wav: &Path,
    lang: &str,
    threads: &str,
    prompt: &str,
    out_base: &Path,
    json: &Path,
    progress: &impl Fn(&'static str, f64),
) -> Result<Vec<Segment>, String> {
    let mut cmd = Command::new(program);
    cmd.arg("-m")
        .arg(model_file)
        .arg("-f")
        .arg(wav)
        .args(["-l", lang, "-t", threads])
        // Tam JSON (kelime zamanlarıyla), satırlar en fazla ~50 karakter ve kelime sınırında bölünür.
        .args(["-ojf", "-ml", "50", "-sow", "-pp"])
        // Konuşma dışı simgeleri ([Müzik], ♪…) bastır: müzikte uydurma satırları azaltır.
        .arg("--suppress-nst")
        .arg("-of")
        .arg(out_base)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let prompt = prompt.trim();
    if !prompt.is_empty() {
        cmd.arg("--prompt").arg(prompt);
    }
    if let Some(d) = program.parent() {
        cmd.current_dir(d);
    }
    hide_console(&mut cmd);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Whisper başlatılamadı: {e}"))?;
    let mut last_err = String::new();
    if let Some(stderr) = child.stderr.take() {
        let mut reader = BufReader::new(stderr);
        let mut buf = Vec::new();
        while reader.read_until(b'\n', &mut buf).is_ok_and(|n| n > 0) {
            let line = String::from_utf8_lossy(&buf).to_string();
            buf.clear();
            // "whisper_print_progress_callback: progress =  35%"
            if let Some(p) = line
                .split("progress =")
                .nth(1)
                .and_then(|r| r.trim().trim_end_matches('%').trim().parse::<f64>().ok())
            {
                progress("whisper", (p / 100.0).clamp(0.0, 1.0));
            } else if line.to_lowercase().contains("error") {
                last_err = line.trim().to_string();
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() && json.is_file() {
        std::fs::read_to_string(json)
            .map_err(|e| e.to_string())
            .and_then(|s| parse_whisper(&s))
    } else {
        crate::log!("Whisper başarısız: {last_err}");
        Err(if last_err.is_empty() {
            "Whisper çalışırken hata oluştu.".into()
        } else {
            format!("Whisper: {last_err}")
        })
    }
}

/// Kaydın ortasından 30 saniyelik bir parça alıp dilini Whisper'a sorar ("en", "tr"…).
fn detect_language(
    root: &Path,
    model_file: &Path,
    wav: &Path,
    clip: &Path,
    threads: &str,
) -> Option<String> {
    // 16 kHz, tek kanal, 16 bit → saniyede 32 000 bayt.
    let secs = std::fs::metadata(wav).ok()?.len().saturating_sub(44) / 32_000;
    if secs < 60 {
        return None;
    }
    let mut cmd = Command::new(ffmpeg(root));
    cmd.args(["-nostdin", "-y", "-loglevel", "error", "-ss"])
        .arg((secs * 2 / 5).to_string())
        .args(["-t", "30", "-i"])
        .arg(wav)
        .args(["-c", "copy"])
        .arg(clip)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_console(&mut cmd);
    let ok = cmd.status().is_ok_and(|s| s.success());
    let found = ok
        .then(|| {
            let mut cmd = Command::new(cli(root));
            cmd.arg("-m")
                .arg(model_file)
                .arg("-f")
                .arg(clip)
                .args(["-l", "auto", "-dl", "-t", threads])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            if let Some(d) = cli(root).parent() {
                cmd.current_dir(d);
            }
            hide_console(&mut cmd);
            cmd.output().ok()
        })
        .flatten()
        .and_then(|out| {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stderr),
                String::from_utf8_lossy(&out.stdout)
            );
            parse_detected(&text)
        });
    let _ = std::fs::remove_file(clip);
    crate::log!("Whisper dil tespiti: {found:?}");
    found
}

/// "whisper_full_with_state: auto-detected language: en (p = 0.958)" → "en"
fn parse_detected(log: &str) -> Option<String> {
    let rest = log.split("auto-detected language:").nth(1)?.trim_start();
    let code: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let p: f64 = rest
        .split("p =")
        .nth(1)?
        .split(')')
        .next()?
        .trim()
        .parse()
        .ok()?;
    (!code.is_empty() && p >= 0.5).then_some(code)
}

#[cfg(test)]
mod tests {
    #[test]
    fn detected_language() {
        let log = "whisper_full_with_state: auto-detected language: en (p = 0.958340)\nmain: done";
        assert_eq!(super::parse_detected(log).as_deref(), Some("en"));
        let unsure = "auto-detected language: tr (p = 0.21)";
        assert_eq!(super::parse_detected(unsure), None);
    }
}
