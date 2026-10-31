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
pub mod vocab;
#[cfg(windows)]
mod win;

use crate::db::Db;
use crate::paths::DataDir;
use engine::{Engine, Heard, WhisperCfg};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

pub const ENABLED_KEY: &str = "dictation.enabled";
/// Oyun açıkken sesle yazma dinlemesin (varsayılan kapalı: oyunda da çalışır).
pub const OFF_IN_GAME_KEY: &str = "dictation.offInGame";
/// Açılışta modelleri önceden yükle (varsayılan açık).
pub const PRELOAD_KEY: &str = "dictation.preload";

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

/// Sağ alttaki sözlük kartı: `added` ise kural kendiliğinden eklendi ("Geri al" birkaç saniye görünür),
/// değilse "Bunu böyle mi yazayım?" diye sorulur.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Learn {
    pub id: u64,
    pub from: String,
    pub to: String,
    pub added: bool,
}

/// Düzeltmelerden kendiliğinden öğren (varsayılan açık); kapalıysa her seferinde sorulur.
pub const AUTO_LEARN_KEY: &str = "dictation.autoLearn";
/// Kendiliğinden eklenen kuralın "Geri al" kartı bu kadar saniye durur.
const UNDO_SECS: u64 = 20;
/// Öğrenince haber ver (sağ altta Geri al kartı + kısa bildirim); varsayılan açık.
pub const LEARN_NOTIFY_KEY: &str = "dictation.learnNotify";

#[derive(Default)]
pub struct Dictation {
    pub engine: Engine,
    hook: AtomicBool,
    enabled: AtomicBool,
    busy: AtomicBool,
    /// Meşgul olmaya başladığı an (ms): bir dikte takılı kalırsa sonrakileri sonsuza dek engellemesin.
    busy_since: AtomicU64,
    phase: Mutex<(&'static str, Option<String>)>,
    history: Mutex<VecDeque<Entry>>,
    /// Her dikte artırır: yapıştırılan yazının izlenmesi yeni diktede biter.
    generation: AtomicU64,
    pending: Mutex<Vec<Learn>>,
    next_id: AtomicU64,
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
    rules: Vec<vocab::Rule>,
    only_unsure: bool,
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
        rules: vocab::rules(db),
        only_unsure: db.setting_json(vocab::ONLY_UNSURE_KEY).unwrap_or(true),
    }
}

/// Ders kaydı için: sesle yazmanın Whisper ayarları (aynı model, aynı sunucu) ve dilin noktalama örneği.
/// Kişisel sözlük ve sözcükler derse karışmasın diye verilmez.
pub(crate) fn lecture_setup(db: &Db) -> (WhisperCfg, &'static str) {
    let s = settings(db);
    let style = match s.whisper.lang.as_str() {
        "tr" => "Bugünkü derste konuyu adım adım anlatacağım, notlarınızı alın.",
        "en" => "In today's lecture I will explain the topic step by step, please take notes.",
        _ => "",
    };
    (s.whisper, style)
}

/// Whisper'a ipucu: kullanıcının özel sözcükleri ve noktalamalı örnek bir cümle (noktalama ve
/// büyük harf kullanmaya yönlendirir).
fn whisper_prompt(s: &Settings) -> String {
    let style = match s.whisper.lang.as_str() {
        "tr" => "Merhaba, bugün neler yaptık? Notlarımı yazıyorum.",
        "en" => "Hello, how are you? I'm writing my notes.",
        _ => "",
    };
    // Kullanıcının sözcükleri + sözlükteki doğru yazılışlar (Whisper bunları duyunca öyle yazmaya yatkın olur).
    let mut words: Vec<String> = s
        .words
        .split(',')
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty())
        .collect();
    for w in vocab::hint_words(&s.rules) {
        if !words.iter().any(|x| vocab::fold(x) == vocab::fold(&w)) {
            words.push(w);
        }
    }
    if words.is_empty() {
        style.to_string()
    } else {
        format!("{}. {style}", words.join(", "))
    }
}

impl Dictation {
    /// Bekleyen "Bunu böyle mi yazayım?" soruları.
    pub fn pending(&self) -> Vec<Learn> {
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Yapıştırılan yazı sonradan düzeltildiyse: yanlış duyulmuşa benzeyen parçalar kendiliğinden sözlüğe eklenir
    /// ("Geri al" kartıyla); kendiliğinden öğrenme kapalıysa sorulur.
    ///
    /// Düzeltme cümlenin bütünüyle değerlendirilir: yalnız Whisper'ın yanlış duyduğu sözcük öğrenilir, kişinin
    /// söylediğini değiştirmesi (fikir değiştirmek) öğrenilmez:
    /// - Cümle baştan yazıldıysa ya da ikiden çok yer değiştiyse hiçbiri öğrenilmez.
    /// - Whisper o sözcüğü net duyduysa (güven ≥ %85) değiştirmek fikir değişikliğidir.
    /// - Whisper bocaladıysa (< %50) hemen; arada kalan ya da güveni bilinmeyen düzeltme ikinci kez görülünce
    ///   öğrenilir.
    pub fn propose(&self, app: &AppHandle, pasted: &str, edited: &str, heard: &Heard) {
        let db = app.state::<Db>();
        let auto = db.setting_json::<bool>(AUTO_LEARN_KEY).unwrap_or(true);
        if vocab::rewritten(pasted, edited) {
            crate::log!("Sesle yazma: cümle yeniden yazılmış, düzeltme öğrenilmedi");
            return;
        }
        let found = vocab::fresh(&db, vocab::suggest(pasted, edited));
        let mut pairs: Vec<(String, String)> = Vec::new();
        for (f, t) in found {
            // Sorulacaksa da kendiliğinden eklenecekse de: yalnız tek sözcük, birleşik/ekli değil.
            if !vocab::acceptable(&f, &t) {
                crate::log!(
                    "Sesle yazma: {f} → {t} öğrenilmedi (tek sözcük düzeltmesine benzemiyor)"
                );
                continue;
            }
            let conf = heard.confidence(&f);
            let case_only = vocab::fold(&f) == vocab::fold(&t);
            if !case_only && conf.is_some_and(|c| c >= vocab::SURE) {
                crate::log!(
                    "Sesle yazma: {f} → {t} öğrenilmedi (Whisper net duymuştu: %{:.0}; fikir değişikliği sayıldı)",
                    conf.unwrap_or(0.0) * 100.0
                );
                continue;
            }
            if auto && !case_only && !conf.is_some_and(|c| c < vocab::UNSURE) {
                let n = vocab::sighting(&db, &f, &t);
                if n < vocab::SIGHTINGS {
                    crate::log!(
                        "Sesle yazma: {f} → {t} aday ({n}/{}); bir daha görülürse öğrenilecek",
                        vocab::SIGHTINGS
                    );
                    continue;
                }
            }
            vocab::forget_sighting(&db, &f, &t);
            pairs.push((f, t));
        }
        if pairs.is_empty() {
            return;
        }
        let mut added = Vec::new();
        let mut p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        for (from, to) in pairs {
            if p.iter()
                .any(|l| vocab::fold(&l.from) == vocab::fold(&from) && l.to == to)
            {
                continue;
            }
            let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
            let learned = auto && vocab::add(&db, &from, &to).is_ok();
            crate::log!(
                "Sesle yazma: düzeltme görüldü ({from} → {to}){}",
                if learned { ", sözlüğe eklendi" } else { "" }
            );
            if learned {
                added.push(id);
            }
            p.push(Learn {
                id,
                from,
                to,
                added: learned,
            });
        }
        let notify = db.setting_json::<bool>(LEARN_NOTIFY_KEY).unwrap_or(true);
        if !added.is_empty() {
            let _ = app.emit("dictation-rules", ());
            if notify {
                if let Some(l) = p.iter().rev().find(|l| l.added) {
                    crate::overlay::show_toast(
                        app,
                        "book",
                        "Sözlüğe eklendi",
                        format!("{} → {} · istersen Geri al", l.from, l.to),
                    );
                }
            } else {
                // Haber verilmeyecekse kart hiç gösterilmez (kural yine eklenir; Kontrol Odası'ndan silinebilir).
                p.retain(|l| !added.contains(&l.id));
            }
            // Geri alınmazsa kart kendiliğinden kapanır, kural kalır.
            let app = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(UNDO_SECS));
                let d = app.state::<Dictation>();
                let mut p = d.pending.lock().unwrap_or_else(|e| e.into_inner());
                let before = p.len();
                p.retain(|l| !added.contains(&l.id));
                if p.len() != before {
                    drop(p);
                    let _ = app.emit("alerts-changed", ());
                }
            });
        }
        let n = p.len();
        if n > 6 {
            p.drain(..n - 6);
        }
        drop(p);
        crate::overlay::raise_alerts_quiet(app);
    }

    /// Kart yanıtı: `accept` ise kural eklenir, değilse bir daha sorulmaz.
    pub fn answer(&self, app: &AppHandle, id: u64, accept: bool) -> Result<(), String> {
        let item = {
            let mut p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            let i = p.iter().position(|l| l.id == id).ok_or("Bulunamadı")?;
            p.remove(i)
        };
        let db = app.state::<Db>();
        match (item.added, accept) {
            // Kendiliğinden eklenmişti: "Tamam" bir şey değiştirmez, "Geri al" kuralı siler.
            (true, true) => {}
            (true, false) => {
                vocab::remove(&db, &item.from)?;
                vocab::reject(&db, &item.from, &item.to);
            }
            (false, true) => {
                vocab::add(&db, &item.from, &item.to)?;
            }
            (false, false) => vocab::reject(&db, &item.from, &item.to),
        }
        let _ = app.emit("alerts-changed", ());
        let _ = app.emit("dictation-rules", ());
        Ok(())
    }

    /// Kontrol Odası'ndaki "Son dikteler"den düzeltme: o diktenin yazısıyla karşılaştırır, önerileri döner.
    pub fn correct(&self, app: &AppHandle, at: i64, edited: &str) -> Vec<(String, String)> {
        let text = self
            .history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|e| e.at == at)
            .map(|e| e.text.clone());
        text.map(|t| vocab::fresh(&app.state::<Db>(), vocab::suggest(&t, edited)))
            .unwrap_or_default()
    }

    /// Önceki dikte hâlâ işleniyor mu? 4 dakikadan uzun süren iş takılmış sayılır (yenisi engellenmez).
    fn is_busy(&self) -> bool {
        if !self.busy.load(Ordering::SeqCst) {
            return false;
        }
        let age =
            (crate::util::now_ms() as u64).saturating_sub(self.busy_since.load(Ordering::SeqCst));
        if age > 240_000 {
            crate::log!(
                "Sesle yazma: önceki dikte {} sn'dir bitmedi, beklenmiyor",
                age / 1000
            );
            return false;
        }
        true
    }

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
    // Bir kez: eski sürümün yanlış öğrendiği kurallar ("bedava.QR", "telefon → telefonlar") temizlenir.
    {
        let db = app.state::<Db>();
        if db
            .setting("dictation.rulesCleaned")
            .ok()
            .flatten()
            .is_none()
        {
            let n = vocab::clean_bad_rules(&db);
            if n > 0 {
                crate::log!("Sesle yazma: {n} hatalı sözlük kuralı silindi");
            }
            let _ = db.set_setting("dictation.rulesCleaned", "true");
        }
    }
    let d = app.state::<Dictation>();
    let enabled = app
        .state::<Db>()
        .setting_json::<bool>(ENABLED_KEY)
        .unwrap_or(false);
    d.enabled.store(enabled, Ordering::SeqCst);
    if !enabled {
        // Sunucu o an yükleniyorsa kapatma bekler; çağıran (kısayol, düğme) takılmasın.
        let app2 = app.clone();
        std::thread::spawn(move || app2.state::<Dictation>().engine.shutdown());
        return;
    }
    // Gösterge penceresi önceden hazır olsun: ilk dikte gecikmeli görünmesin.
    crate::overlay::prepare_osd(app);
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
        // Açılışta modeller arka planda yüklensin: ilk dikte model yüklemesini ve ekran kartının ilk
        // hazırlığını beklemesin (açılış ağırlaşmasın diye biraz sonra; oyun öndeyse yapılmaz).
        let app4 = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(20));
            let d = app4.state::<Dictation>();
            let preload = app4
                .state::<Db>()
                .setting_json::<bool>(PRELOAD_KEY)
                .unwrap_or(true);
            if preload && d.enabled.load(Ordering::SeqCst) && !crate::panel::foreground_fullscreen()
            {
                warm_up(&app4);
            }
        });
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

/// Kısayol ya da üst çubuktaki düğme: sesle yazmayı açar/kapatır. Dönen: yeni durum.
pub fn toggle(app: &AppHandle) -> bool {
    let db = app.state::<Db>();
    let on = !db.setting_json::<bool>(ENABLED_KEY).unwrap_or(false);
    let _ = db.set_setting(ENABLED_KEY, if on { "true" } else { "false" });
    apply(app);
    if on {
        crate::overlay::show_toast(
            app,
            "mic",
            "Sesle yazma açık",
            "Ctrl + Alt basılı tut, konuş, bırak",
        );
    } else {
        crate::overlay::show_toast(
            app,
            "micoff",
            "Sesle yazma kapalı",
            "Yeniden açmak için Ctrl + Alt + Y",
        );
    }
    let _ = app.emit("dictation-toggled", on);
    on
}

/// "Oyundayken kapalı" ayarı açıksa ve bir oyun çalışıyorsa dikte başlamaz (oyun kapanınca kendiliğinden döner).
fn paused_for_game(app: &AppHandle) -> bool {
    app.state::<Db>()
        .setting_json::<bool>(OFF_IN_GAME_KEY)
        .unwrap_or(false)
        && !app.state::<crate::games::Games>().running_ids().is_empty()
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
    /// Bu kadar basılı tutulunca dikte başlar (Ctrl+Alt+N gibi kısayollarla karışmasın).
    const ARM: Duration = Duration::from_millis(220);
    /// Mikrofon biraz önce açılır: açılış gecikmesi gizlenir, ilk hece kaçmaz.
    const PRE_ROLL: Duration = Duration::from_millis(90);

    enum Mode {
        Idle,
        /// Tuşlar basılı, süre dolması bekleniyor (mikrofon erken açılmış olabilir).
        Armed(Instant, Option<win::Recorder>),
        /// Kayıt, başlangıç, gösterilen saniye, başladığında öndeki GMID penceresi (varsa).
        Recording(win::Recorder, Instant, u64, Option<String>),
        /// Başka bir tuşa basıldı: Ctrl ve Alt bırakılana kadar bekle.
        Blocked,
    }

    let d = app.state::<Dictation>();
    let mut held: HashSet<u32> = HashSet::new();
    let mut mode = Mode::Idle;
    let mut last_level = Instant::now();
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
                if combo && !others && d.enabled.load(Ordering::SeqCst) && !paused_for_game(&app) {
                    // Gösterge şimdiden (görünmez) açılır: pencere ve içindeki sayfa uyanmış olur, süre
                    // dolunca yalnız görünür hale gelir; bekleyen pencerenin uyanması gecikme yapmaz.
                    crate::overlay::dictation_view(&app, "armed", 0);
                    Mode::Armed(Instant::now(), None)
                } else {
                    Mode::Idle
                }
            }
            Mode::Armed(t, rec) => {
                if others || !combo {
                    // Kısayol ya da yanlışlıkla basma: erken açılan kayıt atılır.
                    if let Some(r) = rec {
                        std::thread::spawn(move || drop(r.finish()));
                    }
                    crate::overlay::dictation_view(&app, "hidden", 0);
                    if others {
                        Mode::Blocked
                    } else {
                        Mode::Idle
                    }
                } else if t.elapsed() >= ARM {
                    if d.is_busy() {
                        if let Some(r) = rec {
                            std::thread::spawn(move || drop(r.finish()));
                        }
                        crate::overlay::dictation_view(&app, "hidden", 0);
                        crate::overlay::show_osd(&app, "mic", "Önceki yazı hazırlanıyor…");
                        Mode::Blocked
                    } else {
                        win::mask_alt();
                        let rec = rec.unwrap_or_else(win::Recorder::start);
                        if sounds_on(&app) {
                            win::play_start();
                        }
                        d.set_phase(&app, "listening", None);
                        crate::overlay::dictation_view(&app, "listening", 0);
                        // Konuşma sürerken modeller yüklensin. Biraz sonra: model yüklemesi (ekran kartı ve
                        // işlemci yükü) göstergenin ilk çizimiyle aynı ana denk gelip onu geciktirmesin.
                        let app2 = app.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_millis(350));
                            warm_up(&app2);
                        });
                        // Hangi GMID penceresinde konuşulduğu şimdi alınır: gösterge açılınca odak kayabiliyor.
                        Mode::Recording(rec, Instant::now(), 0, our_window_quick(&app))
                    }
                } else if rec.is_none() && t.elapsed() >= PRE_ROLL {
                    Mode::Armed(t, Some(win::Recorder::start()))
                } else {
                    Mode::Armed(t, rec)
                }
            }
            Mode::Recording(rec, start, shown, target) => {
                if others {
                    let _ = rec.finish();
                    d.set_phase(&app, "idle", None);
                    crate::overlay::dictation_view(&app, "hidden", 0);
                    Mode::Blocked
                } else if !combo {
                    if sounds_on(&app) {
                        win::play_stop();
                    }
                    crate::overlay::dictation_view(&app, "processing", 0);
                    let app2 = app.clone();
                    std::thread::spawn(move || process(&app2, rec, target));
                    Mode::Idle
                } else {
                    // Gösterge her saniye tazelenir (kaybolmasın, süre görünsün).
                    let secs = start.elapsed().as_secs();
                    if secs > shown {
                        crate::overlay::dictation_view(&app, "listening", secs as u32);
                    }
                    // Dalga için ses seviyesi (saniyede ~16 kez).
                    if last_level.elapsed() >= Duration::from_millis(60) {
                        last_level = Instant::now();
                        crate::overlay::dictation_level(&app, rec.level());
                    }
                    Mode::Recording(rec, start, secs.max(shown), target)
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

/// Öndeki GMID penceresi; pencere sorgusu ana iş parçacığını bekler, o meşgulse klavye dinleyicisi
/// takılmasın diye en çok 150 ms beklenir (bulunamazsa yazı ana pencereye gider).
#[cfg(windows)]
fn our_window_quick(app: &AppHandle) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let app = app.clone();
    std::thread::spawn(move || {
        let _ = tx.send(win::our_window(&app));
    });
    rx.recv_timeout(std::time::Duration::from_millis(150))
        .ok()
        .flatten()
}

#[cfg(windows)]
fn sounds_on(app: &AppHandle) -> bool {
    app.state::<Db>()
        .setting_json("dictation.sounds")
        .unwrap_or(true)
}

/// Kaydı bitirir, yazıya döker, (isteğe bağlı) toparlar ve yapıştırır.
#[cfg(windows)]
fn process(app: &AppHandle, rec: win::Recorder, target: Option<String>) {
    let d = app.state::<Dictation>();
    d.busy.store(true, Ordering::SeqCst);
    d.busy_since
        .store(crate::util::now_ms() as u64, Ordering::SeqCst);
    d.generation.fetch_add(1, Ordering::SeqCst);
    let released = std::time::Instant::now();
    // Son hece kesilmesin.
    std::thread::sleep(std::time::Duration::from_millis(200));
    let result = rec.finish().and_then(|samples| {
        crate::log!(
            "Sesle yazma: kayıt bitti ({:.1} sn ses, {} ms)",
            samples.len() as f64 / 16_000.0,
            released.elapsed().as_millis()
        );
        run(app, &samples, released, target)
    });
    crate::log!(
        "Sesle yazma: bitti ({} ms, {})",
        released.elapsed().as_millis(),
        match &result {
            Ok(Some(_)) => "yazıldı",
            Ok(None) => "yazılacak bir şey yok",
            Err(_) => "hata",
        }
    );
    match result {
        Ok(Some(entry)) => {
            crate::overlay::dictation_view(app, "hidden", 0);
            let _ = app.emit("dictation-entry", &entry);
            let mut h = d.history.lock().unwrap_or_else(|e| e.into_inner());
            h.push_front(entry);
            h.truncate(HISTORY);
            drop(h);
            d.set_phase(app, "idle", None);
        }
        Ok(None) => {
            crate::overlay::dictation_view(app, "hidden", 0);
            d.set_phase(app, "idle", None);
        }
        Err(e) => {
            crate::log!("Sesle yazma: {e}");
            crate::overlay::dictation_view(app, "error", 0);
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
    target: Option<String>,
) -> Result<Option<Entry>, String> {
    let d = app.state::<Dictation>();
    let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
    if samples.len() < 16_000 * 2 / 5 {
        return Ok(None);
    }
    if rms(samples) < 60.0 {
        return Err("Ses duyulmadı; mikrofon kapalı ya da sessiz olabilir.".into());
    }
    // Konuşma yoksa (tuşa basıp bir şey demeden bırakmak) Whisper'a hiç gönderilmez: sessizliği
    // "Thank you", "Altyazı M.K." gibi uydurmalarla dolduruyor. Varsa baştaki/sondaki sessizlik kesilir;
    // sondaki sessizlik de "…, evet." gibi eklemelere yol açıyordu.
    let Some((from, to)) = speech_span(samples) else {
        crate::log!("Sesle yazma: konuşma duyulmadı, yazılmadı");
        return Ok(None);
    };
    let samples = &samples[from..to];
    let s = settings(&db);
    d.set_phase(app, "transcribing", None);
    // Model henüz yükleniyorsa gösterge bunu söylesin (ilk dikte birkaç saniye sürebilir).
    let loading = !d.engine.loaded();
    if loading {
        crate::overlay::dictation_view(app, "loading", 0);
    }
    let heard = d
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
    crate::log!(
        "Sesle yazma: yazıya döküldü ({} ms{})",
        released.elapsed().as_millis(),
        if loading { ", model yüklendi" } else { "" }
    );
    if loading {
        crate::overlay::dictation_view(app, "processing", 0);
    }
    let raw = drop_ghosts(
        &crate::transcript::parse::clean_dictation(&heard.text),
        &heard,
        &s.whisper.lang,
    );
    if raw.is_empty() {
        return Ok(None);
    }
    // Sözlük: toparlamadan önce (yapay zekâ doğru sözcüğü görsün) ve sonra (geri bozduysa). Whisper net
    // duyduğu sözcükte duyduğunu yazar; sözlüğe yalnız bocaladığında danışılır.
    let conf = |w: &str| heard.confidence(w);
    let (fixed, mut used, skipped) = vocab::apply_heard(&raw, &s.rules, conf, s.only_unsure);
    for i in &skipped {
        if let Some(r) = s.rules.get(*i) {
            crate::log!(
                "Sesle yazma: \"{}\" net duyuldu, sözlük ({} → {}) uygulanmadı",
                r.from,
                r.from,
                r.to
            );
        }
    }
    let mut text = fixed.clone();
    let mut tidied = false;
    if s.tidy && engine::llm_ready(dir.root()) {
        d.set_phase(app, "tidying", None);
        match d.engine.tidy(dir.root(), &fixed, &s.whisper.lang) {
            Ok(t) => {
                tidied = t != fixed;
                let (t, more, _) = vocab::apply_heard(&t, &s.rules, conf, s.only_unsure);
                used.extend(more);
                text = t;
            }
            Err(e) => crate::log!("Toparlanamadı (ham yazı yapıştırılıyor): {e}"),
        }
    }
    used.sort_unstable();
    used.dedup();
    vocab::count_hits(&db, &used);
    if win::foreground_is_ours() {
        // GMID'in kendi penceresi (Notlar, Dene kutusu…): sayfa yazıyı imlecin olduğu yere kendisi koyar.
        // Konuşulan pencere öne alınır (gösterge odağı kaydırmış olabilir), yazı yalnız ona gider; sayfa
        // odak kaçmışsa en son tıklanan yazı alanına döner.
        let label = target.unwrap_or_else(|| crate::window::MAIN.to_string());
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.set_focus();
        }
        crate::log!("Sesle yazma: GMID penceresine ({label}) yazılıyor");
        let _ = app.emit(
            "dictation-insert",
            serde_json::json!({ "text": &text, "window": label }),
        );
    } else {
        win::paste(app, &text)?;
    }
    // Yapıştırılan yerde düzeltilirse öğren (yazı alanı okunabiliyorsa; şifre alanlarına bakılmaz).
    let gen = d.generation.load(Ordering::SeqCst);
    let (app2, pasted) = (app.clone(), text.clone());
    let app3 = app.clone();
    let heard2 = heard.clone();
    win::watch_field(
        text.clone(),
        move || app2.state::<Dictation>().generation.load(Ordering::SeqCst) == gen,
        move |edited| {
            app3.state::<Dictation>()
                .propose(&app3, &pasted, &edited, &heard2)
        },
    );
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

/// Kaydın konuşma içeren bölümü (örnek aralığı), 30 ms'lik pencerelerle. Pencere, ortam gürültüsünün
/// belirgin üstündeyse "ses" sayılır; toplam ~0,2 sn'den azsa (tık, nefes, sessizlik) konuşma yok.
/// Bulunan aralığın iki yanına 0,3 sn pay bırakılır (sözcüğün başı/sonu kesilmesin).
fn speech_span(samples: &[i16]) -> Option<(usize, usize)> {
    const FRAME: usize = 480;
    const PAD: usize = 10;
    let frames: Vec<f64> = samples.chunks(FRAME).map(rms).collect();
    let mut sorted = frames.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let (floor, peak) = (*sorted.get(sorted.len() / 5)?, *sorted.last()?);
    // Gürültü tabanının 3 katı (sürekli konuşmada taban da yüksek olur: o zaman tepenin dörtte biri),
    // ama en az 120 (sessiz odada mikrofon cızırtısı ses sayılmasın).
    let thr = (floor * 3.0).min(peak * 0.25).max(120.0);
    let loud: Vec<usize> = (0..frames.len()).filter(|&i| frames[i] > thr).collect();
    if loud.len() < 7 {
        return None;
    }
    let a = loud[0].saturating_sub(PAD) * FRAME;
    let b = ((loud[loud.len() - 1] + 1 + PAD) * FRAME).min(samples.len());
    Some((a, b))
}

/// Whisper'ın sessizliğe uydurduğu kalıplar: tek başına İngilizce "Thank you." (Türkçe dikte ederken)
/// ve sona eklenen, Whisper'ın kendisinin de emin olmadığı "Evet." / "Teşekkürler." gibi kapanışlar.
fn drop_ghosts(text: &str, heard: &Heard, lang: &str) -> String {
    const ENGLISH: &[&str] = &[
        "thank you",
        "thanks",
        "you",
        "bye",
        "thank you very much",
        "thank you so much",
    ];
    const TAILS: &[&str] = &[
        "evet",
        "tamam",
        "teşekkürler",
        "teşekkür ederim",
        "thank you",
        "thanks",
        "you",
        "bye",
    ];
    let key = |s: &str| {
        s.split_whitespace()
            .map(|w| vocab::fold(w.trim_matches(|c: char| !c.is_alphanumeric())))
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let whole = key(text);
    if lang != "en" && ENGLISH.contains(&whole.as_str()) {
        return String::new();
    }
    // Son cümle: son noktalamadan (. ! ? ,) sonrası; metnin sonundaki noktalama sayılmaz.
    let body = text.trim_end_matches(|c: char| ".!?,… ".contains(c));
    let Some(cut) = body.rfind(['.', '!', '?', ',']) else {
        return text.to_string();
    };
    let tail = key(&body[cut + 1..]);
    let unsure = heard.confidence(&tail).is_some_and(|p| p < 0.6);
    if TAILS.contains(&tail.as_str()) && unsure {
        let kept = body[..cut].trim_end();
        // Virgülle bitiyorsa cümleyi noktayla kapat.
        return format!(
            "{kept}{}",
            if body[cut..].starts_with(',') {
                "."
            } else {
                &body[cut..cut + 1]
            }
        );
    }
    text.to_string()
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

    fn tone(n: usize, amp: f64) -> Vec<i16> {
        (0..n)
            .map(|i| (amp * (i as f64 * 0.07).sin()) as i16)
            .collect()
    }

    #[test]
    fn speech_is_found_and_silence_trimmed() {
        // Sessiz oda (hafif cızırtı): konuşma yok.
        let hiss: Vec<i16> = (0..32_000).map(|i| ((i * 7919 % 61) as i16) - 30).collect();
        assert_eq!(speech_span(&hiss), None);
        // Tek bir tık: konuşma yok.
        let mut click = hiss.clone();
        click[10_000..10_200].copy_from_slice(&tone(200, 8000.0));
        assert_eq!(speech_span(&click), None);
        // 1 sn sessizlik + 1 sn konuşma + 2 sn sessizlik: konuşma ±0,3 sn payla.
        let mut v = vec![0i16; 16_000];
        v.extend(tone(16_000, 3000.0));
        v.extend(vec![0i16; 32_000]);
        let (a, b) = speech_span(&v).unwrap();
        assert!((16_000 - 5_000..=16_000).contains(&a), "{a}");
        assert!((32_000..=32_000 + 5_400).contains(&b), "{b}");
        // Baştan sona konuşma (taban da yüksek): yine bulunur, hepsi kalır.
        let all = tone(48_000, 2500.0);
        assert_eq!(speech_span(&all), Some((0, 48_000)));
        // Kısık mikrofon: konuşma ~300.
        let mut quiet = vec![0i16; 8_000];
        quiet.extend(tone(16_000, 420.0));
        assert!(speech_span(&quiet).is_some());
    }

    #[test]
    fn whisper_ghosts_are_dropped() {
        let heard = |words: &[(&str, f32)]| engine::Heard {
            text: String::new(),
            words: words.iter().map(|(w, p)| (w.to_string(), *p)).collect(),
        };
        let h = heard(&[("örnek", 0.9), ("versene", 0.9), ("evet", 0.3)]);
        assert_eq!(
            drop_ghosts("Bir örnek versene. Evet.", &h, "tr"),
            "Bir örnek versene."
        );
        assert_eq!(
            drop_ghosts("Bir örnek versene, evet.", &h, "tr"),
            "Bir örnek versene."
        );
        // Gerçekten söylenen (net duyulan) "evet" kalır.
        let sure = heard(&[("geliyor", 0.9), ("musun", 0.9), ("evet", 0.95)]);
        assert_eq!(
            drop_ghosts("Geliyor musun? Evet.", &sure, "tr"),
            "Geliyor musun? Evet."
        );
        // Tek başına "Evet." (cevap) kalır.
        assert_eq!(
            drop_ghosts("Evet.", &heard(&[("evet", 0.3)]), "tr"),
            "Evet."
        );
        // Türkçe dikte ederken tek başına "Thank you." atılır; İngilizcede kalır.
        assert_eq!(drop_ghosts("Thank you.", &heard(&[]), "tr"), "");
        assert_eq!(drop_ghosts("Thank you.", &heard(&[]), "en"), "Thank you.");
        // Güven bilgisi yoksa sondaki kalıba dokunulmaz.
        assert_eq!(
            drop_ghosts("Tamam bakarım. Evet.", &heard(&[]), "tr"),
            "Tamam bakarım. Evet."
        );
    }

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
