//! Videodaki konuşmanın ya da şarkı sözlerinin yazısı (transkript).
//!
//! Önce şarkı sözü veritabanı (LRCLIB; şarkılarda en doğrusu), sonra sitenin kendi altyazısı
//! (YouTube'un otomatik altyazısında kelime zamanları da var) denenir; yoksa Whisper bilgisayarda çalışır. Sonuç veritabanında durur ve dosyanın yanına `.srt`
//! (video) ya da `.lrc` (müzik) olarak da yazılır: başka oynatıcılarda da görünsün.

pub mod lrclib;
pub mod parse;
pub mod whisper;

use crate::db::Db;
use crate::downloader::{self, tools, ytdlp::Context};
use crate::paths::DataDir;
use crate::util::{hide_console, new_id, now_ms};
use parse::Segment;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub download_id: String,
    /// "lyrics" (şarkı sözü veritabanı) | "subtitles" (sitenin altyazısı) | "whisper"
    pub source: String,
    pub language: String,
    pub segments: Vec<Segment>,
    pub created_at: i64,
}

pub fn get(db: &Db, id: &str) -> Option<Transcript> {
    db.conn()
        .query_row(
            "SELECT source, language, segments, created_at FROM transcripts WHERE download_id = ?1",
            [id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .ok()
        .flatten()
        .map(|(source, language, segs, created_at)| Transcript {
            download_id: id.to_string(),
            source,
            language,
            segments: serde_json::from_str(&segs).unwrap_or_default(),
            created_at,
        })
}

fn save(db: &Db, t: &Transcript) -> Result<(), String> {
    let segs = serde_json::to_string(&t.segments).map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "INSERT INTO transcripts (download_id, source, language, segments, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(download_id) DO UPDATE SET source = excluded.source,
                language = excluded.language, segments = excluded.segments, created_at = excluded.created_at",
            params![t.download_id, t.source, t.language, segs, t.created_at],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn delete(db: &Db, id: &str) {
    let _ = db
        .conn()
        .execute("DELETE FROM transcripts WHERE download_id = ?1", [id]);
}

#[derive(Default)]
pub struct Transcriber {
    running: Mutex<HashSet<String>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    id: &'a str,
    /// "lyrics" | "subtitles" | "audio" | "whisper"
    stage: &'a str,
    progress: f64,
}

/// Yazıyı çıkarır. `mode`: "auto" (söz veritabanı → altyazı → Whisper) | "lyrics" | "subtitles" | "whisper".
/// `lang`: "orig" (videonun kendi dili) | "tr" | "en".
pub fn create(app: &AppHandle, id: &str, mode: &str, lang: &str) -> Result<Transcript, String> {
    let state = app.state::<Transcriber>();
    if !state
        .running
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(id.to_string())
    {
        return Err("Bu video zaten yazıya dökülüyor.".into());
    }
    let result = run(app, id, mode, lang);
    state
        .running
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(id);
    result
}

fn run(app: &AppHandle, id: &str, mode: &str, lang: &str) -> Result<Transcript, String> {
    let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
    let d = downloader::get(&db, id)
        .map_err(|e| e.to_string())?
        .ok_or("Video bulunamadı")?;
    let file = d
        .file_path
        .clone()
        .filter(|f| Path::new(f).is_file())
        .ok_or("Video dosyası bulunamadı; taşınmış ya da silinmiş olabilir.")?;
    let emit = |stage: &str, progress: f64| {
        let _ = app.emit(
            "transcript-progress",
            Progress {
                id,
                stage,
                progress,
            },
        );
    };

    let mut found: Option<(String, String, Vec<Segment>)> = None;
    let lyrics_db = db.setting_json::<bool>(lrclib::ENABLED_KEY).unwrap_or(true);
    if mode == "lyrics" || (mode == "auto" && lyrics_db) {
        emit("lyrics", 0.0);
        match lrclib::find(&d.title, &d.uploader, d.duration) {
            Ok(Some(segs)) => found = Some(("lyrics".into(), lang.into(), segs)),
            Ok(None) => {}
            Err(e) if mode == "lyrics" => return Err(e),
            Err(e) => crate::log!("{e}"),
        }
    }
    if found.is_none() && (mode == "auto" || mode == "subtitles") {
        emit("subtitles", 0.0);
        let ctx: Context = db
            .conn()
            .query_row("SELECT context FROM downloads WHERE id = ?1", [id], |r| {
                r.get::<_, Option<String>>(0)
            })
            .ok()
            .flatten()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default();
        match fetch_subtitles(dir.root(), &d.url, &ctx, lang) {
            Ok(Some((language, segs))) => found = Some(("subtitles".into(), language, segs)),
            Ok(None) => {}
            Err(e) => crate::log!("Altyazı alınamadı: {e}"),
        }
    }
    if found.is_none() && (mode == "auto" || mode == "whisper") {
        let model: String = db
            .setting_json(whisper::MODEL_KEY)
            .unwrap_or_else(|| "turbo".into());
        let prompt = lrclib::clean_title(&d.title);
        let segs = whisper::transcribe(dir.root(), Path::new(&file), lang, &prompt, &model, emit)?;
        found = Some((
            "whisper".into(),
            if lang == "orig" {
                "auto".into()
            } else {
                lang.into()
            },
            segs,
        ));
    }
    let Some((source, language, segments)) = found.filter(|f| !f.2.is_empty()) else {
        return Err(match mode {
            "subtitles" => "Bu videonun sitede altyazısı yok. Whisper ile dene.".into(),
            "lyrics" => "Bu şarkının zamanlı sözleri veritabanında bulunamadı. Altyazı ya da Whisper ile dene.".into(),
            _ => "Whisper bu kayıtta anlaşılır bir söz duyamadı; müzik çok baskın olabilir.".into(),
        });
    };

    let t = Transcript {
        download_id: id.to_string(),
        source,
        language,
        segments,
        created_at: now_ms(),
    };
    save(&db, &t)?;
    export(Path::new(&file), &d.title, d.audio_only, &t.segments);
    Ok(t)
}

/// Dosyanın yanına başka oynatıcıların da okuyacağı yazı dosyası.
fn export(file: &Path, title: &str, audio: bool, segs: &[Segment]) {
    let (path, text) = if audio {
        (file.with_extension("lrc"), parse::to_lrc(segs, title))
    } else {
        (file.with_extension("srt"), parse::to_srt(segs))
    };
    if let Err(e) = std::fs::write(&path, text) {
        crate::log!("Yazı dosyası kaydedilemedi: {e}");
    }
}

/// Sitenin altyazısını indirir. YouTube'da önce videonun kendi dilindeki otomatik altyazı
/// (kelime zamanlı), sonra elle yazılmış altyazılar denenir.
fn fetch_subtitles(
    root: &Path,
    url: &str,
    ctx: &Context,
    lang: &str,
) -> Result<Option<(String, Vec<Segment>)>, String> {
    if !tools::ytdlp_path(root).is_file() {
        return Err("TOOLS_MISSING".into());
    }
    // (otomatik altyazı mı, dil kalıbı)
    let passes: Vec<(bool, &str)> = match lang {
        "tr" => vec![(false, "tr.*"), (true, "tr.*")],
        "en" => vec![(false, "en.*"), (true, "en.*")],
        _ => vec![(true, ".*-orig"), (false, "all"), (true, "en,tr")],
    };
    let tmp: PathBuf = tools::dir(root).join("tmp").join(new_id());
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let result = (|| {
        for (auto, langs) in passes {
            let mut cmd = tools::ytdlp(root);
            cmd.args(["--skip-download", "--no-playlist", "--no-warnings"])
                .arg(if auto {
                    "--write-auto-subs"
                } else {
                    "--write-subs"
                })
                .args([
                    "--sub-langs",
                    langs,
                    "--sub-format",
                    "vtt/best",
                    "--convert-subs",
                    "vtt",
                ])
                .arg("-o")
                .arg(tmp.join("s.%(ext)s"))
                .args(ctx.args())
                .arg("--")
                .arg(url)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .stdin(Stdio::null());
            hide_console(&mut cmd);
            let _ = cmd.output();
            if let Some((language, segs)) = pick(&tmp, lang) {
                return Ok(Some((language, segs)));
            }
        }
        Ok(None)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

/// İndirilen altyazılardan en uygununu seçer: istenen dil, sonra Türkçe, sonra İngilizce.
fn pick(dir: &Path, lang: &str) -> Option<(String, Vec<Segment>)> {
    let mut files: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let code = name.strip_prefix("s.")?.strip_suffix(".vtt")?.to_string();
            Some((code, e.path()))
        })
        .collect();
    let rank = |code: &str| {
        let base = code.split(['-', '_']).next().unwrap_or(code);
        if code.ends_with("-orig") || base == lang {
            0
        } else if base == "tr" {
            1
        } else if base == "en" {
            2
        } else {
            3
        }
    };
    files.sort_by_key(|(c, _)| rank(c));
    files.into_iter().find_map(|(code, path)| {
        let segs = parse::parse_vtt(&std::fs::read_to_string(path).ok()?);
        (segs.len() >= 2).then(|| (code.trim_end_matches("-orig").to_string(), segs))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_original_language_first() {
        let dir = std::env::temp_dir().join(format!("gmid-subs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let vtt = |t: &str| {
            format!("WEBVTT\n\n00:00:01.000 --> 00:00:02.000\n{t} 1\n\n00:00:03.000 --> 00:00:04.000\n{t} 2\n")
        };
        std::fs::write(dir.join("s.de.vtt"), vtt("Hallo")).unwrap();
        std::fs::write(dir.join("s.en-orig.vtt"), vtt("Hello")).unwrap();
        std::fs::write(dir.join("s.tr.vtt"), vtt("Merhaba")).unwrap();
        let (code, segs) = pick(&dir, "orig").unwrap();
        assert_eq!((code.as_str(), segs[0].text.as_str()), ("en", "Hello 1"));
        let (code, _) = pick(&dir, "tr").unwrap();
        assert_eq!(code, "tr");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
