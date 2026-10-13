//! Hatırlatıcı ve sayaç uyarısı: ekranın ortasında açılan pencere ve kısık, sakin bir müzik kutusu melodisi
//! (Ertele / Kapat). Oyundaysan pencere yerine Oyun İçi Panel açılır ve uyarı orada gösterilir: alt-tab yok.
//! Kapatılınca (ya da hepsi ertelenince) melodi susar; en geç 90 saniyede kendiliğinden susar.

use crate::db::Db;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const ALARM: &str = "alarm";
const SIZE: (f64, f64) = (500.0, 420.0);

#[derive(Default)]
pub struct Alarm {
    /// Çalan melodinin baytları (PlaySound bellekten çalarken yaşamalı).
    wav: Mutex<Option<Vec<u8>>>,
    /// Her başlatma artırır: eski "90 sn'de sustur" zamanlayıcısı yenisini susturmasın.
    generation: AtomicU64,
}

fn setting<T: serde::de::DeserializeOwned>(app: &AppHandle, key: &str, default: T) -> T {
    app.state::<Db>().setting_json(key).unwrap_or(default)
}

/// Bekleyen uyarı var mı (zamanı gelip "Tamam" denmemiş hatırlatıcı ya da bitmiş sayaç).
fn pending(app: &AppHandle) -> bool {
    !crate::reminders::pending_alerts(&app.state::<Db>())
        .unwrap_or_default()
        .is_empty()
        || !app.state::<crate::timers::Timers>().finished().is_empty()
        || app
            .state::<crate::power::Power>()
            .status()
            .is_some_and(|p| p.at - crate::util::now_ms() <= (crate::power::WARN_SECS + 5) * 1000)
}

/// Zamanlı kapatmaya bir dakika kala: geri sayım (İptal / Şimdi). Melodi çalmaz.
pub fn show_power(app: &AppHandle) {
    if crate::panel::foreground_fullscreen() {
        crate::panel::show(app);
        let _ = app.emit_to(crate::panel::PANEL, "panel-alarm", ());
    } else {
        show_window(app);
    }
    let _ = app.emit("alarm-changed", ());
}

/// Zamanı gelen uyarıyı gösterir. Açılır pencere kapalıysa eski sağ alt kartları.
pub fn raise(app: &AppHandle) {
    if !setting(app, "reminders.popup", true) {
        crate::overlay::raise_alerts(app);
        return;
    }
    if setting(app, "reminders.inGame", true) && crate::panel::foreground_fullscreen() {
        crate::panel::show(app);
        let _ = app.emit_to(crate::panel::PANEL, "panel-alarm", ());
    } else {
        show_window(app);
    }
    let _ = app.emit("alarm-changed", ());
    play(app);
}

/// Bir uyarı kapatıldı/ertelendi: hiçbiri kalmadıysa melodi susar, pencere kapanır.
pub fn changed(app: &AppHandle) {
    let _ = app.emit("alarm-changed", ());
    if !pending(app) {
        stop(app);
        if let Some(w) = app.get_webview_window(ALARM) {
            let _ = w.hide();
        }
    }
}

fn show_window(app: &AppHandle) {
    crate::window::on_ui_thread(app, "Uyarı penceresi", |app| {
        let w = match app.get_webview_window(ALARM) {
            Some(w) => w,
            None => WebviewWindowBuilder::new(app, ALARM, WebviewUrl::App("index.html".into()))
                .title("GMID Tool · Hatırlatıcı")
                .inner_size(SIZE.0, SIZE.1)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(false)
                .resizable(false)
                .visible(false)
                .build()?,
        };
        if let Ok(Some(m)) = w.primary_monitor() {
            let s = m.scale_factor();
            let area = m.work_area();
            let (wd, ht) = ((SIZE.0 * s) as i32, (SIZE.1 * s) as i32);
            let _ = w.set_position(tauri::PhysicalPosition::new(
                area.position.x + (area.size.width as i32 - wd) / 2,
                area.position.y + (area.size.height as i32 - ht) / 2,
            ));
        }
        let _ = w.emit_to(ALARM, "alarm-changed", ());
        w.show()?;
        let _ = w.unminimize();
        let _ = w.set_focus();
        Ok(())
    });
}

/// Melodiyi başlatır (ayar kapalıysa çalmaz).
pub fn play(app: &AppHandle) {
    if !setting(app, "reminders.music", true) {
        return;
    }
    let volume: f64 = setting::<f64>(app, "reminders.volume", 40.0).clamp(0.0, 100.0) / 100.0;
    if volume <= 0.0 {
        return;
    }
    let state = app.state::<Alarm>();
    let id = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let wav = music_box(volume);
    let mut slot = state.wav.lock().unwrap_or_else(|e| e.into_inner());
    *slot = Some(wav);
    #[cfg(windows)]
    if let Some(bytes) = slot.as_ref() {
        unsafe {
            use windows::core::PCWSTR;
            use windows::Win32::Media::Audio::{
                PlaySoundW, SND_ASYNC, SND_LOOP, SND_MEMORY, SND_NODEFAULT,
            };
            let _ = PlaySoundW(
                PCWSTR(bytes.as_ptr() as *const u16),
                None,
                SND_MEMORY | SND_ASYNC | SND_LOOP | SND_NODEFAULT,
            );
        }
    }
    drop(slot);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(90));
        if app.state::<Alarm>().generation.load(Ordering::SeqCst) == id {
            stop(&app);
        }
    });
}

pub fn stop(app: &AppHandle) {
    let state = app.state::<Alarm>();
    state.generation.fetch_add(1, Ordering::SeqCst);
    let mut slot = state.wav.lock().unwrap_or_else(|e| e.into_inner());
    if slot.is_some() {
        #[cfg(windows)]
        unsafe {
            use windows::core::PCWSTR;
            use windows::Win32::Media::Audio::{PlaySoundW, SND_FLAGS};
            let _ = PlaySoundW(PCWSTR::null(), None, SND_FLAGS(0));
        }
        *slot = None;
    }
}

const RATE: u32 = 22_050;

/// Kısık, yumuşak bir müzik kutusu: pentatonik sekiz notalık bir cümle ve uzun bir es (~6 sn, döngü).
/// Sinüs + hafif üst ses, yavaş sönüm; ani başlangıç yok, kulak tırmalamaz.
pub fn music_box(volume: f64) -> Vec<u8> {
    // Do majör pentatonik: C5 E5 G5 A5 C6 … (Hz)
    const NOTES: [f64; 8] = [523.25, 659.25, 783.99, 880.0, 1046.5, 880.0, 783.99, 659.25];
    let step = 0.42;
    let total = step * NOTES.len() as f64 + 2.6;
    let n = (RATE as f64 * total) as usize;
    let mut buf = vec![0f64; n];
    for (i, f) in NOTES.iter().enumerate() {
        let start = (i as f64 * step * RATE as f64) as usize;
        let len = (RATE as f64 * 2.2) as usize;
        for k in 0..len {
            let Some(slot) = buf.get_mut(start + k) else {
                break;
            };
            let t = k as f64 / RATE as f64;
            let attack = (t / 0.012).min(1.0);
            let env = attack * (-t * 2.4).exp();
            let s = (2.0 * std::f64::consts::PI * f * t).sin()
                + 0.18 * (2.0 * std::f64::consts::PI * f * 2.0 * t).sin()
                + 0.06 * (2.0 * std::f64::consts::PI * f * 3.0 * t).sin();
            *slot += s * env;
        }
    }
    // Döngü dikişinde tık olmasın: sonda yumuşak sönüm.
    let fade = (RATE as f64 * 0.3) as usize;
    for k in 0..fade.min(n) {
        buf[n - 1 - k] *= k as f64 / fade as f64;
    }
    let gain = 0.16 * volume * i16::MAX as f64;
    let samples: Vec<i16> = buf
        .iter()
        .map(|v| (v * gain).clamp(i16::MIN as f64, i16::MAX as f64) as i16)
        .collect();
    wav(&samples)
}

fn wav(samples: &[i16]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut w = Vec::with_capacity(44 + data as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&RATE.to_le_bytes());
    w.extend_from_slice(&(RATE * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn music_box_is_quiet_and_seamless() {
        let w = music_box(1.0);
        assert_eq!(&w[..4], b"RIFF");
        let samples: Vec<i16> = w[44..]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
        // En yüksek seste bile tam ölçeğin yarısının altında (kısık).
        assert!(peak < i16::MAX as u16 / 2, "{peak}");
        assert!(peak > 2000, "duyulabilir olmalı: {peak}");
        // Döngünün sonu sessiz (tık yok).
        assert!(samples.last().unwrap().unsigned_abs() < 50);
        let secs = samples.len() as f64 / RATE as f64;
        assert!((5.0..7.0).contains(&secs), "{secs}");
        // Ses düzeyi ayarı.
        let half = music_box(0.5);
        let peak_half = half[44..]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]).unsigned_abs())
            .max()
            .unwrap();
        assert!(peak_half < peak * 6 / 10);
    }
}
