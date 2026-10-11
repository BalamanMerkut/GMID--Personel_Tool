//! Sesle yazma (Wispr Flow benzeri): **Ctrl + sol Alt** basılı tutulurken konuşulan, bırakınca
//! odaktaki programa yazılır. Her şey bu bilgisayarda olur:
//! 1. Kayıt: varsayılan mikrofon, 16 kHz.
//! 2. Yazıya dökme: whisper-server (NVIDIA kuruluysa ekran kartında).
//! 3. İsteğe bağlı toparlama: yerel dil modeli (Gemma 3 4B) noktalama, paragraf, devrik cümle.
//! 4. Yapıştırma: pano + Ctrl+V; eski pano içeriği geri konur.
//!
//! Türkçe klavyede AltGr (@, €, {…) Windows'a "Ctrl + sağ Alt" olarak gider: dikte yalnızca
//! **sol** Alt ile başlar ve iki tuş tek başına bir süre basılı tutulmalıdır; araya başka bir tuş
//! girerse (Ctrl+Alt+N gibi kısayollar) dikte başlamaz ya da iptal olur.

// Kayıt ve klavye yalnızca Windows'ta; diğer sistemlerde (geliştirme) bu parçalar kullanılmaz.
#![cfg_attr(not(windows), allow(dead_code))]

pub mod engine;
#[cfg(windows)]
mod win;

use crate::db::Db;
use crate::paths::DataDir;
use engine::{Engine, WhisperCfg};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

pub const ENABLED_KEY: &str = "dictation.enabled";

/// Son diktelerden kaç tanesi hatırlansın (yalnızca bellekte).
const HISTORY: usize = 20;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Yapıştırılan yazı.
    pub text: String,
    /// Whisper'ın ham çıktısı (toparlanmadan önce).
    pub raw: String,
    pub at: i64,
    /// Bırakıştan yapıştırmaya kadar geçen süre (ms).
    pub ms: u64,
    pub tidied: bool,
}

#[derive(Default)]
pub struct Dictation {
    pub engine: Engine,
    hook: AtomicBool,
    enabled: AtomicBool,
    busy: AtomicBool,
    phase: Mutex<(&'static str, Option<String>)>,
    history: Mutex<VecDeque<Entry>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub supported: bool,
    pub enabled: bool,
    /// Seçili Whisper modeli ve sunucu programı kurulu mu?
    pub whisper_ready: bool,
    pub gpu_installed: bool,
    /// `Some(true)`: şu an ekran kartında çalışıyor.
    pub gpu_active: Option<bool>,
    pub gpu_error: Option<String>,
    pub llm_installed: bool,
    /// "idle" | "listening" | "transcribing" | "tidying"
    pub phase: &'static str,
    pub last_error: Option<String>,
    pub history: Vec<Entry>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PhaseEvent<'a> {
    phase: &'a str,
    error: Option<&'a str>,
}

struct Settings {
    whisper: WhisperCfg,
    tidy: bool,
    words: String,
}

fn settings(db: &Db) -> Settings {
    let lang: String = db
        .setting_json("dictation.lang")
        .unwrap_or_else(|| "tr".into());
    Settings {
        whisper: WhisperCfg {
            model: db
                .setting_json(crate::transcript::whisper::MODEL_KEY)
                .unwrap_or_else(|| "turbo".into()),
            lang: match lang.as_str() {
                "tr" | "en" => lang,
                _ => "auto".into(),
            },
            gpu: db.setting_json("dictation.gpu").unwrap_or(true),
        },
        tidy: db.setting_json("dictation.tidy").unwrap_or(false),
        words: db.setting_json("dictation.words").unwrap_or_default(),
    }
}

/// Whisper'a ipucu: kullanıcının özel sözcükleri ve noktalamalı örnek bir cümle (noktalama ve
/// büyük harf kullanmaya yönlendirir).
fn whisper_prompt(s: &Settings) -> String {
    let style = match s.whisper.lang.as_str() {
        "tr" => "Merhaba, bugün neler yaptık? Notlarımı yazıyorum.",
        "en" => "Hello, how are you? I'm writing my notes.",
        _ => "",
    };
    let words = s.words.trim();
    if words.is_empty() {
        style.to_string()
    } else {
        format!("{words}. {style}")
    }
}

impl Dictation {
    fn set_phase(&self, app: &AppHandle, phase: &'static str, error: Option<String>) {
        let _ = app.emit(
            "dictation-state",
            PhaseEvent {
                phase,
                error: error.as_deref(),
            },
        );
        *self.phase.lock().unwrap_or_else(|e| e.into_inner()) = (phase, error);
    }

    pub fn status(&self, app: &AppHandle) -> Status {
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        let s = settings(&db);
        let root = dir.root();
        let model = crate::transcript::whisper::find_model(&s.whisper.model);
        let (phase, last_error) = self.phase.lock().unwrap_or_else(|e| e.into_inner()).clone();
        Status {
            supported: cfg!(windows),
            enabled: db.setting_json(ENABLED_KEY).unwrap_or(false),
            whisper_ready: crate::transcript::whisper::model_path(root, model).is_file()
                && (crate::transcript::whisper::server_exe(root, false).is_file()
                    || crate::transcript::whisper::cuda_ready(root)),
            gpu_installed: crate::transcript::whisper::cuda_ready(root),
            gpu_active: self.engine.running_on_gpu(),
            gpu_error: self
                .engine
                .gpu_failed
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            llm_installed: engine::llm_ready(root),
            phase,
            last_error,
            history: self
                .history
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .cloned()
                .collect(),
        }
    }
}

/// Ayarlar değişince (ve açılışta): açıksa klavye dinlemeyi başlatır.
pub fn apply(app: &AppHandle) {
    let d = app.state::<Dictation>();
    let enabled = app
        .state::<Db>()
        .setting_json::<bool>(ENABLED_KEY)
        .unwrap_or(false);
    d.enabled.store(enabled, Ordering::SeqCst);
    if !enabled {
        d.engine.shutdown();
        return;
    }
    #[cfg(windows)]
    if !d.hook.swap(true, Ordering::SeqCst) {
        let (tx, rx) = std::sync::mpsc::channel();
        if let Err(e) = win::install_hook(tx) {
            crate::log!("Sesle yazma başlatılamadı: {e}");
            return;
        }
        let app2 = app.clone();
        std::thread::Builder::new()
            .name("dikte".into())
            .spawn(move || controller(app2, rx))
            .ok();
        // Uzun süre kullanılmayan sunucular kapanır.
        let app3 = app.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            app3.state::<Dictation>().engine.stop_if_idle();
        });
    }
    #[cfg(not(windows))]
    let _ = &d.hook;
}

/// Modelleri önceden yükler (ayarlar ekranındaki "Hazırla" düğmesi).
pub fn warm_up(app: &AppHandle) {
    let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
    let s = settings(&db);
    app.state::<Dictation>()
        .engine
        .warm_up(dir.root(), &s.whisper, s.tidy);
}

#[cfg(windows)]
fn controller(app: AppHandle, rx: std::sync::mpsc::Receiver<win::KeyEvent>) {
    use std::collections::HashSet;
    use std::time::{Duration, Instant};

    const LCTRL: u32 = 0xA2;
    const RCTRL: u32 = 0xA3;
    const LALT: u32 = 0xA4;
    /// Bu kadar basılı tutulunca kayıt başlar (Ctrl+Alt+N gibi kısayollarla karışmasın).
    const ARM: Duration = Duration::from_millis(250);

    enum Mode {
        Idle,
        Armed(Instant),
        Recording(win::Recorder, Instant, u64),
        /// Başka bir tuşa basıldı: Ctrl ve Alt bırakılana kadar bekle.
        Blocked,
    }

    let d = app.state::<Dictation>();
    let mut held: HashSet<u32> = HashSet::new();
    let mut mode = Mode::Idle;
    loop {
        let ev = rx.recv_timeout(Duration::from_millis(30));
        if let Ok(ev) = &ev {
            if ev.fake && ev.vk == LCTRL {
                continue; // AltGr'nin sahte Ctrl'u
            }
            if ev.down {
                if matches!(mode, Mode::Idle) {
                    // Kaçırılmış tuş bırakmaları (kilit ekranı, UAC) takılı kalmasın.
                    held.retain(|k| win::is_down(*k));
                }
                held.insert(ev.vk);
            } else {
                held.remove(&ev.vk);
            }
        } else if matches!(ev, Err(std::sync::mpsc::RecvTimeoutError::Disconnected)) {
            return;
        }
        let ctrl = held.contains(&LCTRL) || held.contains(&RCTRL);
        let combo = ctrl && held.contains(&LALT);
        let others = held.iter().any(|k| ![LCTRL, RCTRL, LALT].contains(k));

        mode = match mode {
            Mode::Idle => {
                if combo && !others && d.enabled.load(Ordering::SeqCst) {
                    Mode::Armed(Instant::now())
                } else {
                    Mode::Idle
                }
            }
            Mode::Armed(t) => {
                if others {
                    Mode::Blocked
                } else if !combo {
                    Mode::Idle
                } else if t.elapsed() >= ARM {
                    if d.busy.load(Ordering::SeqCst) {
                        crate::overlay::show_osd(&app, "mic", "Önceki yazı hazırlanıyor…");
                        Mode::Blocked
                    } else {
                        win::mask_alt();
                        let rec = win::Recorder::start();
                        d.set_phase(&app, "listening", None);
                        crate::overlay::show_osd(&app, "mic", "Dinliyorum… bırakınca yazarım");
                        // Konuşma sürerken modeller yüklensin.
                        let app2 = app.clone();
                        std::thread::spawn(move || warm_up(&app2));
                        Mode::Recording(rec, Instant::now(), 0)
                    }
                } else {
                    Mode::Armed(t)
                }
            }
            Mode::Recording(rec, start, shown) => {
                if others {
                    let _ = rec.finish();
                    d.set_phase(&app, "idle", None);
                    crate::overlay::show_osd(&app, "mic", "Sesle yazma iptal edildi");
                    Mode::Blocked
                } else if !combo {
                    let app2 = app.clone();
                    std::thread::spawn(move || process(&app2, rec));
                    Mode::Idle
                } else {
                    // Gösterge her saniye tazelenir (kaybolmasın, süre görünsün).
                    let secs = start.elapsed().as_secs();
                    if secs > shown {
                        crate::overlay::show_osd(
                            &app,
                            "mic",
                            format!("Dinliyorum · {}:{:02}", secs / 60, secs % 60),
                        );
                    }
                    Mode::Recording(rec, start, secs.max(shown))
                }
            }
            Mode::Blocked => {
                if ctrl || held.contains(&LALT) {
                    Mode::Blocked
                } else {
                    Mode::Idle
                }
            }
        };
    }
}

/// Kaydı bitirir, yazıya döker, (isteğe bağlı) toparlar ve yapıştırır.
#[cfg(windows)]
fn process(app: &AppHandle, rec: win::Recorder) {
    let d = app.state::<Dictation>();
    d.busy.store(true, Ordering::SeqCst);
    let released = std::time::Instant::now();
    // Son hece kesilmesin.
    std::thread::sleep(std::time::Duration::from_millis(200));
    let result = rec
        .finish()
        .and_then(|samples| run(app, &samples, released));
    match result {
        Ok(Some(entry)) => {
            let _ = app.emit("dictation-entry", &entry);
            let mut h = d.history.lock().unwrap_or_else(|e| e.into_inner());
            h.push_front(entry);
            h.truncate(HISTORY);
            drop(h);
            d.set_phase(app, "idle", None);
        }
        Ok(None) => d.set_phase(app, "idle", None),
        Err(e) => {
            crate::log!("Sesle yazma: {e}");
            crate::overlay::show_osd(app, "mic", format!("Sesle yazma: {e}"));
            d.set_phase(app, "idle", Some(e));
        }
    }
    d.busy.store(false, Ordering::SeqCst);
}

#[cfg(windows)]
fn run(
    app: &AppHandle,
    samples: &[i16],
    released: std::time::Instant,
) -> Result<Option<Entry>, String> {
    let d = app.state::<Dictation>();
    let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
    if samples.len() < 16_000 * 2 / 5 {
        crate::overlay::show_osd(app, "mic", "Çok kısa — Ctrl+Alt'ı basılı tutup konuş");
        return Ok(None);
    }
    if rms(samples) < 60.0 {
        return Err("Ses duyulmadı; mikrofon kapalı ya da sessiz olabilir.".into());
    }
    let s = settings(&db);
    d.set_phase(app, "transcribing", None);
    crate::overlay::show_osd(app, "mic", "Yazıya dökülüyor…");
    let raw = d
        .engine
        .transcribe(
            dir.root(),
            &s.whisper,
            &to_wav(samples),
            &whisper_prompt(&s),
        )
        .map_err(|e| {
            if e == "WHISPER_MISSING" {
                "Whisper modeli kurulu değil (Kontrol Odası → Sesle yazma).".to_string()
            } else {
                e
            }
        })?;
    let raw = crate::transcript::parse::clean_dictation(&raw);
    if raw.is_empty() {
        crate::overlay::show_osd(app, "mic", "Anlaşılır bir şey duyulmadı");
        return Ok(None);
    }
    let mut text = raw.clone();
    let mut tidied = false;
    if s.tidy && engine::llm_ready(dir.root()) {
        d.set_phase(app, "tidying", None);
        crate::overlay::show_osd(app, "mic", "Toparlanıyor…");
        match d.engine.tidy(dir.root(), &raw, &s.whisper.lang) {
            Ok(t) => {
                tidied = t != raw;
                text = t;
            }
            Err(e) => crate::log!("Toparlanamadı (ham yazı yapıştırılıyor): {e}"),
        }
    }
    win::paste(app, &text)?;
    Ok(Some(Entry {
        text,
        raw,
        at: crate::util::now_ms(),
        ms: released.elapsed().as_millis() as u64,
        tidied,
    }))
}

fn rms(samples: &[i16]) -> f64 {
    let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
    (sum / samples.len().max(1) as f64).sqrt()
}

/// 16 kHz, tek kanal, 16 bit PCM → WAV dosyası.
pub fn to_wav(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // tek kanal
    w.extend_from_slice(&16_000u32.to_le_bytes());
    w.extend_from_slice(&32_000u32.to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header() {
        let w = to_wav(&[0, 1, -1]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), 16_000);
        assert_eq!(u32::from_le_bytes(w[40..44].try_into().unwrap()), 6);
        assert_eq!(w.len(), 50);
        assert!(rms(&[0, 0]) < 1.0 && rms(&[1000, -1000]) > 999.0);
    }
}
