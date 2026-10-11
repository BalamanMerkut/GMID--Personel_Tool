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

fn dir(root: &Path) -> PathBuf {
    tools::dir(root).join("whisper")
}

fn cli(root: &Path) -> PathBuf {
    dir(root).join(if cfg!(windows) {
        "whisper-cli.exe"
    } else {
        "whisper-cli"
    })
}

fn model_path(root: &Path, m: &Model) -> PathBuf {
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

    if !cli(root).is_file() {
        if !cfg!(windows) {
            return Err(
                "Whisper programı şimdilik sadece Windows için kendiliğinden kurulabiliyor.".into(),
            );
        }
        let url = windows_asset_url()?;
        let tmp = base.join("whisper.indiriliyor");
        let r = tools::download(&url, &tmp, |d, t| {
            emit("whisper", "Whisper", "download", d, t, None)
        })
        .and_then(|_| {
            emit("whisper", "Whisper", "extract", 0, 0, None);
            tools::extract(&tmp, &base, &["whisper-cli.exe", "*.dll"])
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

/// GitHub'daki son sürümler arasında Windows dosyasını taşıyan ilkinin adresi.
fn windows_asset_url() -> Result<String, String> {
    let agent = crate::net::agent(Duration::from_secs(15), Duration::from_secs(30));
    let mut resp = agent
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("GitHub'a ulaşılamadı ({e})"))?;
    if resp.status() != 200 {
        return Err(format!("GitHub {} döndü", resp.status()));
    }
    let body = resp
        .body_mut()
        .with_config()
        .limit(8 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let releases: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    releases
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|r| r["assets"].as_array().cloned().unwrap_or_default())
        .find(|a| a["name"] == WINDOWS_ASSET)
        .and_then(|a| a["browser_download_url"].as_str().map(String::from))
        .ok_or_else(|| "Whisper'ın Windows sürümü bulunamadı.".into())
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

/// Dosyayı yazıya döker. `lang`: "auto" (ya da "orig"), "tr", "en"… `progress`: 0..1
pub fn transcribe(
    root: &Path,
    file: &Path,
    lang: &str,
    model: &str,
    progress: impl Fn(&'static str, f64),
) -> Result<Vec<Segment>, String> {
    let m = find_model(model);
    let model_file = model_path(root, m);
    if !cli(root).is_file() || !model_file.is_file() {
        return Err("WHISPER_MISSING".into());
    }
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
    let lang = match lang {
        "tr" | "en" => lang,
        _ => "auto",
    };
    let mut cmd = Command::new(cli(root));
    cmd.arg("-m")
        .arg(&model_file)
        .arg("-f")
        .arg(&wav)
        .args(["-l", lang, "-t", &threads])
        // Tam JSON (kelime zamanlarıyla), satırlar en fazla ~50 karakter ve kelime sınırında bölünür.
        .args(["-ojf", "-ml", "50", "-sow", "-pp"])
        .arg("-of")
        .arg(&out_base)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(d) = cli(root).parent() {
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
    let result = if status.success() && json.is_file() {
        std::fs::read_to_string(&json)
            .map_err(|e| e.to_string())
            .and_then(|s| parse_whisper(&s))
    } else {
        crate::log!("Whisper başarısız: {last_err}");
        Err(if last_err.is_empty() {
            "Whisper çalışırken hata oluştu.".into()
        } else {
            format!("Whisper: {last_err}")
        })
    };
    cleanup();
    result
}
