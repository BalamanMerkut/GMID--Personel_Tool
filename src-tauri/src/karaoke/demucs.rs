//! HTDemucs ile sesi ayırma: şarkı 7,8 sn'lik, %25 üst üste binen parçalara bölünür; her parça modele verilir,
//! çıktılar üçgen ağırlıkla birleştirilir (Demucs'un `apply_model` / `split=True` yolu).

use super::dsp::{ispec, spec, Plans, BINS};
use ort::session::Session;
use ort::value::Tensor;
use rustfft::num_complex::Complex32;

pub const RATE: usize = 44100;
/// Modelin eğitildiği parça uzunluğu (7,8 sn).
pub const SEGMENT: usize = 343_980;
/// Modelin kaynak sırası: davul, bas, diğer, vokal.
pub const VOCALS: usize = 3;

/// Stereo ses, kanal başına ayrı dizi.
pub type Stereo = [Vec<f32>; 2];

fn triangle(len: usize) -> Vec<f32> {
    let half = len / 2;
    let mut w: Vec<f32> = (0..len)
        .map(|i| {
            if i <= half {
                (i + 1) as f32
            } else {
                (len - i) as f32
            }
        })
        .collect();
    let max = w.iter().cloned().fold(0f32, f32::max);
    w.iter_mut().for_each(|x| *x /= max);
    w
}

/// Bir parçayı model girdisine çevirir: (karışım `[1,2,SEGMENT]`, spektrogram `[1,4,BINS,frames]`, frames).
fn inputs(chunk: &Stereo, plans: &Plans) -> (Vec<f32>, Vec<f32>, usize) {
    let (z0, frames) = spec(&chunk[0], plans);
    let (z1, _) = spec(&chunk[1], plans);
    let plane = BINS * frames;
    let mut mag = vec![0f32; 4 * plane];
    for (c, z) in [z0, z1].iter().enumerate() {
        for (i, v) in z.iter().enumerate() {
            mag[2 * c * plane + i] = v.re;
            mag[(2 * c + 1) * plane + i] = v.im;
        }
    }
    let mut mix = Vec::with_capacity(2 * SEGMENT);
    mix.extend_from_slice(&chunk[0]);
    mix.extend_from_slice(&chunk[1]);
    (mix, mag, frames)
}

/// Modelin iki çıktısından (frekans ve zaman kolu) istenen kaynağın stereo sesi.
fn source(out_x: &[f32], out_xt: &[f32], s: usize, frames: usize, plans: &Plans) -> Stereo {
    let plane = BINS * frames;
    let ch = |c: usize| {
        // out_x: [1, 4 kaynak, 4 (kanal×gerçel/sanal), BINS, frames]
        let re = &out_x[(s * 4 + 2 * c) * plane..][..plane];
        let im = &out_x[(s * 4 + 2 * c + 1) * plane..][..plane];
        let z: Vec<Complex32> = re
            .iter()
            .zip(im)
            .map(|(&r, &i)| Complex32::new(r, i))
            .collect();
        let mut x = ispec(&z, frames, SEGMENT, plans);
        // out_xt: [1, 4 kaynak, 2 kanal, SEGMENT]
        let t = &out_xt[(s * 2 + c) * SEGMENT..][..SEGMENT];
        x.iter_mut().zip(t).for_each(|(a, b)| *a += b);
        x
    };
    [ch(0), ch(1)]
}

/// `wanted` kaynaklarını ayırır (ör. yalnız `[VOCALS]`). `progress(bitti, toplam)`; `false` dönerse durur.
pub fn separate(
    session: &mut Session,
    mix: &Stereo,
    wanted: &[usize],
    progress: impl Fn(usize, usize) -> bool,
) -> Result<Vec<Stereo>, String> {
    let n = mix[0].len();
    let plans = Plans::new();
    let weight = triangle(SEGMENT);
    let stride = (0.75 * SEGMENT as f64) as usize;
    let total = n.div_ceil(stride);
    let mut out: Vec<Stereo> = wanted
        .iter()
        .map(|_| [vec![0f32; n], vec![0f32; n]])
        .collect();
    let mut sum_w = vec![0f32; n];
    let mut offset = 0;
    let mut done = 0;
    while offset < n {
        let len = SEGMENT.min(n - offset);
        // Kısa son parça: iki yanı gerçek sesle (varsa) doldurulur, sonra ortası alınır.
        let delta = SEGMENT - len;
        let start = offset as isize - (delta / 2) as isize;
        let take = |c: usize| -> Vec<f32> {
            (0..SEGMENT as isize)
                .map(|i| {
                    let j = start + i;
                    if j >= 0 && (j as usize) < n {
                        mix[c][j as usize]
                    } else {
                        0.0
                    }
                })
                .collect()
        };
        let chunk = [take(0), take(1)];
        let (m, mag, frames) = inputs(&chunk, &plans);
        let feeds = ort::inputs![
            Tensor::from_array(([1usize, 2, SEGMENT], m)).map_err(|e| e.to_string())?,
            Tensor::from_array(([1usize, 4, BINS, frames], mag)).map_err(|e| e.to_string())?,
        ];
        let result = session
            .run(feeds)
            .map_err(|e| format!("Model çalışmadı: {e}"))?;
        let (_, out_x) = result[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| e.to_string())?;
        let (_, out_xt) = result[1]
            .try_extract_tensor::<f32>()
            .map_err(|e| e.to_string())?;
        let trim = delta / 2;
        for (k, &s) in wanted.iter().enumerate() {
            let sep = source(out_x, out_xt, s, frames, &plans);
            for c in 0..2 {
                for t in 0..len {
                    out[k][c][offset + t] += weight[t] * sep[c][trim + t];
                }
            }
        }
        for t in 0..len {
            sum_w[offset + t] += weight[t];
        }
        offset += stride;
        done += 1;
        if !progress(done, total) {
            return Err("Durduruldu".into());
        }
    }
    for st in out.iter_mut() {
        for ch in st.iter_mut() {
            ch.iter_mut()
                .zip(&sum_w)
                .for_each(|(x, w)| *x /= w.max(1e-8));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn triangle_weights() {
        let w = super::triangle(6);
        assert_eq!(w, vec![0.25, 0.5, 0.75, 1.0, 0.5, 0.25]);
    }
}
