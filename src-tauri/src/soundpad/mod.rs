//! Ses Paneli (Soundpad gibi): bir tuşa bir ses ya da şarkı atanır; tuşa basınca o ses mikrofondan çalar,
//! Discord'daki herkes duyar.
//!
//! Windows bir programın başka bir programın mikrofonuna ses koymasına izin vermez. Bu yüzden ses bir
//! **sanal ses kablosuna** (VB-CABLE, ücretsiz) gönderilir: GMID gerçek mikrofonu ve çalan sesleri karıştırıp
//! kablonun girişine ("CABLE Input") yazar, Discord'da mikrofon olarak kablonun çıkışı ("CABLE Output")
//! seçilir. Sesler aynı anda kulaklığa da çalınır (kendin duy). Panel kapalıyken hiçbir şey dinlenmez.

// Ses motoru yalnız Windows'ta derlenir; diğer sistemlerde karıştırıcı parçaları yalnız testlerde kullanılır.
#![cfg_attr(not(windows), allow(dead_code))]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[cfg(windows)]
mod win;

/// Motorun çalıştığı örnekleme hızı (Windows gerekirse dönüştürür).
pub const RATE: u32 = 48_000;
/// Belleğe çözülmüş seslerin toplam sınırı (~ 6 dk'lık 25 şarkı): aşılınca en eski kullanılan atılır.
const CACHE_BYTES: usize = 300 * 1024 * 1024;
/// En uzun ses: 15 dakika (tek kanal, 16 bit ≈ 86 MB).
const MAX_SECONDS: usize = 15 * 60;
pub const SETTING: &str = "soundpad";
/// Sanal kablonun indirme sayfası (sürücü olduğu için GMID kendisi kurmaz).
pub const CABLE_URL: &str = "https://vb-audio.com/Cable/";
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "wav", "ogg", "opus", "m4a", "aac", "flac", "wma", "mp4", "webm", "mkv", "mov",
];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Pad {
    pub id: String,
    /// Kısayol ("Numpad1", "Ctrl+Alt+1"…); boşsa yalnız tıklayarak çalar.
    pub key: String,
    pub path: String,
    pub name: String,
    /// Bu sesin kendi seviyesi (0–2, 1 = olduğu gibi).
    pub volume: f32,
}

impl Default for Pad {
    fn default() -> Self {
        Pad {
            id: String::new(),
            key: String::new(),
            path: String::new(),
            name: String::new(),
            volume: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub enabled: bool,
    pub pads: Vec<Pad>,
    /// Karşıya giden seslerin seviyesi (0–1).
    pub sound_volume: f32,
    /// Gerçek mikrofonun seviyesi (0–2).
    pub mic_volume: f32,
    /// Gerçek mikrofon da kabloya aktarılsın mı (kapalıysa yalnız sesler gider).
    pub passthrough: bool,
    /// Sesler kulaklıktan da duyulsun mu.
    pub monitor: bool,
    pub monitor_volume: f32,
    /// Seçilen mikrofon (boşsa Windows'un varsayılanı).
    pub mic_id: String,
    /// Çalan her şeyi durduran tuş.
    pub stop_key: String,
    /// Yeni ses çalan sesi kesmesin, üstüne binsin.
    pub overlap: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: false,
            pads: Vec::new(),
            sound_volume: 0.5,
            mic_volume: 1.0,
            passthrough: true,
            monitor: true,
            monitor_volume: 0.5,
            mic_id: String::new(),
            stop_key: "Numpad0".into(),
            overlap: false,
        }
    }
}

impl Config {
    /// Arayüzden gelen değerleri sınırlar içine alır.
    fn clamp(mut self) -> Self {
        self.sound_volume = self.sound_volume.clamp(0.0, 1.0);
        self.mic_volume = self.mic_volume.clamp(0.0, 2.0);
        self.monitor_volume = self.monitor_volume.clamp(0.0, 1.0);
        for p in &mut self.pads {
            p.volume = p.volume.clamp(0.0, 2.0);
            p.key = p.key.trim().to_string();
        }
        self.stop_key = self.stop_key.trim().to_string();
        self
    }

    /// Aynı tuş iki işe atanmış mı? (ilk çakışan tuş)
    pub fn duplicate_key(&self) -> Option<String> {
        let mut seen: Vec<String> = Vec::new();
        let keys = self
            .pads
            .iter()
            .map(|p| p.key.as_str())
            .chain([self.stop_key.as_str()]);
        for k in keys.filter(|k| !k.is_empty()) {
            let k = k.to_ascii_lowercase();
            if seen.contains(&k) {
                return Some(k);
            }
            seen.push(k);
        }
        None
    }

    /// Yeni ses için ilk boş numpad tuşu (1–9).
    pub fn free_key(&self) -> String {
        (1..=9)
            .map(|n| format!("Numpad{n}"))
            .find(|k| {
                !self.pads.iter().any(|p| p.key.eq_ignore_ascii_case(k))
                    && !self.stop_key.eq_ignore_ascii_case(k)
            })
            .unwrap_or_default()
    }
}

/// Bir ses aygıtı (seçim listesi için).
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    /// Sanal kablo mu (VB-CABLE, Voicemeeter).
    pub virtual_cable: bool,
}

/// Aygıtların durumu: kablo var mı, Windows varsayılanları kabloya mı kaymış.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Devices {
    /// Seslerin yazılacağı kablo girişi (ör. "CABLE Input (VB-Audio Virtual Cable)").
    pub cable: Option<Endpoint>,
    /// Discord'da seçilecek adı (ör. "CABLE Output").
    pub cable_mic: String,
    pub inputs: Vec<Endpoint>,
    /// Windows'un varsayılan hoparlörü kablo olmuş: kullanıcı hiçbir şey duymaz.
    pub default_output_is_cable: bool,
    /// Windows'un varsayılan mikrofonu kablo olmuş: GMID kendi sesini dinlerdi.
    pub default_input_is_cable: bool,
    pub default_output_name: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub supported: bool,
    pub config: Config,
    pub devices: Devices,
    pub running: bool,
    pub error: Option<String>,
    /// Çalan seslerin kimlikleri.
    pub playing: Vec<String>,
    /// Bulunamayan dosyalar (silinmiş, taşınmış).
    pub missing: Vec<String>,
    /// Tuşu kaydedilemeyen sesler (başka program kullanıyor).
    pub key_errors: HashMap<String, String>,
    pub ffmpeg: bool,
}

/// Belleğe çözülen ses: 48 kHz tek kanal. Çözme sürerken çalmaya başlanabilir; okuyucu yetişirse bekler.
pub struct Clip {
    pub samples: RwLock<Vec<i16>>,
    pub done: AtomicBool,
    pub failed: Mutex<Option<String>>,
    used: Mutex<Instant>,
}

impl Clip {
    fn new() -> Arc<Clip> {
        Arc::new(Clip {
            samples: RwLock::new(Vec::new()),
            done: AtomicBool::new(false),
            failed: Mutex::new(None),
            used: Mutex::new(Instant::now()),
        })
    }

    fn bytes(&self) -> usize {
        self.samples.read().map(|s| s.capacity() * 2).unwrap_or(0)
    }
}

/// Motora giden komutlar.
pub enum Cmd {
    Play {
        id: String,
        clip: Arc<Clip>,
        gain: f32,
        exclusive: bool,
    },
    Stop(Option<String>),
    Levels(Levels),
    Quit,
}

#[derive(Clone, Copy, Debug)]
pub struct Levels {
    pub sound: f32,
    pub mic: f32,
    pub passthrough: bool,
    pub monitor: bool,
    pub monitor_volume: f32,
}

impl Levels {
    fn of(c: &Config) -> Levels {
        Levels {
            sound: c.sound_volume,
            mic: c.mic_volume,
            passthrough: c.passthrough,
            monitor: c.monitor,
            monitor_volume: c.monitor_volume,
        }
    }
}

/// Çalışan motor: komut kanalı ve çalanlar listesi.
struct Engine {
    tx: std::sync::mpsc::Sender<Cmd>,
    playing: Arc<Mutex<Vec<String>>>,
    alive: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
}

#[derive(Default)]
struct Inner {
    engine: Option<Engine>,
    cache: HashMap<String, Arc<Clip>>,
    /// Kayıtlı kısayollar: kısayol kimliği → ses kimliği ("" = durdur).
    shortcuts: Vec<(Shortcut, String)>,
    key_errors: HashMap<String, String>,
    /// Son başlatma hatası (motor açılamadıysa).
    error: Option<String>,
}

#[derive(Default)]
pub struct Soundpad(Mutex<Inner>);

pub fn config(app: &AppHandle) -> Config {
    app.state::<crate::db::Db>()
        .setting_json::<Config>(SETTING)
        .unwrap_or_default()
}

/// Bir ses aygıtının adı sanal kabloya mı ait.
pub fn is_virtual(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("vb-audio")
        || n.contains("cable input")
        || n.contains("cable output")
        || n.contains("voicemeeter")
}

impl Soundpad {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn status(&self, app: &AppHandle) -> Status {
        let config = config(app);
        let devices = devices();
        let mut inner = self.lock();
        let (running, playing, error) = match &inner.engine {
            Some(e) if e.alive.load(Ordering::SeqCst) => (
                true,
                e.playing.lock().map(|p| p.clone()).unwrap_or_default(),
                None,
            ),
            Some(e) => (
                false,
                Vec::new(),
                e.error.lock().ok().and_then(|x| x.clone()),
            ),
            None => (false, Vec::new(), None),
        };
        if !running && inner.engine.is_some() {
            inner.engine = None;
        }
        let error = error.or_else(|| inner.error.clone());
        if error.is_some() {
            inner.error = error.clone();
        }
        let missing = config
            .pads
            .iter()
            .filter(|p| !Path::new(&p.path).is_file())
            .map(|p| p.id.clone())
            .collect();
        Status {
            supported: cfg!(windows),
            missing,
            key_errors: inner.key_errors.clone(),
            ffmpeg: crate::karaoke::ffmpeg_path(app).is_ok(),
            config,
            devices,
            running,
            error,
            playing,
        }
    }

    /// Ayarları kaydeder ve uygular: tuşlar yeniden kaydedilir, motor açılır/kapanır, seviyeler değişir.
    pub fn save(&self, app: &AppHandle, config: Config) -> Result<Status, String> {
        let config = config.clamp();
        if let Some(k) = config.duplicate_key() {
            return Err(format!(
                "\"{k}\" tuşu iki kez atanmış. Her tuş tek bir işe atanabilir."
            ));
        }
        let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
        app.state::<crate::db::Db>()
            .set_setting(SETTING, &json)
            .map_err(|e| e.to_string())?;
        self.apply(app, &config);
        Ok(self.status(app))
    }

    /// Açılışta ve her kayıtta: kısayollar, motor ve seviyeler ayarlara uydurulur.
    pub fn apply(&self, app: &AppHandle, config: &Config) {
        self.bind_keys(app, config);
        if config.enabled {
            let mut inner = self.lock();
            let alive = inner
                .engine
                .as_ref()
                .is_some_and(|e| e.alive.load(Ordering::SeqCst));
            if alive {
                if let Some(e) = &inner.engine {
                    let _ = e.tx.send(Cmd::Levels(Levels::of(config)));
                }
            } else {
                inner.engine = None;
                match start(app, config) {
                    Ok(e) => {
                        inner.engine = Some(e);
                        inner.error = None;
                    }
                    Err(e) => {
                        crate::log!("Ses Paneli başlatılamadı: {e}");
                        inner.error = Some(e);
                    }
                }
            }
        } else {
            self.shutdown();
            self.lock().error = None;
        }
        let _ = app.emit("soundpad-changed", ());
    }

    pub fn shutdown(&self) {
        if let Some(e) = self.lock().engine.take() {
            let _ = e.tx.send(Cmd::Quit);
        }
    }

    fn bind_keys(&self, app: &AppHandle, config: &Config) {
        let gs = app.global_shortcut();
        let mut inner = self.lock();
        for (sc, _) in inner.shortcuts.drain(..) {
            let _ = gs.unregister(sc);
        }
        inner.key_errors.clear();
        if !config.enabled {
            return;
        }
        let taken: Vec<String> = crate::hotkeys::list(app)
            .into_iter()
            .map(|h| h.accel.to_ascii_lowercase())
            .filter(|a| !a.is_empty())
            .collect();
        let wanted = config
            .pads
            .iter()
            .map(|p| (p.key.clone(), p.id.clone()))
            .chain([(config.stop_key.clone(), String::new())]);
        for (key, id) in wanted.filter(|(k, _)| !k.is_empty()) {
            let label = if id.is_empty() {
                "stop".to_string()
            } else {
                id.clone()
            };
            if taken.contains(&key.to_ascii_lowercase()) {
                inner.key_errors.insert(
                    label,
                    "Bu tuş GMID'in başka bir kısayolunda kullanılıyor.".into(),
                );
                continue;
            }
            match key.parse::<Shortcut>() {
                Err(_) => {
                    inner.key_errors.insert(label, "Bu tuş tanınmadı.".into());
                }
                Ok(sc) => match gs.register(sc) {
                    Ok(()) => inner.shortcuts.push((sc, id)),
                    Err(_) => {
                        inner
                            .key_errors
                            .insert(label, "Başka bir program bu tuşu kullanıyor.".into());
                    }
                },
            }
        }
    }

    /// Kısayol işleyicisinden: bu kısayol Ses Paneli'nin mi? Öyleyse çal/durdur.
    pub fn on_shortcut(&self, app: &AppHandle, shortcut_id: u32) -> bool {
        let target = self
            .lock()
            .shortcuts
            .iter()
            .find(|(sc, _)| sc.id() == shortcut_id)
            .map(|(_, id)| id.clone());
        let Some(id) = target else { return false };
        let app = app.clone();
        std::thread::spawn(move || {
            let pad = app.state::<Soundpad>();
            if id.is_empty() {
                pad.stop(&app, None);
            } else if let Err(e) = pad.toggle(&app, &id, true) {
                crate::overlay::show_osd(&app, "speaker", &e);
            }
        });
        true
    }

    /// Çalıyorsa durdurur, çalmıyorsa çalar. `osd`: oyundayken köşede adı görünsün.
    pub fn toggle(&self, app: &AppHandle, id: &str, osd: bool) -> Result<(), String> {
        let playing = self
            .lock()
            .engine
            .as_ref()
            .and_then(|e| e.playing.lock().ok().map(|p| p.contains(&id.to_string())))
            .unwrap_or(false);
        if playing {
            self.stop(app, Some(id.to_string()));
            return Ok(());
        }
        self.play(app, id, osd)
    }

    pub fn play(&self, app: &AppHandle, id: &str, osd: bool) -> Result<(), String> {
        let config = config(app);
        if !config.enabled {
            return Err("Ses Paneli kapalı".into());
        }
        let pad = config
            .pads
            .iter()
            .find(|p| p.id == id)
            .ok_or("Bu ses artık yok")?
            .clone();
        if !Path::new(&pad.path).is_file() {
            return Err(format!("Dosya bulunamadı: {}", pad.name));
        }
        // Motor durmuşsa (aygıt çıkarıldı vb.) yeniden açılır.
        let dead = self
            .lock()
            .engine
            .as_ref()
            .map_or(true, |e| !e.alive.load(Ordering::SeqCst));
        if dead {
            self.apply(app, &config);
        }
        let clip = self.clip(app, &pad.path)?;
        let inner = self.lock();
        let engine = inner.engine.as_ref().ok_or_else(|| {
            inner
                .error
                .clone()
                .unwrap_or("Ses Paneli çalışmıyor".into())
        })?;
        engine
            .tx
            .send(Cmd::Play {
                id: pad.id.clone(),
                clip,
                gain: pad.volume,
                exclusive: !config.overlap,
            })
            .map_err(|_| "Ses Paneli çalışmıyor".to_string())?;
        drop(inner);
        if osd {
            crate::overlay::show_osd(app, "play", &pad.name);
        }
        Ok(())
    }

    pub fn stop(&self, app: &AppHandle, id: Option<String>) {
        if let Some(e) = &self.lock().engine {
            let _ = e.tx.send(Cmd::Stop(id));
        }
        let _ = app.emit("soundpad-changed", ());
    }

    /// Önbellekteki sesi verir; yoksa FFmpeg ile çözmeye başlar (çözme bitmeden çalmaya başlanabilir).
    fn clip(&self, app: &AppHandle, path: &str) -> Result<Arc<Clip>, String> {
        let mut inner = self.lock();
        if let Some(c) = inner.cache.get(path) {
            if c.failed.lock().map(|f| f.is_none()).unwrap_or(true) {
                *c.used.lock().unwrap_or_else(|e| e.into_inner()) = Instant::now();
                return Ok(c.clone());
            }
        }
        let ffmpeg = crate::karaoke::ffmpeg_path(app)?;
        let clip = Clip::new();
        decode(ffmpeg, path.to_string(), clip.clone());
        inner.cache.insert(path.to_string(), clip.clone());
        // Sınır aşıldıysa en uzun süredir kullanılmayanlar atılır (çalan ses `Arc` ile yaşamaya devam eder).
        loop {
            let total: usize = inner.cache.values().map(|c| c.bytes()).sum();
            if total <= CACHE_BYTES || inner.cache.len() <= 1 {
                break;
            }
            let oldest = inner
                .cache
                .iter()
                .filter(|(k, _)| k.as_str() != path)
                .min_by_key(|(_, c)| *c.used.lock().unwrap_or_else(|e| e.into_inner()))
                .map(|(k, _)| k.clone());
            match oldest {
                Some(k) => {
                    inner.cache.remove(&k);
                }
                None => break,
            }
        }
        Ok(clip)
    }
}

/// Dosyayı arka planda 48 kHz tek kanal 16 bit olarak çözer; parçalar geldikçe `clip`e eklenir.
fn decode(ffmpeg: std::path::PathBuf, path: String, clip: Arc<Clip>) {
    std::thread::spawn(move || {
        let fail = |msg: String| {
            crate::log!("Ses Paneli: {msg}");
            *clip.failed.lock().unwrap_or_else(|e| e.into_inner()) = Some(msg);
            clip.done.store(true, Ordering::SeqCst);
        };
        let mut cmd = Command::new(ffmpeg);
        cmd.args(["-nostdin", "-loglevel", "error", "-i"])
            .arg(&path)
            .args([
                "-vn",
                "-ac",
                "1",
                "-ar",
                &RATE.to_string(),
                "-f",
                "s16le",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        crate::util::hide_console(&mut cmd);
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return fail(format!("FFmpeg başlatılamadı: {e}")),
        };
        let Some(mut out) = child.stdout.take() else {
            return fail("FFmpeg çıktısı alınamadı".into());
        };
        let max = RATE as usize * MAX_SECONDS;
        let mut buf = vec![0u8; 64 * 1024];
        let mut carry: Option<u8> = None;
        loop {
            let n = match out.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            let mut bytes: Vec<u8> = Vec::with_capacity(n + 1);
            bytes.extend(carry.take());
            bytes.extend_from_slice(&buf[..n]);
            if bytes.len() % 2 == 1 {
                carry = bytes.pop();
            }
            let mut s = clip.samples.write().unwrap_or_else(|e| e.into_inner());
            s.extend(
                bytes
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]])),
            );
            if s.len() >= max {
                s.truncate(max);
                break;
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        if clip.samples.read().map(|s| s.is_empty()).unwrap_or(true) {
            return fail(format!(
                "Ses okunamadı: {}",
                Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ));
        }
        clip.samples
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .shrink_to_fit();
        clip.done.store(true, Ordering::SeqCst);
    });
}

/// Çalan bir ses: iki çıkışın (kablo ve kulaklık) her biri kendi hızında ilerler.
pub struct Voice {
    pub id: String,
    pub clip: Arc<Clip>,
    pub gain: f32,
    pub pos: [usize; 2],
}

impl Voice {
    /// `out` için sıradaki örnekler; ses bitince `true` (çözme sürüyorsa eksik kısım sessiz bırakılır).
    pub fn mix_into(&mut self, out: usize, buf: &mut [f32], gain: f32) -> bool {
        let s = self.clip.samples.read().unwrap_or_else(|e| e.into_inner());
        let pos = self.pos[out];
        let take = buf.len().min(s.len().saturating_sub(pos));
        let g = gain * self.gain / 32768.0;
        for (o, v) in buf.iter_mut().zip(&s[pos..pos + take]) {
            *o += *v as f32 * g;
        }
        self.pos[out] = pos + take;
        self.pos[out] >= s.len() && self.clip.done.load(Ordering::SeqCst)
    }
}

/// Karıştırılmış örneği 16 bite çevirir; taşanlar yumuşakça sınırlanır (patlama olmasın).
pub fn to_i16(x: f32) -> i16 {
    let y = if x.abs() <= 0.9 {
        x
    } else {
        x.signum() * (0.9 + 0.09 * ((x.abs() - 0.9) * 10.0).tanh())
    };
    (y.clamp(-1.0, 1.0) * 32767.0) as i16
}

#[cfg(windows)]
fn start(app: &AppHandle, config: &Config) -> Result<Engine, String> {
    let devs = devices();
    let cable = devs.cable.ok_or(
        "Sanal ses kablosu (VB-CABLE) bulunamadı. Kurulumu Ses Paneli sayfasında anlatılıyor.",
    )?;
    let mic = if !config.mic_id.is_empty() && devs.inputs.iter().any(|d| d.id == config.mic_id) {
        Some(config.mic_id.clone())
    } else {
        // Varsayılan mikrofon kabloysa (VB-CABLE kurulunca Windows bazen yapar) gerçek bir mikrofon seçilir.
        devs.inputs
            .iter()
            .find(|d| d.is_default && !d.virtual_cable)
            .or_else(|| devs.inputs.iter().find(|d| !d.virtual_cable))
            .map(|d| d.id.clone())
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let playing = Arc::new(Mutex::new(Vec::new()));
    let alive = Arc::new(AtomicBool::new(true));
    let error = Arc::new(Mutex::new(None));
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let shared = win::Shared {
        playing: playing.clone(),
        alive: alive.clone(),
        error: error.clone(),
        app: app.clone(),
    };
    let levels = Levels::of(config);
    std::thread::Builder::new()
        .name("ses-paneli".into())
        .spawn(move || win::run(cable.id, mic, levels, rx, shared, ready_tx))
        .map_err(|e| e.to_string())?;
    match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(Engine {
            tx,
            playing,
            alive,
            error,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Ses aygıtları açılamadı (zaman aşımı)".into()),
    }
}

#[cfg(not(windows))]
fn start(_app: &AppHandle, _config: &Config) -> Result<Engine, String> {
    Err("Ses Paneli yalnız Windows'ta çalışır".into())
}

#[cfg(windows)]
pub fn devices() -> Devices {
    win::devices().unwrap_or_default()
}

#[cfg(not(windows))]
pub fn devices() -> Devices {
    Devices::default()
}

/// Açılışta: panel açık bırakıldıysa yeniden başlatılır.
pub fn start_saved(app: &AppHandle) {
    let config = config(app);
    if config.enabled {
        let app = app.clone();
        std::thread::spawn(move || app.state::<Soundpad>().apply(&app, &config));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(id: &str, key: &str) -> Pad {
        Pad {
            id: id.into(),
            key: key.into(),
            ..Default::default()
        }
    }

    #[test]
    fn keys_are_unique_and_filled_in_order() {
        let mut c = Config {
            pads: vec![pad("a", "Numpad1"), pad("b", "Numpad3")],
            ..Default::default()
        };
        assert_eq!(c.free_key(), "Numpad2");
        assert_eq!(c.duplicate_key(), None);
        c.pads.push(pad("c", "numpad3"));
        assert_eq!(c.duplicate_key().as_deref(), Some("numpad3"));
        c.pads.pop();
        c.stop_key = "Numpad1".into();
        assert!(c.duplicate_key().is_some());
    }

    #[test]
    fn levels_are_clamped() {
        let c = Config {
            sound_volume: 3.0,
            mic_volume: -1.0,
            pads: vec![Pad {
                volume: 9.0,
                key: " Numpad1 ".into(),
                ..Default::default()
            }],
            ..Default::default()
        }
        .clamp();
        assert_eq!(c.sound_volume, 1.0);
        assert_eq!(c.mic_volume, 0.0);
        assert_eq!(c.pads[0].volume, 2.0);
        assert_eq!(c.pads[0].key, "Numpad1");
    }

    #[test]
    fn virtual_cables_are_recognised() {
        assert!(is_virtual("CABLE Input (VB-Audio Virtual Cable)"));
        assert!(is_virtual("CABLE Output (VB-Audio Virtual Cable)"));
        assert!(is_virtual("VoiceMeeter Input (VB-Audio VoiceMeeter VAIO)"));
        assert!(!is_virtual("Mikrofon (HyperX Cloud II)"));
        assert!(!is_virtual("Hoparlör (Realtek(R) Audio)"));
    }

    #[test]
    fn voices_mix_and_finish() {
        let clip = Clip::new();
        clip.samples
            .write()
            .unwrap()
            .extend([16384i16, 16384, 16384]);
        let mut v = Voice {
            id: "a".into(),
            clip: clip.clone(),
            gain: 1.0,
            pos: [0, 0],
        };
        let mut buf = [0f32; 2];
        // Çözme sürerken sona gelinse de bitmiş sayılmaz.
        assert!(!v.mix_into(0, &mut buf, 1.0));
        assert!((buf[0] - 0.5).abs() < 1e-3);
        let mut rest = [0f32; 4];
        assert!(!v.mix_into(0, &mut rest, 1.0));
        assert_eq!(rest[1], 0.0);
        clip.done.store(true, Ordering::SeqCst);
        assert!(v.mix_into(0, &mut [0f32; 1], 1.0));
        // Diğer çıkış kendi yerinden devam eder.
        assert!(!v.mix_into(1, &mut [0f32; 1], 1.0));
    }

    #[test]
    fn loud_mixes_are_softly_limited() {
        assert_eq!(to_i16(0.5), 16383);
        assert!(to_i16(1.8) < 32767);
        assert!(to_i16(1.8) > to_i16(0.95));
        assert!(to_i16(-3.0) >= -32767);
    }
}
