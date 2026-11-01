//! Sesle yazmanın arka planda çalışan iki programı:
//! - **whisper-server**: modeli bir kez yükler, her dikteyi saniyeler içinde yazıya döker.
//! - **llama-server** (isteğe bağlı): küçük bir dil modeliyle (Gemma 3 4B) metni toparlar.
//!
//! İkisi de yalnızca bu bilgisayarda (127.0.0.1) çalışır; ekran kartı belleğini boşa tutmasın diye
//! bir süre kullanılmayınca kapatılır, sonraki dikte onları yeniden başlatır.

use crate::downloader::tools;
use crate::transcript::whisper;
use crate::util::{hide_console, kill_tree};
use serde::Serialize;
use std::io::Write as _;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const LLAMA_RELEASES: &str = "https://api.github.com/repos/ggml-org/llama.cpp/releases?per_page=5";
pub const LLM_FILE: &str = "google_gemma-3-4b-it-Q4_K_M.gguf";
const LLM_URL: &str =
    "https://huggingface.co/bartowski/google_gemma-3-4b-it-GGUF/resolve/main/google_gemma-3-4b-it-Q4_K_M.gguf";
/// Kullanılmayan sunucular bu süreden sonra kapanır (oyunda ekran kartı belleği boş kalsın).
pub const IDLE_STOP: Duration = Duration::from_secs(15 * 60);

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

pub fn llama_dir(root: &Path) -> PathBuf {
    tools::dir(root).join("llama")
}

fn llama_exe(root: &Path) -> PathBuf {
    llama_dir(root).join(exe("llama-server"))
}

fn llm_path(root: &Path) -> PathBuf {
    llama_dir(root).join("models").join(LLM_FILE)
}

pub fn llm_ready(root: &Path) -> bool {
    llama_exe(root).is_file() && llm_path(root).is_file()
}

struct Server {
    child: Child,
    port: u16,
    /// Hangi ayarlarla başlatıldı (değişince yeniden başlar).
    key: String,
    gpu: bool,
}

impl Server {
    fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
    fn stop(mut self) {
        kill_tree(self.child.id());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Default)]
pub struct Engine {
    whisper: Mutex<Option<Server>>,
    llama: Mutex<Option<Server>>,
    last_used: Mutex<Option<Instant>>,
    /// NVIDIA sürümü bu oturumda çalışmadıysa nedeni (işlemciye geçildi).
    pub gpu_failed: Mutex<Option<String>>,
    /// Ne zaman başarısız oldu: bir süre sonra ekran kartı yeniden denenir (oyun ekran kartı belleğini
    /// doldurmuştu, sürücü güncellendi…); oturum boyunca işlemcide kalınmaz.
    gpu_failed_at: Mutex<Option<Instant>>,
}

/// Whisper sunucusunun ayarları.
pub struct WhisperCfg {
    pub model: String,
    pub lang: String,
    pub gpu: bool,
}

fn free_port() -> Result<u16, String> {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .map_err(|e| format!("Boş bağlantı noktası bulunamadı: {e}"))
}

fn log_file(root: &Path, name: &str) -> Stdio {
    std::fs::File::create(tools::dir(root).join(name))
        .map(Stdio::from)
        .unwrap_or_else(|_| Stdio::null())
}

/// Sunucu hazır olana kadar bekler (model yükleniyor). Program kapanırsa hemen hata döner.
fn wait_ready(child: &mut Child, url: &str, timeout: Duration) -> Result<(), String> {
    let agent = crate::net::agent(Duration::from_secs(2), Duration::from_secs(5));
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("program kapandı ({status})"));
        }
        if let Ok(r) = agent.get(url).call() {
            if r.status() == 200 {
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(SLOW.into())
}

/// `wait_ready`: program açık ama süresinde hazır olmadı (ilk açılışta virüs taraması, sürücü derlemesi…).
const SLOW: &str = "zamanında hazır olmadı";

/// İlk istek: ekran kartı ilk kez kullanılırken CUDA çekirdekleri hazırlanır (yeni kartlarda sürücü
/// bunları derler, birkaç dakika sürebilir). Bunu boş bir sesle önceden yaptırırız; kişinin ilk diktesi
/// beklemesin.
fn warm_whisper(port: u16) {
    let started = Instant::now();
    let silence = vec![0i16; 16_000];
    match post_inference(
        port,
        &super::to_wav(&silence),
        "",
        Duration::from_secs(300),
        false,
    ) {
        Ok(_) => crate::log!(
            "Sesle yazma: Whisper ısındı ({:.1} sn)",
            started.elapsed().as_secs_f64()
        ),
        Err(e) => crate::log!("Sesle yazma: Whisper ısınamadı: {e}"),
    }
}

impl Engine {
    pub fn touch(&self) {
        *self.last_used.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
    }

    /// Uzun süre kullanılmadıysa sunucuları kapatır (arka plan zamanlayıcısı çağırır).
    pub fn stop_if_idle(&self) {
        let idle = self
            .last_used
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some_and(|t| t.elapsed() > IDLE_STOP);
        if idle {
            self.shutdown();
        }
    }

    pub fn shutdown(&self) {
        if let Some(s) = self
            .whisper
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            s.stop();
        }
        if let Some(s) = self.llama.lock().unwrap_or_else(|e| e.into_inner()).take() {
            s.stop();
        }
    }

    /// Whisper sunucusunu (gerekirse) başlatır ve adresini döner. NVIDIA sürümü açılamazsa
    /// işlemci sürümüne geçer.
    fn whisper_port(&self, root: &Path, cfg: &WhisperCfg) -> Result<u16, String> {
        let mut guard = self.whisper.lock().unwrap_or_else(|e| e.into_inner());
        let model = whisper::model_path(root, whisper::find_model(&cfg.model));
        if !model.is_file() {
            return Err("WHISPER_MISSING".into());
        }
        let gpu_ok = {
            let mut failed = self.gpu_failed.lock().unwrap_or_else(|e| e.into_inner());
            let at = *self.gpu_failed_at.lock().unwrap_or_else(|e| e.into_inner());
            if failed.is_some() && at.is_some_and(|t| t.elapsed() > GPU_RETRY) {
                crate::log!("Sesle yazma: ekran kartı yeniden deneniyor");
                *failed = None;
            }
            failed.is_none()
        };
        let use_gpu = cfg.gpu && gpu_ok && whisper::cuda_ready(root);
        let key = format!("{}|{}|{use_gpu}", model.display(), cfg.lang);
        if let Some(s) = guard.as_mut() {
            if s.key == key && s.alive() {
                return Ok(s.port);
            }
        }
        if let Some(s) = guard.take() {
            s.stop();
        }
        let mut tries = vec![use_gpu];
        if use_gpu {
            tries.push(false);
        }
        let mut last = String::new();
        for gpu in tries {
            let program = whisper::server_exe(root, gpu);
            if !program.is_file() {
                last = "Whisper sunucusu kurulu değil".into();
                continue;
            }
            let port = free_port()?;
            let threads = std::thread::available_parallelism()
                .map(|n| n.get().clamp(1, 8))
                .unwrap_or(4)
                .to_string();
            let mut cmd = Command::new(&program);
            cmd.arg("-m")
                .arg(&model)
                .args(["-l", &cfg.lang, "-t", &threads, "--host", "127.0.0.1"])
                .args(["--port", &port.to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(log_file(root, "whisper-sunucu.log"));
            if let Some(d) = program.parent() {
                cmd.current_dir(d);
            }
            hide_console(&mut cmd);
            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    last = format!("başlatılamadı: {e}");
                    continue;
                }
            };
            let started = Instant::now();
            // Ekran kartı sürümünün ilk açılışı uzun sürebilir (büyük CUDA dosyaları taranır, yüklenir).
            let limit = Duration::from_secs(if gpu { 240 } else { 90 });
            match wait_ready(&mut child, &format!("http://127.0.0.1:{port}/"), limit) {
                Ok(()) => {
                    crate::log!(
                        "Sesle yazma: Whisper sunucusu hazır ({}, {:.1} sn)",
                        if gpu { "NVIDIA" } else { "işlemci" },
                        started.elapsed().as_secs_f64()
                    );
                    if gpu {
                        warm_whisper(port);
                    }
                    *guard = Some(Server {
                        child,
                        port,
                        key: if gpu {
                            key.clone()
                        } else {
                            format!("{}|{}|false", model.display(), cfg.lang)
                        },
                        gpu,
                    });
                    return Ok(port);
                }
                Err(e) => {
                    kill_tree(child.id());
                    let _ = child.kill();
                    let _ = child.wait();
                    crate::log!("Sesle yazma: Whisper sunucusu açılamadı (gpu={gpu}): {e}");
                    // Yalnızca program çöktüyse bu oturumda işlemciye geçilir; yavaş açıldıysa
                    // sonraki dikte ekran kartını yeniden dener.
                    if gpu && e != SLOW {
                        *self.gpu_failed.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(e.clone());
                        *self.gpu_failed_at.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(Instant::now());
                    }
                    last = e;
                }
            }
        }
        Err(format!("Whisper sunucusu açılamadı: {last}"))
    }

    /// Whisper sunucusu açık ve hazır mı? (Yükleniyorsa beklemeden `false`.)
    pub fn loaded(&self) -> bool {
        match self.whisper.try_lock() {
            Ok(mut g) => g.as_mut().is_some_and(|s| s.alive()),
            Err(_) => false,
        }
    }

    /// Ekran kartını hemen yeniden denesin ("Hazırla" düğmesi, ekran kartı anahtarı).
    pub fn retry_gpu(&self) {
        *self.gpu_failed.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Şu an NVIDIA ile mi çalışıyor? (`None`: sunucu kapalı)
    /// Sunucu o an başlatılıyorsa beklemez (`None`): durum sorgusu yükleme bitene kadar takılmasın.
    pub fn running_on_gpu(&self) -> Option<bool> {
        let mut guard = self.whisper.try_lock().ok()?;
        let s = guard.as_mut()?;
        s.alive().then_some(s.gpu)
    }

    /// Modeli önceden yükler (tuşa basılınca konuşma sürerken).
    pub fn warm_up(&self, root: &Path, cfg: &WhisperCfg, cleanup: bool) {
        self.touch();
        if let Err(e) = self.whisper_port(root, cfg) {
            crate::log!("Sesle yazma ısınma: {e}");
        }
        if cleanup && llm_ready(root) {
            if let Err(e) = self.llama_port(root) {
                crate::log!("Toparlayıcı ısınma: {e}");
            }
        }
    }

    /// 16 kHz tek kanal WAV → yazı ve her sözcükte Whisper'ın ne kadar emin olduğu (sözlüğe ne zaman
    /// danışılacağı için).
    pub fn transcribe(
        &self,
        root: &Path,
        cfg: &WhisperCfg,
        wav: &[u8],
        prompt: &str,
    ) -> Result<Heard, String> {
        self.transcribe_with(root, cfg, wav, prompt, false)
    }

    /// `careful`: ışın araması (beam search, 5 aday) ile: daha yavaş ama özellikle uzun ve gürültülü konuşmada
    /// yakın ama yanlış sözcükleri azaltır (ders kaydı; beklemesi önemli olmayan işler).
    pub fn transcribe_with(
        &self,
        root: &Path,
        cfg: &WhisperCfg,
        wav: &[u8],
        prompt: &str,
        careful: bool,
    ) -> Result<Heard, String> {
        self.touch();
        let mut last = String::new();
        // Sunucu kapanmışsa (çökme, uyku) bir kez yeniden başlatıp dener. Yanıt vermiyorsa (takıldıysa)
        // beklemeye devam edilmez: sunucu kapatılır, sonraki dikte temiz bir sunucuyla başlar.
        for _ in 0..2 {
            let port = self.whisper_port(root, cfg)?;
            let gpu = self.running_on_gpu() == Some(true);
            let limit = inference_limit(wav.len(), careful, gpu);
            let started = Instant::now();
            match post_inference(port, wav, prompt, limit, careful) {
                Ok(t) => return Ok(t),
                Err(e) => {
                    let hung = started.elapsed() + Duration::from_secs(1) >= limit;
                    crate::log!(
                        "Sesle yazma: Whisper isteği {:.1} sn sonra başarısız{}: {e}",
                        started.elapsed().as_secs_f64(),
                        if hung { " (yanıt vermedi)" } else { "" }
                    );
                    last = e;
                    if let Some(s) = self
                        .whisper
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .take()
                    {
                        s.stop();
                    }
                    if hung {
                        return Err(
                            "Whisper yanıt vermedi; yeniden başlatılıyor, birazdan tekrar dene."
                                .into(),
                        );
                    }
                }
            }
        }
        Err(format!("Yazıya dökülemedi: {last}"))
    }

    fn llama_port(&self, root: &Path) -> Result<u16, String> {
        let mut guard = self.llama.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(s) = guard.as_mut() {
            if s.alive() {
                return Ok(s.port);
            }
        }
        if let Some(s) = guard.take() {
            s.stop();
        }
        if !llm_ready(root) {
            return Err("LLM_MISSING".into());
        }
        let port = free_port()?;
        let program = llama_exe(root);
        let mut cmd = Command::new(&program);
        cmd.arg("-m")
            .arg(llm_path(root))
            // Bütün katmanlar ekran kartında; kısa metinler için küçük bağlam yeter.
            .args(["-ngl", "99", "-c", "4096", "--host", "127.0.0.1"])
            .args(["--port", &port.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log_file(root, "toparlayici.log"));
        cmd.current_dir(llama_dir(root));
        hide_console(&mut cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Toparlayıcı başlatılamadı: {e}"))?;
        if let Err(e) = wait_ready(
            &mut child,
            &format!("http://127.0.0.1:{port}/health"),
            Duration::from_secs(120),
        ) {
            kill_tree(child.id());
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Toparlayıcı açılamadı: {e}"));
        }
        crate::log!("Sesle yazma: toparlayıcı hazır");
        // İlk istek ekran kartını hazırlar (Whisper'daki gibi); kısa bir yanıt yeter.
        let agent = crate::net::agent(Duration::from_secs(5), Duration::from_secs(180));
        let _ = agent
            .post(&format!("http://127.0.0.1:{port}/v1/chat/completions"))
            .header("Content-Type", "application/json")
            .send(r#"{"messages":[{"role":"user","content":"Merhaba"}],"max_tokens":1}"#);
        *guard = Some(Server {
            child,
            port,
            key: String::new(),
            gpu: true,
        });
        Ok(port)
    }

    /// Metni dil modeliyle toparlar. Sonuç şüpheliyse (boş, çok kısa/uzun) özgün metin kalır.
    pub fn tidy(&self, root: &Path, text: &str, lang: &str) -> Result<String, String> {
        self.touch();
        let port = self.llama_port(root)?;
        let agent = crate::net::agent(Duration::from_secs(5), Duration::from_secs(60));
        let body = serde_json::json!({
            "messages": [
                { "role": "system", "content": system_prompt(lang) },
                { "role": "user", "content": format!("<metin>\n{text}\n</metin>") }
            ],
            "temperature": 0.2,
            "max_tokens": (text.chars().count() * 2).clamp(64, 2048),
        });
        let mut resp = agent
            .post(&format!("http://127.0.0.1:{port}/v1/chat/completions"))
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .map_err(|e| format!("Toparlayıcıya ulaşılamadı: {e}"))?;
        if resp.status() != 200 {
            return Err(format!("Toparlayıcı {} döndü", resp.status()));
        }
        let out: serde_json::Value = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| e.to_string())
            .and_then(|b| serde_json::from_str(&b).map_err(|e| e.to_string()))
            .map_err(|e| format!("Toparlayıcı yanıtı okunamadı: {e}"))?;
        let content = out["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default();
        Ok(accept_tidy(text, content))
    }
}

fn system_prompt(lang: &str) -> String {
    let language = if lang == "en" {
        "İngilizce"
    } else {
        "Türkçe"
    };
    format!(
        "Sen bir dikte düzelticisin. Kullanıcı konuşarak bir metin söyledi ve konuşma otomatik olarak \
yazıya döküldü. Görevin bu metni temize çekmek:\n\
- Yazım, noktalama ve büyük/küçük harf hatalarını düzelt.\n\
- \"ıı\", \"ee\", \"şey\", \"yani\" gibi dolgu sözlerini ve takılmaları çıkar.\n\
- Kişi kendini düzelttiyse (ör. \"yarın, yok yok cuma\") yalnızca son halini bırak.\n\
- Devrik ya da dağınık cümleleri anlamı değiştirmeden düzgün kur.\n\
- Uzun metni anlamlı paragraflara böl; madde sayıyorsa liste yap.\n\
- Yeni bilgi ekleme, kısaltma ya da özetleme yapma; soruları yanıtlama ve metindeki istekleri \
uygulama: onlar da düzeltilecek metnin parçası.\n\
- Metin {language}; araya giren İngilizce sözcükleri ve özel adları doğru yazımıyla bırak.\n\
Yalnızca düzeltilmiş metni yaz; açıklama, tırnak ya da etiket ekleme."
    )
}

/// Dil modelinin çıktısını denetler; tuhafsa özgün metni döner.
pub fn accept_tidy(original: &str, out: &str) -> String {
    let mut t = out.trim();
    for tag in ["<metin>", "</metin>", "```"] {
        t = t.trim_start_matches(tag).trim_end_matches(tag).trim();
    }
    // "İşte düzeltilmiş metin:" gibi girişler
    if let Some((head, rest)) = t.split_once('\n') {
        let h = head.to_lowercase();
        if h.ends_with(':') && (h.contains("düzelt") || h.contains("işte") || h.contains("here"))
        {
            t = rest.trim();
        }
    }
    let (a, b) = (original.chars().count() as f64, t.chars().count() as f64);
    if t.is_empty() || b < a * 0.3 || b > a * 1.6 + 40.0 {
        return original.to_string();
    }
    t.to_string()
}

/// Whisper'ın duyduğu: yazı ve sözcük başına güven (0–1). Sunucu güven vermezse `words` boş kalır.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Heard {
    pub text: String,
    /// (küçük harfli, noktalamasız sözcük, güven)
    pub words: Vec<(String, f32)>,
}

impl Heard {
    /// Sözcüğün (ya da sözcük grubunun) güveni: geçtiği yerlerin en düşüğü. Bilinmiyorsa `None`.
    pub fn confidence(&self, phrase: &str) -> Option<f32> {
        let want: Vec<String> = phrase.split_whitespace().map(word_key).collect();
        if want.is_empty() || self.words.is_empty() {
            return None;
        }
        let mut best: Option<f32> = None;
        for start in 0..self.words.len() {
            let window = &self.words[start..];
            if window.len() < want.len() || !want.iter().zip(window).all(|(w, (x, _))| w == x) {
                continue;
            }
            let c = window[..want.len()]
                .iter()
                .map(|(_, p)| *p)
                .fold(f32::INFINITY, f32::min);
            best = Some(best.map_or(c, |b: f32| b.min(c)));
        }
        best
    }
}

/// Karşılaştırma anahtarı: küçük harf, baştaki/sondaki noktalama atılmış.
fn word_key(s: &str) -> String {
    super::vocab::fold(s.trim_matches(|c: char| !c.is_alphanumeric()))
}

/// whisper-server'ın `verbose_json` yanıtından sözcük güvenleri. Parçalar (token) birleştirilir: boşlukla
/// başlayan parça yeni sözcüktür; sözcüğün güveni parçalarının en düşüğü. İki biçim okunur:
/// `segments[].words[] {word, probability}` ve `segments[].tokens[] {text, p}`.
pub fn heard_words(v: &serde_json::Value) -> Vec<(String, f32)> {
    let mut pieces: Vec<(String, f32)> = Vec::new();
    for seg in v["segments"].as_array().into_iter().flatten() {
        let list = seg["words"]
            .as_array()
            .filter(|a| a.iter().any(|w| w.is_object()))
            .or_else(|| {
                seg["tokens"]
                    .as_array()
                    .filter(|a| a.iter().any(|t| t.is_object()))
            });
        for t in list.into_iter().flatten() {
            let text = t["word"]
                .as_str()
                .or(t["text"].as_str())
                .unwrap_or_default();
            let p = t["probability"].as_f64().or(t["p"].as_f64());
            let (Some(p), false) = (
                p,
                text.trim_start().starts_with("[_") || text.contains("<|"),
            ) else {
                continue;
            };
            pieces.push((text.to_string(), p as f32));
        }
    }
    let mut words: Vec<(String, f32)> = Vec::new();
    for (text, p) in pieces {
        let starts_word = text.starts_with(char::is_whitespace) || words.is_empty();
        for (k, part) in text.split_whitespace().enumerate() {
            let w = word_key(part);
            if starts_word || k > 0 {
                words.push((w, p));
            } else if let Some(last) = words.last_mut() {
                last.0.push_str(&w);
                last.1 = last.1.min(p);
            }
        }
    }
    words.retain(|(w, _)| !w.is_empty());
    words
}

/// `verbose_json` yanıtının yazısı. Sunucu bu kipte parçaları ~60 harfte, gerekirse sözcüğün ortasından
/// böler ve `text` alanında aralarına satır sonu koyar ("versene" → "vers\nene" → "vers ene"). Parçaların
/// kendi yazıları uç uca eklenir: yeni sözcük zaten boşlukla başlar, sözcüğün devamı boşluksuz gelir.
pub fn verbose_text(v: &serde_json::Value) -> String {
    let joined: Option<String> = v["segments"]
        .as_array()
        .filter(|a| !a.is_empty())
        .and_then(|a| a.iter().map(|s| s["text"].as_str()).collect());
    joined
        .unwrap_or_else(|| v["text"].as_str().unwrap_or_default().to_string())
        .trim()
        .to_string()
}

/// İşlemciye geçildikten bu kadar sonra ekran kartı yeniden denenir.
const GPU_RETRY: Duration = Duration::from_secs(10 * 60);

/// Bir isteğin en çok ne kadar sürebileceği; bundan uzunu sunucunun takıldığını gösterir. Ekran kartında
/// kısa dikte 1-2 sn sürer: 30 sn + sesin 3 katı. İşlemcide (özellikle "En doğru" modelle) sesin birkaç
/// katı sürebilir: 90 sn + sesin 12 katı. Dikkatli okuma (ders) daha uzun.
fn inference_limit(wav_len: usize, careful: bool, gpu: bool) -> Duration {
    let secs = wav_len.saturating_sub(44) as u64 / 32_000;
    let limit = match (careful, gpu) {
        (true, true) => 180,
        (true, false) => 600,
        (false, true) => (30 + secs * 3).min(180),
        (false, false) => (90 + secs * 12).min(600),
    };
    Duration::from_secs(limit)
}

/// whisper-server'a WAV gönderir (multipart/form-data). Önce sözcük güvenleriyle (`verbose_json`); sunucu
/// bunu desteklemiyorsa düz yazıyla.
fn post_inference(
    port: u16,
    wav: &[u8],
    prompt: &str,
    timeout: Duration,
    careful: bool,
) -> Result<Heard, String> {
    match post_inference_as(port, wav, prompt, timeout, "verbose_json", careful) {
        Ok(v) => Ok(Heard {
            text: verbose_text(&v),
            words: heard_words(&v),
        }),
        Err(e) if e.starts_with("sunucu ") => {
            post_inference_as(port, wav, prompt, timeout, "json", careful).map(|v| Heard {
                text: v["text"].as_str().unwrap_or_default().to_string(),
                words: Vec::new(),
            })
        }
        Err(e) => Err(e),
    }
}

fn post_inference_as(
    port: u16,
    wav: &[u8],
    prompt: &str,
    timeout: Duration,
    format: &str,
    careful: bool,
) -> Result<serde_json::Value, String> {
    let boundary = format!("gmid{}", crate::util::new_id());
    let mut body: Vec<u8> = Vec::with_capacity(wav.len() + 1024);
    let mut field = |name: &str, value: &str| {
        let _ = write!(
            body,
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
        );
    };
    field("temperature", "0.0");
    // Sıcaklık artırarak yeniden deneme (fallback) yalnız dikkatli okumada: kısa diktede ve sessizlikte
    // Whisper aynı sesi 6 kez baştan çözüp büyük modelde dakikalarca takılıyordu.
    field("temperature_inc", if careful { "0.2" } else { "0.0" });
    if careful {
        field("beam_size", "5");
        field("best_of", "5");
    }
    field("response_format", format);
    if !prompt.trim().is_empty() {
        field("prompt", prompt.trim());
    }
    let _ = write!(
        body,
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"dikte.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
    );
    body.extend_from_slice(wav);
    let _ = write!(body, "\r\n--{boundary}--\r\n");

    let agent = crate::net::agent(Duration::from_secs(5), timeout);
    let mut resp = agent
        .post(&format!("http://127.0.0.1:{port}/inference"))
        .header(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .send(&body[..])
        .map_err(|e| e.to_string())?;
    if resp.status() != 200 {
        return Err(format!("sunucu {} döndü", resp.status()));
    }
    let v: serde_json::Value = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_str(&b).map_err(|e| e.to_string()))?;
    if let Some(e) = v["error"].as_str() {
        return Err(e.to_string());
    }
    Ok(v)
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

/// llama.cpp'nin NVIDIA sürümü ("llama-bXXXX-bin-win-cuda-12.4-x64.zip") ve aynı CUDA sürümünün
/// kütüphaneleri ("cudart-llama-bin-win-cuda-12.4-x64.zip").
fn llama_cuda_version(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("llama-")?.split_once("-bin-win-cuda-")?.1;
    let v = rest.strip_suffix("-x64.zip")?;
    v.starts_with("12.").then_some(v)
}

/// Toparlayıcıyı kurar: llama.cpp (NVIDIA) + Gemma 3 4B modeli (~2,5 GB).
pub fn install_llm(app: &AppHandle, root: &Path) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Toparlayıcı yalnızca Windows'ta kurulabiliyor.".into());
    }
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
    let dir = llama_dir(root);
    std::fs::create_dir_all(dir.join("models")).map_err(|e| e.to_string())?;
    let fail = |tool: &str, name: &str, e: String| {
        emit(tool, name, "error", 0, 0, Some(e.clone()));
        e
    };

    if !llama_exe(root).is_file() {
        let (url, _) = whisper::release_asset(LLAMA_RELEASES, |n| llama_cuda_version(n).is_some())
            .map_err(|e| fail("llama", "Toparlayıcı programı", e))?;
        let version = url
            .rsplit('/')
            .next()
            .and_then(llama_cuda_version)
            .unwrap_or("12.4")
            .to_string();
        let runtime_name = format!("cudart-llama-bin-win-cuda-{version}-x64.zip");
        let (runtime, _) = whisper::release_asset(LLAMA_RELEASES, |n| n == runtime_name)
            .map_err(|e| fail("llama", "Toparlayıcı programı", e))?;
        for (i, u) in [url, runtime].iter().enumerate() {
            let tmp = dir.join(format!("paket{i}.indiriliyor"));
            let r = tools::download(u, &tmp, |d, t| {
                emit("llama", "Toparlayıcı programı", "download", d, t, None)
            })
            .and_then(|_| {
                emit("llama", "Toparlayıcı programı", "extract", 0, 0, None);
                tools::extract(&tmp, &dir, &["llama-server.exe", "*.dll"])
            });
            let _ = std::fs::remove_file(&tmp);
            r.map_err(|e| fail("llama", "Toparlayıcı programı", e))?;
        }
        emit("llama", "Toparlayıcı programı", "done", 0, 0, None);
    }

    let model = llm_path(root);
    if !model.is_file() {
        let tmp = model.with_extension("indiriliyor");
        let r = tools::download(LLM_URL, &tmp, |d, t| {
            emit("llm", "Dil modeli (Gemma 3 4B)", "download", d, t, None)
        })
        .and_then(|_| tools::replace(&tmp, &model));
        if let Err(e) = r {
            let _ = std::fs::remove_file(&tmp);
            return Err(fail("llm", "Dil modeli (Gemma 3 4B)", e));
        }
        emit("llm", "Dil modeli (Gemma 3 4B)", "done", 0, 0, None);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sahte whisper-server ve llama-server (Python) ile sunucu başlatma, multipart gönderimi ve
    /// toparlama yanıtının okunması. Elle çalıştırılır: `cargo test fake_servers -- --ignored`
    #[test]
    #[ignore]
    #[cfg(unix)]
    fn fake_servers() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("gmid-dikte-{}", std::process::id()));
        let wdir = whisper::dir(&root);
        std::fs::create_dir_all(wdir.join("models")).unwrap();
        std::fs::create_dir_all(llama_dir(&root).join("models")).unwrap();
        std::fs::write(
            wdir.join("models").join("ggml-large-v3-turbo-q5_0.bin"),
            "x",
        )
        .unwrap();
        std::fs::write(llm_path(&root), "x").unwrap();
        let py = root.join("fake.py");
        std::fs::write(
            &py,
            r#"
import sys, json, email
from http.server import BaseHTTPRequestHandler, HTTPServer
kind = sys.argv[1]; port = int(sys.argv[sys.argv.index('--port') + 1])
class H(BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def reply(self, obj):
        b = json.dumps(obj).encode(); self.send_response(200)
        self.send_header('Content-Type', 'application/json'); self.send_header('Content-Length', str(len(b)))
        self.end_headers(); self.wfile.write(b)
    def do_GET(self): self.reply({'status': 'ok'})
    def do_POST(self):
        body = self.rfile.read(int(self.headers['Content-Length']))
        if kind == 'whisper':
            msg = email.message_from_bytes(b'Content-Type: ' + self.headers['Content-Type'].encode() + b'\r\n\r\n' + body)
            parts = {p.get_param('name', header='content-disposition'): p.get_payload(decode=True) for p in msg.get_payload()}
            wav = parts['file']
            self.reply({'text': ' merhaba ıı dünya [BLANK_AUDIO] prompt=%s wav=%d riff=%s fmt=%s lang=%s' % (
                parts.get('prompt', b'').decode(), len(wav), wav[:4] == b'RIFF', parts['response_format'].decode(),
                sys.argv[sys.argv.index('-l') + 1])})
        else:
            req = json.loads(body)
            text = req['messages'][1]['content'].replace('<metin>', '').replace('</metin>', '').strip()
            self.reply({'choices': [{'message': {'content': 'İşte düzeltilmiş metin:\n' + text.replace(' ıı', '')}}]})
HTTPServer(('127.0.0.1', port), H).serve_forever()
"#,
        )
        .unwrap();
        for (path, kind) in [
            (whisper::server_exe(&root, false), "whisper"),
            (llama_exe(&root), "llama"),
        ] {
            std::fs::write(
                &path,
                format!("#!/bin/sh\nexec python3 {} {kind} \"$@\"\n", py.display()),
            )
            .unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let engine = Engine::default();
        let cfg = WhisperCfg {
            model: "turbo".into(),
            lang: "tr".into(),
            gpu: true, // NVIDIA kurulu değil: işlemci sürümü kullanılmalı
        };
        let wav = crate::dictation::to_wav(&vec![100i16; 16_000]);
        let raw = engine
            .transcribe(&root, &cfg, &wav, "GMID, Merkut")
            .unwrap()
            .text;
        assert!(raw.contains("prompt=GMID, Merkut"), "{raw}");
        assert!(raw.contains(&format!("wav={}", wav.len())), "{raw}");
        assert!(raw.contains("riff=True fmt=verbose_json lang=tr"), "{raw}");
        assert_eq!(engine.running_on_gpu(), Some(false));
        // İkinci istek aynı sunucuyu kullanır.
        let port = engine.whisper.lock().unwrap().as_ref().unwrap().port;
        engine.transcribe(&root, &cfg, &wav, "").unwrap();
        assert_eq!(engine.whisper.lock().unwrap().as_ref().unwrap().port, port);
        let cleaned = crate::transcript::parse::clean_dictation(&raw);
        assert!(!cleaned.contains("BLANK_AUDIO"));
        let tidy = engine.tidy(&root, &cleaned, "tr").unwrap();
        assert!(tidy.starts_with("merhaba dünya prompt="), "{tidy}");
        engine.shutdown();
        assert_eq!(engine.running_on_gpu(), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn word_confidence_is_read() {
        // whisper.cpp: parçalar (token) boşlukla başlayınca yeni sözcük; özel parçalar atlanır.
        let v = serde_json::json!({
            "text": " Merhaba GMID ekibi.",
            "segments": [{ "tokens": [
                { "text": "[_BEG_]", "p": 0.99 },
                { "text": " Mer", "p": 0.98 }, { "text": "haba", "p": 0.95 },
                { "text": " GM", "p": 0.41 }, { "text": "ID", "p": 0.90 },
                { "text": " ekibi.", "p": 0.97 },
                { "text": "<|endoftext|>", "p": 0.99 }
            ]}]
        });
        let w = heard_words(&v);
        assert_eq!(
            w.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
            ["merhaba", "gmid", "ekibi"]
        );
        assert!((w[1].1 - 0.41).abs() < 1e-6);
        let h = Heard {
            text: v["text"].as_str().unwrap().into(),
            words: w,
        };
        assert!(h.confidence("GMID").unwrap() < 0.5);
        assert!(h.confidence("gmid ekibi").unwrap() < 0.5);
        assert!(h.confidence("Merhaba").unwrap() > 0.9);
        assert_eq!(h.confidence("yok"), None);
        // Diğer biçim: words[] {word, probability}
        let v2 = serde_json::json!({ "segments": [{ "words": [{ "word": " VDS", "probability": 0.3 }] }] });
        assert_eq!(heard_words(&v2), vec![("vds".to_string(), 0.3)]);
        // Güven yoksa boş.
        assert!(heard_words(&serde_json::json!({ "text": "x" })).is_empty());
    }

    #[test]
    fn inference_limit_scales_with_audio() {
        let s = Duration::from_secs;
        assert_eq!(inference_limit(44 + 32_000 * 5, false, true), s(45));
        assert_eq!(inference_limit(44 + 32_000 * 600, false, true), s(180));
        assert_eq!(inference_limit(44 + 32_000 * 10, false, false), s(210));
        assert_eq!(inference_limit(100, true, true), s(180));
        assert_eq!(inference_limit(100, true, false), s(600));
    }

    #[test]
    fn verbose_text_does_not_split_words() {
        // Sunucu parçayı sözcük ortasından bölüp `text`e satır sonu koyuyor: "vers\nene".
        let v = serde_json::json!({
            "text": " Bunlar çok daha iyiydi ama şöyle bir 5 tane daha örnek vers\nene ona göre seçeyim.\n",
            "segments": [
                { "text": " Bunlar çok daha iyiydi ama şöyle bir 5 tane daha örnek vers" },
                { "text": "ene ona göre seçeyim." }
            ]
        });
        assert_eq!(
            verbose_text(&v),
            "Bunlar çok daha iyiydi ama şöyle bir 5 tane daha örnek versene ona göre seçeyim."
        );
        // Parça yoksa düz yazı.
        assert_eq!(
            verbose_text(&serde_json::json!({ "text": " Merhaba.\n" })),
            "Merhaba."
        );
        // Sözcük güvenleri de bölünmüş parçayı tek sözcük sayar.
        let t = serde_json::json!({ "segments": [
            { "tokens": [{ "text": " örnek", "p": 0.9 }, { "text": " vers", "p": 0.8 }] },
            { "tokens": [{ "text": "ene", "p": 0.7 }, { "text": " ona", "p": 0.9 }] }
        ]});
        assert_eq!(
            heard_words(&t)
                .iter()
                .map(|x| x.0.as_str())
                .collect::<Vec<_>>(),
            ["örnek", "versene", "ona"]
        );
    }

    #[test]
    fn llama_assets() {
        assert_eq!(
            llama_cuda_version("llama-b11557-bin-win-cuda-12.4-x64.zip"),
            Some("12.4")
        );
        assert_eq!(
            llama_cuda_version("llama-b11557-bin-win-cuda-13.4-x64.zip"),
            None
        );
        assert_eq!(llama_cuda_version("llama-b11557-bin-win-cpu-x64.zip"), None);
        assert_eq!(
            llama_cuda_version("cudart-llama-bin-win-cuda-12.4-x64.zip"),
            None
        );
        assert!(whisper::is_cuda_asset(
            "whisper-bin-win-cuda-12.4.0-x64.zip"
        ));
        assert!(whisper::is_cuda_asset("whisper-cublas-12.4.0-bin-x64.zip"));
        assert!(!whisper::is_cuda_asset("whisper-cublas-11.8.0-bin-x64.zip"));
        assert!(!whisper::is_cuda_asset("whisper-bin-x64.zip"));
    }

    #[test]
    fn tidy_output_is_checked() {
        let raw = "yarın şey toplantı var yok yok cuma günü toplantı var";
        assert_eq!(
            accept_tidy(raw, "İşte düzeltilmiş metin:\nCuma günü toplantı var."),
            "Cuma günü toplantı var."
        );
        assert_eq!(accept_tidy(raw, ""), raw);
        assert_eq!(accept_tidy(raw, "Tamam."), raw, "çok kısalırsa özgün kalır");
        assert_eq!(
            accept_tidy("Merhaba nasılsın", "<metin>Merhaba, nasılsın?</metin>"),
            "Merhaba, nasılsın?"
        );
    }
}
