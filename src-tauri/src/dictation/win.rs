//! Windows'a özel parçalar: klavye kancası, mikrofon kaydı (WASAPI) ve yazıyı yapıştırma.
#![allow(non_snake_case)]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::Media::Audio::{
    eCapture, eConsole, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
    AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, WAVEFORMATEX, WAVE_FORMAT_PCM,
};
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, HC_ACTION,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

/// Kancadan denetleyiciye giden tuş olayı.
pub struct KeyEvent {
    pub vk: u32,
    pub down: bool,
    /// AltGr'ye basınca Windows'un kendiliğinden gönderdiği sahte sol Ctrl.
    pub fake: bool,
}

static KEYS: OnceLock<Sender<KeyEvent>> = OnceLock::new();

/// Bütün klavye olaylarını dinleyen kancayı kurar (bir kez). Olaylar engellenmez, sadece izlenir.
pub fn install_hook(tx: Sender<KeyEvent>) -> Result<(), String> {
    if KEYS.set(tx).is_err() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("dikte-klavye".into())
        .spawn(|| unsafe {
            let module = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
            match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0) {
                Ok(_) => {
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
                Err(e) => crate::log!("Sesle yazma: klavye kancası kurulamadı: {e}"),
            }
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && lparam.0 != 0 {
        let k = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        // Programların gönderdiği tuşlar (bizim yapıştırma da dahil) sayılmaz.
        if k.flags.0 & LLKHF_INJECTED.0 == 0 {
            let msg = wparam.0 as u32;
            if let Some(tx) = KEYS.get() {
                let _ = tx.send(KeyEvent {
                    vk: k.vkCode,
                    down: msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN,
                    fake: k.scanCode & 0x200 != 0,
                });
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Tuş şu an basılı mı (kaçırılan tuş bırakmalarını düzeltmek için).
pub fn is_down(vk: u32) -> bool {
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Alt bırakılınca açık programın menü çubuğu açılmasın diye araya anlamsız bir tuş girer
/// (AutoHotkey'in "menü maskesi" yöntemi; 0xE8 hiçbir şeye atanmamış bir tuş).
pub fn mask_alt() {
    let vk = VIRTUAL_KEY(0xE8);
    send(&[key(vk, false), key(vk, true)]);
}

/// Yazıyı odaktaki programa yapıştırır: panoya koyar, Ctrl+V gönderir, sonra panoyu eski haline getirir.
pub fn paste(app: &tauri::AppHandle, text: &str) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    // Kullanıcı Ctrl/Alt'ı tamamen bırakmadan gönderirsek tuş Ctrl+Alt+V olur.
    for _ in 0..100 {
        let held = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
            .iter()
            .any(|k| is_down(k.0 as u32));
        if !held {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let clip = app.clipboard();
    let old = clip.read_text().ok();
    clip.write_text(text.to_string())
        .map_err(|e| format!("Pano kullanılamadı: {e}"))?;
    std::thread::sleep(Duration::from_millis(40));
    send(&[
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ]);
    // Yapıştırma bitince eski pano içeriği geri gelir (yazı ise).
    let (app, text) = (app.clone(), text.to_string());
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(700));
        let clip = app.clipboard();
        if let Some(old) = old {
            if clip.read_text().ok().as_deref() == Some(text.as_str()) {
                let _ = clip.write_text(old);
            }
        }
    });
    Ok(())
}

static START_SOUND: OnceLock<Vec<u8>> = OnceLock::new();
static STOP_SOUND: OnceLock<Vec<u8>> = OnceLock::new();

/// Kısa, yumuşak iki notalı "tık" sesi (WAV, bellekte).
fn chime(notes: &[f64]) -> Vec<u8> {
    const RATE: f64 = 16_000.0;
    let per = (RATE * 0.075) as usize;
    let mut samples: Vec<i16> = Vec::with_capacity(per * notes.len());
    for f in notes {
        for i in 0..per {
            let t = i as f64 / RATE;
            // Yumuşak giriş ve sönüm: tıkırtı olmasın.
            let env = (i as f64 / 80.0).min(1.0) * (1.0 - i as f64 / per as f64).powf(1.6);
            let v = (2.0 * std::f64::consts::PI * f * t).sin() * env * 0.22;
            samples.push((v * i16::MAX as f64) as i16);
        }
    }
    super::to_wav(&samples)
}

fn play(buf: &'static [u8]) {
    unsafe {
        let _ = PlaySoundW(
            PCWSTR(buf.as_ptr() as *const u16),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

/// Dinlemeye başlarken yükselen iki nota.
pub fn play_start() {
    play(START_SOUND.get_or_init(|| chime(&[660.0, 990.0])));
}

/// Bırakınca alçalan iki nota.
pub fn play_stop() {
    play(STOP_SOUND.get_or_init(|| chime(&[990.0, 660.0])));
}

/// Varsayılan mikrofondan 16 kHz, tek kanal, 16 bit kayıt (Windows dönüştürür).
pub struct Recorder {
    stop: Arc<AtomicBool>,
    /// Son ses parçasının seviyesi (0..1, f32 bitleri): göstergedeki dalga için.
    level: Arc<AtomicU32>,
    handle: JoinHandle<Result<Vec<i16>, String>>,
}

impl Recorder {
    pub fn start() -> Recorder {
        let stop = Arc::new(AtomicBool::new(false));
        let level = Arc::new(AtomicU32::new(0));
        let (flag, lvl) = (stop.clone(), level.clone());
        let handle = std::thread::spawn(move || record(&flag, &lvl));
        Recorder {
            stop,
            level,
            handle,
        }
    }

    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    pub fn finish(self) -> Result<Vec<i16>, String> {
        self.stop.store(true, Ordering::SeqCst);
        self.handle
            .join()
            .map_err(|_| "Kayıt beklenmedik şekilde durdu".to_string())?
    }
}

/// En fazla 5 dakika.
const MAX_SAMPLES: usize = 16_000 * 300;

fn record(stop: &AtomicBool, level: &AtomicU32) -> Result<Vec<i16>, String> {
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
        let dev = en
            .GetDefaultAudioEndpoint(eCapture, eConsole)
            .map_err(|e| err("Mikrofon bulunamadı", e))?;
        let client: IAudioClient = dev
            .Activate(CLSCTX_ALL, None)
            .map_err(|e| err("Mikrofon açılamadı", e))?;
        let fmt = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_PCM as u16,
            nChannels: 1,
            nSamplesPerSec: 16_000,
            nAvgBytesPerSec: 32_000,
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
        let capture: IAudioCaptureClient = client
            .GetService()
            .map_err(|e| err("Mikrofon okunamadı", e))?;
        client.Start().map_err(|e| err("Kayıt başlatılamadı", e))?;
        let mut samples: Vec<i16> = Vec::with_capacity(16_000 * 20);
        loop {
            let stopping = stop.load(Ordering::SeqCst);
            while capture.GetNextPacketSize().unwrap_or(0) > 0 {
                let mut data: *mut u8 = std::ptr::null_mut();
                let mut frames = 0u32;
                let mut flags = 0u32;
                if capture
                    .GetBuffer(&mut data, &mut frames, &mut flags, None, None)
                    .is_err()
                {
                    break;
                }
                if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null() {
                    samples.extend(std::iter::repeat(0i16).take(frames as usize));
                } else {
                    let chunk = std::slice::from_raw_parts(data as *const i16, frames as usize);
                    let sum: f64 = chunk.iter().map(|v| (*v as f64).powi(2)).sum();
                    let rms = (sum / chunk.len().max(1) as f64).sqrt();
                    level.store(
                        ((rms / 6000.0).min(1.0) as f32).to_bits(),
                        Ordering::Relaxed,
                    );
                    samples.extend_from_slice(chunk);
                }
                let _ = capture.ReleaseBuffer(frames);
            }
            if stopping || samples.len() >= MAX_SAMPLES {
                break;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        let _ = client.Stop();
        Ok(samples)
    }
}

/// Yazının yapıştırıldığı alanı (UI Automation ile) bir süre izler: kullanıcı yazıyı düzeltip gönderene ya da
/// silene kadar son hali `done`a verilir. Şifre alanları ve okunamayan alanlar (oyunlar, bazı programlar)
/// atlanır; yazı hiçbir yere kaydedilmez, yalnız karşılaştırılır. `alive` yeni dikte başlayınca `false` olur.
pub fn watch_field(
    pasted: String,
    alive: impl Fn() -> bool + Send + 'static,
    done: impl FnOnce(String) + Send + 'static,
) {
    use windows::Win32::System::Com::CLSCTX_INPROC_SERVER;
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        // Yapıştırma otursun.
        std::thread::sleep(Duration::from_millis(500));
        let Ok(uia) =
            CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
        else {
            return;
        };
        let Ok(el) = uia.GetFocusedElement() else {
            crate::log!("Sesle yazma: odaktaki yazı alanı bulunamadı; bu dikteden öğrenilmeyecek");
            return;
        };
        if el.CurrentIsPassword().is_ok_and(|p| p.as_bool()) {
            return;
        }
        let mut last: Option<String> = None;
        // Aynı kaldığı tur sayısı (1,5 sn'de bir bakılır).
        let mut still = 0;
        // En çok ~2 dakika. Düzeltme bitti sayılır: gönderilince/silinince, ya da yazı değişip 6 sn dokunulmayınca.
        for i in 0..80 {
            if !alive() {
                break;
            }
            let Some(text) = field_text(&el) else {
                if i == 0 {
                    crate::log!("Sesle yazma: bu programın yazı alanı okunamıyor; düzeltmeden öğrenme burada çalışmaz");
                }
                break;
            };
            // Gönderildi ya da silindi: son görülen hal karşılaştırılır.
            if super::vocab::overlap(&pasted, &text) < 0.5 {
                break;
            }
            still = if last.as_deref() == Some(text.as_str()) {
                still + 1
            } else {
                0
            };
            last = Some(text);
            if still >= 4
                && last
                    .as_deref()
                    .is_some_and(|t| !super::vocab::suggest(&pasted, t).is_empty())
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(1500));
        }
        if let Some(t) = last {
            done(t);
        }
    });
}

/// Bir yazı alanının içeriği: önce değer (tek satırlık kutular), yoksa belge metni (zengin yazı alanları:
/// Discord, tarayıcılar, Word). Çok uzunsa (belge) okunmaz.
unsafe fn field_text(
    el: &windows::Win32::UI::Accessibility::IUIAutomationElement,
) -> Option<String> {
    use windows::Win32::UI::Accessibility::{
        IUIAutomationTextPattern, IUIAutomationValuePattern, UIA_TextPatternId, UIA_ValuePatternId,
    };
    if let Ok(v) = el.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
        if let Ok(s) = v.CurrentValue() {
            let s = s.to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    let t = el
        .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
        .ok()?;
    let range = t.DocumentRange().ok()?;
    let s = range.GetText(20_000).ok()?.to_string();
    Some(s)
}
