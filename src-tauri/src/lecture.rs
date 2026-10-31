//! Ders kaydı: hoca konuşurken uzun kayıt; ses ~25 saniyelik parçalar halinde (sessiz bir anda bölünerek)
//! sesle yazmanın Whisper'ına gönderilir, yazı canlı akar. Bitince Notlar'a ders adı ve tarihle kaydedilir.
//! İstenirse ses de `Dersler` klasörüne WAV olarak saklanır (varsayılan kapalı).
//!
//! Her şey bu bilgisayarda olur; ses hiçbir yere gönderilmez.

// Kayıt yalnız Windows'ta; diğer sistemlerde (geliştirme) parçalar kullanılmaz.
#![cfg_attr(not(windows), allow(dead_code))]

use crate::db::Db;
use crate::paths::DataDir;
use serde::Serialize;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const RATE: usize = 16_000;
/// Bu kadar ses birikince bölünür (sessiz bir anda; en erken `MIN_CHUNK`).
const CHUNK: usize = RATE * 25;
const MIN_CHUNK: usize = RATE * 18;
/// Sessizlik aranırken bakılan pencere.
const QUIET_WIN: usize = RATE * 3 / 10;
/// En uzun ders: 4 saat (unutulan kayıt sonsuza kadar sürmesin).
const MAX_SECONDS: u64 = 4 * 3600;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    /// Kaydın başından saniye.
    pub at: u64,
    pub text: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub supported: bool,
    pub recording: bool,
    pub paused: bool,
    /// Kayıt bitti, son parça yazıya dökülüyor.
    pub finishing: bool,
    pub title: String,
    pub started_at: i64,
    /// Kaydedilen süre (duraklatılanlar hariç), saniye.
    pub elapsed: u64,
    /// Mikrofon seviyesi (0–1).
    pub level: f32,
    pub segments: Vec<Segment>,
    /// Bir parça şu an yazıya dökülüyor.
    pub busy: bool,
    /// Henüz kesinleşmemiş parçanın taslağı: birkaç saniyede bir hızlıca okunur, kelime kelime görünür;
    /// parça bitince dikkatli okumayla kesinleşip `segments`e geçer.
    pub draft: String,
    pub error: Option<String>,
    /// Bitince kaydedilen notun kimliği.
    pub note_id: Option<String>,
    /// Saklanan ses dosyası.
    pub audio: Option<String>,
}

/// Kayıt ve yazıya dökme iş parçacıklarının paylaştığı durum.
struct Shared {
    stop: AtomicBool,
    paused: AtomicBool,
    /// Henüz yazıya dökülmemiş ses.
    pending: Mutex<Vec<i16>>,
    /// `pending`in ilk örneğinin kayıttaki yeri (örnek sayısı).
    pending_at: AtomicU64,
    /// Toplam kaydedilen örnek.
    recorded: AtomicU64,
    level: AtomicU32,
    busy: AtomicBool,
    capture_done: AtomicBool,
    draft: Mutex<String>,
    segments: Mutex<Vec<Segment>>,
    error: Mutex<Option<String>>,
    /// Dersin terimleri (Whisper'a ipucu: "türev, integral, Laplace").
    terms: String,
}

struct Session {
    title: String,
    started_at: i64,
    shared: Arc<Shared>,
    finished: bool,
    note_id: Option<String>,
    audio: Option<String>,
}

#[derive(Default)]
pub struct Lecture(Mutex<Option<Session>>);

impl Lecture {
    pub fn status(&self) -> Status {
        let g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let Some(s) = g.as_ref() else {
            return Status {
                supported: cfg!(windows),
                ..Default::default()
            };
        };
        let sh = &s.shared;
        let stopping = sh.stop.load(Ordering::SeqCst);
        Status {
            supported: cfg!(windows),
            recording: !s.finished && !stopping,
            paused: sh.paused.load(Ordering::SeqCst),
            finishing: stopping && !s.finished,
            title: s.title.clone(),
            started_at: s.started_at,
            elapsed: sh.recorded.load(Ordering::SeqCst) / RATE as u64,
            level: f32::from_bits(sh.level.load(Ordering::Relaxed)),
            segments: sh.segments.lock().map(|v| v.clone()).unwrap_or_default(),
            busy: sh.busy.load(Ordering::SeqCst),
            draft: sh.draft.lock().map(|d| d.clone()).unwrap_or_default(),
            error: sh.error.lock().ok().and_then(|e| e.clone()),
            note_id: s.note_id.clone(),
            audio: s.audio.clone(),
        }
    }

    /// Kaydı başlatır. `mic`: aygıt kimliği (boşsa Windows'un varsayılanı).
    pub fn start(
        &self,
        app: &AppHandle,
        title: &str,
        mic: &str,
        terms: &str,
        keep_audio: bool,
    ) -> Result<Status, String> {
        {
            let g = self.0.lock().unwrap_or_else(|e| e.into_inner());
            if g.as_ref().is_some_and(|s| !s.finished) {
                return Err("Zaten bir ders kaydı sürüyor".into());
            }
        }
        if !cfg!(windows) {
            return Err("Ders kaydı yalnız Windows'ta çalışır".into());
        }
        let title = {
            let t = title.trim();
            if t.is_empty() {
                "Ders".to_string()
            } else {
                t.chars().take(80).collect()
            }
        };
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            pending: Mutex::new(Vec::with_capacity(CHUNK * 2)),
            pending_at: AtomicU64::new(0),
            recorded: AtomicU64::new(0),
            level: AtomicU32::new(0),
            busy: AtomicBool::new(false),
            capture_done: AtomicBool::new(false),
            draft: Mutex::new(String::new()),
            segments: Mutex::new(Vec::new()),
            error: Mutex::new(None),
            terms: terms
                .split([',', '\n'])
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .take(40)
                .collect::<Vec<_>>()
                .join(", "),
        });
        let started_at = crate::util::now_ms();
        let audio = keep_audio.then(|| {
            app.state::<DataDir>().folder(Some("Dersler")).join(format!(
                "{}-{}.wav",
                safe_name(&title),
                chrono::Local::now().format("%Y-%m-%d_%H-%M")
            ))
        });
        if let Some(a) = &audio {
            if let Some(dir) = a.parent() {
                std::fs::create_dir_all(dir)
                    .map_err(|e| format!("Dersler klasörü açılamadı: {e}"))?;
            }
        }
        // Mikrofon açılabildi mi, kayıt başlamadan öğrenilir.
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        {
            let (sh, mic, audio) = (shared.clone(), mic.to_string(), audio.clone());
            std::thread::Builder::new()
                .name("ders-kaydi".into())
                .spawn(move || capture(&sh, &mic, audio, ready_tx))
                .map_err(|e| e.to_string())?;
        }
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err("Mikrofon açılamadı (zaman aşımı)".into()),
        }
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(Session {
            title: title.clone(),
            started_at,
            shared: shared.clone(),
            finished: false,
            note_id: None,
            audio: audio.map(|a| a.to_string_lossy().into_owned()),
        });
        let app2 = app.clone();
        std::thread::Builder::new()
            .name("ders-yazi".into())
            .spawn(move || worker(&app2, &shared))
            .map_err(|e| e.to_string())?;
        crate::log!("Ders kaydı başladı: {title}");
        let _ = app.emit("lecture-changed", ());
        Ok(self.status())
    }

    pub fn pause(&self, app: &AppHandle, paused: bool) {
        if let Some(s) = self.0.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            s.shared.paused.store(paused, Ordering::SeqCst);
        }
        let _ = app.emit("lecture-changed", ());
    }

    /// Kaydı bitirir: kalan ses yazıya dökülür, sonra not kaydedilir (`lecture-changed` ile haber verilir).
    pub fn stop(&self, app: &AppHandle) {
        if let Some(s) = self.0.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            s.shared.paused.store(false, Ordering::SeqCst);
            s.shared.stop.store(true, Ordering::SeqCst);
        }
        let _ = app.emit("lecture-changed", ());
    }

    /// Bitmiş kaydın ekranını temizler (yeni ders için).
    pub fn clear(&self) {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if g.as_ref().is_some_and(|s| s.finished) {
            *g = None;
        }
    }

    /// Uygulama kapanırken: kayıt durur, yazıya dökülmüş kısım not olarak saklanır.
    pub fn shutdown(&self, app: &AppHandle) {
        let running = {
            let g = self.0.lock().unwrap_or_else(|e| e.into_inner());
            g.as_ref().filter(|s| !s.finished).map(|s| s.shared.clone())
        };
        if let Some(sh) = running {
            sh.stop.store(true, Ordering::SeqCst);
            // Yazıya dökme sürüyorsa kısa bir süre beklenir; bitmezse eldeki kaydedilir.
            for _ in 0..30 {
                if !sh.busy.load(Ordering::SeqCst) && sh.capture_done.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            self.finish(app, true);
        }
    }

    /// Notu kaydeder ve oturumu "bitti" yapar.
    fn finish(&self, app: &AppHandle, interrupted: bool) {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let Some(s) = g.as_mut() else { return };
        if s.finished {
            return;
        }
        s.finished = true;
        let segments = s
            .shared
            .segments
            .lock()
            .map(|v| v.clone())
            .unwrap_or_default();
        let seconds = s.shared.recorded.load(Ordering::SeqCst) / RATE as u64;
        let body = note_body(
            s.started_at,
            seconds,
            &segments,
            s.audio.as_deref(),
            interrupted,
        );
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        let date = chrono::Local::now().format("%d.%m.%Y").to_string();
        let input = crate::notes::NoteInput {
            id: None,
            title: format!("{} · {date}", s.title),
            body,
            color: "default".into(),
            font: "inter".into(),
            pinned: false,
        };
        match crate::notes::save(&db, &dir.folder(Some("Notlar")), input) {
            Ok(n) => {
                crate::log!("Ders kaydı Notlar'a kaydedildi: {} ({seconds} sn)", s.title);
                let _ = app.emit("notes-changed", &n.id);
                s.note_id = Some(n.id);
            }
            Err(e) => {
                *s.shared.error.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(format!("Not kaydedilemedi: {e}"));
            }
        }
        drop(g);
        let _ = app.emit("lecture-changed", ());
    }
}

fn safe_name(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(50)
        .collect();
    let out = out.trim().to_string();
    if out.is_empty() {
        "Ders".into()
    } else {
        out
    }
}

/// "01:05:09" ya da "05:09"
pub fn clock(sec: u64) -> String {
    let (h, m, s) = (sec / 3600, sec / 60 % 60, sec % 60);
    if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Notun gövdesi (Markdown): süre bilgisi, dakika işaretli paragraflar.
pub fn note_body(
    started_at: i64,
    seconds: u64,
    segments: &[Segment],
    audio: Option<&str>,
    interrupted: bool,
) -> String {
    use chrono::TimeZone;
    let start = chrono::Local
        .timestamp_millis_opt(started_at)
        .single()
        .map(|t| t.format("%H:%M").to_string())
        .unwrap_or_default();
    let mins = seconds.div_ceil(60);
    let len = if mins >= 60 {
        format!("{} sa {} dk", mins / 60, mins % 60)
    } else {
        format!("{mins} dk")
    };
    let mut out = format!("*Ders kaydı · {start} başladı · {len}*\n\n");
    if interrupted {
        out.push_str("*GMID kapanırken kayıt yarıda kaldı; buraya kadar olan kısım.*\n\n");
    }
    if segments.is_empty() {
        out.push_str("*Yazıya dökülecek konuşma duyulmadı.*\n");
    }
    for s in segments {
        out.push_str(&format!("**{}** {}\n\n", clock(s.at), s.text.trim()));
    }
    if let Some(a) = audio {
        out.push_str(&format!("---\n\nSes kaydı: `{a}`\n"));
    }
    out
}

/// Sesi nerede bölmeli: `[min, buf.len())` aralığındaki en sessiz pencerenin ortası.
pub fn cut_point(buf: &[i16], min: usize) -> usize {
    if buf.len() <= min + QUIET_WIN {
        return buf.len();
    }
    let step = QUIET_WIN / 3;
    let mut best = (f64::MAX, buf.len());
    let mut i = min;
    while i + QUIET_WIN <= buf.len() {
        let e: f64 = buf[i..i + QUIET_WIN]
            .iter()
            .map(|v| (*v as f64).powi(2))
            .sum();
        if e < best.0 {
            best = (e, i + QUIET_WIN / 2);
        }
        i += step;
    }
    best.1
}

/// Ses seviyesini dengeler: kısık gelen ders (bilgisayar sesi kısık, hoca mikrofondan uzak) Whisper'a duyulur
/// seviyede gider. En çok 8 kat yükseltilir; tepeler sınırlanır (cızırtı olmasın).
pub fn normalize(s: &[i16]) -> Vec<i16> {
    let level = rms(s);
    if level < 1.0 {
        return s.to_vec();
    }
    let peak = s
        .iter()
        .map(|v| (*v as f64).abs())
        .fold(0.0, f64::max)
        .max(1.0);
    // Hedef ~ -20 dBFS ortalama, tepeler -1 dBFS'i geçmesin.
    let gain = (3300.0 / level).min(8.0).min(29_000.0 / peak).max(1.0);
    s.iter()
        .map(|v| (*v as f64 * gain).clamp(-32767.0, 32767.0) as i16)
        .collect()
}

fn rms(s: &[i16]) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    (s.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / s.len() as f64).sqrt()
}

/// Biriken sesi parça parça yazıya döker; kayıt bitince kalanı da döker ve notu kaydeder.
/// Taslak en erken bu kadar ses birikince ve bu aralıkla okunur (ekran kartı yoksa daha seyrek).
const DRAFT_MIN: usize = RATE * 3 / 2;
const DRAFT_EVERY: Duration = Duration::from_millis(2500);
const DRAFT_EVERY_CPU: Duration = Duration::from_secs(6);

fn worker(app: &AppHandle, sh: &Arc<Shared>) {
    let mut last_draft = std::time::Instant::now();
    loop {
        let stopping = sh.stop.load(Ordering::SeqCst) && sh.capture_done.load(Ordering::SeqCst);
        let chunk = {
            let mut p = sh.pending.lock().unwrap_or_else(|e| e.into_inner());
            let take = if p.len() >= CHUNK {
                cut_point(&p, MIN_CHUNK)
            } else if stopping {
                p.len()
            } else {
                0
            };
            if take == 0 {
                None
            } else {
                let at = sh.pending_at.fetch_add(take as u64, Ordering::SeqCst);
                Some((at, p.drain(..take).collect::<Vec<i16>>()))
            }
        };
        match chunk {
            Some((at, samples)) => {
                transcribe(app, sh, at, &samples);
                sh.draft.lock().unwrap_or_else(|e| e.into_inner()).clear();
                last_draft = std::time::Instant::now();
            }
            None if stopping => break,
            None => {
                let gpu = app
                    .state::<crate::dictation::Dictation>()
                    .engine
                    .running_on_gpu()
                    != Some(false);
                let every = if gpu { DRAFT_EVERY } else { DRAFT_EVERY_CPU };
                if last_draft.elapsed() >= every && !sh.paused.load(Ordering::SeqCst) {
                    let snapshot = {
                        let p = sh.pending.lock().unwrap_or_else(|e| e.into_inner());
                        (p.len() >= DRAFT_MIN).then(|| p.clone())
                    };
                    if let Some(samples) = snapshot {
                        draft(app, sh, &samples);
                        last_draft = std::time::Instant::now();
                        continue;
                    }
                }
                std::thread::sleep(Duration::from_millis(300));
            }
        }
    }
    let lecture = app.state::<Lecture>();
    lecture.finish(app, false);
}

/// Biriken (henüz bölünmemiş) sesin hızlı okuması: yalnız ekranda taslak olarak gösterilir, nota girmez.
fn draft(app: &AppHandle, sh: &Shared, samples: &[i16]) {
    if rms(samples) < 60.0 {
        return;
    }
    let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
    let (cfg, style) = crate::dictation::lecture_setup(&db);
    let prompt = [sh.terms.as_str(), style]
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(". ");
    let engine = &app.state::<crate::dictation::Dictation>().engine;
    let wav = crate::dictation::to_wav(&normalize(samples));
    if let Ok(h) = engine.transcribe_with(dir.root(), &cfg, &wav, &prompt, false) {
        let text = crate::transcript::parse::clean_dictation(&h.text);
        // Bu arada parça kesinleştiyse eski taslak yazılmasın.
        let still = sh
            .pending
            .lock()
            .map(|p| p.len() >= samples.len())
            .unwrap_or(false);
        if still {
            *sh.draft.lock().unwrap_or_else(|e| e.into_inner()) = text;
            let _ = app.emit("lecture-changed", ());
        }
    }
}

fn transcribe(app: &AppHandle, sh: &Shared, at: u64, samples: &[i16]) {
    // Yarım saniyeden kısa ya da sessiz parça atlanır (ders arası, sessizlik).
    if samples.len() < RATE / 2 || rms(samples) < 60.0 {
        return;
    }
    sh.busy.store(true, Ordering::SeqCst);
    let _ = app.emit("lecture-changed", ());
    let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
    // Sesle yazmanın kişisel sözlüğü ve sözcükleri (oyun adları, takma adlar) derse karışmasın: yalnız dersin
    // terimleri, dilin noktalama örneği ve önceki parçanın sonu (cümle bölünse de devamı anlaşılsın).
    let (cfg, style) = crate::dictation::lecture_setup(&db);
    let tail: String = {
        let segs = sh.segments.lock().unwrap_or_else(|e| e.into_inner());
        let last = segs.last().map(|s| s.text.as_str()).unwrap_or_default();
        let chars: Vec<char> = last.chars().collect();
        chars[chars.len().saturating_sub(200)..].iter().collect()
    };
    let prompt = [sh.terms.as_str(), style, tail.as_str()]
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(". ");
    let engine = &app.state::<crate::dictation::Dictation>().engine;
    // Ders beklemeye dayanır: ışın aramasıyla (daha az "yakın ama yanlış" sözcük); ses seviyesi dengelenir.
    let wav = crate::dictation::to_wav(&normalize(samples));
    let result = engine.transcribe_with(dir.root(), &cfg, &wav, &prompt, true);
    match result {
        Ok(heard) => {
            let text = crate::transcript::parse::clean_dictation(&heard.text);
            if !text.trim().is_empty() {
                sh.segments
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(Segment {
                        at: at / RATE as u64,
                        text,
                    });
            }
            *sh.error.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
        Err(e) => {
            let msg = if e == "WHISPER_MISSING" {
                "Whisper modeli kurulu değil (Kontrol Odası → Sesle yazma).".to_string()
            } else {
                e
            };
            crate::log!("Ders kaydı: {msg}");
            *sh.error.lock().unwrap_or_else(|e| e.into_inner()) = Some(msg);
        }
    }
    sh.busy.store(false, Ordering::SeqCst);
    let _ = app.emit("lecture-changed", ());
}

/// WAV dosyası: başlık sonda düzeltilir (uzunluk kayıt bitince belli olur).
struct WavFile {
    file: std::fs::File,
    bytes: u32,
}

impl WavFile {
    fn create(path: &PathBuf) -> std::io::Result<WavFile> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(&crate::dictation::to_wav(&[]))?;
        Ok(WavFile { file, bytes: 0 })
    }

    fn push(&mut self, s: &[i16]) {
        let b: Vec<u8> = s.iter().flat_map(|v| v.to_le_bytes()).collect();
        if self.file.write_all(&b).is_ok() {
            self.bytes = self.bytes.saturating_add(b.len() as u32);
        }
    }

    fn close(mut self) {
        let _ = self.file.seek(SeekFrom::Start(4));
        let _ = self.file.write_all(&(36 + self.bytes).to_le_bytes());
        let _ = self.file.seek(SeekFrom::Start(40));
        let _ = self.file.write_all(&self.bytes.to_le_bytes());
    }
}

/// Gelen ses (kanallar ve hız ne olursa) → 16 kHz tek kanal 16 bit. Bilgisayarın sesi genelde 48 kHz stereo
/// gelir: kanallar ortalanır, sonra hız düşürülmeden önce 7 kHz üstü süzülür (pencereli sinc, 95 katsayı).
/// Süzmeden düşürmek yüksek sesleri ("s", "ş", "ç") konuşma bandına katlar; Whisper yakın ama yanlış sözcük yazar.
pub struct Downmix {
    step: f64,
    pos: f64,
    taps: Vec<f32>,
    hist: Vec<f32>,
    at: usize,
}

impl Downmix {
    pub fn new(rate: u32) -> Downmix {
        let rate = rate.max(1) as f64;
        const N: usize = 95;
        // Kesim: 16 kHz'in yarısının biraz altı (7 kHz), giriş hızına göre.
        let fc = 7000.0 / rate;
        let mid = (N - 1) as f64 / 2.0;
        let mut taps: Vec<f32> = (0..N)
            .map(|i| {
                let x = i as f64 - mid;
                let sinc = if x == 0.0 {
                    2.0 * fc
                } else {
                    (2.0 * std::f64::consts::PI * fc * x).sin() / (std::f64::consts::PI * x)
                };
                // Blackman penceresi.
                let w = 0.42 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (N - 1) as f64).cos()
                    + 0.08 * (4.0 * std::f64::consts::PI * i as f64 / (N - 1) as f64).cos();
                (sinc * w) as f32
            })
            .collect();
        let sum: f32 = taps.iter().sum();
        for t in &mut taps {
            *t /= sum;
        }
        Downmix {
            step: RATE as f64 / rate,
            pos: 0.0,
            hist: vec![0.0; N],
            taps,
            at: 0,
        }
    }

    /// Bir karenin (tüm kanalların ortalaması, -1..1) eklenmesi.
    pub fn push(&mut self, mono: f32, out: &mut Vec<i16>) {
        let n = self.hist.len();
        self.hist[self.at] = mono;
        self.at = (self.at + 1) % n;
        self.pos += self.step;
        if self.pos >= 1.0 {
            self.pos -= 1.0;
            // Halka arabellek: en eski örnekten en yeniye.
            let mut acc = 0.0f32;
            for (k, t) in self.taps.iter().enumerate() {
                acc += t * self.hist[(self.at + k) % n];
            }
            out.push((acc.clamp(-1.0, 1.0) * 32767.0) as i16);
        }
    }
}

/// Kayıt kaynağı: `"system"` bilgisayarın sesi (online ders: tarayıcı, Zoom, Teams…; kulaklık takılıyken de),
/// boş metin varsayılan mikrofon, başka bir şey mikrofonun kimliği.
pub const SYSTEM: &str = "system";

#[cfg(windows)]
fn capture(
    sh: &Shared,
    source: &str,
    audio: Option<PathBuf>,
    ready: std::sync::mpsc::Sender<Result<(), String>>,
) {
    use windows::core::{GUID, HSTRING};
    use windows::Win32::Media::Audio::{
        eCapture, eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
        MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_LOOPBACK,
        AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
        WAVE_FORMAT_PCM,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    /// Gelen örneklerin biçimi.
    #[derive(Clone, Copy, PartialEq)]
    enum Sample {
        I16,
        I32,
        F32,
    }
    struct Opened {
        client: IAudioClient,
        cap: IAudioCaptureClient,
        channels: usize,
        sample: Sample,
        rate: u32,
    }
    const FLOAT: GUID = GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);
    let system = source == SYSTEM;

    let open = || -> Result<Opened, String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let err = |what: &str, e: windows::core::Error| {
                if e.code().0 as u32 == 0x8007_0005 {
                    "Mikrofon izni yok: Windows Ayarlar → Gizlilik → Mikrofon → \"Masaüstü uygulamalarının erişimine izin ver\" açık olmalı.".to_string()
                } else {
                    format!("{what}: {e}")
                }
            };
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| err("Ses sistemi açılamadı", e))?;
            if system {
                // Bilgisayarın sesi: varsayılan çıkışın "geri döngüsü" (kulaklıktan duyulan her şey).
                let dev = en
                    .GetDefaultAudioEndpoint(eRender, eConsole)
                    .map_err(|e| err("Ses çıkışı bulunamadı", e))?;
                let client: IAudioClient = dev
                    .Activate(CLSCTX_ALL, None)
                    .map_err(|e| err("Ses çıkışı açılamadı", e))?;
                let pfmt = client
                    .GetMixFormat()
                    .map_err(|e| err("Ses biçimi okunamadı", e))?;
                let f: WAVEFORMATEX = *pfmt;
                let float = f.wFormatTag == 3
                    || (f.wFormatTag == 0xFFFE && {
                        let ext = std::ptr::read_unaligned(pfmt as *const WAVEFORMATEXTENSIBLE);
                        let sub = ext.SubFormat;
                        sub == FLOAT
                    });
                let sample = match (float, f.wBitsPerSample) {
                    (true, 32) => Some(Sample::F32),
                    (false, 16) => Some(Sample::I16),
                    (false, 32) => Some(Sample::I32),
                    _ => None,
                };
                let init = client.Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK,
                    2_000_000,
                    0,
                    pfmt,
                    None,
                );
                CoTaskMemFree(Some(pfmt as *const _));
                let sample = sample.ok_or("Bilgisayarın ses biçimi desteklenmiyor")?;
                init.map_err(|e| err("Bilgisayarın sesi dinlenemedi", e))?;
                let cap: IAudioCaptureClient =
                    client.GetService().map_err(|e| err("Ses okunamadı", e))?;
                client.Start().map_err(|e| err("Kayıt başlatılamadı", e))?;
                return Ok(Opened {
                    client,
                    cap,
                    channels: f.nChannels.max(1) as usize,
                    sample,
                    rate: f.nSamplesPerSec,
                });
            }
            let dev = if source.is_empty() {
                en.GetDefaultAudioEndpoint(eCapture, eConsole)
            } else {
                en.GetDevice(&HSTRING::from(source))
                    .or_else(|_| en.GetDefaultAudioEndpoint(eCapture, eConsole))
            }
            .map_err(|e| err("Mikrofon bulunamadı", e))?;
            let client: IAudioClient = dev
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| err("Mikrofon açılamadı", e))?;
            let fmt = WAVEFORMATEX {
                wFormatTag: WAVE_FORMAT_PCM as u16,
                nChannels: 1,
                nSamplesPerSec: RATE as u32,
                nAvgBytesPerSec: RATE as u32 * 2,
                nBlockAlign: 2,
                wBitsPerSample: 16,
                cbSize: 0,
            };
            client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                    2_000_000,
                    0,
                    &fmt,
                    None,
                )
                .map_err(|e| err("Mikrofon başlatılamadı", e))?;
            let cap: IAudioCaptureClient = client
                .GetService()
                .map_err(|e| err("Mikrofon okunamadı", e))?;
            client.Start().map_err(|e| err("Kayıt başlatılamadı", e))?;
            Ok(Opened {
                client,
                cap,
                channels: 1,
                sample: Sample::I16,
                rate: RATE as u32,
            })
        }
    };
    let o = match open() {
        Ok(x) => x,
        Err(e) => {
            let _ = ready.send(Err(e));
            sh.capture_done.store(true, Ordering::SeqCst);
            return;
        }
    };
    let mut wav = audio.and_then(|p| WavFile::create(&p).ok());
    let _ = ready.send(Ok(()));
    let mut down = Downmix::new(o.rate);
    let mut chunk: Vec<i16> = Vec::with_capacity(RATE);
    // Bilgisayarın sesi: hiçbir şey çalmazken Windows örnek göndermez. Zaman çizelgesi kaymasın diye geçen süre
    // kadar sessizlik eklenir (duraklatılan süre sayılmaz).
    let mut last = std::time::Instant::now();
    let mut wall: f64 = 0.0;
    let mut produced: u64 = 0;
    while !sh.stop.load(Ordering::SeqCst) {
        chunk.clear();
        unsafe {
            while o.cap.GetNextPacketSize().unwrap_or(0) > 0 {
                let mut data: *mut u8 = std::ptr::null_mut();
                let (mut frames, mut flags) = (0u32, 0u32);
                if o.cap
                    .GetBuffer(&mut data, &mut frames, &mut flags, None, None)
                    .is_err()
                {
                    break;
                }
                let n = frames as usize;
                let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null();
                if o.sample == Sample::I16 && o.channels == 1 && o.rate == RATE as u32 {
                    if silent {
                        chunk.extend(std::iter::repeat(0i16).take(n));
                    } else {
                        chunk.extend_from_slice(std::slice::from_raw_parts(data as *const i16, n));
                    }
                } else {
                    let ch = o.channels;
                    for f in 0..n {
                        let mono = if silent {
                            0.0
                        } else {
                            let mut sum = 0.0f32;
                            for c in 0..ch {
                                let i = f * ch + c;
                                sum += match o.sample {
                                    Sample::F32 => *(data as *const f32).add(i),
                                    Sample::I16 => *(data as *const i16).add(i) as f32 / 32768.0,
                                    Sample::I32 => {
                                        *(data as *const i32).add(i) as f32 / 2_147_483_648.0
                                    }
                                };
                            }
                            sum / ch as f32
                        };
                        down.push(mono, &mut chunk);
                    }
                }
                let _ = o.cap.ReleaseBuffer(frames);
            }
        }
        let paused = sh.paused.load(Ordering::SeqCst);
        let dt = last.elapsed().as_secs_f64();
        last = std::time::Instant::now();
        if paused {
            chunk.clear();
            sh.level.store(0f32.to_bits(), Ordering::Relaxed);
        } else if system {
            wall += dt * RATE as f64;
            let have = produced + chunk.len() as u64;
            if (have as f64) < wall - (RATE / 4) as f64 {
                let pad = (wall - (RATE / 10) as f64) as u64 - have;
                chunk.extend(std::iter::repeat(0i16).take(pad as usize));
            }
        }
        if !chunk.is_empty() && !paused {
            produced += chunk.len() as u64;
            sh.level.store(
                ((rms(&chunk) / 6000.0).min(1.0) as f32).to_bits(),
                Ordering::Relaxed,
            );
            sh.pending
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend_from_slice(&chunk);
            let total =
                sh.recorded.fetch_add(chunk.len() as u64, Ordering::SeqCst) + chunk.len() as u64;
            if let Some(w) = wav.as_mut() {
                w.push(&chunk);
            }
            if total / RATE as u64 >= MAX_SECONDS {
                sh.stop.store(true, Ordering::SeqCst);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    unsafe {
        let _ = o.client.Stop();
    }
    if let Some(w) = wav {
        w.close();
    }
    sh.capture_done.store(true, Ordering::SeqCst);
}

#[cfg(not(windows))]
fn capture(
    sh: &Shared,
    _mic: &str,
    _audio: Option<PathBuf>,
    ready: std::sync::mpsc::Sender<Result<(), String>>,
) {
    let _ = ready.send(Err("Ders kaydı yalnız Windows'ta çalışır".into()));
    sh.capture_done.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_at_the_quietest_moment() {
        // 20 sn ses, 21. saniyede kısa bir sessizlik, sonra yine ses.
        let mut buf = vec![3000i16; RATE * 26];
        for v in &mut buf[RATE * 21..RATE * 21 + RATE / 2] {
            *v = 0;
        }
        let cut = cut_point(&buf, MIN_CHUNK);
        assert!((RATE * 21..RATE * 21 + RATE / 2).contains(&cut), "{cut}");
        // Kısa arabellek: hepsi.
        assert_eq!(cut_point(&buf[..RATE * 10], MIN_CHUNK), RATE * 10);
    }

    #[test]
    fn note_has_times_and_text() {
        let segs = vec![
            Segment {
                at: 0,
                text: "Bugün türevlere başlıyoruz.".into(),
            },
            Segment {
                at: 3725,
                text: "Ödev: sayfa 42.".into(),
            },
        ];
        let body = note_body(0, 3800, &segs, Some("C:\\GMID\\Dersler\\Fizik.wav"), false);
        assert!(body.contains("1 sa 4 dk"), "{body}");
        assert!(body.contains("**00:00** Bugün türevlere başlıyoruz."));
        assert!(body.contains("**01:02:05** Ödev: sayfa 42."));
        assert!(body.contains("Fizik.wav"));
        assert!(note_body(0, 10, &[], None, true).contains("yarıda kaldı"));
        assert_eq!(clock(65), "01:05");
    }

    #[test]
    fn computer_sound_is_converted_to_16k_mono() {
        // 48 kHz → 16 kHz: sayı üçte bir, düz ses aynen geçer (süzgeç yerleştikten sonra).
        let mut d = Downmix::new(48_000);
        let mut out = Vec::new();
        for _ in 0..48_000 {
            d.push(0.5, &mut out);
        }
        assert!((15_999..=16_001).contains(&out.len()), "{}", out.len());
        assert!((out[1000] - 16383).abs() <= 40, "{}", out[1000]);
        // 12 kHz'lik ıslık (16 kHz'de katlanıp 4 kHz'e düşerdi) süzülür; 1 kHz'lik ses kalır.
        let tone = |hz: f64| {
            let mut d = Downmix::new(48_000);
            let mut out = Vec::new();
            for i in 0..48_000 {
                d.push(
                    (0.5 * (2.0 * std::f64::consts::PI * hz * i as f64 / 48_000.0).sin()) as f32,
                    &mut out,
                );
            }
            rms(&out[1000..])
        };
        assert!(tone(1000.0) > 10_000.0);
        assert!(tone(12_000.0) < 300.0, "{}", tone(12_000.0));
    }

    #[test]
    fn quiet_lectures_are_boosted() {
        let quiet: Vec<i16> = (0..16_000)
            .map(|i| if i % 2 == 0 { 300 } else { -300 })
            .collect();
        let n = normalize(&quiet);
        assert!(rms(&n) > 2000.0, "{}", rms(&n));
        // Yüksek ses kısılmaz, tepeler taşmaz.
        let loud: Vec<i16> = vec![20_000; 100];
        assert_eq!(normalize(&loud), loud);
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(safe_name("Fizik / Türev: 1"), "Fizik _ Türev_ 1");
        assert_eq!(safe_name("   "), "Ders");
    }
}
