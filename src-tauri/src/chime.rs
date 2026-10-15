//! Kısa bildirim sesleri (bellekte üretilir, dosya yok). Mikrofon açılınca yükselen, kapanınca alçalan iki
//! yumuşak nota: hangisi olduğu kulaktan anlaşılır; Discord'unkinden kısa ve kısık.

use std::sync::OnceLock;

/// Mikrofon sesi ayarı (varsayılan açık).
pub const MIC_KEY: &str = "audio.micSound";

static MIC_ON: OnceLock<Vec<u8>> = OnceLock::new();
static MIC_OFF: OnceLock<Vec<u8>> = OnceLock::new();

/// İki nota, her biri `ms` milisaniye; `gain` 0..1.
pub fn tones(notes: &[f64], ms: f64, gain: f64) -> Vec<u8> {
    const RATE: f64 = 22_050.0;
    let per = (RATE * ms / 1000.0) as usize;
    let mut samples: Vec<i16> = Vec::with_capacity(per * notes.len());
    for f in notes {
        for i in 0..per {
            let t = i as f64 / RATE;
            // Yumuşak giriş, hızlı sönüm: tıkırtısız ve kısa.
            let env = (i as f64 / 60.0).min(1.0) * (1.0 - i as f64 / per as f64).powf(2.2);
            // Biraz üst harmonik: daha "tık" gibi, daha az "bip".
            let w = 2.0 * std::f64::consts::PI * f * t;
            let v = (w.sin() + 0.18 * (2.0 * w).sin()) * env * gain;
            samples.push((v.clamp(-1.0, 1.0) * i16::MAX as f64) as i16);
        }
    }
    wav(&samples, RATE as u32)
}

fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Mikrofon açıldı (`muted == false`: yükselen) ya da kapandı (alçalan, biraz daha pes).
pub fn mic(app: &tauri::AppHandle, muted: bool) {
    use tauri::Manager;
    let on = app
        .state::<crate::db::Db>()
        .setting_json::<bool>(MIC_KEY)
        .unwrap_or(true);
    if !on {
        return;
    }
    let buf = if muted {
        MIC_OFF.get_or_init(|| tones(&[620.0, 415.0], 55.0, 0.14))
    } else {
        MIC_ON.get_or_init(|| tones(&[523.0, 784.0], 55.0, 0.14))
    };
    play(buf);
}

#[cfg(windows)]
fn play(buf: &'static [u8]) {
    use windows::core::PCWSTR;
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
    unsafe {
        let _ = PlaySoundW(
            PCWSTR(buf.as_ptr() as *const u16),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

#[cfg(not(windows))]
fn play(_buf: &'static [u8]) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_and_quiet() {
        let w = tones(&[523.0, 784.0], 55.0, 0.14);
        assert_eq!(&w[..4], b"RIFF");
        // İki nota × 55 ms ≈ 110 ms.
        let samples = (w.len() - 44) / 2;
        assert!((2400..=2450).contains(&samples));
        let peak = w[44..]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]).unsigned_abs())
            .max()
            .unwrap();
        assert!(peak < (i16::MAX as f64 * 0.2) as u16);
    }
}
