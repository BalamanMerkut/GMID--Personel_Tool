//! Demucs'un (HTDemucs) spektrogram adımları: PyTorch'taki `_spec` / `_ispec` ile aynı hesap.
//! STFT: 4096 noktalı Hann penceresi, 1024 adım, `normalized=True`, `center=True` (yansıtmalı dolgu).

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

pub const NFFT: usize = 4096;
pub const HOP: usize = 1024;
/// Modelin kullandığı frekans sayısı (Nyquist atılır).
pub const BINS: usize = NFFT / 2;

pub struct Plans {
    fwd: Arc<dyn Fft<f32>>,
    inv: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
}

impl Plans {
    pub fn new() -> Self {
        let mut p = FftPlanner::new();
        // torch.hann_window (periyodik)
        let window = (0..NFFT)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / NFFT as f32).cos()))
            .collect();
        Plans {
            fwd: p.plan_fft_forward(NFFT),
            inv: p.plan_fft_inverse(NFFT),
            window,
        }
    }
}

/// PyTorch `F.pad(mode="reflect")`: kenar örneği tekrarlanmadan aynalanır.
pub fn reflect(x: &[f32], left: usize, right: usize) -> Vec<f32> {
    let n = x.len();
    debug_assert!(left < n && right < n);
    let mut out = Vec::with_capacity(n + left + right);
    out.extend((0..left).map(|i| x[left - i]));
    out.extend_from_slice(x);
    out.extend((0..right).map(|k| x[n - 2 - k]));
    out
}

/// Tek kanalın spektrogramı: `[BINS][le]` (frekans ana, zaman iç), `le = ceil(len / HOP)`.
pub fn spec(x: &[f32], plans: &Plans) -> (Vec<Complex32>, usize) {
    let le = x.len().div_ceil(HOP);
    let pad = HOP / 2 * 3;
    let padded = reflect(x, pad, pad + le * HOP - x.len());
    let centered = reflect(&padded, NFFT / 2, NFFT / 2);
    let norm = 1.0 / (NFFT as f32).sqrt();
    let mut out = vec![Complex32::new(0.0, 0.0); BINS * le];
    let mut buf = vec![Complex32::new(0.0, 0.0); NFFT];
    // `_spec` ilk iki çerçeveyi atar: kullanılan çerçeve t, STFT'nin t+2. çerçevesi.
    for t in 0..le {
        let start = (t + 2) * HOP;
        for (i, b) in buf.iter_mut().enumerate() {
            *b = Complex32::new(centered[start + i] * plans.window[i] * norm, 0.0);
        }
        plans.fwd.process(&mut buf);
        for f in 0..BINS {
            out[f * le + t] = buf[f];
        }
    }
    (out, le)
}

/// `spec`in tersi: `[BINS][frames]` karmaşık spektrogramdan `length` örnek.
pub fn ispec(z: &[Complex32], frames: usize, length: usize, plans: &Plans) -> Vec<f32> {
    let pad = HOP / 2 * 3;
    // Başa ve sona ikişer boş çerçeve; Nyquist sıfır.
    let total_frames = frames + 4;
    let le = HOP * length.div_ceil(HOP) + 2 * pad;
    let full = NFFT + HOP * (total_frames - 1);
    let mut y = vec![0f32; full];
    let mut env = vec![0f32; full];
    let scale = (NFFT as f32).sqrt() / NFFT as f32;
    let mut buf = vec![Complex32::new(0.0, 0.0); NFFT];
    for f in 0..total_frames {
        let start = f * HOP;
        for (i, w) in plans.window.iter().enumerate() {
            env[start + i] += w * w;
        }
        if f < 2 || f >= frames + 2 {
            continue;
        }
        let t = f - 2;
        buf[0] = z[t];
        for k in 1..BINS {
            let v = z[k * frames + t];
            buf[k] = v;
            buf[NFFT - k] = v.conj();
        }
        buf[BINS] = Complex32::new(0.0, 0.0);
        plans.inv.process(&mut buf);
        for (i, w) in plans.window.iter().enumerate() {
            y[start + i] += buf[i].re * scale * w;
        }
    }
    // center=True: baştan NFFT/2 atılır; `_ispec` ayrıca `pad` kadar kırpar.
    let from = NFFT / 2 + pad;
    debug_assert!(from + length <= full && le <= full);
    (from..from + length)
        .map(|i| if env[i] > 1e-11 { y[i] / env[i] } else { 0.0 })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflect_matches_torch() {
        // torch: F.pad([1,2,3,4,5], (2,3), "reflect") = [3,2,1,2,3,4,5,4,3,2]
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(
            reflect(&x, 2, 3),
            vec![3.0, 2.0, 1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0]
        );
    }

    #[test]
    fn spec_then_ispec_is_identity() {
        let plans = Plans::new();
        let n: usize = 44100;
        let x: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / 44100.0;
                (t * 440.0 * std::f32::consts::TAU).sin() * 0.5
                    + (t * 97.0 * std::f32::consts::TAU).cos() * 0.2
                    + ((i * 7919 % 101) as f32 / 101.0 - 0.5) * 0.1
            })
            .collect();
        let (z, frames) = spec(&x, &plans);
        assert_eq!(frames, n.div_ceil(HOP));
        let back = ispec(&z, frames, n, &plans);
        // `_ispec` baştaki/sondaki iki çerçeveyi boş bırakır (Demucs'ta da öyle; parça kenarları zaten
        // üçgen ağırlıkla sönümlenir): kenardaki 4096 örnek dışında birebir aynı olmalı.
        let err = x[NFFT..n - NFFT]
            .iter()
            .zip(&back[NFFT..n - NFFT])
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        assert!(err < 1e-3, "en büyük fark {err}");
    }
}
